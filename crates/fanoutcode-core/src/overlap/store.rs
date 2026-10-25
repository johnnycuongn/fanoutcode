//! `task_overlaps` CRUD (§9): one row per (task, repo, path, other task), the source of truth for the Overlap tab,
//! the counts, the needs-you pin and the taken-over exclusion from Done detection.
//!
//! `other_task` is NULL for a claim of *another session*; SQLite keeps NULLs distinct in the primary key, so every
//! lookup matches it with `IS` and an upsert is an update, then an insert when nothing matched.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{OverlapRow, PendingDecision, Relation};
use crate::changes::AttributedFile;
use crate::db::{enum_from, enum_str, parse_time, ts};
use crate::domain::TaskId;
use crate::Result;

const COLUMNS: &str = "task_id, other_task, repo_root, path, relation, pending_json, held, noted_out, noted_in, \
                       decided_at, updated_at";

/// The row's key as SQL parameters, matched by `KEY_WHERE` (`?1`..`?4`).
const KEY_WHERE: &str = "task_id = ?1 AND repo_root = ?2 AND path = ?3 AND other_task IS ?4";

fn key<'a>(
    task: &'a TaskId,
    repo: &Path,
    path: &'a str,
    other: Option<&'a TaskId>,
) -> (&'a str, String, &'a str, Option<&'a str>) {
    (
        task.as_str(),
        repo.to_string_lossy().into_owned(),
        path,
        other.map(TaskId::as_str),
    )
}

type RawRow = (
    String,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    bool,
    bool,
    bool,
    Option<String>,
    String,
);

fn raw(r: &Row<'_>) -> rusqlite::Result<RawRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
        r.get(10)?,
    ))
}

fn decode(raw: RawRow) -> Result<OverlapRow> {
    let (task_id, other, repo, path, relation, pending, held, noted_out, noted_in, decided_at, updated_at) = raw;
    Ok(OverlapRow {
        task_id: TaskId::parse(&task_id)?,
        other_task: other.as_deref().map(TaskId::parse).transpose()?,
        repo_root: PathBuf::from(repo),
        path,
        relation: enum_from(&relation)?,
        pending: pending.as_deref().map(serde_json::from_str).transpose()?,
        held,
        noted_out,
        noted_in,
        decided_at: decided_at.as_deref().map(parse_time).transpose()?,
        updated_at: parse_time(&updated_at)?,
    })
}

fn select(conn: &Connection, filter: &str, task: &TaskId) -> Result<Vec<OverlapRow>> {
    let sql =
        format!("SELECT {COLUMNS} FROM task_overlaps WHERE task_id = ?1 {filter} ORDER BY repo_root, path, other_task");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([task.as_str()], raw)?;
    rows.map(|r| decode(r?)).collect()
}

fn pending_json(pending: Option<&PendingDecision>) -> Result<Option<String>> {
    Ok(pending.map(serde_json::to_string).transpose()?)
}

/// Inserts `row`, or replaces every field of the row with its key.
pub fn upsert(conn: &Connection, row: &OverlapRow) -> Result<()> {
    let (task, repo, path, other) = key(&row.task_id, &row.repo_root, &row.path, row.other_task.as_ref());
    let values = params![
        task,
        repo,
        path,
        other,
        enum_str(&row.relation)?,
        pending_json(row.pending.as_ref())?,
        row.held,
        row.noted_out,
        row.noted_in,
        row.decided_at.as_ref().map(ts),
        ts(&row.updated_at),
    ];
    let updated = conn.execute(
        &format!(
            "UPDATE task_overlaps SET relation = ?5, pending_json = ?6, held = ?7, noted_out = ?8, noted_in = ?9,
             decided_at = ?10, updated_at = ?11 WHERE {KEY_WHERE}"
        ),
        values,
    )?;
    if updated == 0 {
        conn.execute(
            "INSERT INTO task_overlaps (task_id, repo_root, path, other_task, relation, pending_json, held,
             noted_out, noted_in, decided_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            values,
        )?;
    }
    Ok(())
}

/// Every overlap of `task`, by repo, path and other task.
pub fn list(conn: &Connection, task: &TaskId) -> Result<Vec<OverlapRow>> {
    select(conn, "", task)
}

/// The overlaps of `task` with a decision waiting on the developer.
pub fn pending(conn: &Connection, task: &TaskId) -> Result<Vec<OverlapRow>> {
    select(conn, "AND pending_json IS NOT NULL", task)
}

fn get(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
) -> Result<Option<OverlapRow>> {
    let (t, r, p, o) = key(task, repo, path, other);
    let found = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM task_overlaps WHERE {KEY_WHERE}"),
            params![t, r, p, o],
            raw,
        )
        .optional()?;
    found.map(decode).transpose()
}

/// The row with this key, or a fresh `Claimed` one (a denial can arrive before the overlap sync saw the claim).
fn existing_or_claimed(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
) -> Result<OverlapRow> {
    Ok(get(conn, task, repo, path, other)?.unwrap_or_else(|| OverlapRow {
        task_id: task.clone(),
        other_task: other.cloned(),
        repo_root: repo.to_owned(),
        path: path.to_owned(),
        relation: Relation::Claimed,
        pending: None,
        held: false,
        noted_out: false,
        noted_in: false,
        decided_at: None,
        updated_at: Utc::now(),
    }))
}

