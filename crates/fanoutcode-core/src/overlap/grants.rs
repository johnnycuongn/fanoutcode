//! The per-task grants file (§5.2): `<app_data>/grants/<task-id>.json`, the developer's answers that let this task
//! edit on top of another task's claim. Written by the app when the developer decides, read by the claim-check hook,
//! pruned once the claims it answered are gone (§4).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::claims::ClaimsFile;
use super::CommandKind;
use crate::domain::TaskId;
use crate::memory::store::write_atomic;
use crate::Result;

/// What a grant covers: one file of one repo or every file the other task claims (edits only), or one run of one
/// exact shell command over the other task's claims (the git guard, §5.6; edit grants never allow a command).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum GrantScope {
    File {
        repo: PathBuf,
        path: String,
    },
    AllFiles,
    /// Single use: taken out of the grants file before the hook allows the command (`take_commands`). `repo` is
    /// the repo the denied command ran in; the grant matches the command only there. `paths` are the other task's
    /// files the command touched when the developer answered: a spend fails once that set has changed, and once the
    /// grant is older than `COMMAND_GRANT_MINUTES` (the `Grant`'s `granted_at`).
    Command {
        repo: PathBuf,
        command: String,
        #[serde(default)]
        paths: Vec<String>,
    },
}

/// One answer: this task may edit on top of `other_task`'s claims in `scope` (`other_task: None`: an unattributed
/// *another session* claim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Grant {
    pub other_task: Option<TaskId>,
    pub scope: GrantScope,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct GrantsFile {
    pub grants: Vec<Grant>,
}

/// `<app_data>/grants/<task-id>.json`. The id was validated when it was parsed, so it is safe in a path.
pub fn grants_path(app_data: &Path, task_id: &TaskId) -> PathBuf {
    app_data.join("grants").join(format!("{}.json", task_id.as_str()))
}

/// The grants at `path`; empty when the file is missing or does not parse (the hook then asks again, never allows
/// more than it was told).
pub fn read_grants(path: &Path) -> GrantsFile {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Writes `file` atomically to the task's `grants_path` (the id re-validated, since it becomes a file name).
pub fn write_grants(app_data: &Path, task_id: &TaskId, file: &GrantsFile) -> Result<PathBuf> {
    let id = TaskId::parse(task_id.as_str())?;
    let path = grants_path(app_data, &id);
    write_atomic(&path, &serde_json::to_vec(file)?)?;
    Ok(path)
}

impl GrantsFile {
    /// Whether a grant lets this task write `path` of `repo` over the claim of `other` (`None`: unattributed). A
    /// grant only ever covers claims of the task it names.
    pub fn covers(&self, repo: &Path, path: &str, other: Option<&TaskId>) -> bool {
        self.grants.iter().any(|g| {
            g.other_task.as_ref() == other
                && match &g.scope {
                    GrantScope::AllFiles => true,
                    GrantScope::File { repo: r, path: p } => r == repo && p == path,
                    GrantScope::Command { .. } => false,
                }
        })
    }

    /// Whether a `Command` grant lets this task run exactly `command` (both trimmed) in `repo` over the claims of
    /// `other`.
    pub fn allows_command(&self, command: &str, repo: &Path, other: Option<&TaskId>) -> bool {
        self.grants.iter().any(|g| command_matches(g, command, repo, other))
    }

    /// Adds `grant`, replacing an earlier one with the same other task and scope.
    pub fn add(&mut self, grant: Grant) {
        self.grants
            .retain(|g| !(g.other_task == grant.other_task && g.scope == grant.scope));
        self.grants.push(grant);
    }

    /// Drops grants whose claims are gone: a File grant whose path is no longer claimed by its other task, an
    /// AllFiles grant whose other task holds no claim in any of `claims`.
    pub fn prune(&mut self, claims: &[&ClaimsFile]) {
        self.grants.retain(|g| match &g.scope {
            GrantScope::File { repo, path } => claims
                .iter()
                .filter(|f| &f.repo_root == repo)
                .any(|f| f.claims.iter().any(|c| &c.path == path && c.task_id == g.other_task)),
            GrantScope::AllFiles | GrantScope::Command { .. } => claims
                .iter()
                .any(|f| f.claims.iter().any(|c| c.task_id == g.other_task)),
        });
    }
}

fn command_matches(g: &Grant, command: &str, repo: &Path, other: Option<&TaskId>) -> bool {
    g.other_task.as_ref() == other
        && matches!(&g.scope, GrantScope::Command { repo: r, command: c, .. } if r == repo && c.trim() == command.trim())
}

/// How long a `Command` grant may wait to be spent.
pub const COMMAND_GRANT_MINUTES: i64 = 10;

fn sorted(paths: &[String]) -> Vec<&str> {
    let mut v: Vec<&str> = paths.iter().map(String::as_str).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Whether `g` may be spent on `need` at `now`: the same command, repo and other task, the same set of that task's
/// files touched now as when it was granted, and not yet expired.
fn spendable(g: &Grant, command: &str, need: &CommandNeed, now: DateTime<Utc>) -> bool {
    let GrantScope::Command { paths, .. } = &g.scope else {
        return false;
    };
    command_matches(g, command, &need.repo, need.other.as_ref())
        && sorted(paths) == sorted(&need.paths)
        && now - g.granted_at <= chrono::Duration::minutes(COMMAND_GRANT_MINUTES)
}

/// One `Command` grant a Bash call needs: the repo, the other task whose claims it touches there, those claimed
/// paths now, and what the command does to them (logged in `CommandRan` once spent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandNeed {
    pub repo: PathBuf,
    pub other: Option<TaskId>,
    pub paths: Vec<String>,
    pub kind: CommandKind,
}

/// The `Command` grants of `file` the go-ahead typed at `delivered_at` covered (granted then or before): unspent at
/// the session's next main-agent `Stop`, they are dropped, so a later run of the command is asked about again.
pub fn stale_command_grants(file: &GrantsFile, delivered_at: DateTime<Utc>) -> Vec<Grant> {
    file.grants
        .iter()
        .filter(|g| matches!(g.scope, GrantScope::Command { .. }) && g.granted_at <= delivered_at)
        .cloned()
        .collect()
}

/// `<grants>.lock`, the sibling every writer of the grants file locks.
fn lock_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".lock");
    PathBuf::from(p)
}

