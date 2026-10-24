//! Git commands for the Changes tab (§6.4, §9). Every task id is re-parsed, every repo must be one of the task's,
//! and every path goes through `changes::checked_path`.

use std::path::PathBuf;

use tauri::{AppHandle, State};
use teamcode_core::changes::{self, FileDiff, RepoChanges};
use teamcode_core::domain::TaskId;
use teamcode_core::git::{self, FileStatus};
use teamcode_core::memory::store::write_atomic;
use teamcode_core::Result;

use crate::events::{self, GitChanges};
use crate::git_watcher;
use crate::observe;
use crate::state::AppState;

/// The task, its repo as named by the webview (validated), and the file's absolute path.
fn target(
    state: &AppState,
    task_id: &TaskId,
    repo: &std::path::Path,
    path: &str,
) -> Result<(teamcode_core::domain::Task, PathBuf, PathBuf)> {
    let id = TaskId::parse(task_id.as_str())?;
    let (task, home) = observe::task_and_home(state, &id)?;
    let repo = observe::checked_repo(&observe::repos(&task, &home), repo)?;
    let abs = changes::checked_path(&repo, path)?;
    Ok((task, repo, abs))
}

/// The task's change set, rebuilt now.
#[tauri::command]
#[specta::specta]
pub async fn git_changes(app: AppHandle, task_id: TaskId) -> Result<Vec<RepoChanges>> {
    let id = TaskId::parse(task_id.as_str())?;
    Ok(git_watcher::refresh(&app, &id).await?.1.repos)
}

/// Both sides of one file: `before` at the repo's base (empty when absent there), `after` the working tree.
#[tauri::command]
#[specta::specta]
pub async fn git_diff(state: State<'_, AppState>, task_id: TaskId, repo: PathBuf, path: String) -> Result<FileDiff> {
    let (task, repo, abs) = target(&state, &task_id, &repo, &path)?;
    let entry = git::status(&repo).await?.into_iter().find(|e| e.path == path);
    let base = match git::head(&repo).await {
        Ok(head) => git::effective_base(&task, &repo, &head),
        Err(_) => None,
    };
    let before_path = entry
        .as_ref()
        .and_then(|e| e.orig_path.clone())
        .unwrap_or_else(|| path.clone());
    let before_bytes = match &base {
        Some(b) => git::show_at(&repo, b, &before_path).await?.unwrap_or_default(),
        None => String::new(),
    };
    let after_bytes = tokio::fs::read(&abs).await.unwrap_or_default();
    let binary = before_bytes.contains('\0') || after_bytes.contains(&0);
    Ok(FileDiff {
        repo_root: repo,
        status: entry.map_or(FileStatus::Modified, |e| e.status),
        before: if binary { String::new() } else { before_bytes },
        after: if binary {
            String::new()
        } else {
            String::from_utf8_lossy(&after_bytes).into_owned()
        },
        path,
        binary,
    })
}

/// The working-tree file's text.
#[tauri::command]
#[specta::specta]
pub async fn git_file_read(state: State<'_, AppState>, task_id: TaskId, repo: PathBuf, path: String) -> Result<String> {
    let (_, _, abs) = target(&state, &task_id, &repo, &path)?;
    Ok(tokio::fs::read_to_string(&abs).await?)
}

/// Saves the working-tree side from the diff editor (atomic); the watcher recomputes the diff.
#[tauri::command]
#[specta::specta]
pub fn git_file_write(
    state: State<'_, AppState>,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
    contents: String,
) -> Result<()> {
    let (_, _, abs) = target(&state, &task_id, &repo, &path)?;
    write_atomic(&abs, contents.as_bytes())
}

/// Pulls a file from "Other changes" into the task's set.
#[tauri::command]
#[specta::specta]
pub fn git_include_other(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
) -> Result<()> {
    let (task, repo, _) = target(&state, &task_id, &repo, &path)?;
    let (_, home) = observe::task_and_home(&state, &task.id)?;
    let repos = observe::repos(&task, &home);
    let files = observe::with_attributor(&state, &task.id, &repos, |a| {
        a.include(&repo, &path);
        a.files().to_vec()
    })?;
    observe::save_attributions(&state, &task.id, &files)?;
    events::emit(&app, GitChanges { task_id: task.id })
}
