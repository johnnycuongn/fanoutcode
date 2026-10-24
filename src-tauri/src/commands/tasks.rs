//! Task commands. Each runs in one database lock: re-parse the id, read, compute in the core, persist with
//! `pipeline::apply`; then, with the lock released, dispatch the external effects and notify every window.

use chrono::{Local, Utc};
use serde::Serialize;
use tauri::{AppHandle, State};
use teamcode_core::domain::{Container, Project, ProjectId, Stage, Task, TaskId, WorkspaceId};
use teamcode_core::memory::TaskFolder;
use teamcode_core::pipeline::{self, ctx, Effect, NewTask, StageAction, TaskPatch};
use teamcode_core::{db, Error, Result};

use super::parse_container;
use crate::events::{self, TasksReloaded};
use crate::{effects, state::AppState};

/// The task after a stage action, and the toasts for the webview to show.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ActionOutcome {
    pub task: Task,
    pub toasts: Vec<String>,
}

fn project_of(conn: &rusqlite::Connection, task: &Task) -> Result<Project> {
    db::projects::get(conn, &task.project_id)?
        .ok_or_else(|| Error::Invalid(format!("Unknown project {}", task.project_id)))
}

/// Dispatches, notifies, and returns the outcome. Called with no lock held.
fn finish(app: &AppHandle, state: &AppState, task: Task, external: Vec<Effect>) -> Result<ActionOutcome> {
    let toasts = effects::dispatch(app, state, &task, external);
    // a runner may have changed the task (its worktree), so the row is the truth
    let conn = state.db.lock().expect("db lock");
    let task = db::tasks::get(&conn, &task.id)?.unwrap_or(task);
    events::emit_task_changed(app, &conn, &task)?;
    Ok(ActionOutcome { task, toasts })
}

/// The `tasks_action` sequence Plans 04, 05, 07 and 08 rely on: lock, require the task and its project, build the
/// transition context, compute the effects, apply them, release the lock, dispatch, emit.
fn run(
    app: &AppHandle,
    state: &AppState,
    id: &TaskId,
    compute: impl FnOnce(&Task, &pipeline::TransitionCtx) -> Result<Vec<Effect>>,
) -> Result<ActionOutcome> {
    let id = TaskId::parse(id.as_str())?;
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, &id)?;
        let project = project_of(&conn, &task)?;
        let now = Utc::now();
        let tctx = ctx::build_transition_ctx(
            &conn,
            &task,
            &project,
            &state.config_dir,
            &state.data_dir,
            now,
            state.last_test(&id),
        )?;
        let effects = compute(&task, &tctx)?;
        pipeline::apply(
            &mut conn,
            &TaskFolder::open(task.memory_dir.clone()),
            &task,
            effects,
            now,
        )?
    };
    finish(app, state, applied.task, applied.external)
}

/// A stage action the app itself takes (Done detection, a test result): `tasks_action` without the webview. The
/// toasts have no window to go to, so they are logged.
pub(crate) fn run_action(app: &AppHandle, state: &AppState, id: &TaskId, action: StageAction) -> Result<Task> {
    let outcome = run(app, state, id, |task, tctx| pipeline::transition(task, action, tctx))?;
    for t in &outcome.toasts {
        eprintln!("Team Code: {} ({})", t, outcome.task.id);
    }
    Ok(outcome.task)
}

/// Persists an edited task (no stage change) and rewrites its `index.md`.
fn save(app: &AppHandle, state: &AppState, task: Task) -> Result<Task> {
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        pipeline::apply(
            &mut conn,
            &TaskFolder::open(task.memory_dir.clone()),
            &task,
            Vec::new(),
            task.updated_at,
        )?
    };
    Ok(finish(app, state, applied.task, applied.external)?.task)
}

#[tauri::command]
#[specta::specta]
pub fn tasks_create(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    intent: String,
    jira_key: Option<String>,
    workspace_id: Option<WorkspaceId>,
) -> Result<Task> {
    let project_id = ProjectId::parse(project_id.as_str())?;
    let workspace_id = workspace_id.map(|w| WorkspaceId::parse(w.as_str())).transpose()?;
    let created = {
        let mut conn = state.db.lock().expect("db lock");
        let new = NewTask {
            project_id: project_id.clone(),
            workspace_id: workspace_id.clone(),
            intent,
            jira_key,
            title: None,
            now: Utc::now(),
            date: Local::now().date_naive(),
        };
        pipeline::create_task(&mut conn, &state.data_dir, new)?
    };
    let outcome = finish(&app, &state, created.task, created.external)?;
    let mut containers = vec![Container::Project { id: project_id }];
    containers.extend(workspace_id.map(|id| Container::Workspace { id }));
    for container in containers {
        events::emit(&app, TasksReloaded { container })?;
    }
    Ok(outcome.task)
}

/// The container's tasks, archived included.
#[tauri::command]
#[specta::specta]
pub fn tasks_list(state: State<'_, AppState>, container: Container) -> Result<Vec<Task>> {
    db::tasks::list(&state.db.lock().expect("db lock"), &parse_container(&container)?)
}

#[tauri::command]
#[specta::specta]
pub fn tasks_get(state: State<'_, AppState>, id: TaskId) -> Result<Task> {
    db::tasks::require(&state.db.lock().expect("db lock"), &TaskId::parse(id.as_str())?)
}

#[tauri::command]
#[specta::specta]
pub fn tasks_update(app: AppHandle, state: State<'_, AppState>, id: TaskId, patch: TaskPatch) -> Result<Task> {
    let id = TaskId::parse(id.as_str())?;
    let task = db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    save(&app, &state, pipeline::apply_patch(&task, &patch, Utc::now())?)
}

#[tauri::command]
#[specta::specta]
pub fn tasks_rename(app: AppHandle, state: State<'_, AppState>, id: TaskId, title: String) -> Result<Task> {
    let id = TaskId::parse(id.as_str())?;
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Invalid("Title cannot be empty".into()));
    }
    let mut task = db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    task.title = title.to_owned();
    task.updated_at = Utc::now();
    save(&app, &state, task)
}

/// A board drag of any distance (`pipeline::move_to`).
#[tauri::command]
#[specta::specta]
pub fn tasks_move(app: AppHandle, state: State<'_, AppState>, id: TaskId, stage: Stage) -> Result<ActionOutcome> {
    run(&app, &state, &id, |task, tctx| pipeline::move_to(task, stage, tctx))
}

#[tauri::command]
#[specta::specta]
pub fn tasks_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: TaskId,
    action: StageAction,
) -> Result<ActionOutcome> {
    run(&app, &state, &id, |task, tctx| pipeline::transition(task, action, tctx))
}

#[tauri::command]
#[specta::specta]
pub fn tasks_archive(app: AppHandle, state: State<'_, AppState>, id: TaskId) -> Result<ActionOutcome> {
    run(&app, &state, &id, |task, tctx| {
        pipeline::transition(task, StageAction::Archive, tctx)
    })
}

#[tauri::command]
#[specta::specta]
pub fn tasks_restore(app: AppHandle, state: State<'_, AppState>, id: TaskId) -> Result<ActionOutcome> {
    run(&app, &state, &id, |task, tctx| {
        pipeline::transition(task, StageAction::Restore, tctx)
    })
}
