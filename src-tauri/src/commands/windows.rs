//! The window commands (A4). One window per container; opening a container already shown focuses its window.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{AppHandle, Manager, State, Window};
use teamcode_core::domain::{Container, ProjectId, WindowId, WorkspaceId};
use teamcode_core::recent::{self, RecentEntry, RecentKind};
use teamcode_core::windows::WindowInfo;
use teamcode_core::{db, Error, Result};

use crate::state::AppState;
use crate::windows;

fn unknown_window() -> Error {
    Error::Invalid("Unknown window.".into())
}

/// The caller's registry id; its label must be a registered `w-<n>`.
fn caller_id(state: &AppState, window: &Window) -> Result<WindowId> {
    let id = WindowId::parse(window.label()).map_err(|_| unknown_window())?;
    state
        .windows
        .lock()
        .expect("windows lock")
        .get(&id)
        .map(|_| id)
        .ok_or_else(unknown_window)
}

fn focus(app: &AppHandle, id: &WindowId) -> Result<()> {
    let window = app.get_webview_window(id.as_str()).ok_or_else(unknown_window)?;
    let _ = window.unminimize();
    window
        .set_focus()
        .map_err(|e| Error::Provider(format!("Couldn't focus the window: {e}")))
}

#[tauri::command]
#[specta::specta]
pub fn windows_list(state: State<'_, AppState>) -> Vec<WindowInfo> {
    state.windows.lock().expect("windows lock").list()
}

/// The calling window's entry.
#[tauri::command]
#[specta::specta]
pub fn windows_current(window: Window, state: State<'_, AppState>) -> Result<WindowInfo> {
    let id = caller_id(&state, &window)?;
    state
        .windows
        .lock()
        .expect("windows lock")
        .get(&id)
        .ok_or_else(unknown_window)
}

#[tauri::command]
#[specta::specta]
pub fn windows_focus(app: AppHandle, state: State<'_, AppState>, window_id: WindowId) -> Result<()> {
    let id = WindowId::parse(window_id.as_str()).map_err(|_| unknown_window())?;
    state
        .windows
        .lock()
        .expect("windows lock")
        .get(&id)
        .ok_or_else(unknown_window)?;
    focus(&app, &id)
}

/// Focuses the next window in open order; a no-op with one window.
#[tauri::command]
#[specta::specta]
pub fn windows_focus_next(window: Window, state: State<'_, AppState>) -> Result<()> {
    let id = caller_id(&state, &window)?;
    let next = state.windows.lock().expect("windows lock").next_after(&id);
    match next {
        Some(next) => focus(window.app_handle(), &next),
        None => Ok(()),
    }
}

/// Closes that window; its `CloseRequested` handler removes the registry entry and saves.
#[tauri::command]
#[specta::specta]
pub fn windows_close(app: AppHandle, window_id: WindowId) -> Result<()> {
    let id = WindowId::parse(window_id.as_str()).map_err(|_| unknown_window())?;
    let window = app.get_webview_window(id.as_str()).ok_or_else(unknown_window)?;
    window
        .close()
        .map_err(|e| Error::Provider(format!("Couldn't close the window: {e}")))
}

/// Recent projects and workspaces, newest first; moved or deleted ones are marked `missing`.
#[tauri::command]
#[specta::specta]
pub fn windows_recent(state: State<'_, AppState>) -> Result<Vec<RecentEntry>> {
    recent::list(&state.db.lock().expect("db lock"))
}

/// Opens the project at `path` in its own window (or focuses the window already showing it).
#[tauri::command]
#[specta::specta]
pub async fn windows_open_project(window: Window, state: State<'_, AppState>, path: PathBuf) -> Result<WindowInfo> {
    let root = canonical(&path, "That folder no longer exists. Relocate or remove it.")?;
    let project = db::find_or_create_project(&state.db.lock().expect("db lock"), &root)?;
    let entry = recent_entry(RecentKind::Project, project.id.as_str(), &project.name, &project.root);
    open_container(
        &window,
        &state,
        Container::Project { id: project.id.clone() },
        project.name,
        vec![project.id],
        entry,
    )
}

/// The two keys Plan 01 reads from a `<name>.teamcode-workspace.json`; Plan 10 owns the rest.
#[derive(Deserialize)]
struct Envelope {
    id: String,
    name: String,
}

/// Opens the workspace file at `path` in its own window (or focuses the window already showing it).
#[tauri::command]
#[specta::specta]
pub async fn windows_open_workspace(window: Window, state: State<'_, AppState>, path: PathBuf) -> Result<WindowInfo> {
    let file = canonical(&path, "That workspace file no longer exists. Relocate or remove it.")?;
    let not_a_workspace = || Error::Invalid("Not a Team Code workspace file.".into());
    let text = std::fs::read_to_string(&file)?;
    let envelope: Envelope = serde_json::from_str(&text).map_err(|_| not_a_workspace())?;
    let id = WorkspaceId::parse(&envelope.id).map_err(|_| not_a_workspace())?;
    let workspace = db::find_or_create_workspace(&state.db.lock().expect("db lock"), &id, &envelope.name, &file)?;
    let project_ids: Vec<ProjectId> = workspace.projects.iter().map(|p| p.project_id.clone()).collect();
    let entry = recent_entry(RecentKind::Workspace, id.as_str(), &workspace.name, &file);
    open_container(
        &window,
        &state,
        Container::Workspace { id },
        workspace.name,
        project_ids,
        entry,
    )
}

fn canonical(path: &Path, missing: &str) -> Result<PathBuf> {
    std::fs::canonicalize(path).map_err(|_| Error::Precondition(missing.into()))
}

fn recent_entry(kind: RecentKind, id: &str, name: &str, path: &Path) -> RecentEntry {
    RecentEntry {
        kind,
        id: id.into(),
        name: name.into(),
        path: path.into(),
        opened_at: chrono::Utc::now(),
        missing: false,
    }
}

/// One window per container: focus the existing one, or build a new one, record it as recent, save the window list,
/// and close the calling window when it is a start screen (§4.4).
fn open_container(
    caller: &Window,
    state: &AppState,
    container: Container,
    title: String,
    project_ids: Vec<ProjectId>,
    entry: RecentEntry,
) -> Result<WindowInfo> {
    let app = caller.app_handle();
    let (info, existed) = state
        .windows
        .lock()
        .expect("windows lock")
        .open(Some(container), title, project_ids);
    if existed {
        focus(app, &info.window_id)?;
    } else if let Err(e) = windows::build(app, &info) {
        state.windows.lock().expect("windows lock").close(&info.window_id);
        return Err(e);
    }
    recent::push(&state.db.lock().expect("db lock"), entry)?;
    let start_screen = WindowId::parse(caller.label())
        .ok()
        .filter(|id| id != &info.window_id)
        .filter(|id| {
            state
                .windows
                .lock()
                .expect("windows lock")
                .get(id)
                .is_some_and(|w| w.container.is_none())
        });
    if let Some(id) = &start_screen {
        state.windows.lock().expect("windows lock").close(id);
    }
    state.save_windows()?;
    if start_screen.is_some() {
        caller
            .close()
            .map_err(|e| Error::Provider(format!("Couldn't close the start screen: {e}")))?;
    }
    Ok(info)
}
