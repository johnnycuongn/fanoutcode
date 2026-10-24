//! Runs `Effect::RunHelper(kind)` (§7.4): one slot per `(task, kind)`, a new run replacing (aborting) the old, as
//! LiveSpec does. Naming runs on the fast tier; the spec watcher and commit messages on the deep tier. The prompts,
//! parsing and rules live in `teamcode_core::helpers`.

use livespec_core::llm::Usage;
use livespec_core::settings::Tier;
use tauri::{AppHandle, Manager};
use teamcode_core::domain::TaskId;
use teamcode_core::helpers::HelperKind;
use teamcode_core::Result;

use crate::observe::log_err;
use crate::state::AppState;

/// The tier a helper runs on.
pub fn tier_for(kind: HelperKind) -> Tier {
    match kind {
        HelperKind::Naming => Tier::Fast,
        _ => Tier::Deep,
    }
}

/// Starts the helper for the task, replacing any run of the same kind still in flight.
pub fn run(app: &AppHandle, task_id: &TaskId, kind: HelperKind) {
    let (app2, id) = (app.clone(), task_id.clone());
    let handle = tauri::async_runtime::spawn(async move {
        log_err(&format!("the {kind:?} helper"), job(&app2, &id, kind).await);
    });
    let state = app.state::<AppState>();
    let old = state
        .helper_runs
        .lock()
        .expect("helper runs lock")
        .insert((task_id.clone(), kind), handle);
    if let Some(old) = old {
        old.abort();
    }
}

async fn job(app: &AppHandle, id: &TaskId, kind: HelperKind) -> Result<()> {
    // each kind's runner lands with its own task of this plan
    let _ = (app, id, kind);
    Ok(())
}

/// After a helper call: LiveSpec's ledger is flushed.
pub fn note_usage(app: &AppHandle, _task_id: &TaskId, _usage: &[Usage]) {
    let state = app.state::<AppState>();
    if let Err(e) = state.usage.flush_if_dirty() {
        eprintln!("Team Code: couldn't save the usage ledger: {e}");
    }
}
