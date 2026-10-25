//! The board card's activity snapshot (A6, §10.2 Medium and Detailed cards): the file being edited, the task's
//! `+/-`, its team with each member's state, and its cost. Built from in-memory state only, so it returns at once.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::changes::{repo_for, ChangeSet, MAIN_AGENT};
pub use crate::cost::LiveCost;
use crate::domain::{AgentState, HandbackReason, Task, TaskId};
use crate::hooks::{HookEvent, TaskEvent};

/// One member of the task's team: the main session, or a subagent named by its type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TeamRow {
    pub name: String,
    pub state: AgentState,
}

/// What a board card shows beyond the task (A6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct TaskActivity {
    pub task_id: TaskId,
    pub current_file: Option<String>,
    pub plus: u32,
    pub minus: u32,
    pub team: Vec<TeamRow>,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone)]
struct Member {
    agent_id: String,
    name: String,
    state: AgentState,
}

/// One task's live activity, fed every hook event (live and replayed).
#[derive(Debug, Clone, Default)]
pub struct ActivityTracker {
    current_file: Option<String>,
    team: Vec<Member>,
}

/// The edited file of an `Edit`, `Write`, `MultiEdit` or `NotebookEdit`.
fn edited(ev: &HookEvent) -> Option<&str> {
    let HookEvent::PostToolUse { tool, input, .. } = ev else {
        return None;
    };
    let key = match tool.as_str() {
        "Edit" | "Write" | "MultiEdit" => "file_path",
        "NotebookEdit" => "notebook_path",
        _ => return None,
    };
    input.get(key).and_then(Value::as_str)
}

impl ActivityTracker {
    fn member(&mut self, agent: Option<&str>) -> Option<&mut Member> {
        let id = agent?;
        self.team.iter_mut().find(|m| m.agent_id == id)
    }

    pub fn on_event(&mut self, ev: &TaskEvent, repos: &[PathBuf]) {
        let agent = ev.agent_id.as_deref();
        if let Some(path) = edited(&ev.event) {
            let abs = Path::new(path);
            self.current_file = Some(match repo_for(abs, repos).and_then(|r| abs.strip_prefix(r).ok()) {
                Some(rel) => rel.to_string_lossy().into_owned(),
                None => path.to_owned(),
            });
        }
        match &ev.event {
            HookEvent::SubagentStart {
                agent_id, agent_type, ..
            } => {
                self.team.retain(|m| &m.agent_id != agent_id);
                self.team.push(Member {
                    agent_id: agent_id.clone(),
                    name: agent_type.clone(),
                    state: AgentState::Working,
                });
            }
            HookEvent::SubagentStop { agent_id } => {
                if let Some(m) = self.member(Some(agent_id)) {
                    m.state = AgentState::Idle;
                }
            }
            HookEvent::PermissionRequest { .. } => {
                if let Some(m) = self.member(agent) {
                    m.state = AgentState::NeedsYou {
                        reason: HandbackReason::PermissionNeeded,
                    };
                }
            }
            HookEvent::PreToolUse { .. } | HookEvent::PostToolUse { .. } => {
                if let Some(m) = self.member(agent) {
                    m.state = AgentState::Working;
                }
            }
            HookEvent::Stop { .. } | HookEvent::StopFailure { .. } if agent.is_none() => self.current_file = None,
            HookEvent::UserPromptSubmit if agent.is_none() => self.team.retain(|m| m.state != AgentState::Idle),
            _ => {}
        }
    }

    /// The card's activity: `plus`/`minus` over every repo's own `files` (never `other`), the main row first.
    pub fn snapshot(&self, task: &Task, set: Option<&ChangeSet>, cost_usd: Option<f64>) -> TaskActivity {
        let files = set.into_iter().flat_map(|s| &s.repos).flat_map(|r| &r.files);
        let (plus, minus) = files.fold((0, 0), |(p, m), f| (p + f.added, m + f.removed));
        let mut team = vec![TeamRow {
            name: MAIN_AGENT.to_owned(),
            state: task.state.clone(),
        }];
        team.extend(self.team.iter().map(|m| TeamRow {
            name: m.name.clone(),
            state: m.state.clone(),
        }));
        TaskActivity {
            task_id: task.id.clone(),
            current_file: self.current_file.clone(),
            plus,
            minus,
            team,
            cost_usd,
        }
    }
}

