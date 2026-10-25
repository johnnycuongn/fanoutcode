//! Tests of `transition`, `next_stage` and `on_test_finished`.

use super::*;
use crate::domain::{AgentKind, AgentState, HandbackReason, SessionId, Stage, WorktreeChoice};
use crate::helpers::HelperKind;
use crate::memory::{IndexEntry, TimelineKind};
use crate::test_support::{ctx, fixture};
use crate::Error;

fn sess() -> Option<SessionId> {
    Some(SessionId::parse("s-1a-1").unwrap())
}

#[test]
fn approve_spec_records_handback_and_enters_plan_with_a_new_session() {
    let fx = transition(
        &fixture(Stage::Spec, AgentKind::ClaudeCode),
        StageAction::Approve,
        &ctx(),
    )
    .unwrap();
    assert!(
        matches!(fx[0], Effect::WriteIndex(IndexEntry { kind: TimelineKind::Handback { ref chip }, stage: Stage::Spec, .. }) if chip == "Spec ready")
    );
    assert_eq!(
        fx[1..],
        [
            Effect::SetStage(Stage::Plan),
            Effect::StartSession,
            Effect::InjectPrompt("PLAN".into()),
            Effect::SetState(AgentState::Working)
        ]
    );
}

#[test]
fn plan_entry_creates_the_worktree_before_the_session_when_the_flag_is_on() {
    let mut t = fixture(Stage::Spec, AgentKind::ClaudeCode);
    t.worktree = WorktreeChoice::On;
    let fx = transition(&t, StageAction::Advance, &ctx()).unwrap();
    let i = |e: &Effect| fx.iter().position(|x| x == e).unwrap();
    assert!(i(&Effect::CreateWorktree) < i(&Effect::StartSession));
}

#[test]
fn existing_session_is_reused_not_restarted() {
    let mut t = fixture(Stage::Plan, AgentKind::ClaudeCode);
    t.primary_session = sess();
    let fx = transition(&t, StageAction::Advance, &ctx()).unwrap();
    assert_eq!(
        fx,
        [
            Effect::SetStage(Stage::Build),
            Effect::InjectPrompt("BUILD".into()),
            Effect::SetState(AgentState::Working)
        ]
    );
}

#[test]
fn shell_tasks_get_a_terminal_but_no_prompt() {
    let fx = transition(&fixture(Stage::Spec, AgentKind::Shell), StageAction::Approve, &ctx()).unwrap();
    assert!(fx.contains(&Effect::StartSession));
    assert!(!fx.iter().any(|e| matches!(e, Effect::InjectPrompt(_))));
    assert!(fx.contains(&Effect::SetState(AgentState::Idle)));
}

#[test]
fn review_entry_starts_the_spec_watcher() {
    let fx = transition(
        &fixture(Stage::Build, AgentKind::ClaudeCode),
        StageAction::RequestReview,
        &ctx(),
    )
    .unwrap();
    assert_eq!(fx.last(), Some(&Effect::RunHelper(HelperKind::SpecWatcher)));
}

#[test]
fn approve_changes_goes_to_test_only_when_a_command_exists_and_test_is_enabled() {
    let t = fixture(Stage::Review, AgentKind::ClaudeCode);
    assert_eq!(
        transition(&t, StageAction::ApproveChanges, &ctx()).unwrap()[0],
        Effect::SetStage(Stage::Test)
    );
    let mut c = ctx();
    c.test_command = None;
    assert_eq!(
        transition(&t, StageAction::ApproveChanges, &c).unwrap()[0],
        Effect::SetStage(Stage::Commit)
    );
    let mut c = ctx();
    c.test_enabled = false;
    assert_eq!(
        transition(&t, StageAction::ApproveChanges, &c).unwrap()[0],
        Effect::SetStage(Stage::Commit)
    );
}

#[test]
fn test_entry_runs_the_tests_and_commit_entry_drafts_messages() {
    assert_eq!(
        transition(
            &fixture(Stage::Review, AgentKind::ClaudeCode),
            StageAction::ApproveChanges,
            &ctx()
        )
        .unwrap(),
        [
            Effect::SetStage(Stage::Test),
            Effect::RunTest,
            Effect::SetState(AgentState::Working)
        ]
    );
    assert_eq!(
        transition(
            &fixture(Stage::Test, AgentKind::ClaudeCode),
            StageAction::Advance,
            &ctx()
        )
        .unwrap(),
        [
            Effect::SetStage(Stage::Commit),
            Effect::RunHelper(HelperKind::CommitMessages),
            Effect::SetState(AgentState::Working)
        ]
    );
}

#[test]
fn fix_tests_needs_a_failed_run_and_returns_to_build_with_the_fix_prompt() {
    let t = fixture(Stage::Test, AgentKind::ClaudeCode);
    assert!(matches!(
        transition(&t, StageAction::FixTests, &ctx()),
        Err(Error::Precondition(_))
    ));
    let mut c = ctx();
    c.last_test = Some(TestRun {
        command: "npm test".into(),
        exit_code: 1,
        tail: "FAIL".into(),
    });
    let fx = transition(&t, StageAction::FixTests, &c).unwrap();
    assert_eq!(
        fx,
        [
            Effect::SetStage(Stage::Build),
            Effect::StartSession,
            Effect::InjectPrompt("FIX".into()),
            Effect::SetState(AgentState::Working)
        ]
    );
}

