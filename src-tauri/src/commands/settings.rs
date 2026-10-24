//! First-run state (§4.4). Plan 03 adds the scoped settings commands here.

use tauri::State;
use teamcode_core::db;
use teamcode_core::domain::{ThemePreference, WorktreeConvention};
use teamcode_core::Result;

use crate::state::AppState;

/// True until onboarding has been completed; a database that cannot be read counts as pending.
#[tauri::command]
#[specta::specta]
pub fn settings_first_run_pending(state: State<'_, AppState>) -> bool {
    db::first_run_pending(&state.db.lock().expect("db lock")).unwrap_or(true)
}

/// Records the two first-run answers and marks first run complete.
#[tauri::command]
#[specta::specta]
pub fn settings_first_run_complete(
    state: State<'_, AppState>,
    convention: WorktreeConvention,
    theme: ThemePreference,
) -> Result<()> {
    db::complete_first_run(&state.db.lock().expect("db lock"), convention, theme)
}
