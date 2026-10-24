//! Card edits that are not stage moves: priority, colour, flags, Jira key, and the worktree and agent choices
//! while no session has started.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::domain::{AgentKind, Priority, Task, TaskColor, WorktreeChoice};
use crate::{Error, Result};

/// What `tasks_update` may change; `None` (or a missing field) leaves a field as it is, and `clear_*` empties one.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, specta::Type)]
#[serde(default)]
pub struct TaskPatch {
    pub priority: Option<Priority>,
    pub color: Option<TaskColor>,
    pub clear_color: bool,
    pub flagged: Option<bool>,
    pub hidden: Option<bool>,
    pub jira_key: Option<String>,
    pub clear_jira_key: bool,
    /// Only before the first session starts.
    pub worktree: Option<WorktreeChoice>,
    /// Only before the first session starts.
    pub agent_kind: Option<AgentKind>,
}

/// The task with the patch's fields copied and `updated_at = now`. The worktree and agent choices are fixed once a
/// session has started (or a Claude session can be resumed): changing them then is a `Precondition`.
pub fn apply_patch(task: &Task, patch: &TaskPatch, now: DateTime<Utc>) -> Result<Task> {
    let started = task.primary_session.is_some() || task.claude_session_id.is_some();
    if started && (patch.worktree.is_some() || patch.agent_kind.is_some()) {
        return Err(Error::Precondition(
            "The worktree and agent settings are fixed once the session has started. Use Start fresh session to change them."
                .into(),
        ));
    }
    let mut t = task.clone();
    if let Some(priority) = patch.priority {
        t.priority = priority;
    }
    if patch.clear_color {
        t.color = None;
    } else if let Some(color) = &patch.color {
        t.color = Some(color.clone());
    }
    if let Some(flagged) = patch.flagged {
        t.flagged = flagged;
    }
    if let Some(hidden) = patch.hidden {
        t.hidden = hidden;
    }
    if patch.clear_jira_key {
        t.jira_key = None;
    } else if let Some(key) = patch.jira_key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        t.jira_key = Some(key.to_owned());
    }
    if let Some(worktree) = patch.worktree {
        t.worktree = worktree;
    }
    if let Some(kind) = patch.agent_kind {
        t.agent_kind = kind;
    }
    t.updated_at = now;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::{SessionId, Stage};
    use crate::test_support::fixture;

    #[test]
    fn patch_copies_only_the_given_fields() {
        let t = fixture(Stage::Spec, AgentKind::ClaudeCode);
        let now = Utc::now();
        let p = TaskPatch {
            priority: Some(Priority::High),
            color: Some(TaskColor("violet".into())),
            clear_color: false,
            flagged: Some(true),
            hidden: None,
            jira_key: Some("TC-3".into()),
            clear_jira_key: false,
            worktree: Some(WorktreeChoice::On),
            agent_kind: None,
        };
        let u = apply_patch(&t, &p, now).unwrap();
        assert_eq!(
            (
                u.priority,
                u.color.as_ref().map(|c| c.0.as_str()),
                u.flagged,
                u.hidden,
                u.jira_key.as_deref(),
                u.worktree,
                u.updated_at
            ),
            (
                Priority::High,
                Some("violet"),
                true,
                false,
                Some("TC-3"),
                WorktreeChoice::On,
                now
            )
        );
        let cleared = apply_patch(
            &u,
            &TaskPatch {
                clear_color: true,
                clear_jira_key: true,
                ..TaskPatch::default()
            },
            now,
        )
        .unwrap();
        assert!(cleared.color.is_none() && cleared.jira_key.is_none());
    }

    #[test]
    fn worktree_and_agent_kind_are_fixed_once_a_session_exists() {
        let mut t = fixture(Stage::Plan, AgentKind::ClaudeCode);
        t.primary_session = Some(SessionId::parse("s-1a-1").unwrap());
        let p = TaskPatch {
            worktree: Some(WorktreeChoice::On),
            ..TaskPatch::default()
        };
        assert!(
            matches!(apply_patch(&t, &p, Utc::now()), Err(Error::Precondition(m)) if m.contains("Start fresh session"))
        );
        let p = TaskPatch {
            agent_kind: Some(AgentKind::Codex),
            ..TaskPatch::default()
        };
        assert!(matches!(apply_patch(&t, &p, Utc::now()), Err(Error::Precondition(_))));
        let p = TaskPatch {
            flagged: Some(true),
            ..TaskPatch::default()
        };
        assert!(
            apply_patch(&t, &p, Utc::now()).unwrap().flagged,
            "other fields still change"
        );
        let mut resumed = fixture(Stage::Plan, AgentKind::ClaudeCode);
        resumed.claude_session_id = Some("abc".into());
        assert!(matches!(
            apply_patch(
                &resumed,
                &TaskPatch {
                    worktree: Some(WorktreeChoice::On),
                    ..TaskPatch::default()
                },
                Utc::now()
            ),
            Err(Error::Precondition(_))
        ));
    }
}
