//! Fetching a context doc's text on demand (`context_docs_fetch`), cached by the shell at `text_cache_path`.

use super::context_docs::{ContextDoc, ContextDocKind};
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
/// read with the linked Atlassian site. The token is sent only to that site's own host.
pub async fn fetch_text(
    doc: &ContextDoc,
    atlassian: Option<(&crate::jira::client::JiraSite, &str)>,
    http: &reqwest::Client,
) -> Result<String> {
    fetch_text_capped(doc, atlassian, http, MAX_TEXT_BYTES).await
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
    atlassian: Option<(&crate::jira::client::JiraSite, &str)>,
    http: &reqwest::Client,
    cap: usize,
) -> Result<String> {
    use crate::jira::confluence::{html_to_text, page_id_from_url, site_of_url, ConfluenceClient};
    match doc.kind {
        ContextDocKind::File => {
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(&doc.target)?
                .take(cap as u64 + 1)
                .read_to_end(&mut bytes)?;
            Ok(capped_text(bytes, cap))
        }
        ContextDocKind::Folder => folder_listing(std::path::Path::new(&doc.target)),
        ContextDocKind::Url => {
            let ok = url::Url::parse(&doc.target).is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
            if !ok {
                return Err(Error::Invalid("A link must start with http:// or https://".into()));
            }
            let mut resp = http
                .get(&doc.target)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| Error::Provider(format!("Fetching {}: {}", doc.target, e.without_url())))?;
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
            let client = ConfluenceClient::new(site, token)?;
            if client.base_url() != host {
                return Err(no_site());
            }
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

    fn doc(id: &str, kind: ContextDocKind, target: &str, fetched: Option<chrono::DateTime<chrono::Utc>>) -> ContextDoc {
        ContextDoc {
            id: id.into(),
            kind,
            target: target.into(),
            label: super::super::context_docs::label_for(&kind, target),
            page_id: None,
            fetched_at: fetched,
            order: 0,
        }
    }

    #[tokio::test]
    async fn fetch_text_reads_files_lists_folders_and_requires_site_for_confluence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "alpha").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.md"), "b").unwrap();
        std::fs::write(dir.path().join(".hidden"), "h").unwrap();
        let http = reqwest::Client::new();
        assert_eq!(
            fetch_text(
                &doc(
                    "doc-1",
                    ContextDocKind::File,
                    dir.path().join("a.md").to_str().unwrap(),
                    None
                ),
                None,
                &http
            )
            .await
            .unwrap(),
            "alpha"
        );
        assert_eq!(
            fetch_text(
                &doc("doc-2", ContextDocKind::Folder, dir.path().to_str().unwrap(), None),
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
            matches!(fetch_text(&conf, None, &http).await, Err(Error::Precondition(m)) if m.contains("link Jira first"))
        );
        let other = crate::jira::test_support::site("https://other.atlassian.net");
        assert!(
            matches!(
                fetch_text(&conf, Some((&other, "tok")), &http).await,
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
        let http = reqwest::Client::new();
        assert_eq!(
            fetch_text(
                &doc("doc-4", ContextDocKind::Url, &format!("{}/spec", server.uri()), None),
                None,
                &http
            )
            .await
            .unwrap(),
            "Spec > v2"
        );
        assert_eq!(
            fetch_text(
                &doc("doc-5", ContextDocKind::Url, &format!("{}/raw", server.uri()), None),
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
        (format!("http://{addr}/big"), written)
    }

    #[tokio::test]
    async fn a_body_over_the_cap_is_truncated() {
        let (url, _written) = chunked_server(vec![b'x'; 64 * 1024]).await;
        let text = fetch_text_capped(
            &doc("doc-6", ContextDocKind::Url, &url, None),
            None,
            &reqwest::Client::new(),
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
        let d = doc("doc-7", ContextDocKind::Url, &format!("{}/huge", server.uri()), None);
        assert!(matches!(
            fetch_text_capped(&d, None, &reqwest::Client::new(), 4096).await,
            Err(Error::Precondition(m)) if m.contains("larger than")
        ));
    }

    #[tokio::test]
    async fn a_large_file_is_read_only_to_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.txt");
        std::fs::write(&p, "é".repeat(5000)).unwrap(); // 10 000 bytes, two per character
        let text = fetch_text_capped(
            &doc("doc-8", ContextDocKind::File, p.to_str().unwrap(), None),
            None,
            &reqwest::Client::new(),
            4095,
        )
        .await
        .unwrap();
        assert!(text.ends_with(TRUNCATED_NOTE) && !text.contains('\u{FFFD}'));
        assert_eq!(text.trim_end_matches(TRUNCATED_NOTE).chars().count(), 2047);
    }
}
