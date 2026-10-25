//! Cross-task file awareness (Plan 11, `docs/superpowers/specs/2026-10-25-cross-task-file-awareness-design.md`):
//! claims, grants, the claim-check hook, the overlap and footprint stores, and every overlap prompt text.

pub mod check;
pub mod claims;
pub mod compute;
pub mod decide;
pub mod footprint;
pub mod git_rules;
pub mod grants;
pub mod integrate;
pub mod notes;
pub mod others;
pub mod prompts;
pub mod store;

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{AgentState, HandbackReason, TaskId};

/// How a file in this task's footprint or change set relates to the other task's work on it (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Relation {
    /// The other task has uncommitted changes in it (a shared working tree).
    Claimed,
    /// The developer let this task edit on top of the other task's changes.
    GrantedOnTop,
    /// This task replaced the other task's changes; the path left the other task's change set.
    TakenOver,
    /// The other task lists it under `## Files` in its plan (worktree mode).
    PlannedBy,
    /// The other task committed its changes to it.
    Committed,
}

/// A stop waiting on the developer: the write the hook denied, or the shell command (§5.4, §5.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum PendingDecision {
    /// An `Edit`, `Write`, `MultiEdit` or `NotebookEdit` and its tool input snapshot.
    Edit {
        tool: String,
        input: serde_json::Value,
    },
    Command {
        command: String,
    },
}

/// The developer's answer to a pending decision (§5.4): on top for this file, on top for every file the other task
/// touches, replace the other task's changes in this file, or hold off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum Decision {
    OnTopFile,
    OnTopAll,
    Replace,
    Hold,
}

/// One `task_overlaps` row: `path` in `repo_root` of `task_id` overlaps `other_task` (`None`: an unattributed
/// session). `held` is set while the developer chose Hold; `pending` stays until the claim is released.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct OverlapRow {
    pub task_id: TaskId,
    pub other_task: Option<TaskId>,
    pub repo_root: PathBuf,
    pub path: String,
    pub relation: Relation,
    pub pending: Option<PendingDecision>,
    pub held: bool,
    pub noted_out: bool,
    pub noted_in: bool,
    pub decided_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// The chip feed's derived state while `pending` decisions wait (§5.4): suppressed (`None`, so the caller keeps the
/// pinned `NeedsYou`) while any is pending; with none, `derived` unchanged.
pub fn pinned(pending: u32, derived: Option<AgentState>) -> Option<AgentState> {
    if pending > 0 {
        None
    } else {
        derived
    }
}

