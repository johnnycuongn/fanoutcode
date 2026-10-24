//! The pipeline (§6): stage helpers, hand-back detection, the transition state machine and applying its effects.
//!
//! `transition` is pure: it returns `Effect`s and never touches a PTY, git or the database.

pub mod apply;
mod enter;
pub mod handback;
pub mod move_to;
pub mod stage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{AgentKind, AgentState, HandbackReason, Stage, Task};
use crate::helpers::HelperKind;
use crate::memory::{IndexEntry, TimelineKind};
use crate::{Error, Result};

pub use apply::{apply, index_doc_for, Applied};
pub use handback::{detect_handback, SENTINEL_PREFIX};
pub use move_to::move_to;
pub use stage::StageArtifact;

/// What the developer (or Plan 05, for Done detection) asks of a task. Internally tagged with struct variants only
/// (A1, A17): `{"type":"Approve"}`, `{"type":"Back","stage":"Plan"}`, `{"type":"Reply","prompt":"…"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum StageAction {
    Approve,
    Advance,
    Back { stage: Stage },
    Reply { prompt: String },
    StartFresh,
    RequestReview,
    ApproveChanges,
    RunTests,
    FixTests,
    CopyCommit,
    Commit,
    Archive,
    Restore,
}

impl StageAction {
    /// The variant name, for error messages.
    pub fn name(&self) -> &'static str {
        match self {
            StageAction::Approve => "Approve",
            StageAction::Advance => "Advance",
            StageAction::Back { .. } => "Back",
            StageAction::Reply { .. } => "Reply",
            StageAction::StartFresh => "Start fresh session",
            StageAction::RequestReview => "Request review",
            StageAction::ApproveChanges => "Approve changes",
            StageAction::RunTests => "Run tests",
            StageAction::FixTests => "Fix failing tests",
            StageAction::CopyCommit => "Copy commit message",
            StageAction::Commit => "Commit",
            StageAction::Archive => "Archive",
            StageAction::Restore => "Restore",
        }
    }
}

/// What a transition asks for. Persistence-shaped variants are applied by `apply`; the rest go back to the shell.
/// Internal to core and shell, never serialised.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    SetStage(Stage),
    SetState(AgentState),
    SetArchived(bool),
    InjectPrompt(String),
    StartSession,
    EndSession,
    CreateWorktree,
    RunTest,
    RunHelper(HelperKind),
    WriteIndex(IndexEntry),
    Toast(String),
}

/// The last run of the project's test command; Plan 05 fills it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct TestRun {
    pub command: String,
    pub exit_code: i32,
    /// The last 200 lines of output.
    pub tail: String,
}

/// The assembled stage prompts (`prompts::assemble`); `fix_tests` only when there is a failed run.
#[derive(Debug, Clone, PartialEq)]
pub struct StagePrompts {
    pub spec: String,
    pub plan: String,
    pub build: String,
    pub review: String,
    pub fix_tests: Option<String>,
}

/// Everything `transition` reads besides the task.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionCtx {
    pub now: DateTime<Utc>,
    pub test_command: Option<String>,
    pub test_enabled: bool,
    pub advance_on_green: bool,
    pub may_commit: bool,
    pub last_test: Option<TestRun>,
    pub prompts: StagePrompts,
}

fn unavailable(action: &StageAction, stage: Stage) -> Error {
    Error::Invalid(format!("{} is not available in {:?}", action.name(), stage))
}

fn already_done() -> Error {
    Error::Precondition("Already done".into())
}

/// The stage `Advance` goes to: Review → Test only with a test command and tests enabled, else Commit.
pub fn next_stage(task: &Task, ctx: &TransitionCtx) -> Result<Stage> {
    match task.stage {
        Stage::Review if ctx.test_command.is_none() || !ctx.test_enabled => Ok(Stage::Commit),
        Stage::Done => Err(already_done()),
        s => Ok(s.next().expect("only Done has no next stage")),
    }
}

