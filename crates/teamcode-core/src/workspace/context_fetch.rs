//! Fetching a context doc's text on demand (`context_docs_fetch`), cached by the shell at `text_cache_path`.

use super::context_docs::{readable_path, ContextDoc, ContextDocKind};
use super::url_guard::{client_for, client_plan, guarded_get, validate_hop, NetGuard};
use crate::{Error, Result};

/// The most text a fetched reference keeps (prompts point at the cache file, so this only bounds disk use).
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// Appended when a reference was cut at the cap.
pub const TRUNCATED_NOTE: &str = "\n\n[Team Code: truncated at the 2 MB limit]";

/// One relative path per line for a folder: files to depth two, at most 200, sorted, hidden entries skipped and
/// symlinks not followed.
fn folder_listing(root: &std::path::Path) -> Result<String> {
    fn walk(dir: &std::path::Path, rel: &std::path::Path, depth: usize, out: &mut Vec<String>) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let ft = e.file_type()?;
            let path = rel.join(&name);
            if ft.is_file() {
                out.push(path.to_string_lossy().into_owned());
            } else if ft.is_dir() && depth < 2 {
                walk(&e.path(), &path, depth + 1, out)?;
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, std::path::Path::new(""), 1, &mut out)?;
    out.sort();
    out.truncate(200);
    Ok(out.join("\n"))
}

/// The reference's text now: a file's contents, a folder's listing, a URL's body (HTML stripped), a Confluence page
/// read with the linked Atlassian site. The token is sent only to that site's own host. Every network hop passes
/// `url_guard` (never local or link-local; private networks only for a trusted doc) and is pinned to the checked
/// address.
///
/// A file or folder is read only when the doc is trusted (added in the app, or confirmed in an import preview) and
/// its path still resolves inside one of `allowed_roots` (the container's project roots) or to exactly the path
/// that was confirmed; a symlink retargeted elsewhere is refused.
pub async fn fetch_text(
    doc: &ContextDoc,
    allowed_roots: &[std::path::PathBuf],
    atlassian: Option<(&crate::jira::client::JiraSite, &str)>,
    guard: &NetGuard<'_>,
) -> Result<String> {
    fetch_text_capped(doc, allowed_roots, atlassian, guard, MAX_TEXT_BYTES).await
}

