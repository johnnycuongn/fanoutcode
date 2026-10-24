//! Runs the effects `pipeline::apply` hands back. Toasts return to the caller for the webview; every other effect
//! goes to its runner: sessions (`StartSession`, `EndSession`, `InjectPrompt`) are Plan 04's, worktrees, tests and
//! helpers (`CreateWorktree`, `RunTest`, `RunHelper`) Plan 05's. Each arm is one line so its plan replaces it in place.

use tauri::AppHandle;
use teamcode_core::domain::Task;
use teamcode_core::pipeline::Effect;

use crate::state::AppState;
use crate::{observation, observe};

/// Forwards `external` in order and returns the toast texts. Call it with no lock held: a runner may need the
/// database. A runner that changes the task (a new worktree) hands the later arms the updated task.
pub fn dispatch(app: &AppHandle, state: &AppState, task: &Task, external: Vec<Effect>) -> Vec<String> {
    observation::on_task_applied(app, state, task);
    let mut task = task.clone();
    let mut toasts = Vec::new();
    for effect in external {
        match effect {
            Effect::Toast(text) => toasts.push(text),
            e @ Effect::StartSession => no_runner(&task, &e), // STUB(plan 04)
            e @ Effect::EndSession => no_runner(&task, &e),   // STUB(plan 04)
            e @ Effect::InjectPrompt(_) => no_runner(&task, &e), // STUB(plan 04)
            Effect::CreateWorktree => {
                if let Some(t) =
                    observe::log_err("creating the worktree", observation::create_worktree(app, state, &task))
                {
                    task = t;
                }
            }
            e @ Effect::RunTest => no_runner(&task, &e), // STUB(plan 05)
            e @ Effect::RunHelper(_) => no_runner(&task, &e), // STUB(plan 05)
            // apply consumes the persistence effects; none reaches here
            e @ (Effect::SetStage(_) | Effect::SetState(_) | Effect::SetArchived(_) | Effect::WriteIndex(_)) => {
                eprintln!(
                    "Team Code: effect {e:?} was not applied before dispatch (task {})",
                    task.id
                )
            }
        }
    }
    toasts
}

fn no_runner(task: &Task, effect: &Effect) {
    eprintln!("Team Code: effect {effect:?} has no runner yet (task {})", task.id);
}