/// The hand-back reason a task's pending rows pin it to (§5.4): none pending, `None`; every pending row held,
/// `HoldingFor` the first held row's other task (its title, "another session" when unattributed); otherwise
/// `FileInUse` with the pending count.
pub fn pin_for(rows: &[OverlapRow], titles: &HashMap<TaskId, String>) -> Option<HandbackReason> {
    let pending: Vec<&OverlapRow> = rows.iter().filter(|r| r.pending.is_some()).collect();
    let first = pending.first()?;
    if pending.iter().all(|r| r.held) {
        let other = first
            .other_task
            .as_ref()
            .and_then(|id| titles.get(id).cloned())
            .unwrap_or_else(|| "another session".to_owned());
        return Some(HandbackReason::HoldingFor { other });
    }
    Some(HandbackReason::FileInUse {
        pending: pending.len() as u32,
    })
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::domain::TaskId;

    fn rt<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(v: T) -> serde_json::Value {
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(serde_json::from_value::<T>(json.clone()).unwrap(), v);
        json
    }

    #[test]
    fn relation_is_a_plain_variant_name() {
        for r in [
            Relation::Claimed,
            Relation::GrantedOnTop,
            Relation::TakenOver,
            Relation::PlannedBy,
            Relation::Committed,
        ] {
            rt(r);
        }
        assert_eq!(rt(Relation::TakenOver), json!("TakenOver"));
    }

    #[test]
    fn pending_decision_is_internally_tagged() {
        let edit = PendingDecision::Edit {
            tool: "Edit".into(),
            input: json!({"file_path": "/r/a.ts", "old_string": "a", "new_string": "b"}),
        };
        assert_eq!(rt(edit)["type"], "Edit");
        assert_eq!(
            rt(PendingDecision::Command {
                command: "git add -A".into()
            }),
            json!({"type": "Command", "command": "git add -A"})
        );
    }

    #[test]
    fn decision_is_internally_tagged_with_unit_variants() {
        for d in [
            Decision::OnTopFile,
            Decision::OnTopAll,
            Decision::Replace,
            Decision::Hold,
        ] {
            rt(d);
        }
        assert_eq!(rt(Decision::OnTopAll), json!({"type": "OnTopAll"}));
    }

    #[test]
    fn overlap_row_round_trips_every_field() {
        let row = OverlapRow {
            task_id: TaskId::parse("t-a1").unwrap(),
            other_task: Some(TaskId::parse("t-b2").unwrap()),
            repo_root: "/r".into(),
            path: "src/a.ts".into(),
            relation: Relation::Claimed,
            pending: Some(PendingDecision::Command {
                command: "git stash".into(),
            }),
            held: true,
            noted_out: true,
            noted_in: false,
            decided_at: Some("2026-10-25T10:00:00Z".parse().unwrap()),
            updated_at: "2026-10-25T10:01:00Z".parse().unwrap(),
        };
        let json = rt(row);
        assert_eq!(
            (json["held"].clone(), json["relation"].clone()),
            (json!(true), json!("Claimed"))
        );
        rt(OverlapRow {
            other_task: None,
            pending: None,
            decided_at: None,
            ..serde_json::from_value(json).unwrap()
        });
    }

    fn row(other: Option<&str>, pending: bool, held: bool) -> OverlapRow {
        OverlapRow {
            task_id: TaskId::parse("t-a1").unwrap(),
            other_task: other.map(|o| TaskId::parse(o).unwrap()),
            repo_root: "/r".into(),
            path: "src/a.ts".into(),
            relation: Relation::Claimed,
            pending: pending.then(|| PendingDecision::Command {
                command: "git add -A".into(),
            }),
            held,
            noted_out: false,
            noted_in: false,
            decided_at: None,
            updated_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        }
    }

    fn titles() -> HashMap<TaskId, String> {
        HashMap::from([
            (TaskId::parse("t-b2").unwrap(), "Login".to_owned()),
            (TaskId::parse("t-c3").unwrap(), "Billing".to_owned()),
        ])
    }

    fn replied() -> AgentState {
        AgentState::NeedsYou {
            reason: HandbackReason::Replied {
                first_line: "done".into(),
            },
        }
    }

    #[test]
    fn pin_overrides_working_and_replied_while_pending() {
        assert_eq!(pinned(1, Some(AgentState::Working)), None);
        assert_eq!(pinned(3, Some(replied())), None);
        assert_eq!(pinned(2, None), None);
    }

    #[test]
    fn no_pin_without_pending() {
        for derived in [Some(AgentState::Working), Some(replied()), Some(AgentState::Idle), None] {
            assert_eq!(pinned(0, derived.clone()), derived);
        }
    }

    #[test]
    fn all_held_pins_holding_for() {
        let rows = [
            row(Some("t-c3"), false, false),
            row(Some("t-b2"), true, true),
            row(Some("t-c3"), true, true),
        ];
        assert_eq!(
            pin_for(&rows, &titles()),
            Some(HandbackReason::HoldingFor { other: "Login".into() })
        );
        assert_eq!(
            pin_for(&[row(None, true, true)], &titles()),
            Some(HandbackReason::HoldingFor {
                other: "another session".into()
            })
        );
    }

    #[test]
    fn any_unheld_pins_file_in_use_with_the_count() {
        let rows = [
            row(Some("t-b2"), true, true),
            row(Some("t-c3"), true, false),
            row(None, true, false),
            row(Some("t-b2"), false, false),
        ];
        assert_eq!(
            pin_for(&rows, &titles()),
            Some(HandbackReason::FileInUse { pending: 3 })
        );
    }

    #[test]
    fn none_pending_pins_nothing() {
        assert_eq!(pin_for(&[], &titles()), None);
        assert_eq!(pin_for(&[row(Some("t-b2"), false, true)], &titles()), None);
    }
}
