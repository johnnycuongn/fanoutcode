//! Memory folder commands: read and write a task's files, choose where a project keeps them, list missing folders.

use std::path::PathBuf;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use teamcode_core::domain::{Container, MemoryLocation, ProjectId, TaskId};
use teamcode_core::memory::{MemoryFile, TaskFolder};
use teamcode_core::{db, Error, Result};

use super::projects::{self, ProjectPatch};
use crate::events;
use crate::state::AppState;

fn memory_file(file: &str) -> Result<MemoryFile> {
    MemoryFile::parse(file).ok_or_else(|| Error::Invalid("Unknown memory file".into()))
}

/// The file's text; `None` when it or the task's folder is missing.
#[tauri::command]
#[specta::specta]
pub fn memory_read(state: State<'_, AppState>, task_id: TaskId, file: String) -> Result<Option<String>> {
    let id = TaskId::parse(task_id.as_str())?;
    let file = memory_file(&file)?;
    let task = db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    TaskFolder::open(task.memory_dir).read(file)
}

/// Writes one of the task's files (never `index.md`) and tells every window, so open Memory tabs re-render.
#[tauri::command]
#[specta::specta]
pub fn memory_write(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: TaskId,
    file: String,
    contents: String,
) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    let file = memory_file(&file)?;
    let conn = state.db.lock().expect("db lock");
    let task = db::tasks::require(&conn, &id)?;
    TaskFolder::open(task.memory_dir.clone()).write(file, &contents)?;
    events::emit_task_changed(&app, &conn, &task)
}

/// Asks for a folder and makes it the project's memory location; `None` when the picker is cancelled.
#[tauri::command]
#[specta::specta]
pub async fn memory_pick_location(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
) -> Result<Option<PathBuf>> {
    let id = ProjectId::parse(project_id.as_str())?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose where this project keeps task memory")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    let Some(folder) = rx
        .await
        .map_err(|_| Error::Provider("The folder picker closed unexpectedly".into()))?
    else {
        return Ok(None);
    };
    let path = folder
        .into_path()
        .map_err(|e| Error::Invalid(format!("Not a local folder: {e}")))?;
    let patch = ProjectPatch {
        memory_location: Some(MemoryLocation::Folder { path }),
        ..ProjectPatch::default()
    };
    let project = projects::update(&state, &id, patch)?;
    match project.memory_location {
        MemoryLocation::Folder { path } => Ok(Some(path)),
        MemoryLocation::AppStorage => Ok(None),
    }
}

/// The container's tasks whose memory folder no longer exists; their cards show "memory missing".
#[tauri::command]
#[specta::specta]
pub fn memory_missing(state: State<'_, AppState>, container: Container) -> Result<Vec<TaskId>> {
    let tasks = db::tasks::list(&state.db.lock().expect("db lock"), &super::parse_container(&container)?)?;
    Ok(tasks
        .into_iter()
        .filter(|t| TaskFolder::open(t.memory_dir.clone()).is_missing())
        .map(|t| t.id)
        .collect())
}
