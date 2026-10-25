//! A small Jira Cloud REST v3 client: issues, enhanced JQL search, comments, transitions, and the credential check.

use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::adf::adf_to_text;
use crate::{Error, Result};

/// A linked Atlassian site. `account` names the keychain entry holding the API token (e.g. `jira:project:p-1`); the
/// token itself never leaves the keychain except into a request. Owned here (A19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JiraSite {
    pub site_url: String,
    pub email: String,
    pub account: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Issue {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: Option<String>,
    pub issue_type: String,
    /// Plain text from the ADF description.
    pub description: String,
    pub url: String,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Transition {
    pub id: String,
    pub name: String,
}

const FIELDS: &str = "summary,status,assignee,issuetype,description,updated";

pub struct JiraClient {
    http: reqwest::Client,
    base: String,
    email: String,
    token: String,
}

/// `scheme://host[:port]` of `site_url`: a pasted `/browse/KEY` or trailing slash is dropped.
pub fn normalise_site(site_url: &str) -> Result<String> {
    let invalid = || Error::Invalid("Jira site URL must start with http:// or https://".into());
    let u = url::Url::parse(site_url.trim()).map_err(|_| invalid())?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() {
        return Err(invalid());
    }
    Ok(u.origin().ascii_serialization())
}

/// `<site>/browse/<KEY>`.
pub fn issue_url(site_url: &str, key: &str) -> String {
    format!("{}/browse/{key}", site_url.trim_end_matches('/'))
}

/// `^[A-Z][A-Z0-9_]*-[0-9]+$`.
pub fn valid_issue_key(key: &str) -> bool {
    let Some((project, number)) = key.split_once('-') else {
        return false;
    };
    project.starts_with(|c: char| c.is_ascii_uppercase())
        && project
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

fn check_key(key: &str) -> Result<()> {
    if valid_issue_key(key) {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{key:?} is not a Jira issue key")))
    }
}

fn parse_updated(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3f%z")
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// A shared HTTP client: 15 s timeout, JSON accepted.
pub(crate) fn http_client() -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .default_headers(headers)
        .build()
        .map_err(|e| Error::Provider(format!("Jira: {e}")))
}

impl JiraClient {
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

    fn map_status(&self, key: Option<&str>, status: StatusCode) -> Error {
        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Error::Provider(format!(
                "Jira: authentication failed for {} at {} (check the API token)",
                self.email, self.base
            )),
            StatusCode::NOT_FOUND => Error::Precondition(match key {
                Some(k) => format!("Jira: {k} not found"),
                None => "Jira: not found".to_owned(),
            }),
            s => Error::Provider(format!("Jira: HTTP {}", s.as_u16())),
        }
    }

    async fn send(&self, req: reqwest::RequestBuilder, key: Option<&str>) -> Result<reqwest::Response> {
        let resp = req
            .basic_auth(&self.email, Some(&self.token))
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Jira: {}", e.without_url())))?;
        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(self.map_status(key, resp.status()))
        }
    }

    async fn json(&self, req: reqwest::RequestBuilder, key: Option<&str>) -> Result<Value> {
        self.send(req, key)
            .await?
            .json()
            .await
            .map_err(|e| Error::Provider(format!("Jira: {}", e.without_url())))
    }

    fn issue_from(&self, v: &Value) -> Issue {
        let f = &v["fields"];
        let s = |p: &Value| p.as_str().unwrap_or_default().to_owned();
        let key = s(&v["key"]);
        Issue {
            url: issue_url(&self.base, &key),
            key,
            summary: s(&f["summary"]),
            status: s(&f["status"]["name"]),
            assignee: f["assignee"]["displayName"].as_str().map(str::to_owned),
            issue_type: s(&f["issuetype"]["name"]),
            description: adf_to_text(&f["description"]),
            updated: parse_updated(f["updated"].as_str().unwrap_or_default()),
        }
    }

    pub async fn issue(&self, key: &str) -> Result<Issue> {
        check_key(key)?;
        let url = format!("{}/rest/api/3/issue/{key}", self.base);
        let v = self
            .json(self.http.get(url).query(&[("fields", FIELDS)]), Some(key))
            .await?;
        Ok(self.issue_from(&v))
    }

    pub async fn search(&self, jql: &str, max_results: u32) -> Result<Vec<Issue>> {
        let url = format!("{}/rest/api/3/search/jql", self.base);
        let max = max_results.to_string();
        let v = self
            .json(
                self.http
                    .get(url)
                    .query(&[("jql", jql), ("maxResults", &max), ("fields", FIELDS)]),
                None,
            )
            .await?;
        Ok(v["issues"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|i| self.issue_from(i))
            .collect())
    }

    /// Posts `body` as an ADF document with one paragraph per line.
    pub async fn comment(&self, key: &str, body: &str) -> Result<()> {
        check_key(key)?;
        let content: Vec<Value> = body
            .lines()
            .map(|l| {
                if l.is_empty() {
                    json!({"type": "paragraph", "content": []})
                } else {
                    json!({"type": "paragraph", "content": [{"type": "text", "text": l}]})
                }
            })
            .collect();
        let url = format!("{}/rest/api/3/issue/{key}/comment", self.base);
        let doc = json!({"body": {"type": "doc", "version": 1, "content": content}});
        self.send(self.http.post(url).json(&doc), Some(key)).await.map(|_| ())
    }

    pub async fn transitions(&self, key: &str) -> Result<Vec<Transition>> {
        check_key(key)?;
        let url = format!("{}/rest/api/3/issue/{key}/transitions", self.base);
        let v = self.json(self.http.get(url), Some(key)).await?;
        Ok(v["transitions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|t| Transition {
                id: t["id"].as_str().unwrap_or_default().to_owned(),
                name: t["name"].as_str().unwrap_or_default().to_owned(),
            })
            .collect())
    }

    pub async fn transition(&self, key: &str, transition_id: &str) -> Result<()> {
        check_key(key)?;
        let url = format!("{}/rest/api/3/issue/{key}/transitions", self.base);
        let body = json!({"transition": {"id": transition_id}});
        self.send(self.http.post(url).json(&body), Some(key)).await.map(|_| ())
    }

    /// The linked account's display name: checks the site, email and token together.
    pub async fn verify(&self) -> Result<String> {
        let v = self
            .json(self.http.get(format!("{}/rest/api/3/myself", self.base)), None)
            .await?;
        Ok(v["displayName"].as_str().unwrap_or_default().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::test_support::{issue_json, site};
    use serde_json::json;
    use wiremock::matchers::{body_json, body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn issue_parses_fields_and_sends_basic_auth() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-1"))
            .and(header("Authorization", "Basic bWVAYWNtZS5jb206dG9r"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_json("PROJ-1")))
            .expect(1)
            .mount(&server)
            .await;
        let i = JiraClient::new(&site(&server.uri()), "tok")
            .unwrap()
            .issue("PROJ-1")
            .await
            .unwrap();
        assert_eq!(
            (
                i.summary.as_str(),
                i.status.as_str(),
                i.assignee.as_deref(),
                i.issue_type.as_str(),
                i.description.as_str()
            ),
            ("Add login", "In Progress", Some("Jane"), "Story", "Users sign in.")
        );
        assert_eq!(i.url, format!("{}/browse/PROJ-1", server.uri()));
    }

    #[tokio::test]
    async fn not_found_is_precondition_and_unauthorised_is_provider() {
        let server = MockServer::start().await;
        Mock::given(path("/rest/api/3/issue/PROJ-404"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(path("/rest/api/3/issue/PROJ-401"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let c = JiraClient::new(&site(&server.uri()), "tok").unwrap();
        assert!(matches!(c.issue("PROJ-404").await, Err(Error::Precondition(m)) if m.contains("PROJ-404")));
        assert!(matches!(c.issue("PROJ-401").await, Err(Error::Provider(m)) if m.contains("API token")));
        assert!(matches!(c.issue("../x").await, Err(Error::Invalid(_))));
    }

    #[tokio::test]
    async fn search_uses_enhanced_jql_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/3/search/jql"))
            .and(query_param("jql", "text ~ \"login*\""))
            .and(query_param("maxResults", "20"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"issues": [issue_json("PROJ-1"), issue_json("PROJ-2")], "isLast": true})),
            )
            .mount(&server)
            .await;
        assert_eq!(
            JiraClient::new(&site(&server.uri()), "tok")
                .unwrap()
                .search("text ~ \"login*\"", 20)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn comment_posts_one_adf_paragraph_per_line() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue/PROJ-1/comment"))
            .and(body_partial_json(json!({"body": {"type": "doc", "version": 1}})))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;
        JiraClient::new(&site(&server.uri()), "tok")
            .unwrap()
            .comment("PROJ-1", "line one\nline two")
            .await
            .unwrap();
        let req = &server.received_requests().await.unwrap()[0];
        assert_eq!(
            req.body_json::<serde_json::Value>().unwrap()["body"]["content"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn transitions_list_and_apply() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-1/transitions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"transitions": [{"id": "31", "name": "In Review"}]})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/issue/PROJ-1/transitions"))
            .and(body_json(json!({"transition": {"id": "31"}})))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let c = JiraClient::new(&site(&server.uri()), "tok").unwrap();
        assert_eq!(
            c.transitions("PROJ-1").await.unwrap(),
            vec![Transition {
                id: "31".into(),
                name: "In Review".into()
            }]
        );
        c.transition("PROJ-1", "31").await.unwrap();
    }

    #[tokio::test]
    async fn verify_returns_the_display_name() {
        let server = MockServer::start().await;
        Mock::given(path("/rest/api/3/myself"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"displayName": "Jane"})))
            .mount(&server)
            .await;
        assert_eq!(
            JiraClient::new(&site(&server.uri()), "tok")
                .unwrap()
                .verify()
                .await
                .unwrap(),
            "Jane"
        );
    }

    #[test]
    fn new_normalises_trailing_slash_and_browse_path() {
        let c = JiraClient::new(&site("https://acme.atlassian.net/browse/PROJ-1/"), "t").unwrap();
        assert_eq!(c.base_url(), "https://acme.atlassian.net");
        assert!(matches!(
            JiraClient::new(&site("acme.atlassian.net"), "t"),
            Err(Error::Invalid(_))
        ));
        assert!(valid_issue_key("PROJ-12") && !valid_issue_key("proj-12") && !valid_issue_key("PROJ"));
    }
}
