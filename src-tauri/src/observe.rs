//! Shared plumbing for the watchers and the observation commands: a task with its home root, its repos, and its
//! attributor. No lock here is held across an `.await`.

use std::path::{Path, PathBuf};

use fanoutcode_core::changes::{self, attributor::Attributor, AttributedFile};
use fanoutcode_core::domain::{Task, TaskId};
use fanoutcode_core::{db, Error, Result};

use crate::state::AppState;

/// The task and its home project's root.
pub fn task_and_home(state: &AppState, id: &TaskId) -> Result<(Task, PathBuf)> {
    let conn = state.db.lock().expect("db lock");
    let task = db::tasks::require(&conn, id)?;
    let project = db::projects::get(&conn, &task.project_id)?
        .ok_or_else(|| Error::Invalid(format!("Unknown project {}", task.project_id)))?;
    Ok((task, project.root))
}

/// The task's repos (`changes::repos_of`).
pub fn repos(task: &Task, home: &Path) -> Vec<PathBuf> {
    changes::repos_of(task, home)
}

/// One of the task's repos as the webview named it, matched exactly or after canonicalising both sides.
pub fn checked_repo(repos: &[PathBuf], repo: &Path) -> Result<PathBuf> {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_owned());
    let wanted = canon(repo);
    repos
        .iter()
        .find(|r| r.as_path() == repo || canon(r) == wanted)
        .cloned()
        .ok_or_else(|| Error::Invalid("That repository is not part of this task.".into()))
}

/// Runs `f` on the task's attributor, loading it from `task_files` on first use and rebuilding it (files kept)
/// when the task's repos changed.
pub fn with_attributor<R>(
    state: &AppState,
    task_id: &TaskId,
    repos: &[PathBuf],
    f: impl FnOnce(&mut Attributor) -> R,
) -> Result<R> {
    let current = state
        .attributors
        .lock()
        .expect("attributors lock")
        .get(task_id)
        .map(|a| a.repos() == repos);
    if current != Some(true) {
        let kept: Option<Vec<AttributedFile>> = match current {
            Some(false) => state
                .attributors
                .lock()
                .expect("attributors lock")
                .remove(task_id)
                .map(|a| a.files().to_vec()),
            _ => None,
        };
        let files = match kept {
            Some(f) => f,
            None => changes::store::load(&state.db.lock().expect("db lock"), task_id)?,
        };
        state
            .attributors
            .lock()
            .expect("attributors lock")
            .insert(task_id.clone(), Attributor::with_files(repos.to_vec(), files));
    }
    let mut all = state.attributors.lock().expect("attributors lock");
    let a = all.get_mut(task_id).expect("inserted above");
    Ok(f(a))
}

/// Persists the task's attributions.
pub fn save_attributions(state: &AppState, task_id: &TaskId, files: &[AttributedFile]) -> Result<()> {
    changes::store::save(&state.db.lock().expect("db lock"), task_id, files)
}

/// Logs a background failure; the watchers never stop on one.
pub fn log_err<T>(what: &str, r: Result<T>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("Fanout Code: {what}: {e}");
            None
        }
    }
}