/// Runs `f` holding the exclusive advisory lock on `<path>.lock` (created, with its directory, when missing). Every
/// read-modify-write of a grants file goes through it: the app's decisions and prunes, and the hook's
/// `take_command`. The lock is released when `f` returns.
pub fn with_grants_lock<T>(path: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let lock = lock_path(path);
    if let Some(dir) = lock.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock)?;
    file.lock()?;
    let out = f();
    let _ = file.unlock();
    out
}

/// Takes one `Command` grant for each of `needed` out of the grants file at `path`, under the lock: re-read, every
/// one present and `spendable` at `now`, removed, written atomically. True only when all were there and the write
/// succeeded; otherwise the file is left as it was and the hook denies.
pub fn take_commands(path: &Path, command: &str, needed: &[CommandNeed], now: DateTime<Utc>) -> bool {
    let taken = with_grants_lock(path, || {
        let mut file = read_grants(path);
        for need in needed {
            let Some(i) = file.grants.iter().position(|g| spendable(g, command, need, now)) else {
                return Ok(false);
            };
            file.grants.remove(i);
        }
        write_atomic(path, &serde_json::to_vec(&file)?)?;
        Ok(true)
    });
    taken.unwrap_or_else(|e| {
        eprintln!(
            "fanoutcode claim-check: cannot spend a grant in {}: {e}",
            path.display()
        );
        false
    })
}

/// `take_commands` for one repo and other task.
pub fn take_command(
    path: &Path,
    command: &str,
    repo: &Path,
    other: Option<&TaskId>,
    paths: &[String],
    now: DateTime<Utc>,
) -> bool {
    let need = CommandNeed {
        repo: repo.to_owned(),
        other: other.cloned(),
        paths: paths.to_vec(),
        kind: CommandKind::Discards,
    };
    take_commands(path, command, &[need], now)
}

#[cfg(test)]
#[path = "grants_tests.rs"]
mod tests;