/// Opens (`Some`) or closes (`None`) the decision on one overlap, creating a `Claimed` row when there is none.
/// Closing it also clears `held`: a hold only lasts while its decision is open.
pub fn set_pending(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
    pending: Option<PendingDecision>,
) -> Result<()> {
    let mut row = existing_or_claimed(conn, task, repo, path, other)?;
    row.held = row.held && pending.is_some();
    row.pending = pending;
    row.updated_at = Utc::now();
    upsert(conn, &row)
}

/// Records the developer's answer as the overlap's relation, creating the row when there is none.
pub fn set_relation(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
    relation: Relation,
    decided_at: Option<DateTime<Utc>>,
) -> Result<()> {
    let mut row = existing_or_claimed(conn, task, repo, path, other)?;
    row.relation = relation;
    row.decided_at = decided_at;
    row.updated_at = Utc::now();
    upsert(conn, &row)
}

/// Sets or clears the Hold on one existing overlap; a missing row is left missing.
pub fn set_held(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
    held: bool,
) -> Result<()> {
    let (t, r, p, o) = key(task, repo, path, other);
    conn.execute(
        &format!("UPDATE task_overlaps SET held = ?5, updated_at = ?6 WHERE {KEY_WHERE}"),
        params![t, r, p, o, held, ts(&Utc::now())],
    )?;
    Ok(())
}

/// Closes every open decision of `task` (and its holds), returning how many there were: a fresh session or an
/// archive must not inherit them.
pub fn clear_pending(conn: &Connection, task: &TaskId) -> Result<u32> {
    let n = conn.execute(
        "UPDATE task_overlaps SET pending_json = NULL, held = 0, updated_at = ?2
         WHERE task_id = ?1 AND pending_json IS NOT NULL",
        params![task.as_str(), ts(&Utc::now())],
    )?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The paths of `repo` other tasks took over from `task` (rows where `task` is the *other* side and the relation is
/// `TakenOver`): they left `task`'s change set, so its Done detection skips them. The `TakenOverBy` mirror rows (on the
/// side that lost the file, naming the replacer as the other task) are not read, so the replacer keeps the path.
pub fn taken_over_paths(conn: &Connection, task: &TaskId, repo: &Path) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT path FROM task_overlaps WHERE other_task = ?1 AND repo_root = ?2 AND relation = ?3
         ORDER BY path",
    )?;
    let rows = stmt.query_map(
        params![task.as_str(), repo.to_string_lossy(), enum_str(&Relation::TakenOver)?],
        |r| r.get::<_, String>(0),
    )?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The `taken_over_paths` of `task` in `repo` that still left its change set: a path counts again once `task`
/// attributed it after the `Replace` (`attributed` holds an entry of it newer than the row's `decided_at`), because
/// the loser edited the file once more and must commit it like any other. A row without `decided_at` is treated as
/// decided at the epoch, so any attribution brings the path back.
pub fn taken_over_paths_for(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    attributed: &[AttributedFile],
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT path, MAX(COALESCE(decided_at, '')) FROM task_overlaps
         WHERE other_task = ?1 AND repo_root = ?2 AND relation = ?3 GROUP BY path ORDER BY path",
    )?;
    let rows = stmt.query_map(
        params![task.as_str(), repo.to_string_lossy(), enum_str(&Relation::TakenOver)?],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )?;
    let mut out = Vec::new();
    for row in rows {
        let (path, decided) = row?;
        let decided = if decided.is_empty() {
            DateTime::<Utc>::UNIX_EPOCH
        } else {
            parse_time(&decided)?
        };
        let edited_since = attributed
            .iter()
            .any(|f| f.repo_root == repo && f.path == path && f.added_at > decided);
        if !edited_since {
            out.push(path);
        }
    }
    Ok(out)
}

/// Marks the note about one overlap as sent: `outgoing` to this task's session, else to the other task's.
pub fn mark_noted(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
    outgoing: bool,
) -> Result<()> {
    let column = if outgoing { "noted_out" } else { "noted_in" };
    let (t, r, p, o) = key(task, repo, path, other);
    conn.execute(
        &format!("UPDATE task_overlaps SET {column} = 1, updated_at = ?5 WHERE {KEY_WHERE}"),
        params![t, r, p, o, ts(&Utc::now())],
    )?;
    Ok(())
}

/// Deletes every overlap of `task` whose (repo, path, other task) is not in `keep`, returning how many went.
pub fn remove_absent(conn: &Connection, task: &TaskId, keep: &[(PathBuf, String, Option<TaskId>)]) -> Result<u32> {
    let tx = conn.unchecked_transaction()?;
    let mut removed = 0;
    for row in list(&tx, task)? {
        let kept = keep
            .iter()
            .any(|(repo, path, other)| *repo == row.repo_root && *path == row.path && *other == row.other_task);
        if !kept {
            let (t, r, p, o) = key(task, &row.repo_root, &row.path, row.other_task.as_ref());
            removed += tx.execute(
                &format!("DELETE FROM task_overlaps WHERE {KEY_WHERE}"),
                params![t, r, p, o],
            )?;
        }
    }
    tx.commit()?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
