//! Moves that are not one step forward: dragging back, skipping columns, Archive and Restore (spec §6).

use super::enter::enter;
use super::{Effect, TransitionCtx};
use crate::domain::{AgentState, Stage, Task};
use crate::memory::{IndexEntry, TimelineKind};
use crate::{Error, Result};

/// A board drag of any distance (`tasks_move`). The same stage does nothing; an earlier one is `Back`; a later one
/// logs each stage strictly between as skipped (their exits are no-ops: no prompts, no helpers), then enters it.
pub fn move_to(task: &Task, target: Stage, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    let from = task.stage.index();
    let to = target.index();
    if to == from {
        return Ok(Vec::new());
    }
    if to < from {
        return back(task, target, ctx);
    }
    let mut fx: Vec<Effect> = Stage::ALL[from + 1..to]
        .iter()
        .map(|s| {
            Effect::WriteIndex(IndexEntry {
                at: ctx.now,
                stage: *s,
                kind: TimelineKind::Skipped,
            })
        })
        .collect();
    fx.extend(enter(task, target, ctx)?);
    Ok(fx)
}

/// Re-enters an earlier stage with its stage prompt; Done → Commit reopens a task.
pub(crate) fn back(task: &Task, stage: Stage, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    if stage.index() >= task.stage.index() {
        return Err(Error::Invalid("Back must name an earlier stage".into()));
    }
    enter(task, stage, ctx)
}

/// Ends the session and hides the card. Worktree removal is offered by Plan 05 afterwards; the confirmation while
/// the agent works is the board's (Plan 07).
pub(crate) fn archive(task: &Task, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    if task.archived {
        return Err(Error::Invalid("Already archived".into()));
    }
    let mut fx = Vec::new();
    if task.primary_session.is_some() {
        fx.push(Effect::EndSession);
    }
    fx.push(Effect::SetArchived(true));
    fx.push(Effect::WriteIndex(IndexEntry {
        at: ctx.now,
        stage: task.stage,
        kind: TimelineKind::Archived,
    }));
    Ok(fx)
}