/// The effects of `action` on `task` (spec §6).
pub fn transition(task: &Task, action: StageAction, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    let stage = task.stage;
    match (&action, stage) {
        (StageAction::Approve, Stage::Spec) | (StageAction::Advance, Stage::Spec) => {
            let chip = handback::chip_text(&HandbackReason::StageDone, Stage::Spec);
            let mut fx = vec![Effect::WriteIndex(IndexEntry {
                at: ctx.now,
                stage: Stage::Spec,
                kind: TimelineKind::Handback { chip },
            })];
            fx.extend(enter::enter(task, Stage::Plan, ctx)?);
            Ok(fx)
        }
        (StageAction::Advance, Stage::Done) => Err(already_done()),
        (StageAction::Advance, _) | (StageAction::ApproveChanges, Stage::Review) => {
            enter::enter(task, next_stage(task, ctx)?, ctx)
        }
        (StageAction::RequestReview, Stage::Build) => enter::enter(task, Stage::Review, ctx),
        (StageAction::RunTests, Stage::Test) => Ok(vec![Effect::RunTest, Effect::SetState(AgentState::Working)]),
        (StageAction::FixTests, Stage::Test) if task.agent_kind != AgentKind::Shell => {
            let failed = ctx.last_test.as_ref().is_some_and(|r| r.exit_code != 0);
            let (true, Some(fix)) = (failed, ctx.prompts.fix_tests.as_ref()) else {
                return Err(Error::Precondition("Run the tests first".into()));
            };
            let mut fx = vec![Effect::SetStage(Stage::Build)];
            enter::ensure_session(task, &mut fx);
            fx.extend([Effect::InjectPrompt(fix.clone()), Effect::SetState(AgentState::Working)]);
            Ok(fx)
        }
        (StageAction::CopyCommit, Stage::Commit) => Ok(vec![Effect::Toast("Commit message copied".into())]),
        (StageAction::Commit, Stage::Commit) => {
            if !ctx.may_commit {
                return Err(Error::Precondition(
                    "Turn on \"Team Code may commit\" in project settings to commit from here".into(),
                ));
            }
            Ok(vec![Effect::RunHelper(HelperKind::GitCommit)])
        }
        (StageAction::Reply { prompt }, _) => {
            if prompt.trim().is_empty() {
                return Err(Error::Invalid("Reply cannot be empty".into()));
            }
            if task.primary_session.is_none() {
                return Err(Error::Precondition("Start the session before replying".into()));
            }
            // passed through unchanged; Plan 04 holds it until the session is idle
            Ok(vec![Effect::InjectPrompt(prompt.clone())])
        }
        (StageAction::StartFresh, Stage::Plan | Stage::Build | Stage::Review)
            if task.agent_kind != AgentKind::Shell =>
        {
            let mut fx = Vec::new();
            if task.primary_session.is_some() {
                fx.push(Effect::EndSession);
            }
            if enter::needs_worktree(task) {
                fx.push(Effect::CreateWorktree);
            }
            fx.push(Effect::StartSession);
            let p = &ctx.prompts;
            let text = match stage {
                Stage::Plan => &p.plan,
                Stage::Build => &p.build,
                _ => &p.review,
            };
            fx.extend([
                Effect::InjectPrompt(text.clone()),
                Effect::SetState(AgentState::Working),
            ]);
            Ok(fx)
        }
        (StageAction::Back { stage: target }, _) => move_to::back(task, *target, ctx),
        (StageAction::Archive, _) => move_to::archive(task, ctx),
        (StageAction::Restore, _) => move_to::restore(task, ctx),
        _ => Err(unavailable(&action, stage)),
    }
}

/// The test child exited (spec §6.5); Plan 05 calls this.
pub fn on_test_finished(task: &Task, exit_code: i32, ctx: &TransitionCtx) -> Result<Vec<Effect>> {
    if task.stage != Stage::Test {
        return Err(Error::Invalid(format!(
            "Test results are not expected in {:?}",
            task.stage
        )));
    }
    if exit_code == 0 && ctx.advance_on_green {
        return enter::enter(task, Stage::Commit, ctx);
    }
    let reason = if exit_code == 0 {
        HandbackReason::StageDone
    } else {
        HandbackReason::TestFailed { exit_code }
    };
    Ok(vec![Effect::SetState(AgentState::NeedsYou { reason })])
}

#[cfg(test)]
mod tests;
