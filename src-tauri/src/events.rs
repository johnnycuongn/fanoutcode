//! The event catalogue: app-wide facts emitted to every window. Each struct's kebab-case name is the event name
//! (`TaskChanged` → `task-changed`). Emit with `tauri_specta::Event::emit(&payload, &app)`. `team-event` is Plan 05's.

use fanoutcode_core::db;
use fanoutcode_core::domain::{Container, PaneId, SessionId, Task, TaskId, WindowId, Workspace, WorkspaceId};
use fanoutcode_core::hooks::TaskEvent;
use fanoutcode_core::{Error, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TaskChanged {
    pub task: Task,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TasksReloaded {
    pub container: Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct NeedsYou {
    pub container: Container,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SessionExited {
    pub session_id: SessionId,
    pub code: Option<i32>,
}

/// How many prompts wait for the session to go idle (Plan 04).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SessionQueueChanged {
    pub session_id: SessionId,
    pub pending: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SessionAttached {
    pub session_id: SessionId,
    pub window_id: WindowId,
    pub pane_id: PaneId,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct WorkspaceChanged {
    pub workspace: Workspace,
}

/// A workspace file saved outside app data changed on disk (Plan 10's watcher); the window offers a reload.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct WorkspaceFileChanged {
    pub workspace_id: WorkspaceId,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct ContextDocsChanged {
    pub container: Container,
}

/// A window now shows a different container (a project window converted into a workspace).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct WindowContainerChanged {
    pub window_id: WindowId,
    pub container: Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct ViewsChanged {
    pub container: Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct CostUpdated {
    pub task_id: TaskId,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct GitChanges {
    pub task_id: TaskId,
}

/// The spec watcher rewrote `spec.md` (A15), once per applied change.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SpecWatcher {
    pub task_id: TaskId,
    pub at: chrono::DateTime<chrono::Utc>,
    pub reason: String,
}

/// Every parsed hook event except `StatusLine` (Plan 05's tailer).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TeamEvent {
    pub task_id: TaskId,
    pub event: TaskEvent,
}

/// A test run started (by a stage action, a board drag or a Test tab), so an open Test tab attaches to it.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TestStarted {
    pub task_id: TaskId,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TestFinished {
    pub task_id: TaskId,
    pub code: i32,
}

/// A task's overlap rows changed (Plan 11): `count` rows in all, `pending` of them waiting on the developer.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct OverlapChanged {
    pub task_id: TaskId,
    pub count: u32,
    pub pending: u32,
}

/// `task-changed` for `task`, then `needs-you` with the fresh count for its project and, when it has one, its
/// workspace. Call it after the database lock is released by the caller's write (it takes `conn` only to count).
pub fn emit_task_changed(app: &AppHandle, conn: &Connection, task: &Task) -> Result<()> {
    emit(app, TaskChanged { task: task.clone() })?;
    let mut containers = vec![Container::Project {
        id: task.project_id.clone(),
    }];
    if let Some(id) = &task.workspace_id {
        containers.push(Container::Workspace { id: id.clone() });
    }
    for container in containers {
        let count = db::tasks::needs_you_count(conn, &container)?;
        emit(app, NeedsYou { container, count })?;
    }
    Ok(())
}

/// Emits to every window; a failure is a shell problem, reported as `Provider`.
pub fn emit<E: tauri_specta::Event + Serialize + Clone>(app: &AppHandle, event: E) -> Result<()> {
    event
        .emit(app)
        .map_err(|e| Error::Provider(format!("Couldn't notify the windows: {e}")))
}
