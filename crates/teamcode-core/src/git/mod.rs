//! Git through the `git` binary (§9), never a bundled library, so hooks, LFS and credential helpers behave as the
//! developer expects.

use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{Error, Result};

/// `git -C <repo> <args…>` with stdout captured. A non-zero exit is `Error::Git` with git's stderr.
pub async fn run(repo: &Path, args: &[&str]) -> Result<String> {
    run_with_input(repo, args, None).await
}

/// As `run`, writing `input` to git's stdin first (`commit -F -`).
pub async fn run_with_input(repo: &Path, args: &[&str], input: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(args)
        // a parent git process (a hook, a test run inside one) must not redirect us to its repository
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| Error::Git(format!("Couldn't run git: {e}")))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(text.as_bytes()).await?;
    }
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(Error::Git(if err.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            err
        }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The commit `HEAD` points at.
pub async fn head(repo: &Path) -> Result<String> {
    Ok(run(repo, &["rev-parse", "HEAD"]).await?.trim().to_owned())
}

/// `path` is inside a git work tree.
pub async fn is_repo(path: &Path) -> bool {
    run(path, &["rev-parse", "--is-inside-work-tree"])
        .await
        .is_ok_and(|o| o.trim() == "true")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

/// One `git status` entry; `orig_path` is the old name of a rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub path: String,
    pub status: FileStatus,
    pub orig_path: Option<String>,
}

fn status_of(xy: &str) -> FileStatus {
    if xy.contains('R') || xy.contains('C') {
        FileStatus::Renamed
    } else if xy == "??" {
        FileStatus::Untracked
    } else if xy.contains('A') {
        FileStatus::Added
    } else if xy.contains('D') {
        FileStatus::Deleted
    } else {
        FileStatus::Modified
    }
}

/// Parses `status --porcelain=v1 -z`: `XY path\0`, a rename `XY new\0old\0`.
fn parse_status(out: &str) -> Vec<StatusEntry> {
    let mut entries = Vec::new();
    let mut records = out.split('\0').filter(|r| !r.is_empty());
    while let Some(rec) = records.next() {
        if rec.len() < 4 {
            continue;
        }
        let (xy, path) = (&rec[..2], &rec[3..]);
        let status = status_of(xy);
        let orig_path = (status == FileStatus::Renamed)
            .then(|| records.next().map(str::to_owned))
            .flatten();
        entries.push(StatusEntry {
            path: path.to_owned(),
            status,
            orig_path,
        });
    }
    entries
}

/// The working tree and index against `HEAD`, untracked files listed one by one.
pub async fn status(repo: &Path) -> Result<Vec<StatusEntry>> {
    Ok(parse_status(
        &run(repo, &["status", "--porcelain=v1", "-z", "--untracked-files=all"]).await?,
    ))
}

/// Lines added and removed in one file; `binary` files count 0 and 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Numstat {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub binary: bool,
}

/// Parses `diff --numstat -z`: `a\tr\tpath\0`, a rename `a\tr\t\0old\0new\0`, binary `-\t-\t…`.
fn parse_numstat(out: &str) -> Vec<Numstat> {
    let mut stats = Vec::new();
    let mut records = out.split('\0');
    while let Some(rec) = records.next() {
        let mut f = rec.splitn(3, '\t');
        let (Some(a), Some(r), Some(path)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let path = if path.is_empty() {
            let _old = records.next();
            records.next().unwrap_or_default().to_owned()
        } else {
            path.to_owned()
        };
        let binary = a == "-" && r == "-";
        stats.push(Numstat {
            path,
            added: a.parse().unwrap_or(0),
            removed: r.parse().unwrap_or(0),
            binary,
        });
    }
    stats
}

/// Line counts of the working tree against `base` (`HEAD` when `None`), plus every untracked file counted as added
/// (newlines, as `wc -l` counts them; a file with a NUL byte is binary).
pub async fn numstat(repo: &Path, base: Option<&str>) -> Result<Vec<Numstat>> {
    let mut stats = parse_numstat(&run(repo, &["diff", "--numstat", "-z", base.unwrap_or("HEAD")]).await?);
    for e in status(repo).await? {
        if e.status != FileStatus::Untracked || stats.iter().any(|s| s.path == e.path) {
            continue;
        }
        let bytes = tokio::fs::read(repo.join(&e.path)).await.unwrap_or_default();
        let binary = bytes.contains(&0);
        stats.push(Numstat {
            added: if binary {
                0
            } else {
                bytes.iter().filter(|&&b| b == b'\n').count() as u32
            },
            path: e.path,
            removed: 0,
            binary,
        });
    }
    Ok(stats)
}

/// The file at `rev`; `None` when the path (or the revision) does not exist there.
pub async fn show_at(repo: &Path, rev: &str, path: &str) -> Result<Option<String>> {
    let spec = format!("{rev}:{path}");
    if run(repo, &["cat-file", "-e", &spec]).await.is_err() {
        return Ok(None);
    }
    Ok(Some(run(repo, &["show", &spec]).await?))
}

/// HEAD of every repo, in order: the task's base, recorded when it enters Build (§9).
pub async fn record_base(repos: &[std::path::PathBuf]) -> Result<Vec<crate::domain::RepoBase>> {
    let mut bases = Vec::with_capacity(repos.len());
    for repo in repos {
        bases.push(crate::domain::RepoBase {
            repo_root: repo.clone(),
            commit: head(repo).await?,
        });
    }
    Ok(bases)
}

/// The base a repo's diff is taken against (§9): the recorded base when `repo_root` is the task's own worktree,
/// else `head` ("or HEAD when no worktree is used and the developer has committed in between").
pub fn effective_base(task: &crate::domain::Task, repo_root: &Path, head: &str) -> Option<String> {
    let own_worktree =
        task.worktree == crate::domain::WorktreeChoice::On && task.worktree_path.as_deref() == Some(repo_root);
    if own_worktree {
        if let Some(b) = task.base.iter().find(|b| b.repo_root == repo_root) {
            return Some(b.commit.clone());
        }
    }
    Some(head.to_owned())
}

/// True when none of `task_files` is still pending in the working tree or index: each is committed or was undone,
/// so nothing is left for the developer to commit. An empty list is true. The base plays no part.
pub async fn commit_done(repo: &Path, task_files: &[String]) -> Result<bool> {
    if task_files.is_empty() {
        return Ok(true);
    }
    Ok(!status(repo)
        .await?
        .iter()
        .any(|e| task_files.contains(&e.path) || e.orig_path.as_ref().is_some_and(|o| task_files.contains(o))))
}

/// What a file-system event in a watched repo means for the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchHit {
    /// Nothing that changes the task's diff (git objects, lock files).
    None,
    /// The tree, the index or a ref: recompute the change set.
    Tree,
    /// Only the task's memory folder inside the repo changed (the spec watcher's own write, Review Focus 4).
    MemoryOnly,
}

