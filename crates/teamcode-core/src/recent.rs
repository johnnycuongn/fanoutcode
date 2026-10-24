//! The start screen's recent projects and workspaces, stored as one JSON list under the app setting `recent`.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{app_setting, set_app_setting};
use crate::Result;

/// The most entries kept.
pub const MAX: usize = 20;
const KEY: &str = "recent";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum RecentKind {
    Project,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct RecentEntry {
    pub kind: RecentKind,
    /// A project or workspace id.
    pub id: String,
    pub name: String,
    /// The project root or the workspace file.
    pub path: PathBuf,
    pub opened_at: DateTime<Utc>,
    /// Computed by `list` at read time (the folder or file was deleted or moved); the stored value is ignored.
    #[serde(default)]
    pub missing: bool,
}

/// Adds `entry` at the front, dropping any older entry with the same `(kind, id)`, keeping at most `MAX`.
pub fn push(conn: &Connection, mut entry: RecentEntry) -> Result<()> {
    let mut entries = stored(conn)?;
    entries.retain(|e| !(e.kind == entry.kind && e.id == entry.id));
    entry.missing = false;
    entries.insert(0, entry);
    entries.truncate(MAX);
    set_app_setting(conn, KEY, &entries)
}

/// Newest first; an entry whose path no longer exists is kept and marked `missing`.
pub fn list(conn: &Connection) -> Result<Vec<RecentEntry>> {
    let mut entries = stored(conn)?;
    for e in &mut entries {
        e.missing = !e.path.exists();
    }
    Ok(entries)
}

pub fn remove(conn: &Connection, kind: RecentKind, id: &str) -> Result<()> {
    let mut entries = stored(conn)?;
    entries.retain(|e| !(e.kind == kind && e.id == id));
    set_app_setting(conn, KEY, &entries)
}

fn stored(conn: &Connection) -> Result<Vec<RecentEntry>> {
    Ok(app_setting(conn, KEY)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use chrono::Utc;
    use std::path::Path;

    fn entry(kind: RecentKind, id: &str, path: &Path) -> RecentEntry {
        RecentEntry {
            kind,
            id: id.into(),
            name: id.into(),
            path: path.into(),
            opened_at: Utc::now(),
            missing: false,
        }
    }

    #[test]
    fn recent_dedupes_moves_to_front_and_caps_at_max() {
        let conn = open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        for i in 0..25 {
            push(&conn, entry(RecentKind::Project, &format!("p-{i:x}"), dir.path())).unwrap();
        }
        push(&conn, entry(RecentKind::Project, "p-3", dir.path())).unwrap();
        let list = list(&conn).unwrap();
        assert_eq!(list.len(), MAX);
        assert_eq!(list[0].id, "p-3");
        assert_eq!(list.iter().filter(|e| e.id == "p-3").count(), 1);
    }

    #[test]
    fn recent_marks_missing_paths_without_dropping_them() {
        let conn = open_in_memory().unwrap();
        push(
            &conn,
            entry(
                RecentKind::Workspace,
                "ws-1",
                Path::new("/nonexistent/x.teamcode-workspace.json"),
            ),
        )
        .unwrap();
        let list = list(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].missing);
        remove(&conn, RecentKind::Workspace, "ws-1").unwrap();
        assert!(super::list(&conn).unwrap().is_empty());
    }
}
