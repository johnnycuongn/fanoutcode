//! The `task_files` table: the attributions behind a task's change set, kept across launches.

use std::path::PathBuf;

use rusqlite::{params, Connection};

use super::AttributedFile;
use crate::db::{enum_from, enum_str, parse_time, ts};
use crate::domain::TaskId;
use crate::Result;

/// The task's attributions, oldest first.
pub fn load(conn: &Connection, task_id: &TaskId) -> Result<Vec<AttributedFile>> {
    let mut stmt = conn.prepare(
        "SELECT repo_root, path, agent_id, source, added_at FROM task_files WHERE task_id = ?1 ORDER BY added_at, rowid",
    )?;
    let rows = stmt.query_map([task_id.as_str()], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    rows.map(|r| {
        let (repo, path, agent_id, source, at) = r?;
        Ok(AttributedFile {
            repo_root: PathBuf::from(repo),
            path,
            agent_id,
            source: enum_from(&source)?,
            added_at: parse_time(&at)?,
        })
    })
    .collect()
}

/// Replaces the task's attributions with `files`, in one transaction.
pub fn save(conn: &Connection, task_id: &TaskId, files: &[AttributedFile]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM task_files WHERE task_id = ?1", [task_id.as_str()])?;
    for f in files {
        tx.execute(
            "INSERT OR IGNORE INTO task_files (task_id, repo_root, path, agent_id, source, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task_id.as_str(),
                f.repo_root.to_string_lossy(),
                f.path,
                f.agent_id,
                enum_str(&f.source)?,
                ts(&f.added_at)
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}
