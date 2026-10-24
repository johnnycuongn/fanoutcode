//! The event catalogue: app-wide facts emitted to every window. Each struct's kebab-case name is the event name
//! (`TaskChanged` → `task-changed`). Emit with `tauri_specta::Event::emit(&payload, &app)`. `team-event` is Plan 05's.

use serde::{Deserialize, Serialize};
use teamcode_core::domain::{Container, PaneId, SessionId, Task, TaskId, WindowId, Workspace};

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

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct TestFinished {
    pub task_id: TaskId,
    pub code: i32,
}
