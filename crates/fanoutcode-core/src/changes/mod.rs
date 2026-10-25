//! The task's change set (§9), grouped by repo: the task's own files and the "Other changes" beside them.

pub mod attributor;
pub mod repos;
pub mod store;

pub use repos::{done_by_takeover, done_paths, repos_of};

use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::TaskId;
use crate::git::{FileStatus, Numstat, StatusEntry};
use crate::{Error, Result};

/// The agent key of the main session in attributions and per-agent costs.
pub const MAIN_AGENT: &str = "main";

/// Why a file is in the task's set. `Worktree` also marks the "Other changes", which are only seen in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ChangeSource {
    Worktree,
    Hook,
    BashWindow,
    Manual,
    /// Best-effort attribution of a hookless (Codex or shell) session's edits in a shared root (Plan 11).
    Observed,
}

/// One attribution: `path` (relative to `repo_root`) was touched by `agent_id` (`"main"` for the main session).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct AttributedFile {
    pub repo_root: PathBuf,
    pub path: String,
    pub agent_id: String,
    pub source: ChangeSource,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
    pub added: u32,
    pub removed: u32,
    pub binary: bool,
    /// Distinct agents that touched it, first seen first.
    pub owners: Vec<String>,
    pub shared: bool,
    pub source: ChangeSource,
}

/// One repo's part: the task's `files` and the unattributed `other` changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct RepoChanges {
    pub repo_root: PathBuf,
    pub base: Option<String>,
    pub files: Vec<ChangedFile>,
    pub other: Vec<ChangedFile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ChangeSet {
    pub task_id: TaskId,
    pub repos: Vec<RepoChanges>,
}

/// Both sides of one file for the diff editor; `before` is the base version (empty when absent there).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FileDiff {
    pub repo_root: PathBuf,
    pub path: String,
    pub status: FileStatus,
    pub before: String,
    pub after: String,
    pub binary: bool,
}

/// What git said about one repo, and whether the whole tree is the task's (its own worktree).
#[derive(Debug, Clone, PartialEq)]
pub struct RepoSnapshot {
    pub repo_root: PathBuf,
    pub base: Option<String>,
    pub whole_tree_is_task: bool,
    pub status: Vec<StatusEntry>,
    pub numstat: Vec<Numstat>,
}

fn changed(e: &StatusEntry, numstat: &[Numstat], owners: Vec<String>, source: ChangeSource) -> ChangedFile {
    let n = numstat.iter().find(|n| n.path == e.path);
    ChangedFile {
        path: e.path.clone(),
        status: e.status,
        added: n.map_or(0, |n| n.added),
        removed: n.map_or(0, |n| n.removed),
        binary: n.is_some_and(|n| n.binary),
        shared: owners.len() > 1,
        owners,
        source,
    }
}

/// The change set from each repo's status and the task's attributions (§9).
pub fn build(task_id: &TaskId, repos: &[RepoSnapshot], attributed: &[AttributedFile]) -> ChangeSet {
    let repos = repos
        .iter()
        .map(|snap| {
            let (mut files, mut other) = (Vec::new(), Vec::new());
            for e in &snap.status {
                let mine: Vec<&AttributedFile> = attributed
                    .iter()
                    .filter(|a| {
                        a.repo_root == snap.repo_root && (a.path == e.path || e.orig_path.as_deref() == Some(&a.path))
                    })
                    .collect();
                let mut owners: Vec<String> = Vec::new();
                for a in &mine {
                    if !owners.contains(&a.agent_id) {
                        owners.push(a.agent_id.clone());
                    }
                }
                if snap.whole_tree_is_task {
                    if owners.is_empty() {
                        owners.push(MAIN_AGENT.to_owned());
                    }
                    files.push(changed(e, &snap.numstat, owners, ChangeSource::Worktree));
                } else if let Some(first) = mine.first() {
                    files.push(changed(e, &snap.numstat, owners, first.source));
                } else {
                    other.push(changed(e, &snap.numstat, Vec::new(), ChangeSource::Worktree));
                }
            }
            RepoChanges {
                repo_root: snap.repo_root.clone(),
                base: snap.base.clone(),
                files,
                other,
            }
        })
        .collect();
    ChangeSet {
        task_id: task_id.clone(),
        repos,
    }
}

