//! Team-canvas commands (Plan 09): the task's saved node positions.

use tauri::State;
use teamcode_core::canvas::{positions_load, positions_save, CanvasPosition};
use teamcode_core::domain::TaskId;
use teamcode_core::{db, Result};

use crate::state::AppState;

/// The task's saved canvas positions, ordered by node id; empty when none were saved.
#[tauri::command]
#[specta::specta]
pub fn canvas_positions_get(state: State<'_, AppState>, task_id: TaskId) -> Result<Vec<CanvasPosition>> {
    let id = TaskId::parse(task_id.as_str())?;
    positions_load(&state.db.lock().expect("db lock"), &id)
}

/// Replaces every saved canvas position of the task.
#[tauri::command]
#[specta::specta]
pub fn canvas_positions_set(state: State<'_, AppState>, task_id: TaskId, positions: Vec<CanvasPosition>) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    let conn = state.db.lock().expect("db lock");
    db::tasks::require(&conn, &id)?;
    positions_save(&conn, &id, &positions)
}
