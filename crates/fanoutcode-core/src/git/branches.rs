//! Branch questions for "Bring <base> in" (Plan 11, §6.3): the checked-out branch, the repo's default branch, a
//! merge or rebase stopped on conflicts (and its unmerged paths), and the merge-base of two revisions. Re-exported
//! from `git`.

use std::path::Path;

use super::run;
use crate::Result;

/// The branch `HEAD` is on; `None` when detached.
pub async fn current_branch(repo: &Path) -> Result<Option<String>> {
    let name = run(repo, &["branch", "--show-current"]).await?.trim().to_owned();
    Ok((!name.is_empty()).then_some(name))
}

async fn has_ref(repo: &Path, name: &str) -> bool {
    run(repo, &["rev-parse", "--verify", "--quiet", name]).await.is_ok()
}

/// `origin/HEAD`'s target, else `main`, else `master`, else `None`.
pub async fn default_branch(repo: &Path) -> Result<Option<String>> {
    if let Ok(target) = run(repo, &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"]).await {
        if let Some(name) = target.trim().strip_prefix("refs/remotes/origin/") {
            return Ok(Some(name.to_owned()));
        }
    }
    for name in ["main", "master"] {
        if has_ref(repo, &format!("refs/heads/{name}")).await {
            return Ok(Some(name.to_owned()));
        }
    }
    Ok(None)
}

const UNMERGED: [&str; 7] = ["UU", "AA", "DD", "AU", "UA", "DU", "UD"];

/// A merge or rebase is in progress (`MERGE_HEAD` or `REBASE_HEAD`), or `status` lists an unmerged path.
pub async fn has_conflicts(repo: &Path) -> Result<bool> {
    for head in ["MERGE_HEAD", "REBASE_HEAD"] {
        if has_ref(repo, head).await {
            return Ok(true);
        }
    }
    Ok(!conflicted_paths(repo).await?.is_empty())
}

/// The unmerged paths `status` lists (both sides changed, or one deleted), in its order.
pub async fn conflicted_paths(repo: &Path) -> Result<Vec<String>> {
    let out = run(repo, &["status", "--porcelain=v1", "-z"]).await?;
    let mut records = out.split('\0').filter(|r| !r.is_empty());
    let mut paths = Vec::new();
    while let Some(rec) = records.next() {
        let Some(xy) = rec.get(..2) else { continue };
        if UNMERGED.contains(&xy) {
            if let Some(path) = rec.get(3..) {
                paths.push(path.to_owned());
            }
        } else if xy.contains('R') || xy.contains('C') {
            records.next(); // a rename's old name is a bare path record
        }
    }
    Ok(paths)
}

/// `git merge-base <a> <b>`.
pub async fn merge_base(repo: &Path, a: &str, b: &str) -> Result<String> {
    Ok(run(repo, &["merge-base", a, b]).await?.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{commit_all, temp_repo, write};
    use super::*;

    async fn git(repo: &Path, args: &[&str]) -> String {
        run(repo, args).await.unwrap().trim().to_owned()
    }

    #[tokio::test]
    async fn current_branch_reads_head() {
        let r = temp_repo();
        git(r.path(), &["checkout", "-q", "-b", "feature/x"]).await;
        assert_eq!(current_branch(r.path()).await.unwrap().as_deref(), Some("feature/x"));
        git(r.path(), &["checkout", "-q", "--detach"]).await;
        assert_eq!(current_branch(r.path()).await.unwrap(), None);
    }

    #[tokio::test]
    async fn default_branch_prefers_origin_head_then_main_then_master() {
        let r = temp_repo();
        git(r.path(), &["branch", "-m", "work"]).await;
        assert_eq!(default_branch(r.path()).await.unwrap(), None);
        git(r.path(), &["branch", "master"]).await;
        assert_eq!(default_branch(r.path()).await.unwrap().as_deref(), Some("master"));
        git(r.path(), &["branch", "main"]).await;
        assert_eq!(default_branch(r.path()).await.unwrap().as_deref(), Some("main"));
        git(r.path(), &["update-ref", "refs/remotes/origin/trunk", "HEAD"]).await;
        git(
            r.path(),
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"],
        )
        .await;
        assert_eq!(default_branch(r.path()).await.unwrap().as_deref(), Some("trunk"));
    }

    #[tokio::test]
    async fn has_conflicts_is_true_mid_merge() {
        let r = temp_repo();
        write(&r, "a.txt", "base\n");
        commit_all(r.path(), "base");
        let home = current_branch(r.path()).await.unwrap().unwrap();
        assert!(!has_conflicts(r.path()).await.unwrap());
        git(r.path(), &["checkout", "-q", "-b", "other"]).await;
        write(&r, "a.txt", "theirs\n");
        commit_all(r.path(), "theirs");
        git(r.path(), &["checkout", "-q", &home]).await;
        write(&r, "a.txt", "ours\n");
        commit_all(r.path(), "ours");
        assert!(run(r.path(), &["merge", "-q", "other"]).await.is_err());
        assert!(has_conflicts(r.path()).await.unwrap());
        assert_eq!(conflicted_paths(r.path()).await.unwrap(), ["a.txt"]);
        git(r.path(), &["merge", "--abort"]).await;
        assert!(!has_conflicts(r.path()).await.unwrap());
        assert!(conflicted_paths(r.path()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn merge_base_of_two_branches() {
        let r = temp_repo();
        write(&r, "a.txt", "1");
        let fork = commit_all(r.path(), "fork");
        let home = current_branch(r.path()).await.unwrap().unwrap();
        git(r.path(), &["checkout", "-q", "-b", "side"]).await;
        write(&r, "b.txt", "2");
        commit_all(r.path(), "side");
        git(r.path(), &["checkout", "-q", &home]).await;
        write(&r, "c.txt", "3");
        commit_all(r.path(), "home");
        assert_eq!(merge_base(r.path(), &home, "side").await.unwrap(), fork);
    }
}
