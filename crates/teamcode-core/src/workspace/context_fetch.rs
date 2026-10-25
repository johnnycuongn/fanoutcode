//! Fetching a context doc's text on demand (`context_docs_fetch`), cached by the shell at `text_cache_path`.

use super::context_docs::{ContextDoc, ContextDocKind};
use crate::{Error, Result};

/// The most text a fetched reference keeps (prompts point at the cache file, so this only bounds disk use).
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

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
    use crate::jira::confluence::{html_to_text, page_id_from_url, site_of_url, ConfluenceClient};
    match doc.kind {
        ContextDocKind::File => {
            let bytes = std::fs::read(&doc.target)?;
            Ok(String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_TEXT_BYTES)]).into_owned())
        }
        ContextDocKind::Folder => folder_listing(std::path::Path::new(&doc.target)),
        ContextDocKind::Url => {
            let ok = url::Url::parse(&doc.target).is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
            if !ok {
                return Err(Error::Invalid("A link must start with http:// or https://".into()));
            }
            let resp = http
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
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| Error::Provider(format!("Fetching {}: {}", doc.target, e.without_url())))?;
            let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_TEXT_BYTES)]).into_owned();
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
}
