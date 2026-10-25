//! Fixtures shared by the Jira and Confluence tests.
#![allow(dead_code)]

use chrono::TimeZone;
use serde_json::json;

use super::client::{Issue, JiraSite};

pub fn site(uri: &str) -> JiraSite {
    JiraSite {
        site_url: uri.into(),
        email: "me@acme.com".into(),
        account: "jira:project:p-1".into(),
    }
}

pub fn issue_json(key: &str) -> serde_json::Value {
    json!({"key": key, "fields": {"summary": "Add login", "status": {"name": "In Progress"}, "assignee": {"displayName": "Jane"},
        "issuetype": {"name": "Story"}, "updated": "2026-10-24T05:12:00.000+0000",
        "description": {"type":"doc","version":1,"content":[{"type":"paragraph","content":[{"type":"text","text":"Users sign in."}]}]}}})
}

pub fn sample_issue(key: &str, assignee: Option<&str>, description: &str) -> Issue {
    Issue {
        key: key.into(),
        summary: "Add login".into(),
        status: "In Progress".into(),
        assignee: assignee.map(Into::into),
        issue_type: "Story".into(),
        description: description.into(),
        url: format!("https://acme.atlassian.net/browse/{key}"),
        updated: chrono::Utc.with_ymd_and_hms(2026, 10, 24, 5, 12, 0).unwrap(),
    }
}