/// The larger of the ledger total and the live status-line figure (the ledger lags until the next Stop).
pub fn card_cost(ledger_usd: Option<f64>, live: Option<&LiveCost>) -> Option<f64> {
    match (ledger_usd, live.map(|l| l.cost_usd)) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use serde_json::{json, Value};

    use super::*;
    use crate::changes::{ChangeSource, ChangedFile, RepoChanges};
    use crate::git::FileStatus;
    use crate::test_support::task_fixture;

    fn ev(event: HookEvent, agent: Option<&str>) -> TaskEvent {
        TaskEvent {
            seq: 1,
            at: Utc::now(),
            task_id: TaskId::parse("t-1a-1").unwrap(),
            agent_id: agent.map(str::to_owned),
            event,
        }
    }

    fn post_edit(path: &str, agent: Option<&str>) -> TaskEvent {
        ev(
            HookEvent::PostToolUse {
                tool: "Edit".into(),
                input: json!({"file_path": path}),
                response: Value::Null,
            },
            agent,
        )
    }

    fn stop(agent: Option<&str>) -> TaskEvent {
        ev(
            HookEvent::Stop {
                last_assistant_message: "x".into(),
                background_tasks: 0,
            },
            agent,
        )
    }

    fn subagent_start(id: &str, kind: &str) -> TaskEvent {
        ev(
            HookEvent::SubagentStart {
                agent_id: id.into(),
                agent_type: kind.into(),
                description: None,
            },
            Some(id),
        )
    }

    fn subagent_stop(id: &str) -> TaskEvent {
        ev(HookEvent::SubagentStop { agent_id: id.into() }, Some(id))
    }

    fn permission(agent: Option<&str>) -> TaskEvent {
        ev(HookEvent::PermissionRequest { tool: "Bash".into() }, agent)
    }

    fn prompt() -> TaskEvent {
        ev(HookEvent::UserPromptSubmit, None)
    }

    fn task(state: AgentState) -> Task {
        let mut t = task_fixture("t-1a-1", "p-1a-1");
        t.state = state;
        t
    }

    fn change_set(files: &[(&str, u32, u32)], other: &[(&str, u32, u32)]) -> ChangeSet {
        let f = |(p, a, r): &(&str, u32, u32)| ChangedFile {
            path: (*p).into(),
            status: FileStatus::Modified,
            added: *a,
            removed: *r,
            binary: false,
            owners: vec![],
            shared: false,
            source: ChangeSource::Hook,
        };
        ChangeSet {
            task_id: TaskId::parse("t-1a-1").unwrap(),
            repos: vec![RepoChanges {
                repo_root: "/repo".into(),
                base: None,
                files: files.iter().map(f).collect(),
                other: other.iter().map(f).collect(),
            }],
        }
    }

    #[test]
    fn current_file_is_the_last_edit_relative_to_its_repo() {
        let mut a = ActivityTracker::default();
        let repos = vec![PathBuf::from("/repo")];
        a.on_event(&post_edit("/repo/src/a.rs", None), &repos);
        a.on_event(&post_edit("/repo/src/b.rs", Some("a1")), &repos);
        assert_eq!(
            a.snapshot(&task(AgentState::Working), None, None)
                .current_file
                .as_deref(),
            Some("src/b.rs")
        );
    }

    #[test]
    fn current_file_clears_when_the_main_agent_stops() {
        let mut a = ActivityTracker::default();
        let repos = vec![PathBuf::from("/repo")];
        a.on_event(&post_edit("/repo/src/a.rs", None), &repos);
        a.on_event(&stop(None), &repos);
        assert_eq!(a.snapshot(&task(AgentState::Idle), None, None).current_file, None);
    }

    #[test]
    fn plus_and_minus_sum_the_tasks_own_files_only() {
        let set = change_set(&[("a.rs", 3, 1), ("b.rs", 2, 0)], &[("README.md", 10, 10)]); // files, other
        let s = ActivityTracker::default().snapshot(&task(AgentState::Idle), Some(&set), None);
        assert_eq!((s.plus, s.minus), (5, 1));
    }

    #[test]
    fn team_lists_main_then_subagents_with_their_state() {
        let mut a = ActivityTracker::default();
        let r: Vec<PathBuf> = vec![];
        a.on_event(&subagent_start("a1", "frontend"), &r);
        a.on_event(&subagent_start("a2", "backend"), &r);
        a.on_event(&permission(Some("a2")), &r);
        a.on_event(&subagent_stop("a1"), &r);
        assert_eq!(
            a.snapshot(&task(AgentState::Working), None, None).team,
            vec![
                TeamRow {
                    name: "main".into(),
                    state: AgentState::Working
                },
                TeamRow {
                    name: "frontend".into(),
                    state: AgentState::Idle
                },
                TeamRow {
                    name: "backend".into(),
                    state: AgentState::NeedsYou {
                        reason: HandbackReason::PermissionNeeded
                    }
                },
            ]
        );
    }

    #[test]
    fn idle_subagents_are_dropped_at_the_next_prompt() {
        let mut a = ActivityTracker::default();
        let r: Vec<PathBuf> = vec![];
        a.on_event(&subagent_start("a1", "frontend"), &r);
        a.on_event(&subagent_stop("a1"), &r);
        a.on_event(&prompt(), &r);
        assert_eq!(a.snapshot(&task(AgentState::Working), None, None).team.len(), 1);
    }

    #[test]
    fn card_cost_takes_the_larger_of_ledger_and_live() {
        let live = LiveCost {
            cost_usd: 0.42,
            context_used_pct: 10.0,
            at: Utc::now(),
        };
        assert_eq!(card_cost(None, None), None);
        assert_eq!(card_cost(Some(0.10), None), Some(0.10));
        assert_eq!(card_cost(Some(0.10), Some(&live)), Some(0.42));
        assert_eq!(card_cost(Some(0.50), Some(&live)), Some(0.50));
    }

    #[test]
    fn task_activity_serialises_with_snake_case_fields() {
        // overview A16: no rename_all
        let s = TaskActivity {
            task_id: TaskId::parse("t-1").unwrap(),
            current_file: Some("src/a.rs".into()),
            plus: 1,
            minus: 0,
            team: vec![TeamRow {
                name: "main".into(),
                state: AgentState::Working,
            }],
            cost_usd: Some(0.5),
        };
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            json!({"task_id": "t-1", "current_file": "src/a.rs", "plus": 1, "minus": 0,
                "team": [{"name": "main", "state": {"type": "Working"}}], "cost_usd": 0.5})
        );
    }
}
