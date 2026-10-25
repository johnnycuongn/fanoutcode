//! Runs the effects `pipeline::apply` hands back. Toasts return to the caller for the webview; every other effect
//! goes to its runner: sessions (`StartSession`, `EndSession`, `InjectPrompt`) are Plan 04's, worktrees, tests and
//! helpers (`CreateWorktree`, `RunTest`, `RunHelper`) Plan 05's. Each arm is one line so its plan replaces it in place.

use tauri::AppHandle;
use teamcode_core::domain::Task;
use teamcode_core::pipeline::{Effect, StageAction};

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
            e @ Effect::RunTest => no_runner(&task, &e), // STUB(plan 05)
            Effect::RunHelper(kind) => crate::helpers::run(app, &task.id, kind),
            // apply consumes the persistence effects; none reaches here
            e @ (Effect::SetStage(_) | Effect::SetState(_) | Effect::SetArchived(_) | Effect::WriteIndex(_)) => {
                eprintln!(
                    "Team Code: effect {e:?} was not applied before dispatch (task {})",
                    task.id
                )
            }
        }
    }
    session_service::apply_effects(app, &task, action, &session);
    toasts
}

fn no_runner(task: &Task, effect: &Effect) {
    eprintln!("Team Code: effect {effect:?} has no runner yet (task {})", task.id);
}
