//! Fetching a context doc's text on demand (`context_docs_fetch`), cached by the shell at `text_cache_path`.

use super::context_docs::{readable_path, ContextDoc, ContextDocKind};
use super::url_guard::{client_for, client_plan, guarded_get, validate_hop, NetGuard};
use crate::{Error, Result};

/// The most text a fetched reference keeps (prompts point at the cache file, so this only bounds disk use).
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// Appended when a reference was cut at the cap.
pub const TRUNCATED_NOTE: &str = "\n\n[Fanout Code: truncated at the 2 MB limit]";

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
#[path = "context_fetch_tests.rs"]
mod tests;
