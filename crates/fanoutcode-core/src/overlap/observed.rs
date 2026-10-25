//! The claims of *another session* that outlive the app (§4, §5.1): per shared repo, the dirty files no task owns,
//! kept in `<app_data>/claims/<repo_key>.observed.json` beside the claims file. Two kinds land here: the edits seen
//! while several hookless sessions worked in the repo, and the dirty files an archived task left behind (an archived
//! task is no longer active, so it claims nothing itself, but its changes are still on disk and still somebody's
//! work: they claim as *another session* until the file is clean). Each list is rewritten atomically whenever it
//! changes and read back at launch.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::changes::{AttributedFile, ChangeSource};
use crate::git::StatusEntry;
use crate::memory::store::write_atomic;
use crate::Result;

use super::claims::repo_key;

/// `<app_data>/claims/<repo_key>.observed.json`.
pub fn observed_path(app_data: &Path, root: &Path) -> PathBuf {
    app_data
        .join("claims")
        .join(format!("{}.observed.json", repo_key(root)))
}

/// Writes `root`'s observed files atomically.
pub fn write_observed(app_data: &Path, root: &Path, files: &[AttributedFile]) -> Result<()> {
    write_atomic(&observed_path(app_data, root), &serde_json::to_vec(files)?)
}

/// `root`'s observed files; empty when the file is missing or does not parse.
pub fn read_observed(app_data: &Path, root: &Path) -> Vec<AttributedFile> {
    std::fs::read(observed_path(app_data, root))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// The dirty files of `root` that `files` (an archived task's attributions) left behind, as `Observed` entries
/// for the list of *another session*, one per path.
pub fn orphaned(root: &Path, files: &[AttributedFile], status: &[StatusEntry]) -> Vec<AttributedFile> {
    let dirty: HashSet<&str> = status.iter().map(|e| e.path.as_str()).collect();
    let mut seen = HashSet::new();
    files
        .iter()
        .filter(|f| f.repo_root == root && dirty.contains(f.path.as_str()) && seen.insert(f.path.as_str()))
        .map(|f| AttributedFile {
            source: ChangeSource::Observed,
            ..f.clone()
        })
        .collect()
}

/// Adds every entry of `fresh` whose path `list` lacks; whether anything was added.
pub fn merge(list: &mut Vec<AttributedFile>, fresh: Vec<AttributedFile>) -> bool {
    let before = list.len();
    for f in fresh {
        if !list.iter().any(|g| g.path == f.path) {
            list.push(f);
        }
    }
    list.len() != before
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::changes::MAIN_AGENT;
    use crate::git::FileStatus;
    use crate::overlap::claims::{add_unattributed, build_claims, ClaimsFile, ANOTHER_SESSION};

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn file(path: &str, source: ChangeSource) -> AttributedFile {
        AttributedFile {
            repo_root: "/r".into(),
            path: path.into(),
            agent_id: MAIN_AGENT.into(),
            source,
            added_at: at("2026-10-25T09:00:00Z"),
        }
    }

    fn dirty(paths: &[&str]) -> Vec<StatusEntry> {
        paths
            .iter()
            .map(|p| StatusEntry {
                path: (*p).into(),
                status: FileStatus::Modified,
                orig_path: None,
            })
            .collect()
    }

    #[test]
    fn observed_claims_survive_a_reload() {
        let app_data = tempfile::tempdir().unwrap();
        let root = Path::new("/r");
        assert!(read_observed(app_data.path(), root).is_empty());
        let seen = vec![file("src/gen.ts", ChangeSource::Observed)];
        write_observed(app_data.path(), root, &seen).unwrap();
        assert_eq!(
            observed_path(app_data.path(), root),
            app_data.path().join(format!("claims/{}.observed.json", repo_key(root)))
        );
        // a relaunch reads them back, and they claim as another session again
        let reloaded = read_observed(app_data.path(), root);
        assert_eq!(reloaded, seen);
        let status = dirty(&["src/gen.ts"]);
        let mut claims = build_claims(root, &[], &status, at("2026-10-25T10:00:00Z"));
        add_unattributed(&mut claims, &reloaded, &status);
        assert_eq!(claims.claims.len(), 1);
        assert_eq!(
            (claims.claims[0].task_id.as_ref(), claims.claims[0].title.as_str()),
            (None, ANOTHER_SESSION)
        );
        // another repo's list is its own
        assert!(read_observed(app_data.path(), Path::new("/other")).is_empty());
        // a malformed file reads as empty
        std::fs::write(observed_path(app_data.path(), root), "{").unwrap();
        assert!(read_observed(app_data.path(), root).is_empty());
    }

    #[test]
    fn an_archived_tasks_dirty_files_become_another_sessions() {
        let files = [
            file("src/a.ts", ChangeSource::Hook),
            file("src/a.ts", ChangeSource::BashWindow),
            file("src/clean.ts", ChangeSource::Hook),
            AttributedFile {
                repo_root: "/elsewhere".into(),
                ..file("src/b.ts", ChangeSource::Hook)
            },
        ];
        let status = dirty(&["src/a.ts", "src/b.ts"]);
        let left = orphaned(Path::new("/r"), &files, &status);
        assert_eq!(left, vec![file("src/a.ts", ChangeSource::Observed)]);
        let mut list = vec![file("src/gen.ts", ChangeSource::Observed)];
        assert!(merge(&mut list, left.clone()));
        assert!(!merge(&mut list, left));
        assert_eq!(list.len(), 2);
        let mut claims = ClaimsFile {
            repo_root: "/r".into(),
            written_at: at("2026-10-25T10:00:00Z"),
            claims: vec![],
        };
        add_unattributed(&mut claims, &list, &status);
        let paths: Vec<&str> = claims.claims.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["src/a.ts"]);
    }
}
