//! The cost overview (§10.6, A6): the scope's ledger totals and breakdowns, with the live status-line numbers for a
//! task scope.

use fanoutcode_core::cost::{self, CostSummary};
use fanoutcode_core::settings::Scope;
use fanoutcode_core::Result;
use tauri::State;

use super::parse_scope;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn cost_summary(state: State<'_, AppState>, scope: Scope) -> Result<CostSummary> {
    let scope = parse_scope(&scope)?;
    let live = scope
        .task
        .as_ref()
        .and_then(|t| state.live_cost.lock().expect("live cost lock").get(t).cloned());
    cost::summary(&state.db.lock().expect("db lock"), &scope, live.as_ref())
}
