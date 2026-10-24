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
}
