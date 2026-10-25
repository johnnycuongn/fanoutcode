//! Stage effects, hook events and the Codex ticker: when a prompt reaches the agent (§7.2). Everything here returns
//! at once; the waiting happens on spawned threads.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use teamcode_core::domain::{AgentKind, AgentState, SessionId, Task, TaskId};
use teamcode_core::hooks::HookEvent;
use teamcode_core::pipeline::{Effect, StageAction};
use teamcode_core::sessions::{QueueOutcome, INJECT_ENTER_DELAY};
use teamcode_core::{Error, Result};

use super::{open_primary, queue_changed, stop_blocking};
use crate::observe::log_err;
use crate::state::AppState;

/// What `apply_effects` does with the session; internal, never serialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionOp {
    Inject(String),
    /// End the primary session; `remove` asks the driver to clean up after it too (archive).
    End {
        remove: bool,
    },
    /// Start the primary session; `fresh` never resumes the last Claude session.
    Start {
        fresh: bool,
    },
}

/// The session ops among `effects`, in order. A Reply arrives as `InjectPrompt` like any stage prompt.
pub fn effects_for(action: &StageAction, effects: &[Effect]) -> Vec<SessionOp> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::InjectPrompt(p) => Some(SessionOp::Inject(p.clone())),
            Effect::EndSession => Some(SessionOp::End {
                remove: matches!(action, StageAction::Archive),
            }),
            Effect::StartSession => Some(SessionOp::Start {
                fresh: matches!(action, StageAction::StartFresh),
            }),
            _ => None,
        })
        .collect()
}

/// Runs the session ops of a transition on a spawned thread, in order, and returns at once. A failed op is logged
/// and the rest still run. `action` is `None` for a board move or a test result (never fresh, never an archive).
pub fn apply_effects(app: &AppHandle, task: &Task, action: Option<&StageAction>, effects: &[Effect]) {
    let ops = effects_for(action.unwrap_or(&StageAction::Advance), effects);
    if ops.is_empty() {
        return;
    }
    let (app, task) = (app.clone(), task.clone());
    std::thread::spawn(move || {
        for op in ops {
            let r = match op {
                SessionOp::Inject(prompt) => inject_task(&app, &task.id, prompt).map(|_| ()),
                SessionOp::End { remove } => end(&app, &task.id, remove),
                SessionOp::Start { fresh } => open_primary(&app, &task.id, fresh).map(|_| ()),
            };
            log_err(&format!("running a session effect for {}", task.id), r);
        }
    });
}

/// A stage prompt or a reply for the task: into its live session, starting (resuming) one first when there is none
/// (the app was relaunched since, so the task's recorded session is gone).
fn inject_task(app: &AppHandle, task_id: &TaskId, prompt: String) -> Result<QueueOutcome> {
    let live = app.state::<AppState>().registry().live_for_task(task_id);
    let id = match live {
        Some(id) => id,
        None => open_primary(app, task_id, false)?,
    };
    inject(app, &id, prompt)
}

fn end(app: &AppHandle, task_id: &TaskId, remove: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let Some(id) = state.registry().live_for_task(task_id) else {
        return Ok(());
    };
    let session = state.registry().get(&id).cloned();
    stop_blocking(app, &id)?;
    match (remove, session) {
        (true, Some(s)) => state.driver.remove(&s),
        _ => Ok(()),
    }
}

/// Types the prompt now when the agent is idle (Enter follows after `INJECT_ENTER_DELAY`), else queues it; either
/// way `session-queue-changed` goes out.
pub fn inject(app: &AppHandle, id: &SessionId, prompt: String) -> Result<QueueOutcome> {
    let outcome = app.state::<AppState>().registry().inject(id, prompt, Instant::now())?;
    match outcome {
        QueueOutcome::Sent => {
            press_enter(app, id);
            queue_changed(app, id, 0);
        }
        QueueOutcome::Queued { pending } => queue_changed(app, id, pending),
    }
    Ok(outcome)
}

/// Enter, a moment after the paste, from a spawned thread.
fn press_enter(app: &AppHandle, id: &SessionId) {
    let (app, id) = (app.clone(), id.clone());
    std::thread::spawn(move || {
        std::thread::sleep(INJECT_ENTER_DELAY);
        let r = app.state::<AppState>().registry().write(&id, b"\r");
        log_err("submitting a prompt", r);
    });
}

