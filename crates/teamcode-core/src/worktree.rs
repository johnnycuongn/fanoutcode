//! Both worktree conventions (§9): Claude Code's own (`claude --worktree <slug>`, under `.claude/worktrees/`) and
//! Team Code-managed (`git worktree add <worktrees-dir>/<slug> -b tc/<slug>`). Codex and shell tasks always use the
//! managed kind. Only the home project gets a worktree.

use std::path::{Path, PathBuf};

use crate::domain::{AgentKind, Task, WorktreeChoice, WorktreeConvention};
use crate::git;
use crate::{Error, Result};

/// The task's slug: its memory folder name without the `YYYY-MM-DD-` prefix.
pub fn slug_of(task: &Task) -> String {
    let name = task
        .memory_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.get(11..).filter(|s| !s.is_empty()).unwrap_or(&name).to_owned()
}

/// `tc/<slug>`.
pub fn branch_for(slug: &str) -> String {
    format!("tc/{slug}")
}

/// `<worktrees_dir>/<slug>`.
pub fn managed_path(worktrees_dir: &Path, slug: &str) -> PathBuf {
    worktrees_dir.join(slug)
}

/// `<root>/.worktrees`, the managed convention's default `worktrees_dir`.
pub fn default_worktrees_dir(root: &Path) -> PathBuf {
    root.join(".worktrees")
}

/// `<root>/.claude/worktrees/<slug>`, where Claude Code keeps its own.
pub fn claude_code_path(root: &Path, slug: &str) -> PathBuf {
    root.join(".claude").join("worktrees").join(slug)
}

/// Codex and Shell tasks always use the managed kind.
pub fn convention_for(kind: AgentKind, setting: WorktreeConvention) -> WorktreeConvention {
    match kind {
        AgentKind::ClaudeCode => setting,
        AgentKind::Codex | AgentKind::Shell => WorktreeConvention::TeamCodeManaged,
    }
}

/// How to launch a task's session: where, with which extra `claude` arguments, and what to create first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeLaunch {
    pub cwd: PathBuf,
    pub claude_args: Vec<String>,
    pub create: Option<ManagedCreate>,
}

/// A managed worktree to create before the launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedCreate {
    pub repo_root: PathBuf,
    pub path: PathBuf,
    pub branch: String,
}

fn resume_args(resume: Option<&str>) -> Vec<String> {
    resume
        .map(|id| vec!["-r".to_owned(), id.to_owned()])
        .unwrap_or_default()
}

/// The launch for `task` rooted at `root` (§9). Off: the root, resumed when asked. Claude Code: the root with
/// `--worktree <slug>`, or `-w <slug> -r <id>` when resuming; the name, never a recorded path, drives it, so a
/// worktree Claude Code removed is recreated. Managed: `<worktrees_dir>/<slug>`, created first when missing.
pub fn launch_plan(
    task: &Task,
    root: &Path,
    convention: WorktreeConvention,
    worktrees_dir: &Path,
    resume: Option<&str>,
) -> WorktreeLaunch {
    if task.worktree == WorktreeChoice::Off {
        return WorktreeLaunch {
            cwd: root.to_owned(),
            claude_args: resume_args(resume),
            create: None,
        };
    }
    let slug = slug_of(task);
    match convention {
        WorktreeConvention::ClaudeCode => {
            let claude_args = match resume {
                None => vec!["--worktree".to_owned(), slug],
                Some(id) => vec!["-w".to_owned(), slug, "-r".to_owned(), id.to_owned()],
            };
            WorktreeLaunch {
                cwd: root.to_owned(),
                claude_args,
                create: None,
            }
        }
        WorktreeConvention::TeamCodeManaged => {
            let path = managed_path(worktrees_dir, &slug);
            let create = (!path.exists()).then(|| ManagedCreate {
                repo_root: root.to_owned(),
                path: path.clone(),
                branch: branch_for(&slug),
            });
            WorktreeLaunch {
                cwd: path,
                claude_args: resume_args(resume),
                create,
            }
        }
    }
}

/// `git worktree add <path> -b <branch>`, or `git worktree add <path> <branch>` when the branch already exists.
pub async fn create_managed(c: &ManagedCreate) -> Result<()> {
    let path = c.path.to_string_lossy();
    let exists = git::run(
        &c.repo_root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{}", c.branch)],
    )
    .await
    .is_ok();
    if exists {
        git::run(&c.repo_root, &["worktree", "add", &path, &c.branch]).await?;
    } else {
        git::run(&c.repo_root, &["worktree", "add", &path, "-b", &c.branch]).await?;
    }
    Ok(())
}

/// `git worktree remove [--force] <path>`; without `force`, git refuses a dirty tree. Never automatic (§9).
pub async fn remove_managed(root: &Path, path: &Path, force: bool) -> Result<()> {
    let path = path.to_string_lossy();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path);
    git::run(root, &args).await.map(|_| ())
}

