//! What a session's exit means for its task's card (§7.1). A primary session that ends on its own (a dead
//! `--resume`, a missing `claude`, a crash) must not leave the card "working".

use crate::domain::{AgentState, HandbackReason, Session, SessionId, Stage, Task};

/// The task's new state when `session` exits with `code`: idle on a clean exit, needs you with an error otherwise.
/// `None` when the exit was asked for (`stopping`: a stop, Start fresh, archive, quit), when the session is not the
/// task's primary, or when the stage's state belongs to something else (the tests, the commit-message draft).
pub fn state_after_exit(session: &Session, task: &Task, stopping: bool, code: i32) -> Option<AgentState> {
    let primary: Option<&SessionId> = task.primary_session.as_ref();
    let is_primary = session.task_id.as_ref() == Some(&task.id) && primary == Some(&session.id);
    let session_stage = matches!(task.stage, Stage::Plan | Stage::Build | Stage::Review);
    if stopping || !is_primary || !session_stage || task.archived {
        return None;
    }
    Some(if code == 0 {
        AgentState::Idle
    } else {
        AgentState::NeedsYou {
            reason: HandbackReason::Error {
                message: format!("Session ended (exit {code})"),
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, DriverKind};
    use crate::test_support::task_fixture;

    fn primary_of(task: &mut Task) -> Session {
        let s = Session {
            id: SessionId::new(),
            task_id: Some(task.id.clone()),
            kind: AgentKind::ClaudeCode,
            cwd: "/r".into(),
            driver: DriverKind::Owned,
            attached_to: None,
            started_at: chrono::Utc::now(),
            exited: None,
        };
        task.primary_session = Some(s.id.clone());
        task.stage = Stage::Build;
        task.state = AgentState::Working;
        s
    }

    #[test]
    fn a_primary_session_that_dies_on_its_own_needs_you_with_the_exit_code() {
        let mut t = task_fixture("t-1", "p-1");
        let s = primary_of(&mut t);
        assert_eq!(
            state_after_exit(&s, &t, false, 1),
            Some(AgentState::NeedsYou {
                reason: HandbackReason::Error {
                    message: "Session ended (exit 1)".into()
                }
            })
        );
        assert_eq!(state_after_exit(&s, &t, false, 0), Some(AgentState::Idle));
    }

    #[test]
    fn an_asked_for_stop_another_session_or_a_test_stage_leaves_the_card_alone() {
        let mut t = task_fixture("t-1", "p-1");
        let s = primary_of(&mut t);
        assert_eq!(state_after_exit(&s, &t, true, 137), None, "a stop the app asked for");
        let mut old = s.clone();
        old.id = SessionId::new();
        assert_eq!(
            state_after_exit(&old, &t, false, 1),
            None,
            "not the primary (Start fresh)"
        );
        t.stage = Stage::Test;
        assert_eq!(state_after_exit(&s, &t, false, 1), None, "the tests own the state");
    }
}