/// `bytes` as text, cut at `cap` (on a character boundary) with `TRUNCATED_NOTE` when longer.
fn capped_text(mut bytes: Vec<u8>, cap: usize) -> String {
    if bytes.len() <= cap {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    bytes.truncate(cap);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if text.ends_with('\u{FFFD}') {
        text.pop(); // a character split by the cut
    }
    text.push_str(TRUNCATED_NOTE);
    text
}

/// As `fetch_text` with an explicit byte cap.
pub(crate) async fn fetch_text_capped(
    doc: &ContextDoc,
    allowed_roots: &[std::path::PathBuf],
    atlassian: Option<(&crate::jira::client::JiraSite, &str)>,
    guard: &NetGuard<'_>,
    cap: usize,
) -> Result<String> {
    use crate::jira::confluence::{html_to_text, page_id_from_url, site_of_url, ConfluenceClient};
    match doc.kind {
        ContextDocKind::File => {
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(readable_path(doc, allowed_roots)?)?
                .take(cap as u64 + 1)
                .read_to_end(&mut bytes)?;
            Ok(capped_text(bytes, cap))
        }
        ContextDocKind::Folder => folder_listing(&readable_path(doc, allowed_roots)?),
        ContextDocKind::Url => {
            let mut resp = guarded_get(&doc.target, doc.trusted, guard).await?;
            if !resp.status().is_success() {
                return Err(Error::Provider(format!(
                    "Fetching {}: HTTP {}",
                    doc.target,
                    resp.status().as_u16()
                )));
            }
            let html = resp
                .headers()
                .get_all(reqwest::header::CONTENT_TYPE)
                .iter()
                .any(|v| v.to_str().is_ok_and(|v| v.contains("html")));
            if resp.content_length().is_some_and(|n| n > cap as u64) {
                return Err(Error::Precondition(format!(
                    "{} is larger than the {} MB a reference may hold.",
                    doc.target,
                    cap / (1024 * 1024)
                )));
            }
            // stream, and stop reading one byte past the cap
            let mut bytes = Vec::new();
            while let Some(chunk) = resp
                .chunk()
                .await
                .map_err(|e| Error::Provider(format!("Fetching {}: {}", doc.target, e.without_url())))?
            {
                bytes.extend_from_slice(&chunk);
                if bytes.len() > cap {
                    break;
                }
            }
            let text = capped_text(bytes, cap);
            Ok(if html { html_to_text(&text) } else { text })
        }
        ContextDocKind::Confluence => {
            let host = site_of_url(&doc.target).unwrap_or_default();
            let no_site = || Error::Precondition(format!("No Atlassian site linked for {host}; link Jira first"));
            let (site, token) = atlassian.ok_or_else(no_site)?;
            let base = crate::jira::client::normalise_site(&site.site_url)?;
            if base != host {
                return Err(no_site());
            }
            let base_url = url::Url::parse(&base).map_err(|_| no_site())?;
            let (pin_host, addr) = validate_hop(&base_url, doc.trusted, guard)?;
            let plan = client_plan(doc.trusted, addr);
            let client = ConfluenceClient::with_http(site, token, client_for(&pin_host, plan)?)?;
            let page_id = doc
                .page_id
                .clone()
                .or_else(|| page_id_from_url(&doc.target))
                .ok_or_else(|| Error::Invalid("Not a Confluence page URL".into()))?;
            let page = client.page(&page_id).await?;
            Ok(format!("# {}\n\n{}", page.title, page.text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    struct MapResolver(Vec<(&'static str, std::net::IpAddr)>, u16);

    impl crate::workspace::url_guard::Resolve for MapResolver {
        fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<std::net::SocketAddr>> {
            let p = if self.1 == 0 { port } else { self.1 };
            Ok(self
                .0
                .iter()
                .filter(|(h, _)| *h == host)
                .map(|(_, ip)| std::net::SocketAddr::new(*ip, p))
                .collect())
        }
    }

    /// `mock.test` stands for the local mock server (exempt from the loopback refusal); nothing else is.
    fn mock_guard(r: &MapResolver) -> NetGuard<'_> {
        NetGuard {
            resolver: r,
            exempt_host: Some("mock.test".into()),
        }
    }

    fn local(port: u16) -> MapResolver {
        MapResolver(
            vec![
                ("mock.test", "127.0.0.1".parse().unwrap()),
                ("evil.test", "127.0.0.1".parse().unwrap()),
            ],
            port,
        )
    }

    fn port_of(uri: &str) -> u16 {
        url::Url::parse(uri).unwrap().port().unwrap()
    }

    fn trusted(mut d: ContextDoc) -> ContextDoc {
        d.trusted = true;
        d
    }

    fn doc(id: &str, kind: ContextDocKind, target: &str, fetched: Option<chrono::DateTime<chrono::Utc>>) -> ContextDoc {
        ContextDoc {
            id: id.into(),
            kind,
            target: target.into(),
            label: super::super::context_docs::label_for(&kind, target),
            page_id: None,
            fetched_at: fetched,
            order: 0,
            trusted: false,
        }
    }

    #[tokio::test]
    async fn fetch_text_reads_files_lists_folders_and_requires_site_for_confluence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "alpha").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.md"), "b").unwrap();
        std::fs::write(dir.path().join(".hidden"), "h").unwrap();
        let roots = vec![dir.path().to_path_buf()];
        let r = local(0);
        let http = mock_guard(&r);
        assert_eq!(
            fetch_text(
                &trusted(doc(
                    "doc-1",
                    ContextDocKind::File,
                    dir.path().join("a.md").to_str().unwrap(),
                    None
                )),
                &roots,
                None,
                &http
            )
            .await
            .unwrap(),
            "alpha"
        );
        assert_eq!(
            fetch_text(
                &trusted(doc("doc-2", ContextDocKind::Folder, dir.path().to_str().unwrap(), None)),
                &roots,
                None,
                &http
            )
            .await
            .unwrap(),
            "a.md\nsub/b.md"
        );
        let conf = doc(
            "doc-3",
            ContextDocKind::Confluence,
            "https://acme.atlassian.net/wiki/spaces/E/pages/1/X",
            None,
        );
        assert!(
            matches!(fetch_text(&conf, &[], None, &http).await, Err(Error::Precondition(m)) if m.contains("link Jira first"))
        );
        let other = crate::jira::test_support::site("https://other.atlassian.net");
        assert!(
            matches!(
                fetch_text(&conf, &[], Some((&other, "tok")), &http).await,
                Err(Error::Precondition(_))
            ),
            "the token goes only to its own site"
        );
    }

    #[tokio::test]
    async fn fetch_text_strips_html_from_urls() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/spec"))
            .respond_with(
                ResponseTemplate::new(200)
                    // set_body_string would force text/plain over an inserted content-type
                    .set_body_raw("<html><body><p>Spec &gt; v2</p></body></html>", "text/html"),
            )
            .mount(&server)
            .await;
        Mock::given(path("/raw"))
            .respond_with(ResponseTemplate::new(200).set_body_string("a <b>"))
            .mount(&server)
            .await;
        let port = port_of(&server.uri());
        let r = local(port);
        let http = mock_guard(&r);
        assert_eq!(
            fetch_text(
                &doc(
                    "doc-4",
                    ContextDocKind::Url,
                    &format!("http://mock.test:{port}/spec"),
                    None
                ),
                &[],
                None,
                &http
            )
            .await
            .unwrap(),
            "Spec > v2"
        );
        assert_eq!(
            fetch_text(
                &doc(
                    "doc-5",
                    ContextDocKind::Url,
                    &format!("http://mock.test:{port}/raw"),
                    None
                ),
                &[],
                None,
                &http
            )
            .await
            .unwrap(),
            "a <b>"
        );
    }

    /// A one-shot HTTP server that sends `body` chunked (no Content-Length) and records how much it wrote.
    async fn chunked_server(body: Vec<u8>) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let written = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let w = written.clone();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\n\r\n")
                .await;
            for chunk in body.chunks(1024) {
                if sock
                    .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                    .await
                    .is_err()
                    || sock.write_all(chunk).await.is_err()
                    || sock.write_all(b"\r\n").await.is_err()
                {
                    return;
                }
                w.fetch_add(chunk.len(), std::sync::atomic::Ordering::SeqCst);
            }
            let _ = sock.write_all(b"0\r\n\r\n").await;
        });
        (format!("http://mock.test:{}/big", addr.port()), written)
    }

    #[tokio::test]
    async fn a_body_over_the_cap_is_truncated() {
        let (url, _written) = chunked_server(vec![b'x'; 64 * 1024]).await;
        let r = local(port_of(&url));
        let text = fetch_text_capped(
            &doc("doc-6", ContextDocKind::Url, &url, None),
            &[],
            None,
            &mock_guard(&r),
            4096,
        )
        .await
        .unwrap();
        assert!(text.ends_with(TRUNCATED_NOTE));
        assert_eq!(text.len(), 4096 + TRUNCATED_NOTE.len());
    }

    #[tokio::test]
    async fn an_oversized_content_length_is_refused() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/huge"))
            .respond_with(ResponseTemplate::new(200).set_body_string("y".repeat(10_000)))
            .mount(&server)
            .await;
        let port = port_of(&server.uri());
        let r = local(port);
        let d = doc(
            "doc-7",
            ContextDocKind::Url,
            &format!("http://mock.test:{port}/huge"),
            None,
        );
        assert!(matches!(
            fetch_text_capped(&d, &[], None, &mock_guard(&r), 4096).await,
            Err(Error::Precondition(m)) if m.contains("larger than")
        ));
    }

    #[tokio::test]
    async fn a_large_file_is_read_only_to_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.txt");
        std::fs::write(&p, "é".repeat(5000)).unwrap(); // 10 000 bytes, two per character
        let text = fetch_text_capped(
            &trusted(doc("doc-8", ContextDocKind::File, p.to_str().unwrap(), None)),
            &[dir.path().to_path_buf()],
            None,
            &NetGuard::system(),
            4095,
        )
        .await
        .unwrap();
        assert!(text.ends_with(TRUNCATED_NOTE) && !text.contains('\u{FFFD}'));
        assert_eq!(text.trim_end_matches(TRUNCATED_NOTE).chars().count(), 2047);
    }

    #[tokio::test]
    async fn local_metadata_and_untrusted_private_hosts_are_refused() {
        let r = MapResolver(
            vec![
                ("loop.test", "127.0.0.1".parse().unwrap()),
                ("meta.test", "169.254.169.254".parse().unwrap()),
                ("wiki.corp", "10.0.0.5".parse().unwrap()),
                ("mapped.test", "::ffff:127.0.0.1".parse().unwrap()),
            ],
            0,
        );
        let guard = NetGuard {
            resolver: &r,
            exempt_host: None,
        };
        for url in [
            "http://loop.test/x",
            "http://meta.test/latest/meta-data",
            "http://127.0.0.1:9/x",
            "http://169.254.169.254/",
            "http://mapped.test/",
        ] {
            let mut d = doc("doc-9", ContextDocKind::Url, url, None);
            d.trusted = true;
            assert!(
                matches!(fetch_text(&d, &[], None, &guard).await, Err(Error::Precondition(m)) if m.contains("local")),
                "{url}"
            );
        }
        let untrusted = doc("doc-9", ContextDocKind::Url, "http://wiki.corp/page", None);
        assert!(
            matches!(fetch_text(&untrusted, &[], None, &guard).await, Err(Error::Precondition(m)) if m.contains("confirm"))
        );
        let wiki = url::Url::parse("http://wiki.corp/page").unwrap();
        let (host, addr) = crate::workspace::url_guard::validate_hop(&wiki, true, &guard).unwrap();
        assert_eq!(
            (host.as_str(), addr.map(|a| a.ip().to_string())),
            ("wiki.corp", Some("10.0.0.5".into())),
            "trusted: allowed and pinned"
        );
    }

    #[tokio::test]
    async fn a_redirect_to_a_local_address_is_refused() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let port = port_of(&server.uri());
        Mock::given(path("/go"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("http://evil.test:{port}/secret").as_str()),
            )
            .mount(&server)
            .await;
        Mock::given(path("/secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("secret"))
            .mount(&server)
            .await;
        let r = local(port);
        let mut d = doc(
            "doc-9",
            ContextDocKind::Url,
            &format!("http://mock.test:{port}/go"),
            None,
        );
        d.trusted = true;
        assert!(
            matches!(fetch_text(&d, &[], None, &mock_guard(&r)).await, Err(Error::Precondition(m)) if m.contains("local"))
        );
    }

    #[tokio::test]
    async fn file_references_need_trust_and_must_stay_inside_the_projects() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("id_rsa");
        std::fs::write(&secret, "PRIVATE KEY").unwrap();
        std::fs::write(root.path().join("notes.md"), "notes").unwrap();
        let roots = vec![root.path().to_path_buf()];
        let g = NetGuard::system();
        let canon = |p: &std::path::Path| std::fs::canonicalize(p).unwrap().to_string_lossy().into_owned();
        // imported from a file and not confirmed: refused
        let imported = doc("doc-1", ContextDocKind::File, &canon(&secret), None);
        assert!(
            matches!(fetch_text(&imported, &roots, None, &g).await, Err(Error::Precondition(m)) if m.contains("confirm"))
        );
        // confirmed (trusted) at its canonical path: read
        assert_eq!(
            fetch_text(&trusted(imported), &roots, None, &g).await.unwrap(),
            "PRIVATE KEY"
        );
        // a symlink inside a root that points outside: refused even when trusted
        let link = root.path().join("innocent.md");
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        let linked = trusted(doc("doc-2", ContextDocKind::File, link.to_str().unwrap(), None));
        assert!(
            matches!(fetch_text(&linked, &roots, None, &g).await, Err(Error::Precondition(m)) if m.contains("outside"))
        );
        // a normal doc inside a root: read
        let inside = trusted(doc(
            "doc-3",
            ContextDocKind::File,
            &canon(&root.path().join("notes.md")),
            None,
        ));
        assert_eq!(fetch_text(&inside, &roots, None, &g).await.unwrap(), "notes");
    }

    #[tokio::test]
    async fn trust_does_not_follow_a_redirect_to_another_private_host() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let port = port_of(&server.uri());
        Mock::given(path("/away"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "http://other.corp/secret"))
            .mount(&server)
            .await;
        Mock::given(path("/here"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/page"))
            .mount(&server)
            .await;
        Mock::given(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_string("same host"))
            .mount(&server)
            .await;
        let r = MapResolver(
            vec![
                ("mock.test", "127.0.0.1".parse().unwrap()),
                ("other.corp", "10.0.0.9".parse().unwrap()),
            ],
            0,
        );
        let r = MapResolver(r.0, port);
        let away = trusted(doc(
            "doc-1",
            ContextDocKind::Url,
            &format!("http://mock.test:{port}/away"),
            None,
        ));
        assert!(
            matches!(fetch_text(&away, &[], None, &mock_guard(&r)).await, Err(Error::Precondition(m)) if m.contains("private"))
        );
        let here = trusted(doc(
            "doc-2",
            ContextDocKind::Url,
            &format!("http://mock.test:{port}/here"),
            None,
        ));
        assert_eq!(
            fetch_text(&here, &[], None, &mock_guard(&r)).await.unwrap(),
            "same host"
        );
    }
}
