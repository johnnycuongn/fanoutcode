//! Fixtures shared by the crate's tests.
#![allow(dead_code)] // a fixture added for a later test is unused until that test lands

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

/// A migrated in-memory database.
pub fn test_conn() -> rusqlite::Connection {
    crate::db::open_in_memory().unwrap()
}

/// `project_fixture(id, AppStorage)`, inserted.
pub fn insert_project(conn: &rusqlite::Connection, id: &str) -> Project {
    let p = project_fixture(id, MemoryLocation::AppStorage);
    crate::db::projects::insert(conn, &p).unwrap();
    p
}

/// As `insert_project`, rooted at `root` and named after its folder.
pub fn insert_project_at(conn: &rusqlite::Connection, id: &str, root: &str) -> Project {
    let mut p = project_fixture(id, MemoryLocation::AppStorage);
    p.root = PathBuf::from(root);
    p.name = p.root.file_name().unwrap().to_string_lossy().into_owned();
    crate::db::projects::insert(conn, &p).unwrap();
    p
}

/// `task_fixture(id, project_id)`, inserted.
pub fn insert_task(conn: &rusqlite::Connection, id: &str, project_id: &str) -> Task {
    let t = task_fixture(id, project_id);
    crate::db::tasks::insert(conn, &t).unwrap();
    t
}

/// A workspace row and one `workspace_projects` row per project, in order.
pub fn insert_workspace_with(conn: &rusqlite::Connection, ws_id: &str, project_ids: &[&str]) {
    conn.execute(
        "INSERT INTO workspaces (id, name, file_path, updated_at) VALUES (?1, ?1, NULL, '2026-10-24T00:00:00Z')",
        [ws_id],
    )
    .unwrap();
    for (i, p) in project_ids.iter().enumerate() {
        conn.execute(
            "INSERT INTO workspace_projects (workspace_id, project_id, \"order\", protected) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![ws_id, p, i as u32],
        )
        .unwrap();
    }
}

/// A migrated database with project `p-1a-1`, a real task folder under a tempdir (intent `x`), and task `t-1a-1` in
/// `stage` whose `memory_dir` is that folder.
pub fn setup(stage: Stage) -> (rusqlite::Connection, tempfile::TempDir, crate::memory::TaskFolder, Task) {
    let conn = test_conn();
    insert_project(&conn, "p-1a-1");
    let dir = tempfile::tempdir().unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
    let folder = crate::memory::TaskFolder::create(dir.path(), date, "Task", "x").unwrap();
    let mut t = task_fixture("t-1a-1", "p-1a-1");
    t.stage = stage;
    t.memory_dir = folder.dir().to_owned();
    crate::db::tasks::insert(&conn, &t).unwrap();
    (conn, dir, folder, t)
}
