//! Runs the effects `pipeline::apply` hands back. Toasts return to the caller for the webview; every other effect
//! goes to its runner: sessions (`StartSession`, `EndSession`, `InjectPrompt`) to Plan 04's
//! `session_service::apply_effects`, worktrees, tests and helpers (`CreateWorktree`, `RunTest`, `RunHelper`) to
//! Plan 05's (`RunTest` starts the run, or attaches to the one in flight).

use fanoutcode_core::domain::Task;
use fanoutcode_core::pipeline::{Effect, StageAction};
use tauri::AppHandle;

use crate::state::AppState;
use crate::{observation, observe, session_service};

/// Forwards `external` in order and returns the toast texts. Call it with no lock held: a runner may need the
/// database. A runner that changes the task (a new worktree) hands the later arms the updated task. The session
/// effects are collected and handed to `session_service::apply_effects` last, with the task as the earlier arms
/// left it; `action` is the stage action behind them (`None` for a move, a create or an edit).
pub fn dispatch(
    app: &AppHandle,
    state: &AppState,
    task: &Task,
    action: Option<&StageAction>,
    external: Vec<Effect>,
) -> Vec<String> {
    observation::on_task_applied(app, state, task);
    let mut task = task.clone();
    let mut toasts = Vec::new();
    let mut session = Vec::new();
    for effect in external {
        match effect {
            Effect::Toast(text) => toasts.push(text),
            e @ (Effect::StartSession | Effect::EndSession | Effect::InjectPrompt(_)) => session.push(e),
            Effect::CreateWorktree => {
                if let Some(t) =
                    observe::log_err("creating the worktree", observation::create_worktree(app, state, &task))
                {
                    task = t;
                }
            }
            // start-or-attach: a Test tab that opens later (or is open now) attaches to this run
            Effect::RunTest => {
                observe::log_err(
                    "starting the tests",
                    crate::commands::tests::start_or_attach(app, &task.id),
                );
            }
            Effect::RunHelper(kind) => crate::helpers::run(app, &task.id, kind),
            // apply consumes the persistence effects; none reaches here
            e @ (Effect::SetStage(_) | Effect::SetState(_) | Effect::SetArchived(_) | Effect::WriteIndex(_)) => {
                eprintln!(
                    "Fanout Code: effect {e:?} was not applied before dispatch (task {})",
                    task.id
                )
            }
        }
    }
    session_service::apply_effects(app, &task, action, &session);
    toasts
}
