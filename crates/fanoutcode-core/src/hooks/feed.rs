//! What an event means for the card's chip (§6): the pipeline's hand-back detection, fed from hook events.

use super::HookEvent;
use crate::domain::{AgentState, HandbackReason, Stage};
use crate::pipeline::detect_handback;

/// The state an event moves the task to; `None` leaves it. A prompt or a tool call is `Working`; a permission
/// prompt or a failed turn needs you; a `Stop` is Plan 03's `detect_handback` (running background tasks keep it
/// `Working`).
pub fn next_state(stage: Stage, ev: &HookEvent, artifact_changed: bool) -> Option<AgentState> {
    match ev {
        HookEvent::UserPromptSubmit | HookEvent::PreToolUse { .. } => Some(AgentState::Working),
        HookEvent::PermissionRequest { .. } => Some(AgentState::NeedsYou {
            reason: HandbackReason::PermissionNeeded,
        }),
        HookEvent::Stop {
            last_assistant_message,
            background_tasks,
        } => Some(
            match detect_handback(stage, last_assistant_message, artifact_changed, *background_tasks) {
                Some(reason) => AgentState::NeedsYou { reason },
                None => AgentState::Working,
            },
        ),
        HookEvent::StopFailure { message } => Some(AgentState::NeedsYou {
            reason: HandbackReason::Error {
                message: message.clone(),
            },
        }),
        _ => None,
    }
}

/// A derived state while the session answers a note the app typed in (another task's notice), given the state the
/// task had when the note went in. The answer is not the developer's work, so it never replaces that state: busy
/// events keep it, and a plain reply restores it. A finished stage, a permission prompt or a failure still shows.
/// Returns the state to apply (`None` keeps the current one) and whether the note's turn is over.
pub fn through_notice(before: &AgentState, next: AgentState) -> (Option<AgentState>, bool) {
    match next {
        AgentState::Working => (None, false),
        AgentState::NeedsYou {
            reason: HandbackReason::Replied { .. },
        } => (Some(before.clone()), true),
        other => (Some(other), true),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn build_done() -> AgentState {
        AgentState::NeedsYou {
            reason: HandbackReason::StageDone,
        }
    }

    #[test]
    fn a_reply_to_a_notice_keeps_the_earlier_hand_back() {
        let reply = AgentState::NeedsYou {
            reason: HandbackReason::Replied {
                first_line: "Noted. My build is finished.".into(),
            },
        };
        assert_eq!(through_notice(&build_done(), reply), (Some(build_done()), true));
    }

    #[test]
    fn work_during_a_notice_leaves_the_chip_alone() {
        assert_eq!(through_notice(&build_done(), AgentState::Working), (None, false));
    }

    #[test]
    fn a_permission_prompt_during_a_notice_still_needs_you() {
        let ask = AgentState::NeedsYou {
            reason: HandbackReason::PermissionNeeded,
        };
        assert_eq!(through_notice(&build_done(), ask.clone()), (Some(ask), true));
    }

    #[test]
    fn user_prompt_and_tool_use_mean_working() {
        assert_eq!(
            next_state(Stage::Build, &HookEvent::UserPromptSubmit, false),
            Some(AgentState::Working)
        );
        assert_eq!(
            next_state(
                Stage::Build,
                &HookEvent::PreToolUse {
                    tool: "Bash".into(),
                    input: json!({})
                },
                false
            ),
            Some(AgentState::Working)
        );
    }

    #[test]
    fn permission_request_needs_you() {
        assert_eq!(
            next_state(
                Stage::Build,
                &HookEvent::PermissionRequest { tool: "Bash".into() },
                false
            ),
            Some(AgentState::NeedsYou {
                reason: HandbackReason::PermissionNeeded
            })
        );
    }

    #[test]
    fn stop_with_sentinel_and_artifact_is_stage_done() {
        let ev = HookEvent::Stop {
            last_assistant_message: "Done.\n\nFANOUTCODE: build done".into(),
            background_tasks: 0,
        };
        assert_eq!(
            next_state(Stage::Build, &ev, true),
            Some(AgentState::NeedsYou {
                reason: HandbackReason::StageDone
            })
        );
    }

    #[test]
    fn stop_without_sentinel_is_replied_with_first_line() {
        let ev = HookEvent::Stop {
            last_assistant_message: "Which database?\nPostgres or SQLite".into(),
            background_tasks: 0,
        };
        assert_eq!(
            next_state(Stage::Plan, &ev, false),
            Some(AgentState::NeedsYou {
                reason: HandbackReason::Replied {
                    first_line: "Which database?".into()
                }
            })
        );
    }

    #[test]
    fn stop_with_background_tasks_keeps_working() {
        let ev = HookEvent::Stop {
            last_assistant_message: "Running".into(),
            background_tasks: 2,
        };
        assert_eq!(next_state(Stage::Build, &ev, false), Some(AgentState::Working));
    }

    #[test]
    fn stop_failure_is_an_error_and_others_change_nothing() {
        assert_eq!(
            next_state(
                Stage::Build,
                &HookEvent::StopFailure {
                    message: "API Error".into()
                },
                false
            ),
            Some(AgentState::NeedsYou {
                reason: HandbackReason::Error {
                    message: "API Error".into()
                }
            })
        );
        assert_eq!(
            next_state(Stage::Build, &HookEvent::Notification { message: "x".into() }, false),
            None
        );
        assert_eq!(
            next_state(
                Stage::Build,
                &HookEvent::Unknown {
                    name: "TeammateIdle".into()
                },
                false
            ),
            None
        );
    }
}