/// Every parsed hook event, from Plan 05's tailer thread (registered with `hooks_tailer::subscribe`, A18): the
/// task's live primary session updates its idle state and, when it went idle, sends the next queued prompt. Never
/// blocks; a task without a live session (an event replayed after a relaunch) is a no-op.
pub fn note_hook_event(app: &AppHandle, task_id: &TaskId, event: &HookEvent) {
    let state = app.state::<AppState>();
    let sent = {
        let mut registry = state.registry();
        let Some(id) = registry.live_for_task(task_id) else {
            return;
        };
        let Some(live) = registry.get_mut(&id) else { return };
        live.idle.on_hook(event);
        registry
            .pump(&id, Instant::now())
            .map(|p| p.map(|pending| (id, pending)))
    };
    match sent {
        Ok(Some((id, pending))) => {
            press_enter(app, &id);
            queue_changed(app, &id, pending);
        }
        Ok(None) => {}
        Err(e) => eprintln!("Team Code: sending a queued prompt for {task_id}: {e}"),
    }
}

/// Codex has no hooks: once a second, each live Codex session sends its next queued prompt when quiet, and a change
/// of idle state since the last tick moves its task's chip between Idle and Working.
pub fn start_codex_ticker(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("codex-tick".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            codex_tick(&app);
        });
}

pub fn codex_tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = Instant::now();
    let mut changed: Vec<(TaskId, bool)> = Vec::new();
    let mut sent: Vec<(SessionId, usize)> = Vec::new();
    {
        let mut registry = state.registry();
        let codex: Vec<_> = registry
            .list()
            .into_iter()
            .filter(|s| s.kind == AgentKind::Codex && s.exited.is_none())
            .collect();
        let mut last = state.codex_idle.lock().expect("codex idle lock");
        for s in codex {
            if let Ok(Some(pending)) = registry.pump(&s.id, now) {
                sent.push((s.id.clone(), pending));
            }
            let idle = registry.get_mut(&s.id).is_some_and(|l| l.idle.is_idle(now));
            if last.insert(s.id.clone(), idle) != Some(idle) {
                if let Some(task) = s.task_id {
                    changed.push((task, idle));
                }
            }
        }
    }
    for (id, pending) in sent {
        press_enter(app, &id);
        queue_changed(app, &id, pending);
    }
    for (task, idle) in changed {
        let state_now = if idle { AgentState::Idle } else { AgentState::Working };
        let r = crate::commands::tasks::run(app, &state, &task, |_, _| Ok(vec![Effect::SetState(state_now)]));
        if let Err(e) = r {
            if !matches!(e, Error::Invalid(_)) {
                eprintln!("Team Code: updating the Codex task {task}: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use teamcode_core::domain::Stage;
    use teamcode_core::pipeline::{Effect, StageAction};

    use super::*;

    #[test]
    fn start_fresh_maps_to_end_then_fresh_start() {
        assert_eq!(
            effects_for(&StageAction::StartFresh, &[Effect::EndSession, Effect::StartSession]),
            vec![SessionOp::End { remove: false }, SessionOp::Start { fresh: true }]
        );
    }

    #[test]
    fn archive_ends_with_remove_and_other_effects_are_ignored() {
        assert_eq!(
            effects_for(
                &StageAction::Archive,
                &[
                    Effect::SetStage(Stage::Done),
                    Effect::EndSession,
                    Effect::Toast("x".into())
                ]
            ),
            vec![SessionOp::End { remove: true }]
        );
    }

    #[test]
    fn advance_injects_without_fresh() {
        assert_eq!(
            effects_for(
                &StageAction::Advance,
                &[Effect::StartSession, Effect::InjectPrompt("go".into())]
            ),
            vec![SessionOp::Start { fresh: false }, SessionOp::Inject("go".into())]
        );
    }

    #[test]
    fn reply_injects_its_prompt_like_any_other() {
        let reply = StageAction::Reply {
            prompt: "Use the existing auth helper".into(),
        };
        assert_eq!(
            effects_for(&reply, &[Effect::InjectPrompt("Use the existing auth helper".into())]),
            vec![SessionOp::Inject("Use the existing auth helper".into())]
        );
    }
}
