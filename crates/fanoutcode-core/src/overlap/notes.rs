//! The per-task note inbox (§5.5): `<app_data>/notes/<task-id>.json`, a JSON array of messages the app wants a
//! working session to hear. The claim-check hook drains it into `additionalContext` on its next allowed call.

use std::path::{Path, PathBuf};

use crate::domain::TaskId;
use crate::memory::store::write_atomic;
use crate::Result;

/// `<app_data>/notes/<task-id>.json`. The id was validated when it was parsed, so it is safe in a path.
pub fn notes_path(app_data: &Path, task_id: &TaskId) -> PathBuf {
    app_data.join("notes").join(format!("{}.json", task_id.as_str()))
}

fn read(path: &Path) -> Vec<String> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Appends `note` to the inbox at `path` (a malformed inbox starts over), written atomically.
pub fn push_note(path: &Path, note: &str) -> Result<()> {
    let mut notes = read(path);
    notes.push(note.to_owned());
    write_atomic(path, &serde_json::to_vec(&notes)?)
}

/// Every note in the inbox, oldest first, and the inbox emptied. The file is first renamed aside, so a note pushed
/// while the hook drains lands in a fresh inbox instead of being lost (at worst a note is heard twice).
pub fn drain_notes(path: &Path) -> Vec<String> {
    let mut taken = path.as_os_str().to_owned();
    taken.push(format!(".draining.{}", std::process::id()));
    let taken = PathBuf::from(taken);
    if std::fs::rename(path, &taken).is_err() {
        return Vec::new();
    }
    let notes = read(&taken);
    let _ = std::fs::remove_file(&taken);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_drain_returns_in_order_and_empties() {
        let dir = tempfile::tempdir().unwrap();
        let path = notes_path(dir.path(), &TaskId::parse("t-a1").unwrap());
        assert_eq!(path, dir.path().join("notes/t-a1.json"));
        assert!(drain_notes(&path).is_empty());
        push_note(&path, "first").unwrap();
        push_note(&path, "second").unwrap();
        assert_eq!(drain_notes(&path), vec!["first".to_owned(), "second".to_owned()]);
        assert!(drain_notes(&path).is_empty());
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(dir.path().join("notes")).unwrap().count(), 0);
    }

    #[test]
    fn a_malformed_inbox_starts_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("n.json");
        std::fs::write(&path, "[\"half").unwrap();
        push_note(&path, "fresh").unwrap();
        assert_eq!(drain_notes(&path), vec!["fresh".to_owned()]);
    }
}