/// A path inside a git directory: `HEAD`, `index` and `refs/**` are `Tree` so commits are noticed; objects, lock
/// files and the rest are `None`.
pub fn git_dir_hit(changed: &Path, git_dir: &Path) -> WatchHit {
    let Ok(rel) = changed.strip_prefix(git_dir) else {
        return WatchHit::None;
    };
    if changed.extension().is_some_and(|e| e == "lock") || rel.starts_with("objects") {
        return WatchHit::None;
    }
    if rel == Path::new("HEAD")
        || rel == Path::new("index")
        || rel.starts_with("refs")
        || rel == Path::new("packed-refs")
    {
        WatchHit::Tree
    } else {
        WatchHit::None
    }
}

/// Classifies a changed path of `repo`: inside `.git` by `git_dir_hit`, under `memory_dir` `MemoryOnly`, any other
/// path in the repo `Tree`, a path outside it `None`. (`*.lock` is ignored only inside `.git`: `Cargo.lock` is a
/// real change.)
pub fn watch_filter(changed: &Path, repo: &Path, memory_dir: &Path) -> WatchHit {
    let git_dir = repo.join(".git");
    if changed.starts_with(&git_dir) {
        git_dir_hit(changed, &git_dir)
    } else if changed.starts_with(memory_dir) {
        WatchHit::MemoryOnly
    } else if changed.starts_with(repo) {
        WatchHit::Tree
    } else {
        WatchHit::None
    }
}

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests {
    use super::test_support::{commit_all, temp_repo, write};
    use super::*;
    use crate::Error;

    fn kinds(s: &[StatusEntry]) -> Vec<(&str, FileStatus)> {
        s.iter().map(|e| (e.path.as_str(), e.status)).collect()
    }

    fn find(n: &[Numstat], path: &str) -> (u32, u32, bool) {
        let e = n.iter().find(|e| e.path == path).unwrap();
        (e.added, e.removed, e.binary)
    }

    #[tokio::test]
    async fn status_parses_modified_added_deleted_and_untracked() {
        let r = temp_repo();
        write(&r, "a.txt", "1");
        write(&r, "d.txt", "x");
        commit_all(r.path(), "base");
        write(&r, "a.txt", "2");
        std::fs::remove_file(r.path().join("d.txt")).unwrap();
        write(&r, "n.txt", "new");
        let s = status(r.path()).await.unwrap();
        assert_eq!(
            kinds(&s),
            vec![
                ("a.txt", FileStatus::Modified),
                ("d.txt", FileStatus::Deleted),
                ("n.txt", FileStatus::Untracked)
            ]
        );
    }

    #[tokio::test]
    async fn status_handles_spaces_and_renames() {
        let r = temp_repo();
        write(&r, "my file.txt", "1");
        commit_all(r.path(), "base");
        run(r.path(), &["mv", "my file.txt", "their file.txt"]).await.unwrap();
        let s = status(r.path()).await.unwrap();
        assert_eq!(
            (s[0].path.as_str(), s[0].status, s[0].orig_path.as_deref()),
            ("their file.txt", FileStatus::Renamed, Some("my file.txt"))
        );
    }

    #[tokio::test]
    async fn numstat_counts_against_a_base_and_marks_binary() {
        let r = temp_repo();
        write(&r, "a.txt", "1\n2\n");
        let base = commit_all(r.path(), "base");
        write(&r, "a.txt", "1\n3\n4\n");
        std::fs::write(r.path().join("b.bin"), [0u8, 159, 146, 150]).unwrap();
        run(r.path(), &["add", "b.bin"]).await.unwrap();
        let n = numstat(r.path(), Some(&base)).await.unwrap();
        assert_eq!(find(&n, "a.txt"), (2, 1, false));
        assert_eq!(find(&n, "b.bin"), (0, 0, true));
    }

    #[tokio::test]
    async fn show_at_returns_none_for_a_path_absent_at_base() {
        let r = temp_repo();
        let base = head(r.path()).await.unwrap();
        write(&r, "a.txt", "1");
        commit_all(r.path(), "c");
        assert_eq!(show_at(r.path(), &base, "a.txt").await.unwrap(), None);
        assert_eq!(show_at(r.path(), "HEAD", "a.txt").await.unwrap().as_deref(), Some("1"));
    }

    #[tokio::test]
    async fn errors_carry_gits_stderr_as_error_git() {
        let r = temp_repo();
        assert!(
            matches!(run(r.path(), &["checkout", "no-such-branch"]).await, Err(Error::Git(m)) if m.contains("no-such-branch"))
        );
    }

    #[tokio::test]
    async fn record_base_stores_head_per_repo() {
        let (a, b) = (temp_repo(), temp_repo());
        let bases = record_base(&[a.path().to_path_buf(), b.path().to_path_buf()])
            .await
            .unwrap();
        assert_eq!(bases.len(), 2);
        assert_eq!(bases[0].commit, head(a.path()).await.unwrap());
    }

    fn task_with(wt: crate::domain::WorktreeChoice, worktree_path: Option<&str>, base: &str) -> crate::domain::Task {
        let mut t = crate::test_support::task_fixture("t-1a-1", "p-1a-1");
        t.worktree = wt;
        t.worktree_path = worktree_path.map(Into::into);
        t.base = vec![crate::domain::RepoBase {
            repo_root: worktree_path.unwrap_or("/repo").into(),
            commit: base.into(),
        }];
        t
    }

    #[test]
    fn effective_base_moves_to_head_without_a_worktree() {
        use crate::domain::WorktreeChoice;
        let t = task_with(WorktreeChoice::Off, None, "aaa");
        assert_eq!(effective_base(&t, Path::new("/repo"), "bbb").as_deref(), Some("bbb"));
        let t = task_with(WorktreeChoice::On, Some("/repo/.worktrees/x"), "aaa");
        assert_eq!(
            effective_base(&t, Path::new("/repo/.worktrees/x"), "bbb").as_deref(),
            Some("aaa")
        );
    }

    #[tokio::test]
    async fn commit_done_needs_every_task_file_committed() {
        let r = temp_repo();
        write(&r, "a.rs", "1");
        write(&r, "b.rs", "2");
        run(r.path(), &["add", "a.rs"]).await.unwrap();
        run(r.path(), &["commit", "-qm", "a"]).await.unwrap();
        assert!(!commit_done(r.path(), &["a.rs".into(), "b.rs".into()]).await.unwrap());
        run(r.path(), &["add", "b.rs"]).await.unwrap();
        run(r.path(), &["commit", "-qm", "b"]).await.unwrap();
        assert!(commit_done(r.path(), &["a.rs".into(), "b.rs".into()]).await.unwrap());
    }

    #[tokio::test]
    async fn commit_done_is_false_while_a_task_file_is_dirty() {
        let r = temp_repo();
        write(&r, "a.rs", "1");
        commit_all(r.path(), "a");
        write(&r, "a.rs", "2");
        assert!(!commit_done(r.path(), &["a.rs".into()]).await.unwrap());
    }

    #[tokio::test]
    async fn a_set_whose_edits_were_reverted_counts_as_clean() {
        let r = temp_repo();
        write(&r, "a.rs", "1");
        commit_all(r.path(), "a");
        write(&r, "a.rs", "2");
        write(&r, "a.rs", "1");
        assert!(commit_done(r.path(), &["a.rs".into()]).await.unwrap());
        assert!(commit_done(r.path(), &[]).await.unwrap());
    }

    #[test]
    fn watch_filter_classifies_paths() {
        let (repo, mem) = (Path::new("/repo"), Path::new("/repo/docs/tasks/2026-10-24-x"));
        assert_eq!(
            watch_filter(Path::new("/repo/.git/objects/ab/cd"), repo, mem),
            WatchHit::None
        );
        assert_eq!(
            watch_filter(Path::new("/repo/.git/index.lock"), repo, mem),
            WatchHit::None
        );
        assert_eq!(
            watch_filter(Path::new("/repo/.git/refs/heads/main"), repo, mem),
            WatchHit::Tree
        );
        assert_eq!(
            watch_filter(Path::new("/repo/docs/tasks/2026-10-24-x/spec.md"), repo, mem),
            WatchHit::MemoryOnly
        );
        assert_eq!(watch_filter(Path::new("/repo/src/a.rs"), repo, mem), WatchHit::Tree);
    }
}
