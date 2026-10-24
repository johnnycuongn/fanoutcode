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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
            last_assistant_message: "Done.\n\nTEAMCODE: build done".into(),
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
