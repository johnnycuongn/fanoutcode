//! Optional Jira status moves (Plan 10): when a task enters a stage its project maps to a transition
//! (`jira_transitions`), the issue is moved. Failures are logged, never returned to the UI.

use std::collections::HashMap;
use std::sync::Mutex;

use fanoutcode_core::domain::{Stage, TaskId};
use fanoutcode_core::jira::settings::{transition_for, JiraTransitionsKey};
use fanoutcode_core::pipeline::ctx::task_scope;
use fanoutcode_core::settings::resolve;
use fanoutcode_core::{db, Result};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::events::TaskChanged;
use crate::state::AppState;

/// Starts listening to `task-changed`; call once from `setup`.
pub fn install(app: &AppHandle) {
    let last: Mutex<HashMap<TaskId, Stage>> = Mutex::default();
    let handle = app.clone();
    TaskChanged::listen_any(app, move |event| {
        let task = event.payload.task;
        let previous = last
            .lock()
            .expect("stage memory lock")
            .insert(task.id.clone(), task.stage);
        let (Some(previous), Some(key)) = (previous, task.jira_key.clone()) else {
            return;
        };
        if previous == task.stage {
            return;
        }
        let app = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = apply(&app, &task.project_id, &task, &key).await {
                eprintln!("Fanout Code: couldn't move {key} in Jira: {e}");
            }
        });
    });
}

async fn apply(
    app: &AppHandle,
    project: &fanoutcode_core::domain::ProjectId,
    task: &fanoutcode_core::domain::Task,
    key: &str,
) -> Result<()> {
    let state = app.state::<AppState>();
    let cfg = {
        let scope = task_scope(task);
        let values = db::settings::load(&state.db.lock().expect("db lock"), &scope)?;
        resolve::<JiraTransitionsKey>(&values, &scope)
    };
    let Some(name) = transition_for(&cfg, task.stage).map(str::to_owned) else {
        return Ok(());
    };
    let client = crate::commands::jira::client_for(&state, project)?;
    let found = client
        .transitions(key)
        .await?
        .into_iter()
        .find(|t| t.name.eq_ignore_ascii_case(&name));
    match found {
        Some(t) => client.transition(key, &t.id).await,
        None => {
            eprintln!("Fanout Code: {key} has no transition named {name:?}");
            Ok(())
        }
    }
}
