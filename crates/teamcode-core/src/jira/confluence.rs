//! Confluence Cloud pages for context docs, read with the Atlassian credentials linked for Jira (same site, same
//! token).

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::client::{http_client, normalise_site, JiraSite};
use crate::{Error, Result};

/// The numeric page id in `/wiki/spaces/<KEY>/pages/<id>/…` or `/wiki/pages/viewpage.action?pageId=<id>`.
pub fn page_id_from_url(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let numeric = |s: &str| (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())).then(|| s.to_owned());
    let segs: Vec<&str> = u.path_segments()?.collect();
    if let Some(i) = segs.iter().position(|s| *s == "pages") {
        if segs.get(i + 1) == Some(&"viewpage.action") {
            return u
                .query_pairs()
                .find(|(k, _)| k == "pageId")
                .and_then(|(_, v)| numeric(&v));
        }
        if segs.first() == Some(&"wiki") && segs.get(1) == Some(&"spaces") {
            return segs.get(i + 1).and_then(|s| numeric(s));
        }
    }
    None
}

/// `scheme://host[:port]` of a URL.
pub fn site_of_url(url: &str) -> Option<String> {
    normalise_site(url).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ConfluencePage {
    pub id: String,
    pub title: String,
    pub text: String,
}

pub struct ConfluenceClient {
    http: reqwest::Client,
    base: String,
    email: String,
    token: String,
}

impl ConfluenceClient {
    pub fn new(site: &JiraSite, token: &str) -> Result<Self> {
        Ok(Self {
            http: http_client()?,
            base: normalise_site(&site.site_url)?,
            email: site.email.clone(),
            token: token.to_owned(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub async fn page(&self, page_id: &str) -> Result<ConfluencePage> {
        if page_id.is_empty() || !page_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::Invalid("Not a Confluence page id".into()));
        }
        let resp = self
            .http
            .get(format!("{}/wiki/api/v2/pages/{page_id}", self.base))
            .query(&[("body-format", "storage")])
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Confluence: {}", e.without_url())))?;
        match resp.status() {
            s if s.is_success() => {}
            StatusCode::NOT_FOUND => return Err(Error::Precondition(format!("Confluence: page {page_id} not found"))),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(Error::Provider(format!(
                    "Confluence: authentication failed for {} at {} (check the API token)",
                    self.email, self.base
                )))
            }
            s => return Err(Error::Provider(format!("Confluence: HTTP {}", s.as_u16()))),
        }
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Confluence: {}", e.without_url())))?;
        Ok(ConfluencePage {
            id: v["id"].as_str().unwrap_or(page_id).to_owned(),
            title: v["title"].as_str().unwrap_or_default().to_owned(),
            text: html_to_text(v["body"]["storage"]["value"].as_str().unwrap_or_default()),
        })
    }
}

const BLOCK: &[&str] = &[
    "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6", "tr", "pre", "table", "ul", "ol",
];

fn decode_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "#39" | "apos" => '\'',
        "nbsp" => ' ',
        n if n.starts_with("#x") || n.starts_with("#X") => char::from_u32(u32::from_str_radix(&n[2..], 16).ok()?)?,
        n if n.starts_with('#') => char::from_u32(n[1..].parse().ok()?)?,
        _ => return None,
    })
}

