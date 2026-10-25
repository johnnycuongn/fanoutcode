//! The entry effects of each stage: the one place they are defined. Forward moves, `Back` and column skips all call
//! `enter`.

use super::{Effect, TransitionCtx};
use crate::domain::{AgentKind, AgentState, Stage, Task, WorktreeChoice};
use crate::helpers::HelperKind;
use crate::{Error, Result};

pub(crate) const NO_TEST_COMMAND: &str = "Set a test command in project settings before using the Test stage";

/// The effects of entering `stage`, in order.
pub(crate) fn enter(task: &Task, stage: Stage, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    let p = &ctx.prompts;
    let mut fx = Vec::new();
    match stage {
        Stage::Spec => {
            fx.push(Effect::SetStage(Stage::Spec));
            // Spec never starts a session; it re-prompts one that already runs
            if task.primary_session.is_some() {
                prompt(task, &p.spec, &mut fx);
            } else {
                fx.push(Effect::SetState(AgentState::Idle));
            }
        }
        Stage::Plan | Stage::Build | Stage::Review => {
            fx.push(Effect::SetStage(stage));
            ensure_session(task, &mut fx);
            let text = match stage {
                Stage::Plan => &p.plan,
                Stage::Build => &p.build,
                _ => &p.review,
            };
            prompt(task, text, &mut fx);
            if stage == Stage::Review {
                fx.push(Effect::RunHelper(HelperKind::SpecWatcher));
            }
        }
        Stage::Test => {
            if ctx.test_command.is_none() {
                return Err(Error::Precondition(NO_TEST_COMMAND.into()));
            }
            fx.extend([
                Effect::SetStage(Stage::Test),
                Effect::RunTest,
                Effect::SetState(AgentState::Working),
            ]);
        }
        Stage::Commit => fx.extend([
            Effect::SetStage(Stage::Commit),
            Effect::RunHelper(HelperKind::CommitMessages),
            Effect::SetState(AgentState::Working),
        ]),
        Stage::Done => fx.extend([
            Effect::SetStage(Stage::Done),
            Effect::SetState(AgentState::Idle),
            Effect::Toast("Task done".into()),
        ]),
    }
    Ok(fx)
}

/// `CreateWorktree` (when the task wants one and has none) then `StartSession`, unless a session already runs.
pub(crate) fn ensure_session(task: &Task, fx: &mut Vec<Effect>) {
    if task.primary_session.is_some() {
        return;
    }
    if needs_worktree(task) {
        fx.push(Effect::CreateWorktree);
    }
    fx.push(Effect::StartSession);
}

pub(crate) fn needs_worktree(task: &Task) -> bool {
    task.worktree == WorktreeChoice::On && task.worktree_path.is_none()
}

/// An agent gets the prompt and starts working; a shell gets nothing and stays idle.
pub(crate) fn prompt(task: &Task, text: &str, fx: &mut Vec<Effect>) {
    if task.agent_kind == AgentKind::Shell {
        fx.push(Effect::SetState(AgentState::Idle));
    } else {
        fx.push(Effect::InjectPrompt(text.to_owned()));
        fx.push(Effect::SetState(AgentState::Working));
    }
}