/// The repo holding `abs`: the longest root that is a whole-component prefix of it.
pub fn repo_for<'a>(abs: &Path, repos: &'a [PathBuf]) -> Option<&'a Path> {
    repos
        .iter()
        .filter(|r| abs.starts_with(r))
        .max_by_key(|r| r.components().count())
        .map(PathBuf::as_path)
}

/// `repo/rel` for a path from the webview: relative, no `..`, not empty. Never leaves the repo.
pub fn checked_path(repo: &Path, rel: &str) -> Result<PathBuf> {
    let bad = || Error::Invalid(format!("{rel:?} is not a file inside the repository"));
    let mut out = repo.to_path_buf();
    let mut any = false;
    for c in Path::new(rel).components() {
        match c {
            Component::Normal(part) => {
                out.push(part);
                any = true;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Err(bad()),
        }
    }
    if any {
        Ok(out)
    } else {
        Err(bad())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use chrono::Utc;
    use serde_json::{json, Value};

    use super::attributor::{Attributor, Probe};
    use super::*;
    use crate::db;
    use crate::domain::TaskId;
    use crate::git::{FileStatus, StatusEntry};
    use crate::hooks::{HookEvent, TaskEvent};

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    fn ev(event: HookEvent, agent: Option<&str>) -> TaskEvent {
        TaskEvent {
            seq: 1,
            at: Utc::now(),
            task_id: tid(),
            agent_id: agent.map(str::to_owned),
            event,
        }
    }

    fn post(tool: &str, input: Value, agent: Option<&str>) -> TaskEvent {
        ev(
            HookEvent::PostToolUse {
                tool: tool.into(),
                input,
                response: Value::Null,
            },
            agent,
        )
    }

    fn pre(tool: &str, input: Value, agent: Option<&str>) -> TaskEvent {
        ev(
            HookEvent::PreToolUse {
                tool: tool.into(),
                input,
            },
            agent,
        )
    }

    fn entry(path: &str, status: FileStatus) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            status,
            orig_path: None,
        }
    }

    fn untracked(path: &str) -> StatusEntry {
        entry(path, FileStatus::Untracked)
    }

    fn modified(path: &str) -> StatusEntry {
        entry(path, FileStatus::Modified)
    }

    fn snapshot(repo: &str, status: &[StatusEntry]) -> RepoSnapshot {
        RepoSnapshot {
            repo_root: repo.into(),
            base: None,
            whole_tree_is_task: false,
            status: status.to_vec(),
            numstat: Vec::new(),
        }
    }

    fn attributed(repo: &str, path: &str, agent: &str) -> AttributedFile {
        AttributedFile {
            repo_root: repo.into(),
            path: path.into(),
            agent_id: agent.into(),
            source: ChangeSource::Hook,
            added_at: Utc::now(),
        }
    }

    fn names(files: &[ChangedFile]) -> Vec<&str> {
        files.iter().map(|f| f.path.as_str()).collect()
    }

    #[test]
    fn edit_write_multiedit_and_notebook_edits_attribute_their_file() {
        let mut a = Attributor::new(vec!["/repo".into()]);
        for (tool, key) in [
            ("Edit", "file_path"),
            ("Write", "file_path"),
            ("MultiEdit", "file_path"),
            ("NotebookEdit", "notebook_path"),
        ] {
            a.on_event(&post(tool, json!({key: format!("/repo/src/{tool}.rs")}), Some("a1")));
        }
        assert_eq!(a.files().len(), 4);
        assert_eq!(
            (
                a.files()[0].path.as_str(),
                a.files()[0].agent_id.as_str(),
                a.files()[0].source
            ),
            ("src/Edit.rs", "a1", ChangeSource::Hook)
        );
    }

    #[test]
    fn a_bash_window_attributes_files_that_appear_in_status() {
        let mut a = Attributor::new(vec!["/repo".into()]);
        let probes = a.on_event(&pre("Bash", json!({"command": "npm run codegen"}), None));
        assert_eq!(probes, vec![Probe::Snapshot { window: 1 }]);
        a.on_status(Path::new("/repo"), 1, &[]); // snapshot: clean
        let probes = a.on_event(&post("Bash", json!({"command": "npm run codegen"}), None));
        assert_eq!(probes, vec![Probe::Delta { window: 1 }]);
        let added = a.on_status(Path::new("/repo"), 1, &[untracked("src/gen.ts")]);
        assert_eq!(
            (added[0].path.as_str(), added[0].agent_id.as_str(), added[0].source),
            ("src/gen.ts", MAIN_AGENT, ChangeSource::BashWindow)
        );
    }

    #[test]
    fn a_file_edited_by_two_agents_is_shared() {
        let mut a = Attributor::new(vec!["/repo".into()]);
        a.on_event(&post("Edit", json!({"file_path": "/repo/src/x.rs"}), Some("a1")));
        a.on_event(&post("Edit", json!({"file_path": "/repo/src/x.rs"}), Some("a2")));
        let set = build(&tid(), &[snapshot("/repo", &[modified("src/x.rs")])], a.files());
        assert_eq!(set.repos[0].files[0].owners, vec!["a1", "a2"]);
        assert!(set.repos[0].files[0].shared);
    }

    #[test]
    fn unattributed_files_land_in_other_changes() {
        let set = build(
            &tid(),
            &[snapshot("/repo", &[modified("src/x.rs"), modified("README.md")])],
            &[attributed("/repo", "src/x.rs", "a1")],
        );
        assert_eq!(names(&set.repos[0].files), vec!["src/x.rs"]);
        assert_eq!(names(&set.repos[0].other), vec!["README.md"]);
    }

    #[test]
    fn a_worktree_home_repo_takes_the_whole_diff() {
        let mut s = snapshot("/repo/.worktrees/t", &[modified("a.rs"), modified("b.rs")]);
        s.whole_tree_is_task = true;
        let set = build(&tid(), &[s], &[]);
        assert_eq!(set.repos[0].files.len(), 2);
        assert!(set.repos[0].other.is_empty());
        assert_eq!(set.repos[0].files[0].source, ChangeSource::Worktree);
    }

    #[test]
    fn manual_includes_move_a_file_into_the_set() {
        let mut a = Attributor::new(vec!["/repo".into()]);
        let f = a.include(Path::new("/repo"), "README.md");
        assert_eq!(f.source, ChangeSource::Manual);
        let set = build(&tid(), &[snapshot("/repo", &[modified("README.md")])], a.files());
        assert_eq!(names(&set.repos[0].files), vec!["README.md"]);
    }

    #[test]
    fn observed_is_a_change_source_that_round_trips() {
        assert_eq!(serde_json::to_string(&ChangeSource::Observed).unwrap(), "\"Observed\"");
        assert_eq!(
            serde_json::from_str::<ChangeSource>("\"Observed\"").unwrap(),
            ChangeSource::Observed
        );
    }

    #[test]
    fn files_are_grouped_by_repo_by_longest_prefix() {
        let repos: Vec<PathBuf> = vec!["/ws/app".into(), "/ws/app-api".into(), "/ws".into()];
        assert_eq!(
            repo_for(Path::new("/ws/app-api/src/a b.rs"), &repos),
            Some(Path::new("/ws/app-api"))
        );
        assert_eq!(repo_for(Path::new("/ws/other/x"), &repos), Some(Path::new("/ws")));
        assert_eq!(repo_for(Path::new("/elsewhere/x"), &repos), None);
    }

    #[test]
    fn checked_path_rejects_parent_traversal() {
        assert!(checked_path(Path::new("/repo"), "src/../../etc/passwd").is_err());
        assert!(checked_path(Path::new("/repo"), "/abs").is_err());
        assert_eq!(
            checked_path(Path::new("/repo"), "src/x.rs").unwrap(),
            PathBuf::from("/repo/src/x.rs")
        );
    }

    #[test]
    fn the_observation_tables_exist_after_open() {
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        for t in [
            "event_cursors",
            "task_files",
            "task_test_commands",
            "transcript_cursors",
            "cost_agent_buckets",
        ] {
            let n: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [t], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "{t}");
        }
        store::save(&conn, &tid(), &[attributed("/repo", "a.rs", "a1")]).unwrap();
        assert_eq!(store::load(&conn, &tid()).unwrap().len(), 1);
    }
}