/// The per-task worktree flag may change only until the session first starts (§9).
pub fn can_change_flag(task: &Task) -> Result<()> {
    if task.primary_session.is_some() {
        return Err(Error::Precondition(
            "The worktree setting is fixed once a session has started. Use Start fresh session to change it.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::git::test_support::temp_repo;
    use crate::test_support::task_fixture;

    fn task_named(folder: &str, wt: WorktreeChoice) -> Task {
        let mut t = task_fixture("t-1a-1", "p-1a-1");
        t.memory_dir = PathBuf::from("/m").join(folder);
        t.worktree = wt;
        t
    }

    #[test]
    fn codex_and_shell_always_use_the_managed_kind() {
        assert_eq!(
            convention_for(AgentKind::Codex, WorktreeConvention::ClaudeCode),
            WorktreeConvention::TeamCodeManaged
        );
        assert_eq!(
            convention_for(AgentKind::ClaudeCode, WorktreeConvention::ClaudeCode),
            WorktreeConvention::ClaudeCode
        );
    }

    #[test]
    fn claude_code_convention_launches_with_worktree_and_resume_flags() {
        let t = task_named("2026-10-24-board-swimlanes", WorktreeChoice::On);
        let p = launch_plan(
            &t,
            Path::new("/repo"),
            WorktreeConvention::ClaudeCode,
            Path::new("/repo/.worktrees"),
            None,
        );
        assert_eq!(
            (p.cwd.as_path(), p.claude_args.clone(), p.create.is_none()),
            (
                Path::new("/repo"),
                vec!["--worktree".to_string(), "board-swimlanes".into()],
                true
            )
        );
        let p = launch_plan(
            &t,
            Path::new("/repo"),
            WorktreeConvention::ClaudeCode,
            Path::new("/repo/.worktrees"),
            Some("sid"),
        );
        assert_eq!(p.claude_args, vec!["-w", "board-swimlanes", "-r", "sid"]);
    }

    #[test]
    fn managed_convention_creates_under_worktrees_dir_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let t = task_named("2026-10-24-board-swimlanes", WorktreeChoice::On);
        let p = launch_plan(
            &t,
            Path::new("/repo"),
            WorktreeConvention::TeamCodeManaged,
            dir.path(),
            None,
        );
        assert_eq!(p.cwd, dir.path().join("board-swimlanes"));
        assert_eq!(p.create.unwrap().branch, "tc/board-swimlanes");
        assert!(p.claude_args.is_empty());
    }

    #[test]
    fn a_missing_claude_worktree_is_recreated_from_the_same_name() {
        let mut t = task_named("2026-10-24-board-swimlanes", WorktreeChoice::On);
        let a = launch_plan(
            &t,
            Path::new("/repo"),
            WorktreeConvention::ClaudeCode,
            Path::new("/x"),
            Some("sid"),
        );
        t.worktree_path = Some(PathBuf::from("/repo/.claude/worktrees/board-swimlanes")); // recorded earlier, since removed by Claude Code
        let b = launch_plan(
            &t,
            Path::new("/repo"),
            WorktreeConvention::ClaudeCode,
            Path::new("/x"),
            Some("sid"),
        );
        assert_eq!(a.claude_args, b.claude_args); // the name, never the recorded path, drives the launch
        assert_eq!(b.claude_args, vec!["-w", "board-swimlanes", "-r", "sid"]);
    }

    #[tokio::test]
    async fn create_managed_reuses_an_existing_branch() {
        let r = temp_repo();
        let wt = tempfile::tempdir().unwrap();
        let c = ManagedCreate {
            repo_root: r.path().into(),
            path: wt.path().join("s"),
            branch: "tc/s".into(),
        };
        create_managed(&c).await.unwrap();
        remove_managed(r.path(), &c.path, false).await.unwrap();
        create_managed(&c).await.unwrap(); // branch tc/s still exists: must not fail on -b
        assert!(c.path.join(".git").exists());
    }

    #[tokio::test]
    async fn remove_managed_refuses_a_dirty_tree_without_force() {
        let r = temp_repo();
        let wt = tempfile::tempdir().unwrap();
        let c = ManagedCreate {
            repo_root: r.path().into(),
            path: wt.path().join("s"),
            branch: "tc/s".into(),
        };
        create_managed(&c).await.unwrap();
        std::fs::write(c.path.join("dirty.txt"), "x").unwrap();
        assert!(matches!(
            remove_managed(r.path(), &c.path, false).await,
            Err(Error::Git(_))
        ));
        remove_managed(r.path(), &c.path, true).await.unwrap();
    }

    #[test]
    fn the_flag_is_frozen_once_a_session_exists() {
        let mut t = task_named("2026-10-24-x", WorktreeChoice::Off);
        assert!(can_change_flag(&t).is_ok());
        t.primary_session = Some(SessionId::parse("s-1").unwrap());
        assert!(
            matches!(can_change_flag(&t), Err(Error::Precondition(m)) if m == "The worktree setting is fixed once a session has started. Use Start fresh session to change it.")
        );
    }
}
