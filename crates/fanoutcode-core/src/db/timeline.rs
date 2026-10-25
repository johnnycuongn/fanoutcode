//! The `task_timeline` table: stage entries, hand-backs and commits, the source `index.md`'s timeline mirrors.

use rusqlite::{params, Connection};
use serde_json::json;

use super::{enum_from, enum_str, parse_time, ts};
use crate::domain::{Stage, TaskId};
use crate::memory::{IndexEntry, TimelineKind};
use crate::{Error, Result};

/// Appends one entry; `kind` is the timeline word, `detail` the chip or the commit as JSON.
pub fn append(conn: &Connection, task: &TaskId, e: &IndexEntry) -> Result<()> {
    let detail = match &e.kind {
        TimelineKind::Handback { chip } => Some(json!({ "chip": chip })),
        TimelineKind::Commit { repo, sha } => Some(json!({ "repo": repo, "sha": sha })),
        TimelineKind::Spec { reason } => Some(json!({ "reason": reason })),
        _ => None,
    };
    conn.execute(
        "INSERT INTO task_timeline (task_id, at, kind, stage, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            task.as_str(),
            ts(&e.at),
            e.kind.word(),
            enum_str(&e.stage)?,
            detail.map(|d| d.to_string())
        ],
    )?;
    Ok(())
}

fn entry(at: String, kind: String, stage: Option<String>, detail: Option<String>) -> Result<IndexEntry> {
    let bad = || Error::Db(format!("A timeline row does not parse: {kind}"));
    let detail: serde_json::Value = match detail {
        Some(d) => serde_json::from_str(&d).map_err(|_| bad())?,
        None => serde_json::Value::Null,
    };
    let field = |k: &str| {
        detail
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(bad)
    };
    let kind = match kind.as_str() {
        "handback" => TimelineKind::Handback { chip: field("chip")? },
        "commit" => TimelineKind::Commit {
            repo: field("repo")?,
            sha: field("sha")?,
        },
        "spec" => TimelineKind::Spec {
            reason: field("reason")?,
        },
        word => TimelineKind::unit_from_word(word).ok_or_else(bad)?,
    };
    Ok(IndexEntry {
        at: parse_time(&at)?,
        stage: enum_from::<Stage>(&stage.ok_or_else(bad)?)?,
        kind,
    })
}

/// The task's entries in the order they were appended.
pub fn list(conn: &Connection, task: &TaskId) -> Result<Vec<IndexEntry>> {
    let mut stmt = conn.prepare("SELECT at, kind, stage, detail FROM task_timeline WHERE task_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map([task.as_str()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    rows.map(|r| {
        let (at, kind, stage, detail) = r?;
        entry(at, kind, stage, detail)
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::test_support::{insert_project, task_fixture, test_conn};

    #[test]
    fn timeline_appends_in_order_with_details() {
        let c = test_conn();
        insert_project(&c, "p-1a-1");
        let t = task_fixture("t-1a-1", "p-1a-1");
        crate::db::tasks::insert(&c, &t).unwrap();
        let at: DateTime<Utc> = "2026-10-24T05:12:33Z".parse().unwrap();
        append(
            &c,
            &t.id,
            &IndexEntry {
                at,
                stage: Stage::Spec,
                kind: TimelineKind::Created,
            },
        )
        .unwrap();
        append(
            &c,
            &t.id,
            &IndexEntry {
                at,
                stage: Stage::Plan,
                kind: TimelineKind::Handback {
                    chip: "Plan written, go?".into(),
                },
            },
        )
        .unwrap();
        append(
            &c,
            &t.id,
            &IndexEntry {
                at,
                stage: Stage::Commit,
                kind: TimelineKind::Commit {
                    repo: "/r".into(),
                    sha: "abc".into(),
                },
            },
        )
        .unwrap();
        let rows = list(&c, &t.id).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[1].kind,
            TimelineKind::Handback {
                chip: "Plan written, go?".into()
            }
        );
        assert_eq!(
            rows[2].kind,
            TimelineKind::Commit {
                repo: "/r".into(),
                sha: "abc".into()
            }
        );
    }

    #[test]
    fn spec_watcher_entries_keep_their_reason() {
        let c = test_conn();
        insert_project(&c, "p-1a-1");
        let t = task_fixture("t-1a-1", "p-1a-1");
        crate::db::tasks::insert(&c, &t).unwrap();
        let kind = TimelineKind::Spec {
            reason: "rows collapse".into(),
        };
        append(
            &c,
            &t.id,
            &IndexEntry {
                at: Utc::now(),
                stage: Stage::Review,
                kind: kind.clone(),
            },
        )
        .unwrap();
        assert_eq!(list(&c, &t.id).unwrap()[0].kind, kind);
    }
}
