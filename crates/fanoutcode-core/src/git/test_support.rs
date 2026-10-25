//! Temporary git repositories for tests in every module. Identity and signing are set in the repo, so a test never
//! depends on the developer's global git config.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `git init -q` with a local identity and one empty initial commit.
pub fn temp_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    for (k, v) in [
        ("user.name", "Fanout Code Test"),
        ("user.email", "test@fanoutcode.invalid"),
        ("commit.gpgsign", "false"),
    ] {
        git(dir.path(), &["config", k, v]);
    }
    git(dir.path(), &["commit", "-q", "--allow-empty", "-m", "initial"]);
    dir
}

/// Writes `name` (relative to the repo root) with `contents`.
pub fn write(repo: &TempDir, name: &str, contents: &str) {
    let path = repo.path().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

/// `git add -A` then a commit; returns the new HEAD.
pub fn commit_all(repo: &Path, msg: &str) -> String {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", msg]);
    git(repo, &["rev-parse", "HEAD"])
}
