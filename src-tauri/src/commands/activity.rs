//! The board cards' activity (A6): one snapshot per non-archived task of the container, from in-memory trackers,
//! cached change sets and the cost ledger only (no git or log reads), so it returns at once. Live updates keep
//! flowing through `team-event`, `git-changes` and `cost-updated`.

use tauri::State;
use teamcode_core::activity::{card_cost, TaskActivity};
use teamcode_core::cost;
use teamcode_core::domain::Container;
use teamcode_core::settings::Scope;
use teamcode_core::{db, Result};

use super::parse_container;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn tasks_activity(state: State<'_, AppState>, container: Container) -> Result<Vec<TaskActivity>> {
    let container = parse_container(&container)?;
    let conn = state.db.lock().expect("db lock");
    let tasks: Vec<_> = db::tasks::list(&conn, &container)?
        .into_iter()
        .filter(|t| !t.archived)
        .collect();
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        let scope = Scope {
            task: Some(task.id.clone()),
            ..Scope::default()
        };
        let ledger = cost::summary(&conn, &scope, None)?;
        let ledger_usd = (!ledger.by_model.is_empty()).then_some(ledger.cost_usd);
        let live = state.live_cost.lock().expect("live cost lock").get(&task.id).cloned();
        let set = state
            .change_sets
            .lock()
            .expect("change sets lock")
            .get(&task.id)
            .cloned();
        let tracker = state
            .activity
            .lock()
            .expect("activity lock")
            .get(&task.id)
            .cloned()
            .unwrap_or_default();
        out.push(tracker.snapshot(&task, set.as_ref(), card_cost(ledger_usd, live.as_ref())));
    }
    Ok(out)
}

/// Feeds one event to the task's tracker.
pub(crate) fn track(state: &AppState, ev: &teamcode_core::hooks::TaskEvent, repos: &[std::path::PathBuf]) {
    state
        .activity
        .lock()
        .expect("activity lock")
        .entry(ev.task_id.clone())
        .or_default()
        .on_event(ev, repos);
}