/// Block tags become line breaks (`li` a `- ` line), other tags are dropped, `script` and `style` bodies skipped,
/// common entities decoded, runs of spaces collapsed, three or more newlines collapsed to two, trimmed.
pub fn html_to_text(html: &str) -> String {
    let mut raw = String::with_capacity(html.len());
    let mut rest = html;
    let mut skip_until: Option<&str> = None;
    while let Some(c) = rest.chars().next() {
        if c == '<' {
            // `find` results are char boundaries; an unterminated `<tag` takes the rest of the text
            let (tag, end) = match rest.find('>') {
                Some(e) => (rest[1..e].trim(), e + 1),
                None => (rest[1..].trim(), rest.len()),
            };
            let closing = tag.starts_with('/');
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            rest = &rest[end..];
            if let Some(until) = skip_until {
                if closing && name == until {
                    skip_until = None;
                }
                continue;
            }
            match name.as_str() {
                "script" | "style" if !closing => skip_until = Some(if name == "script" { "script" } else { "style" }),
                "li" if !closing => raw.push_str("\n- "),
                n if BLOCK.contains(&n) => raw.push('\n'),
                _ => {}
            }
            continue;
        }
        if skip_until.is_some() {
            rest = &rest[c.len_utf8()..];
            continue;
        }
        if c == '&' {
            // a bounded byte search: `;` is ASCII, so a hit is a char boundary, and a page of `&` stays linear
            if let Some(semi) = rest.as_bytes().iter().take(13).position(|&b| b == b';') {
                if let Some(ch) = decode_entity(&rest[1..semi]) {
                    raw.push(ch);
                    rest = &rest[semi + 1..];
                    continue;
                }
            }
        }
        raw.push(c);
        rest = &rest[c.len_utf8()..];
    }
    let lines: Vec<String> = raw
        .split('\n')
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let mut out = String::new();
    let mut blank = 0;
    for l in lines {
        if l.is_empty() {
            blank += 1;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank > 0 { "\n\n" } else { "\n" });
        }
        blank = 0;
        out.push_str(&l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::test_support::site;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn page_id_from_space_and_viewpage_urls() {
        assert_eq!(
            page_id_from_url("https://acme.atlassian.net/wiki/spaces/ENG/pages/123456/Payments+design").as_deref(),
            Some("123456")
        );
        assert_eq!(
            page_id_from_url("https://acme.atlassian.net/wiki/pages/viewpage.action?pageId=777").as_deref(),
            Some("777")
        );
        assert_eq!(page_id_from_url("https://acme.atlassian.net/wiki/home"), None);
        assert_eq!(
            page_id_from_url("https://acme.atlassian.net/wiki/spaces/ENG/pages/12a/x"),
            None
        );
        assert_eq!(
            site_of_url("https://acme.atlassian.net/wiki/x").as_deref(),
            Some("https://acme.atlassian.net")
        );
    }

    #[test]
    fn html_to_text_decodes_entities_and_breaks_blocks() {
        assert_eq!(
            html_to_text("<h1>Title</h1><p>A &amp; B&nbsp;<b>bold</b></p><ul><li>one</li><li>two</li></ul>"),
            "Title\n\nA & B bold\n\n- one\n- two"
        );
        assert_eq!(
            html_to_text("<script>alert(1)</script><p>x &#65;&#x42; &lt;y&gt;</p>"),
            "x AB <y>"
        );
    }

    #[tokio::test]
    async fn page_fetches_storage_body_as_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/wiki/api/v2/pages/123"))
            .and(query_param("body-format", "storage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"id": "123", "title": "Pay", "body": {"storage": {"value": "<p>Hello <em>there</em></p>", "representation": "storage"}}}),
            ))
            .mount(&server)
            .await;
        let p = ConfluenceClient::new(&site(&server.uri()), "tok")
            .unwrap()
            .page("123")
            .await
            .unwrap();
        assert_eq!((p.title.as_str(), p.text.as_str()), ("Pay", "Hello there"));
    }

    #[tokio::test]
    async fn page_404_is_precondition() {
        let server = MockServer::start().await;
        Mock::given(path("/wiki/api/v2/pages/9"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        assert!(matches!(
            ConfluenceClient::new(&site(&server.uri()), "tok")
                .unwrap()
                .page("9")
                .await,
            Err(Error::Precondition(_))
        ));
    }

    #[test]
    fn html_to_text_never_splits_a_character() {
        assert_eq!(html_to_text("<p>Café crème</p>"), "Café crème");
        assert_eq!(html_to_text("a &😀 b; c"), "a &😀 b; c");
        assert_eq!(
            html_to_text("<h1>設計</h1><p>決済の流れ &amp; 返金</p>"),
            "設計\n\n決済の流れ & 返金"
        );
        assert_eq!(html_to_text("text <unterminated é"), "text");
        assert_eq!(html_to_text("&é;"), "&é;");
    }

    #[test]
    fn many_ampersands_without_semicolons_stay_linear() {
        let page = "&".repeat(1_000_000);
        let start = std::time::Instant::now();
        assert_eq!(html_to_text(&page).len(), 1_000_000);
        assert!(
            start.elapsed() < std::time::Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }
}
