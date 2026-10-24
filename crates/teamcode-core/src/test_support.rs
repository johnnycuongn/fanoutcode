//! Fixtures shared by the crate's tests.

use std::path::PathBuf;

use crate::domain::{
    AgentKind, AgentState, MemoryLocation, Priority, Project, ProjectColor, ProjectId, Stage, Task, TaskId,
    WorktreeChoice,
};
use crate::pipeline::{StagePrompts, TransitionCtx};

/// A project rooted at `/repos/<id>`, named `id`, coloured `indigo`.
pub fn project_fixture(id: &str, loc: MemoryLocation) -> Project {
    Project {
        id: ProjectId::parse(id).unwrap(),
        name: id.to_owned(),
        root: PathBuf::from(format!("/repos/{id}")),
        color: ProjectColor("indigo".into()),
        memory_location: loc,
        created_at: chrono::Utc::now(),
    }
}

/// A Spec, Idle, Claude Code task with no worktree, sessions or links; memory at `/m/<id>`.
pub fn task_fixture(id: &str, project_id: &str) -> Task {
    let now = chrono::Utc::now();
    Task {
        id: TaskId::parse(id).unwrap(),
        project_id: ProjectId::parse(project_id).unwrap(),
        workspace_id: None,
        title: format!("Task {id}"),
        stage: Stage::Spec,
        state: AgentState::Idle,
        priority: Priority::None,
        color: None,
        flagged: false,
        hidden: false,
        archived: false,
        jira_key: None,
        agent_kind: AgentKind::ClaudeCode,
        worktree: WorktreeChoice::Off,
        worktree_path: None,
        branch: None,
        memory_dir: PathBuf::from(format!("/m/{id}")),
        primary_session: None,
        claude_session_id: None,
        linked_roots: Vec::new(),
        base: Vec::new(),
        created_at: now,
        updated_at: now,
        stage_entered_at: now,
        state_entered_at: now,
        links_bound_at: None,
    }
}

/// `task_fixture("t-1a-1", "p-1a-1")` at `stage` with `kind`.
pub fn fixture(stage: Stage, kind: AgentKind) -> Task {
    let mut t = task_fixture("t-1a-1", "p-1a-1");
    t.stage = stage;
    t.agent_kind = kind;
    t
}

/// Test command `npm test`, tests enabled, no auto-advance, commits off, no last run; prompts are their stage names.
pub fn ctx() -> TransitionCtx {
    TransitionCtx {
        now: chrono::Utc::now(),
        test_command: Some("npm test".into()),
        test_enabled: true,
        advance_on_green: false,
        may_commit: false,
        last_test: None,
        prompts: StagePrompts {
            spec: "SPEC".into(),
            plan: "PLAN".into(),
            build: "BUILD".into(),
            review: "REVIEW".into(),
            fix_tests: Some("FIX".into()),
        },
    }
}
