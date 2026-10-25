//! A task's footprint (§4, §6.1): the files its plan names under `## Files`, and the files its change set holds,
//! kept in `task_footprint` so tasks in other worktrees can plan around them.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, Connection};
use serde::{Deserialize, Serialize};

use crate::db::{enum_from, enum_str, parse_time, ts};
use crate::domain::TaskId;
use crate::Result;

/// One line of a plan's `## Files` section: `- path` or `- path — note`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FootprintEntry {
    pub path: String,
    pub note: Option<String>,
}

/// Where a footprint path came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum FootprintSource {
    /// The plan's `## Files` section.
    Plan,
    /// The task's attributed change set.
    ChangeSet,
}

/// One `task_footprint` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FootprintRow {
    pub task_id: TaskId,
    pub repo_root: PathBuf,
    pub path: String,
    pub source: FootprintSource,
    pub note: Option<String>,
    pub added_at: DateTime<Utc>,
}

/// A level-1 or level-2 heading's title, `None` for any other line.
fn heading(line: &str) -> Option<&str> {
    line.strip_prefix("## ")
        .or_else(|| line.strip_prefix("# "))
        .map(str::trim)
}

/// The entries under `## Files` (case-insensitive) up to the next `#` or `##` heading, each a `-` or `*` list item:
/// a path (backticks stripped), then an optional note after `—`. Empty without the section.
pub fn parse_files_section(md: &str) -> Vec<FootprintEntry> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in md.lines() {
        let line = line.trim_end();
        if let Some(title) = heading(line) {
            inside = title.eq_ignore_ascii_case("files");
            continue;
        }
        if !inside {
            continue;
        }
        let Some(item) = line
            .trim_start()
            .strip_prefix("- ")
            .or_else(|| line.trim_start().strip_prefix("* "))
        else {
            continue;
        };
        let (path, note) = match item.split_once('—') {
            Some((p, n)) => (p, Some(n.trim()).filter(|n| !n.is_empty())),
            None => (item, None),
        };
        let path = path.trim().trim_matches('`').trim();
        if !path.is_empty() {
            out.push(FootprintEntry {
                path: path.to_owned(),
                note: note.map(str::to_owned),
            });
        }
    }
    out
}

fn insert(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    path: &str,
    source: FootprintSource,
    note: Option<&str>,
    now: DateTime<Utc>,
) -> Result<()> {
    // a plan row takes the latest note; a change-set row keeps its first `added_at`
    let or = match source {
        FootprintSource::Plan => "REPLACE",
        FootprintSource::ChangeSet => "IGNORE",
    };
    conn.execute(
        &format!(
            "INSERT OR {or} INTO task_footprint (task_id, repo_root, path, source, note, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ),
        params![
            task.as_str(),
            repo.to_string_lossy(),
            path,
            enum_str(&source)?,
            note,
            ts(&now)
        ],
    )?;
    Ok(())
}

/// Replaces the task's `Plan` rows for `repo` with `entries`; its `ChangeSet` rows and other repos stay.
pub fn replace_plan(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    entries: &[FootprintEntry],
    now: DateTime<Utc>,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM task_footprint WHERE task_id = ?1 AND repo_root = ?2 AND source = ?3",
        params![task.as_str(), repo.to_string_lossy(), enum_str(&FootprintSource::Plan)?],
    )?;
    for e in entries {
        insert(&tx, task, repo, &e.path, FootprintSource::Plan, e.note.as_deref(), now)?;
    }
    tx.commit()?;
    Ok(())
}

/// Adds `paths` of `repo` as `ChangeSet` rows; a path already there keeps its first `added_at`.
pub fn add_change_set(
    conn: &Connection,
    task: &TaskId,
    repo: &Path,
    paths: &[String],
    now: DateTime<Utc>,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for p in paths {
        insert(&tx, task, repo, p, FootprintSource::ChangeSet, None, now)?;
    }
    tx.commit()?;
    Ok(())
}

/// The footprint of `task`.
pub fn footprint(conn: &Connection, task: &TaskId) -> Result<Vec<FootprintRow>> {
    footprints_of(conn, std::slice::from_ref(task))
}

/// The footprints of `tasks`, by task, repo, path and source.
pub fn footprints_of(conn: &Connection, tasks: &[TaskId]) -> Result<Vec<FootprintRow>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    let marks = vec!["?"; tasks.len()].join(", ");
    let mut stmt = conn.prepare(&format!(
        "SELECT task_id, repo_root, path, source, note, added_at FROM task_footprint WHERE task_id IN ({marks})
         ORDER BY task_id, repo_root, path, source"
    ))?;
    let rows = stmt.query_map(params_from_iter(tasks.iter().map(TaskId::as_str)), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    rows.map(|r| {
        let (task, repo, path, source, note, at) = r?;
        Ok(FootprintRow {
            task_id: TaskId::parse(&task)?,
            repo_root: PathBuf::from(repo),
            path,
            source: enum_from(&source)?,
            note,
            added_at: parse_time(&at)?,
        })
    })
    .collect()
}

#[cfg(test)]
#[path = "footprint_tests.rs"]
mod tests;
