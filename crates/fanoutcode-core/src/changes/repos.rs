//! Which repos a task touches, and which of their paths Commit-stage Done detection checks.

use std::path::{Path, PathBuf};

use super::{AttributedFile, ChangeSet};
use crate::domain::Task;

/// The task's repos: its worktree (or the home root without one), then every linked root, each once.
pub fn repos_of(task: &Task, home_root: &Path) -> Vec<PathBuf> {
    let mut repos = vec![task.worktree_path.clone().unwrap_or_else(|| home_root.to_owned())];
    for r in &task.linked_roots {
        if !repos.contains(r) {
            repos.push(r.clone());
        }
    }
    repos
}

/// Per repo, the paths that must be committed (or undone) before the task is Done (§6.6): every attributed path plus
/// every file the last change set held for it, less the paths another task took over (Plan 11, §5.4). Repos with
/// none are left out.
pub fn done_paths(
    repos: &[PathBuf],
    attributed: &[AttributedFile],
    last: Option<&ChangeSet>,
    taken_over: &[(PathBuf, Vec<String>)],
) -> Vec<(PathBuf, Vec<String>)> {
    repos
        .iter()
        .filter_map(|repo| {
            let mut paths: Vec<String> = attributed
                .iter()
                .filter(|a| &a.repo_root == repo)
                .map(|a| a.path.clone())
                .collect();
            let cached = last.into_iter().flat_map(|s| &s.repos).filter(|r| &r.repo_root == repo);
            paths.extend(cached.flat_map(|r| r.files.iter().map(|f| f.path.clone())));
            let gone: Vec<&String> = taken_over
                .iter()
                .filter(|(r, _)| r == repo)
                .flat_map(|(_, p)| p)
                .collect();
            paths.retain(|p| !gone.contains(&p));
            paths.sort();
            paths.dedup();
            (!paths.is_empty()).then(|| (repo.clone(), paths))
        })
        .collect()
}

/// Whether Commit-stage Done detection should advance with nothing left to commit: `remaining` (the `done_paths`
/// with `taken_over` excluded) is empty because every path the task had went to another task (Plan 11, §5.4). A task
/// that never touched anything (no taken-over path either) is not done this way.
pub fn done_by_takeover(remaining: &[(PathBuf, Vec<String>)], taken_over: &[(PathBuf, Vec<String>)]) -> bool {
    remaining.is_empty() && taken_over.iter().any(|(_, paths)| !paths.is_empty())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::changes::{ChangeSource, ChangedFile, RepoChanges};
    use crate::git::FileStatus;
    use crate::test_support::task_fixture;

    #[test]
    fn repos_are_the_worktree_or_home_then_linked_roots() {
        let mut t = task_fixture("t-1a-1", "p-1a-1");
        t.linked_roots = vec!["/api".into(), "/api".into()];
        assert_eq!(
            repos_of(&t, Path::new("/app")),
            vec![PathBuf::from("/app"), "/api".into()]
        );
        t.worktree_path = Some("/app/.worktrees/x".into());
        assert_eq!(repos_of(&t, Path::new("/app"))[0], PathBuf::from("/app/.worktrees/x"));
    }

    #[test]
    fn done_paths_join_attributions_and_the_last_change_set() {
        let a = AttributedFile {
            repo_root: "/app".into(),
            path: "a.rs".into(),
            agent_id: "main".into(),
            source: ChangeSource::Hook,
            added_at: Utc::now(),
        };
        let file = |p: &str| ChangedFile {
            path: p.into(),
            status: FileStatus::Modified,
            added: 1,
            removed: 0,
            binary: false,
            owners: vec![],
            shared: false,
            source: ChangeSource::Worktree,
        };
        let set = ChangeSet {
            task_id: crate::domain::TaskId::parse("t-1").unwrap(),
            repos: vec![RepoChanges {
                repo_root: "/app".into(),
                base: None,
                files: vec![file("a.rs"), file("b.rs")],
                other: vec![file("c.rs")],
            }],
        };
        let repos = vec![PathBuf::from("/app"), "/api".into()];
        assert_eq!(
            done_paths(&repos, &[a], Some(&set), &[]),
            vec![(PathBuf::from("/app"), vec!["a.rs".to_owned(), "b.rs".into()])]
        );
        assert!(done_paths(&repos, &[], None, &[]).is_empty());
    }

    #[test]
    fn done_paths_skips_taken_over_paths() {
        let file = |p: &str| AttributedFile {
            repo_root: "/app".into(),
            path: p.into(),
            agent_id: "main".into(),
            source: ChangeSource::Hook,
            added_at: Utc::now(),
        };
        let repos = vec![PathBuf::from("/app")];
        let attributed = [file("a.rs"), file("b.rs")];
        let other_repo = (PathBuf::from("/api"), vec!["b.rs".to_owned()]);
        assert_eq!(
            done_paths(&repos, &attributed, None, std::slice::from_ref(&other_repo)),
            vec![(PathBuf::from("/app"), vec!["a.rs".to_owned(), "b.rs".into()])]
        );
        let taken = [(PathBuf::from("/app"), vec!["b.rs".to_owned()]), other_repo];
        assert_eq!(
            done_paths(&repos, &attributed, None, &taken),
            vec![(PathBuf::from("/app"), vec!["a.rs".to_owned()])]
        );
        let all = [(PathBuf::from("/app"), vec!["a.rs".to_owned(), "b.rs".into()])];
        assert!(done_paths(&repos, &attributed, None, &all).is_empty());
    }

    #[test]
    fn done_by_takeover_needs_nothing_left_and_something_taken() {
        let taken = [(PathBuf::from("/app"), vec!["a.rs".to_owned()])];
        let left = [(PathBuf::from("/app"), vec!["b.rs".to_owned()])];
        // every path went to another task: nothing left to commit, so the task is done
        assert!(done_by_takeover(&[], &taken));
        // something still to commit, or nothing ever touched: not by takeover
        assert!(!done_by_takeover(&left, &taken));
        assert!(!done_by_takeover(&[], &[]));
        assert!(!done_by_takeover(&[], &[(PathBuf::from("/app"), vec![])]));
    }
}