/// Brings the card back idle in its stage; the session is not restarted (the next stage action or Start fresh does).
pub(crate) fn restore(task: &Task, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    if !task.archived {
        return Err(Error::Invalid("Not archived".into()));
    }
    Ok(vec![
        Effect::SetArchived(false),
        Effect::SetState(AgentState::Idle),
        Effect::WriteIndex(IndexEntry {
            at: ctx.now,
            stage: task.stage,
            kind: TimelineKind::Restored,
        }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, AgentState, SessionId};
    use crate::helpers::HelperKind;
    use crate::pipeline::{transition, StageAction};
    use crate::test_support::{ctx, fixture};
    use crate::Error;

    fn sess() -> Option<SessionId> {
        Some(SessionId::parse("s-1a-1").unwrap())
    }

    #[test]
    fn back_reenters_the_earlier_stage_with_its_prompt() {
        let mut t = fixture(Stage::Build, AgentKind::ClaudeCode);
        t.primary_session = sess();
        assert_eq!(
            transition(&t, StageAction::Back { stage: Stage::Plan }, &ctx()).unwrap(),
            [
                Effect::SetStage(Stage::Plan),
                Effect::InjectPrompt("PLAN".into()),
                Effect::SetState(AgentState::Working)
            ]
        );
        assert!(matches!(
            transition(&t, StageAction::Back { stage: Stage::Review }, &ctx()),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            transition(&t, StageAction::Back { stage: Stage::Build }, &ctx()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn back_to_spec_with_a_session_injects_the_spec_prompt_but_never_starts_one() {
        let mut t = fixture(Stage::Plan, AgentKind::ClaudeCode);
        t.primary_session = sess();
        assert_eq!(
            transition(&t, StageAction::Back { stage: Stage::Spec }, &ctx()).unwrap(),
            [
                Effect::SetStage(Stage::Spec),
                Effect::InjectPrompt("SPEC".into()),
                Effect::SetState(AgentState::Working)
            ]
        );
        let t = fixture(Stage::Plan, AgentKind::ClaudeCode);
        assert_eq!(
            transition(&t, StageAction::Back { stage: Stage::Spec }, &ctx()).unwrap(),
            [Effect::SetStage(Stage::Spec), Effect::SetState(AgentState::Idle)]
        );
    }

    #[test]
    fn skipping_columns_logs_each_skipped_stage_then_enters_the_target() {
        let fx = move_to(&fixture(Stage::Plan, AgentKind::ClaudeCode), Stage::Commit, &ctx()).unwrap();
        let skipped: Vec<Stage> = fx
            .iter()
            .filter_map(|e| match e {
                Effect::WriteIndex(IndexEntry {
                    stage,
                    kind: TimelineKind::Skipped,
                    ..
                }) => Some(*stage),
                _ => None,
            })
            .collect();
        assert_eq!(skipped, [Stage::Build, Stage::Review, Stage::Test]);
        assert_eq!(
            fx[3..],
            [
                Effect::SetStage(Stage::Commit),
                Effect::RunHelper(HelperKind::CommitMessages),
                Effect::SetState(AgentState::Working)
            ]
        );
        assert!(!fx
            .iter()
            .any(|e| matches!(e, Effect::InjectPrompt(_) | Effect::RunTest)));
    }

    #[test]
    fn moving_to_the_same_stage_does_nothing_and_backwards_is_back() {
        assert_eq!(
            move_to(&fixture(Stage::Build, AgentKind::ClaudeCode), Stage::Build, &ctx()).unwrap(),
            []
        );
        assert_eq!(
            move_to(&fixture(Stage::Review, AgentKind::ClaudeCode), Stage::Spec, &ctx()).unwrap()[0],
            Effect::SetStage(Stage::Spec)
        );
    }

    #[test]
    fn dragging_into_test_without_a_command_is_refused() {
        let mut c = ctx();
        c.test_command = None;
        let err = move_to(&fixture(Stage::Build, AgentKind::ClaudeCode), Stage::Test, &c).unwrap_err();
        assert!(
            matches!(err, Error::Precondition(ref m) if m.contains("test command")),
            "{err:?}"
        );
        // Build → Test skips Review, so the entry effects follow one Skipped line
        let fx = move_to(&fixture(Stage::Build, AgentKind::ClaudeCode), Stage::Test, &ctx()).unwrap();
        assert!(matches!(
            fx[0],
            Effect::WriteIndex(IndexEntry {
                stage: Stage::Review,
                kind: TimelineKind::Skipped,
                ..
            })
        ));
        assert_eq!(fx[1..3], [Effect::SetStage(Stage::Test), Effect::RunTest]);
    }

    #[test]
    fn archive_ends_the_session_and_restore_brings_it_back_idle() {
        let mut t = fixture(Stage::Build, AgentKind::ClaudeCode);
        t.primary_session = sess();
        let fx = transition(&t, StageAction::Archive, &ctx()).unwrap();
        assert_eq!(fx[0], Effect::EndSession);
        assert_eq!(fx[1], Effect::SetArchived(true));
        assert!(matches!(
            fx[2],
            Effect::WriteIndex(IndexEntry {
                kind: TimelineKind::Archived,
                stage: Stage::Build,
                ..
            })
        ));
        let mut a = fixture(Stage::Build, AgentKind::ClaudeCode);
        a.archived = true;
        assert!(matches!(
            transition(&a, StageAction::Archive, &ctx()),
            Err(Error::Invalid(_))
        ));
        let fx = transition(&a, StageAction::Restore, &ctx()).unwrap();
        assert_eq!(
            fx[0..2],
            [Effect::SetArchived(false), Effect::SetState(AgentState::Idle)]
        );
        assert!(matches!(
            transition(&t, StageAction::Restore, &ctx()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn archive_from_done_without_a_session_has_no_end_session() {
        let fx = transition(
            &fixture(Stage::Done, AgentKind::ClaudeCode),
            StageAction::Archive,
            &ctx(),
        )
        .unwrap();
        assert_eq!(fx[0], Effect::SetArchived(true));
    }
}
