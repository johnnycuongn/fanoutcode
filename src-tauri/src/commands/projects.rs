//! Project commands: open (find or create by root), list, update, close.

use std::path::PathBuf;

use fanoutcode_core::domain::{Container, MemoryLocation, Project, ProjectColor, ProjectId};
use fanoutcode_core::{db, Error, Result};
use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;

/// The fields `projects_update` may change; `None` leaves a field as it is. A new memory location applies to tasks
/// created from now on; existing tasks keep their `memory_dir`.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub color: Option<ProjectColor>,
    pub memory_location: Option<MemoryLocation>,
}

/// The project rooted at `path`, created on first open.
#[tauri::command]
#[specta::specta]
pub fn projects_open(state: State<'_, AppState>, path: PathBuf) -> Result<Project> {
    let root = std::fs::canonicalize(&path).map_err(|_| Error::Precondition("Choose a folder".into()))?;
    if !root.is_dir() {
        return Err(Error::Precondition("Choose a folder".into()));
    }
    db::find_or_create_project(&state.db.lock().expect("db lock"), &root)
}

#[tauri::command]
#[specta::specta]
pub fn projects_list(state: State<'_, AppState>) -> Result<Vec<Project>> {
    db::projects::list(&state.db.lock().expect("db lock"))
}

#[tauri::command]
#[specta::specta]
pub fn projects_update(state: State<'_, AppState>, id: ProjectId, patch: ProjectPatch) -> Result<Project> {
    let id = ProjectId::parse(id.as_str())?;
    update(&state, &id, patch)
}

/// Shared with `memory_pick_location`.
pub(crate) fn update(state: &AppState, id: &ProjectId, patch: ProjectPatch) -> Result<Project> {
    let memory_location = match patch.memory_location {
        Some(MemoryLocation::Folder { path }) => Some(MemoryLocation::Folder {
            path: std::fs::canonicalize(&path)
                .ok()
                .filter(|p| p.is_dir())
                .ok_or_else(|| Error::Precondition("Choose an existing folder for task memory".into()))?,
        }),
        other => other,
    };
    let conn = state.db.lock().expect("db lock");
    let mut project = db::projects::get(&conn, id)?.ok_or_else(|| Error::Invalid(format!("Unknown project {id}")))?;
    if let Some(name) = patch.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("Project name cannot be empty".into()));
        }
        project.name = name.to_owned();
    }
    if let Some(color) = patch.color {
        project.color = color;
    }
    if let Some(location) = memory_location {
        project.memory_location = location;
    }
    db::projects::update(&conn, &project)?;
    Ok(project)
}

/// Closes the window showing this project; the project record stays.
#[tauri::command]
#[specta::specta]
pub fn projects_close(app: AppHandle, state: State<'_, AppState>, id: ProjectId) -> Result<()> {
    let id = ProjectId::parse(id.as_str())?;
    // the registry lock is released before closing: the CloseRequested handler takes it
    let window_id = state
        .windows
        .lock()
        .expect("windows lock")
        .find(&Container::Project { id });
    if let Some(window) = window_id.and_then(|w| app.get_webview_window(w.as_str())) {
        window
            .close()
            .map_err(|e| Error::Provider(format!("Couldn't close the window: {e}")))?;
    }
    Ok(())
}
