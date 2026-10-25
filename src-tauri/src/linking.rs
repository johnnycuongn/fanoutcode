//! Link binding glue (Plan 10): fixes a task's linked projects at its first session start and starts the brief
//! helper for linked projects without a brief. The rules live in `teamcode_core::workspace::{link, launch}`.

use tauri::AppHandle;
use teamcode_core::domain::{Container, Task, TaskId, WindowId};
use teamcode_core::helpers::HelperKind;
use teamcode_core::workspace::fragments::LinkContext;
use teamcode_core::workspace::launch;
use teamcode_core::Result;

use crate::observe::log_err;
use crate::state::AppState;

/// Binds the task on its first start. The issuing window's container is used when `window_label` names one;
/// otherwise the container the task was created in. Call before computing launch arguments.
pub fn bind_if_first_start(
    app: &AppHandle,
    state: &AppState,
    window_label: Option<&str>,
    task_id: &TaskId,
) -> Result<Task> {
    let issuing: Option<Container> = window_label
        .and_then(|l| WindowId::parse(l).ok())
        .and_then(|id| state.windows.lock().expect("windows lock").get(&id))
        .and_then(|w| w.container);
    let (task, needs_briefs) = {
        let conn = state.db.lock().expect("db lock");
        launch::bind_if_first_start(&conn, &state.data_dir, task_id, issuing)?
    };
    if needs_briefs {
        crate::helpers::run(app, task_id, HelperKind::ProjectBrief);
    }
    Ok(task)
}

/// The workspace's part of the task's prompts.
pub fn link_context_for(state: &AppState, task: &Task) -> Option<LinkContext> {
    let conn = state.db.lock().expect("db lock");
    log_err(
        "reading the task's links",
        launch::link_context(&conn, task, &state.data_dir),
    )
    .flatten()
}
