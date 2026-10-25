//! Overlap commands (Plan 11, §5.4): a task's overlap rows, the developer's decision on one, and the tool input a
//! pending decision stopped. The decision flow itself is `overlap_sync::decide`.

use std::path::PathBuf;

use fanoutcode_core::changes;
use fanoutcode_core::domain::TaskId;
use fanoutcode_core::overlap::{store, Decision, OverlapRow, PendingDecision};
use fanoutcode_core::Result;
use tauri::{AppHandle, State};

use crate::observe;
use crate::overlap_sync;
use crate::state::AppState;

/// Every overlap of the task, by repo, path and other task.
#[tauri::command]
#[specta::specta]
pub fn overlap_list(state: State<'_, AppState>, task_id: TaskId) -> Result<Vec<OverlapRow>> {
    let id = TaskId::parse(task_id.as_str())?;
    store::list(&state.db.lock().expect("db lock"), &id)
}

/// The developer's answer to the open decision on `path` of `repo` against `other_task` (`None`: another session).
#[tauri::command]
#[specta::specta]
pub async fn overlap_decide(
    app: AppHandle,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
    other_task: Option<TaskId>,
    decision: Decision,
) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    let other = other_task.map(|o| TaskId::parse(o.as_str())).transpose()?;
    overlap_sync::decide(&app, &id, &repo, &path, other.as_ref(), decision).await
}

/// What the session was about to do when it stopped on `path` of `repo` against `other_task` (`None`: another
/// session): the edit's tool input, or the command.
#[tauri::command]
#[specta::specta]
pub fn overlap_pending_input(
    state: State<'_, AppState>,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
    other_task: Option<TaskId>,
) -> Result<Option<PendingDecision>> {
    let id = TaskId::parse(task_id.as_str())?;
    let other = other_task.map(|o| TaskId::parse(o.as_str())).transpose()?;
    let (task, home) = observe::task_and_home(&state, &id)?;
    let repo = observe::checked_repo(&observe::repos(&task, &home), &repo)?;
    changes::checked_path(&repo, &path)?;
    let rows = store::pending(&state.db.lock().expect("db lock"), &id)?;
    Ok(rows
        .into_iter()
        .find(|r| r.repo_root == repo && r.path == path && r.other_task == other)
        .and_then(|r| r.pending))
}