#[test]
fn commit_button_requires_may_commit() {
    let t = fixture(Stage::Commit, AgentKind::ClaudeCode);
    assert!(
        matches!(transition(&t, StageAction::Commit, &ctx()), Err(Error::Precondition(m)) if m.contains("may commit"))
    );
    let mut c = ctx();
    c.may_commit = true;
    assert_eq!(
        transition(&t, StageAction::Commit, &c).unwrap(),
        [Effect::RunHelper(HelperKind::GitCommit)]
    );
    assert_eq!(
        transition(&t, StageAction::CopyCommit, &ctx()).unwrap(),
        [Effect::Toast("Commit message copied".into())]
    );
}

#[test]
fn commit_advances_to_done_with_a_toast_and_done_cannot_advance() {
    let fx = transition(
        &fixture(Stage::Commit, AgentKind::ClaudeCode),
        StageAction::Advance,
        &ctx(),
    )
    .unwrap();
    assert_eq!(
        fx,
        [
            Effect::SetStage(Stage::Done),
            Effect::SetState(AgentState::Idle),
            Effect::Toast("Task done".into())
        ]
    );
    assert!(matches!(
        transition(
            &fixture(Stage::Done, AgentKind::ClaudeCode),
            StageAction::Advance,
            &ctx()
        ),
        Err(Error::Precondition(_))
    ));
}

#[test]
fn start_fresh_ends_then_starts_and_reinjects_the_current_stage_prompt() {
    let mut t = fixture(Stage::Build, AgentKind::ClaudeCode);
    t.primary_session = sess();
    assert_eq!(
        transition(&t, StageAction::StartFresh, &ctx()).unwrap(),
        [
            Effect::EndSession,
            Effect::StartSession,
            Effect::InjectPrompt("BUILD".into()),
            Effect::SetState(AgentState::Working)
        ]
    );
    assert!(matches!(
        transition(
            &fixture(Stage::Spec, AgentKind::ClaudeCode),
            StageAction::StartFresh,
            &ctx()
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn reply_injects_the_prompt_into_the_running_session() {
    let reply = |p: &str| StageAction::Reply { prompt: p.into() };
    let mut t = fixture(Stage::Plan, AgentKind::ClaudeCode);
    t.state = AgentState::NeedsYou {
        reason: HandbackReason::Replied {
            first_line: "Which DB?".into(),
        },
    };
    assert!(
        matches!(transition(&t, reply("Use SQLite"), &ctx()), Err(Error::Precondition(m)) if m.contains("Start the session"))
    );
    t.primary_session = sess();
    assert_eq!(
        transition(&t, reply("Use SQLite"), &ctx()).unwrap(),
        [Effect::InjectPrompt("Use SQLite".into())]
    );
    t.state = AgentState::Working;
    assert_eq!(
        transition(&t, reply("src/a.ts:12 — rename this"), &ctx()).unwrap(),
        [Effect::InjectPrompt("src/a.ts:12 — rename this".into())]
    );
    assert!(matches!(transition(&t, reply("  \n"), &ctx()), Err(Error::Invalid(_))));
}

#[test]
fn stage_action_serialises_with_a_type_tag() {
    assert_eq!(
        serde_json::to_value(StageAction::Approve).unwrap(),
        serde_json::json!({ "type": "Approve" })
    );
    assert_eq!(
        serde_json::to_value(StageAction::Back { stage: Stage::Plan }).unwrap(),
        serde_json::json!({ "type": "Back", "stage": "Plan" })
    );
    let r: StageAction = serde_json::from_value(serde_json::json!({ "type": "Reply", "prompt": "go" })).unwrap();
    assert_eq!(r, StageAction::Reply { prompt: "go".into() });
}

#[test]
fn test_exit_advances_on_green_or_hands_back() {
    let t = fixture(Stage::Test, AgentKind::ClaudeCode);
    assert_eq!(
        on_test_finished(&t, 0, &ctx()).unwrap(),
        [Effect::SetState(AgentState::NeedsYou {
            reason: HandbackReason::StageDone
        })]
    );
    assert_eq!(
        on_test_finished(&t, 2, &ctx()).unwrap(),
        [Effect::SetState(AgentState::NeedsYou {
            reason: HandbackReason::TestFailed { exit_code: 2 }
        })]
    );
    let mut c = ctx();
    c.advance_on_green = true;
    assert_eq!(on_test_finished(&t, 0, &c).unwrap()[0], Effect::SetStage(Stage::Commit));
    assert!(matches!(
        on_test_finished(&fixture(Stage::Build, AgentKind::ClaudeCode), 0, &ctx()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn actions_outside_their_stage_are_invalid() {
    assert!(matches!(
        transition(
            &fixture(Stage::Plan, AgentKind::ClaudeCode),
            StageAction::Approve,
            &ctx()
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        transition(
            &fixture(Stage::Plan, AgentKind::ClaudeCode),
            StageAction::RequestReview,
            &ctx()
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn a_failed_commit_draft_needs_you_and_can_be_drafted_again() {
    let commit = fixture(Stage::Commit, AgentKind::ClaudeCode);
    assert_eq!(
        on_helper_failed(&commit, HelperKind::CommitMessages, "rate limited"),
        Some(AgentState::NeedsYou {
            reason: HandbackReason::Error {
                message: "Couldn't draft the commit messages: rate limited".into()
            }
        })
    );
    assert_eq!(on_helper_failed(&commit, HelperKind::Naming, "x"), None);
    let moved_on = fixture(Stage::Done, AgentKind::ClaudeCode);
    assert_eq!(on_helper_failed(&moved_on, HelperKind::CommitMessages, "x"), None);
    assert_eq!(
        redraft_commits(&commit).unwrap(),
        [
            Effect::RunHelper(HelperKind::CommitMessages),
            Effect::SetState(AgentState::Working)
        ]
    );
    assert!(matches!(redraft_commits(&moved_on), Err(Error::Invalid(_))));
}
