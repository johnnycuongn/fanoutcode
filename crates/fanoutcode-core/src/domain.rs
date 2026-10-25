//! The shared domain types (contracts "Core types", with amendments A1, A5, A11, A16, A17).
//!
//! Serde: no `rename_all`, so fields stay `snake_case` and variants keep their Rust names; unit-only enums are plain
//! strings; data-carrying enums are internally tagged `#[serde(tag = "type")]` with unit or struct variants only
//! (never tuple variants: serde rejects those at runtime). specta reads the same attributes.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::ids::{PaneId, ProjectId, SessionId, TaskId, ViewId, WindowId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Stage {
    Spec,
    Plan,
    Build,
    Review,
    Test,
    Commit,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum AgentState {
    Working,
    NeedsYou { reason: HandbackReason },
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum HandbackReason {
    /// Sentinel plus artifact; the chip text per stage is `handback::chip_text` (Plan 03).
    StageDone,
    /// Stop without a sentinel.
    Replied {
        first_line: String,
    },
    PermissionNeeded,
    TestFailed {
        exit_code: i32,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum AgentKind {
    ClaudeCode,
    Codex,
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Priority {
    None,
    Low,
    Medium,
    High,
}

/// A swatch token name from `SWATCHES`, e.g. `"violet"`; separate from the stage colours.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
#[specta(transparent)]
pub struct TaskColor(pub String);

/// A swatch token name from `SWATCHES`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
#[specta(transparent)]
pub struct ProjectColor(pub String);

/// Per task, resolved from the project default at creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum WorktreeChoice {
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum WorktreeConvention {
    ClaudeCode,
    FanoutCodeManaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum LinkPolicy {
    ReadAndModifyOnRequest,
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum DetailLevel {
    Simple,
    Medium,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum LiveSpecLevel {
    Off,
    Simple,
    Heavy,
}

/// The `theme` setting. `System` follows macOS; the webview resolves it (`src/lib/theme.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub root: PathBuf,
    pub color: ProjectColor,
    pub memory_location: MemoryLocation,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum MemoryLocation {
    AppStorage,
    Folder { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    pub projects: Vec<WorkspaceProject>,
    pub policy: LinkPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceProject {
    pub project_id: ProjectId,
    pub order: u32,
    pub protected: bool,
}

/// What a window, a view or a board shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum Container {
    Project { id: ProjectId },
    Workspace { id: WorkspaceId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub workspace_id: Option<WorkspaceId>,
    pub title: String,
    pub stage: Stage,
    pub state: AgentState,
    pub priority: Priority,
    pub color: Option<TaskColor>,
    pub flagged: bool,
    pub hidden: bool,
    pub archived: bool,
    pub jira_key: Option<String>,
    pub agent_kind: AgentKind,
    pub worktree: WorktreeChoice,
    pub worktree_path: Option<PathBuf>,
    pub branch: Option<String>,
    pub memory_dir: PathBuf,
    pub primary_session: Option<SessionId>,
    pub claude_session_id: Option<String>,
    /// Fixed at first session start (§4.1).
    pub linked_roots: Vec<PathBuf>,
    /// Per repo (§9).
    pub base: Vec<RepoBase>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub stage_entered_at: DateTime<Utc>,
    /// For "elapsed in the current state" (A11).
    pub state_entered_at: DateTime<Utc>,
    /// When `linked_roots` were fixed (A11).
    pub links_bound_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct RepoBase {
    pub repo_root: PathBuf,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct View {
    pub id: ViewId,
    pub container: Container,
    pub name: String,
    pub order: u32,
    pub layout: LayoutNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum LayoutNode {
    Leaf {
        pane: PaneId,
        content: PaneContent,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// `Horizontal` is side by side (⌘D), `Vertical` is stacked (⌘⇧D) (A5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum PaneContent {
    Empty,
    Task {
        task_id: TaskId,
    },
    /// Rendered by Plan 06 through Plan 09's `CanvasPane` (A5).
    TeamCanvas {
        task_id: TaskId,
    },
    Terminal {
        project_id: ProjectId,
        session_id: SessionId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Session {
    pub id: SessionId,
    pub task_id: Option<TaskId>,
    pub kind: AgentKind,
    pub cwd: PathBuf,
    pub driver: DriverKind,
    pub attached_to: Option<(WindowId, PaneId)>,
    pub started_at: DateTime<Utc>,
    pub exited: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum DriverKind {
    Owned,
    Background,
}

/// Task and project colour token names (`*-swatch-<name>` in `src/styles.css`); none is a stage name.
pub const SWATCHES: [&str; 10] = [
    "rose", "orange", "yellow", "lime", "emerald", "cyan", "indigo", "violet", "fuchsia", "slate",
];

/// The swatch for the `index`-th project or task, round robin.
pub fn swatch_for(index: usize) -> &'static str {
    SWATCHES[index % SWATCHES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    #[test]
    fn enums_serialise_with_type_tags_and_rust_names() {
        assert_eq!(serde_json::to_string(&Stage::Spec).unwrap(), "\"Spec\"");
        assert_eq!(
            serde_json::to_string(&AgentState::NeedsYou {
                reason: HandbackReason::Replied {
                    first_line: "ok".into()
                }
            })
            .unwrap(),
            r#"{"type":"NeedsYou","reason":{"type":"Replied","first_line":"ok"}}"#
        );
        assert_eq!(serde_json::to_string(&AgentState::Idle).unwrap(), r#"{"type":"Idle"}"#);
        assert_eq!(
            serde_json::to_string(&Container::Project {
                id: ProjectId::parse("p-1").unwrap()
            })
            .unwrap(),
            r#"{"type":"Project","id":"p-1"}"#
        );
        assert_eq!(
            serde_json::to_string(&MemoryLocation::Folder { path: "/tmp/m".into() }).unwrap(),
            r#"{"type":"Folder","path":"/tmp/m"}"#
        );
        assert_eq!(
            serde_json::to_string(&MemoryLocation::AppStorage).unwrap(),
            r#"{"type":"AppStorage"}"#
        );
        assert_eq!(
            serde_json::to_string(&PaneContent::TeamCanvas {
                task_id: TaskId::parse("t-1").unwrap()
            })
            .unwrap(),
            r#"{"type":"TeamCanvas","task_id":"t-1"}"#
        );
        assert_eq!(serde_json::to_string(&ThemePreference::System).unwrap(), "\"System\"");
    }

    fn rt<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(v: T) {
        assert_eq!(
            serde_json::from_str::<T>(&serde_json::to_string(&v).unwrap()).unwrap(),
            v
        );
    }

    #[test]
    fn every_data_carrying_enum_round_trips() {
        // serde checks internally tagged variants only when serialising, so every variant of every tagged enum is
        // exercised here (A17)
        let t = TaskId::parse("t-1").unwrap();
        let p = ProjectId::parse("p-1").unwrap();
        let s = SessionId::parse("s-1").unwrap();
        for r in [
            HandbackReason::StageDone,
            HandbackReason::Replied {
                first_line: "ok".into(),
            },
            HandbackReason::PermissionNeeded,
            HandbackReason::TestFailed { exit_code: 1 },
            HandbackReason::Error { message: "x".into() },
        ] {
            rt(AgentState::NeedsYou { reason: r });
        }
        rt(AgentState::Working);
        rt(AgentState::Idle);
        rt(Container::Project { id: p.clone() });
        rt(Container::Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
        });
        rt(MemoryLocation::Folder { path: "/tmp/m".into() });
        rt(MemoryLocation::AppStorage);
        for c in [
            PaneContent::Empty,
            PaneContent::Task { task_id: t.clone() },
            PaneContent::TeamCanvas { task_id: t },
            PaneContent::Terminal {
                project_id: p,
                session_id: s,
            },
        ] {
            rt(LayoutNode::Split {
                direction: SplitDirection::Horizontal,
                ratio: 0.5,
                first: Box::new(LayoutNode::Leaf {
                    pane: PaneId::parse("pane-1").unwrap(),
                    content: c,
                }),
                second: Box::new(LayoutNode::Leaf {
                    pane: PaneId::parse("pane-2").unwrap(),
                    content: PaneContent::Empty,
                }),
            });
        }
    }

    #[test]
    fn task_round_trips_through_json() {
        let now = chrono::Utc::now();
        let task = Task {
            id: TaskId::parse("t-1").unwrap(),
            project_id: ProjectId::parse("p-1").unwrap(),
            workspace_id: Some(WorkspaceId::parse("ws-1").unwrap()),
            title: "Add login".into(),
            stage: Stage::Plan,
            state: AgentState::NeedsYou {
                reason: HandbackReason::StageDone,
            },
            priority: Priority::High,
            color: Some(TaskColor("violet".into())),
            flagged: true,
            hidden: false,
            archived: false,
            jira_key: Some("ACME-1".into()),
            agent_kind: AgentKind::ClaudeCode,
            worktree: WorktreeChoice::On,
            worktree_path: Some("/tmp/wt".into()),
            branch: Some("fo/add-login".into()),
            memory_dir: "/tmp/m/2026-10-24-add-login".into(),
            primary_session: Some(SessionId::parse("s-1").unwrap()),
            claude_session_id: Some("abc".into()),
            linked_roots: vec!["/tmp/other".into()],
            base: vec![RepoBase {
                repo_root: "/tmp/repo".into(),
                commit: "deadbeef".into(),
            }],
            created_at: now,
            updated_at: now,
            stage_entered_at: now,
            state_entered_at: now,
            links_bound_at: Some(now),
        };
        rt(task);
    }

    #[test]
    fn swatches_rotate_and_avoid_stage_names() {
        assert_eq!(swatch_for(0), "rose");
        assert_eq!(swatch_for(10), "rose");
        assert_eq!(SWATCHES.len(), 10);
        for s in SWATCHES {
            assert!(!["spec", "plan", "build", "review", "test", "commit", "done"].contains(&s));
        }
    }

    #[test]
    fn specta_exports_task_with_snake_case_fields() {
        let ts = specta_typescript::export::<Task>(&Default::default()).unwrap();
        for f in [
            "stage_entered_at: string",
            "state_entered_at: string",
            "links_bound_at: string | null",
            "memory_dir: string",
        ] {
            assert!(ts.contains(f), "{f}");
        }
        assert!(!ts.contains("stageEnteredAt"));
    }
}
