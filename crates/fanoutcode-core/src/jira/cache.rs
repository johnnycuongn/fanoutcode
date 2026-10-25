//! The per-project issue cache in `jira_links`, so the Ticket views show something at once.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::client::Issue;
use crate::db::{parse_time, ts, Db};
use crate::domain::ProjectId;
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CachedIssue {
    pub issue: Issue,
    pub fetched_at: DateTime<Utc>,
}

pub struct IssueCache {
    db: Db,
}

/// Older than ten minutes.
pub fn is_stale(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - fetched_at > chrono::Duration::minutes(10)
}

const COLUMNS: &str = "issue_key, summary, status, assignee, issue_type, description, url, updated, fetched_at";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<[Option<String>; 9]> {
    let mut out: [Option<String>; 9] = Default::default();
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = r.get(i)?;
    }
    Ok(out)
}

fn cached(cols: [Option<String>; 9]) -> Result<CachedIssue> {
    let [key, summary, status, assignee, issue_type, description, url, updated, fetched_at] = cols;
    let s = |v: Option<String>| v.unwrap_or_default();
    Ok(CachedIssue {
        issue: Issue {
            key: s(key),
            summary: s(summary),
            status: s(status),
            assignee,
            issue_type: s(issue_type),
            description: s(description),
            url: s(url),
            updated: parse_time(&s(updated))?,
        },
        fetched_at: parse_time(&s(fetched_at))?,
    })
}

impl IssueCache {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Stores the issue as fetched now.
    pub fn upsert(&self, project_id: &ProjectId, issue: &Issue) -> Result<()> {
        self.db.lock().expect("db lock").execute(
            &format!(
                "INSERT INTO jira_links (project_id, {COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (project_id, issue_key) DO UPDATE SET summary = excluded.summary, status = excluded.status,
                 assignee = excluded.assignee, issue_type = excluded.issue_type, description = excluded.description,
                 url = excluded.url, updated = excluded.updated, fetched_at = excluded.fetched_at"
            ),
            params![
                project_id.as_str(),
                issue.key,
                issue.summary,
                issue.status,
                issue.assignee,
                issue.issue_type,
                issue.description,
                issue.url,
                ts(&issue.updated),
                ts(&Utc::now())
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, project_id: &ProjectId, key: &str) -> Result<Option<CachedIssue>> {
        let cols = self
            .db
            .lock()
            .expect("db lock")
            .query_row(
                &format!("SELECT {COLUMNS} FROM jira_links WHERE project_id = ?1 AND issue_key = ?2"),
                params![project_id.as_str(), key],
                row,
            )
            .optional()?;
        cols.map(cached).transpose()
    }

    /// Newest fetch first.
    pub fn for_project(&self, project_id: &ProjectId) -> Result<Vec<CachedIssue>> {
        let conn = self.db.lock().expect("db lock");
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM jira_links WHERE project_id = ?1 ORDER BY fetched_at DESC, issue_key"
        ))?;
        let rows = stmt.query_map([project_id.as_str()], row)?;
        rows.map(|r| cached(r?)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::test_support::sample_issue;
    use crate::workspace::test_support::fixture;
    use chrono::Duration;

    #[test]
    fn upsert_get_and_order_by_fetched() {
        let fx = fixture();
        let c = IssueCache::new(fx.db.clone());
        c.upsert(&fx.api.id, &sample_issue("PROJ-1", None, "a")).unwrap();
        c.upsert(&fx.api.id, &sample_issue("PROJ-2", None, "b")).unwrap();
        c.upsert(&fx.api.id, &sample_issue("PROJ-1", Some("Jane"), "a"))
            .unwrap();
        assert_eq!(
            c.get(&fx.api.id, "PROJ-1").unwrap().unwrap().issue.assignee.as_deref(),
            Some("Jane")
        );
        assert_eq!(
            c.for_project(&fx.api.id)
                .unwrap()
                .iter()
                .map(|x| x.issue.key.as_str())
                .collect::<Vec<_>>(),
            vec!["PROJ-1", "PROJ-2"]
        );
        assert!(c.for_project(&fx.web.id).unwrap().is_empty());
        assert!(is_stale(Utc::now() - Duration::minutes(11), Utc::now()) && !is_stale(Utc::now(), Utc::now()));
    }
}
