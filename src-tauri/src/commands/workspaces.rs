//! Workspace commands (Plan 10): the record and its file, opening a file from elsewhere through a preview,
//! membership and policy, fragments and agents, conversions from project windows, and link status. The logic lives
//! in `teamcode_core::workspace`; the stores lock the database themselves, so no command here holds `state.db`
//! while calling one.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State, Window};
use teamcode_core::domain::{Container, LinkPolicy, ProjectId, TaskId, WindowId, Workspace, WorkspaceId};
use teamcode_core::workspace::convert::{close_panes_for_project, merge_containers};
use teamcode_core::workspace::import::{self, OpenEnv, WorkspaceImportPreview};
use teamcode_core::workspace::link::LinkStatus;
use teamcode_core::workspace::open::{self, WorkspaceStatus};
use teamcode_core::{db, Error, Result};

use crate::events::{self, ViewsChanged, WindowContainerChanged, WorkspaceChanged};
use crate::observe::log_err;
use crate::state::AppState;

fn ws_id(id: &WorkspaceId) -> Result<WorkspaceId> {
    WorkspaceId::parse(id.as_str())
}

/// `workspace-changed`, then the workspace as returned.
fn changed(app: &AppHandle, ws: Workspace) -> Result<Workspace> {
    log_err(
        "announcing the workspace",
        events::emit(app, WorkspaceChanged { workspace: ws.clone() }),
    );
    Ok(ws)
}

/// An existing file or folder, canonicalised.
fn existing(path: &Path, missing: &str) -> Result<PathBuf> {
    std::fs::canonicalize(path).map_err(|_| Error::Precondition(missing.into()))
}

/// A path that may not exist yet: its folder canonicalised, its name kept.
fn target_path(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::Invalid("Choose a file name.".into()))?;
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(existing(dir, "Choose an existing folder.")?.join(name))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

fn with_env<T>(state: &AppState, f: impl FnOnce(&OpenEnv) -> Result<T>) -> Result<T> {
    let home = home_dir();
    f(&OpenEnv {
        home_dir: &home,
        app_config: &state.config_dir,
    })
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_list(state: State<'_, AppState>) -> Result<Vec<Workspace>> {
    state.workspaces.list()
}

/// Reopens a workspace file this app wrote for the record already at that path (and that adds nothing since).
/// Any other file needs `workspaces_preview_open` and `workspaces_confirm_open`.
#[tauri::command]
#[specta::specta]
pub fn workspaces_open(app: AppHandle, state: State<'_, AppState>, path: PathBuf) -> Result<Workspace> {
    let file = existing(&path, "That workspace file no longer exists. Relocate or remove it.")?;
    changed(&app, import::open_file(&state.workspaces, &file)?)
}

/// What opening the file would bring in; writes nothing.
#[tauri::command]
#[specta::specta]
pub fn workspaces_preview_open(state: State<'_, AppState>, path: PathBuf) -> Result<WorkspaceImportPreview> {
    let file = existing(&path, "That workspace file no longer exists.")?;
    with_env(&state, |env| import::preview_open(&state.workspaces, &file, env))
}

/// Imports exactly what the preview with this token showed. `allow_outside_docs` also trusts the file and folder
/// references the preview marked as outside the workspace's projects.
#[tauri::command]
#[specta::specta]
pub fn workspaces_confirm_open(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    allow_outside_docs: bool,
) -> Result<Workspace> {
    let ws = with_env(&state, |env| {
        import::confirm_open(&state.workspaces, &token, allow_outside_docs, env)
    })?;
    changed(&app, ws)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    project_ids: Vec<ProjectId>,
) -> Result<Workspace> {
    let ids = project_ids
        .iter()
        .map(|p| ProjectId::parse(p.as_str()))
        .collect::<Result<Vec<_>>>()?;
    changed(&app, state.workspaces.create(&name, &ids)?)
}

/// Adds the project at `path` (opened as `projects_open` does).
#[tauri::command]
#[specta::specta]
pub fn workspaces_add_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    path: PathBuf,
) -> Result<Workspace> {
    let id = ws_id(&id)?;
    let root = existing(&path, "Choose a folder")?;
    let project = db::find_or_create_project(&state.db.lock().expect("db lock"), &root)?;
    changed(&app, state.workspaces.add_project(&id, &project.id)?)
}

/// Removes the project; panes showing its tasks or terminals close in this workspace's views, and their sessions
/// keep running, detached.
#[tauri::command]
#[specta::specta]
pub fn workspaces_remove_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    project_id: ProjectId,
) -> Result<Workspace> {
    let id = ws_id(&id)?;
    let project_id = ProjectId::parse(project_id.as_str())?;
    let ws = state.workspaces.remove_project(&id, &project_id)?;
    let container = Container::Workspace { id: id.clone() };
    let closed = {
        let conn = state.db.lock().expect("db lock");
        let tasks: HashSet<TaskId> = db::tasks::list(&conn, &Container::Project { id: project_id.clone() })?
            .into_iter()
            .map(|t| t.id)
            .collect();
        let mut closed = Vec::new();
        for view in db::views::list(&conn, &container)? {
            let (layout, panes) = close_panes_for_project(&view.layout, &project_id, &tasks);
            if !panes.is_empty() {
                db::views::update_layout(&conn, &view.id, &layout)?;
                closed.extend(panes);
            }
        }
        closed
    };
    if !closed.is_empty() {
        let mut registry = state.registry();
        let attached: Vec<_> = registry
            .list()
            .into_iter()
            .filter(|s| s.attached_to.as_ref().is_some_and(|(_, pane)| closed.contains(pane)))
            .map(|s| s.id)
            .collect();
        for s in attached {
            let _ = registry.detach(&s);
        }
        drop(registry);
        log_err("announcing the views", events::emit(&app, ViewsChanged { container }));
    }
    changed(&app, ws)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_save(app: AppHandle, state: State<'_, AppState>, id: WorkspaceId) -> Result<Workspace> {
    changed(&app, state.workspaces.save(&ws_id(&id)?)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_save_as(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    path: PathBuf,
) -> Result<Workspace> {
    changed(&app, state.workspaces.save_as(&ws_id(&id)?, &target_path(&path)?)?)
}

/// The policy and the protected targets (`overrides`); applies to sessions started from now on.
#[tauri::command]
#[specta::specta]
pub fn workspaces_set_policy(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    policy: LinkPolicy,
    overrides: Vec<ProjectId>,
) -> Result<Workspace> {
    let protected = overrides
        .iter()
        .map(|p| ProjectId::parse(p.as_str()))
        .collect::<Result<Vec<_>>>()?;
    changed(&app, state.workspaces.set_policy(&ws_id(&id)?, policy, &protected)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_status(state: State<'_, AppState>, id: WorkspaceId) -> Result<WorkspaceStatus> {
    open::status(&state.workspaces, &ws_id(&id)?)
}

/// Re-reads the file; refuses unsaved changes unless `discard_unsaved`, and a file that now adds folders, agents
/// or instructions (those go through the preview).
#[tauri::command]
#[specta::specta]
pub fn workspaces_reload(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    discard_unsaved: bool,
) -> Result<Workspace> {
    changed(&app, open::reload(&state.workspaces, &ws_id(&id)?, discard_unsaved)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_relocate_file(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    path: PathBuf,
) -> Result<Workspace> {
    let file = existing(&path, "That file does not exist.")?;
    changed(&app, open::relocate_file(&state.workspaces, &ws_id(&id)?, &file)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_relocate_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    project_id: ProjectId,
    path: PathBuf,
) -> Result<Workspace> {
    let root = existing(&path, "Choose an existing folder.")?;
    let project_id = ProjectId::parse(project_id.as_str())?;
    changed(
        &app,
        open::relocate_project(&state.workspaces, &ws_id(&id)?, &project_id, &root)?,
    )
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_reorder_projects(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    project_ids: Vec<ProjectId>,
) -> Result<Workspace> {
    let ids = project_ids
        .iter()
        .map(|p| ProjectId::parse(p.as_str()))
        .collect::<Result<Vec<_>>>()?;
    changed(&app, state.workspaces.reorder_projects(&ws_id(&id)?, &ids)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_fragments(state: State<'_, AppState>, id: WorkspaceId) -> Result<BTreeMap<String, String>> {
    state.workspaces.fragments(&ws_id(&id)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_set_fragment(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    stage: String,
    text: Option<String>,
) -> Result<Workspace> {
    changed(
        &app,
        state.workspaces.set_fragment(&ws_id(&id)?, &stage, text.as_deref())?,
    )
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_agents(state: State<'_, AppState>, id: WorkspaceId) -> Result<BTreeMap<String, String>> {
    state.workspaces.agents(&ws_id(&id)?)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_set_agent(
    app: AppHandle,
    state: State<'_, AppState>,
    id: WorkspaceId,
    name: String,
    markdown: Option<String>,
) -> Result<Workspace> {
    changed(
        &app,
        state.workspaces.set_agent(&ws_id(&id)?, &name, markdown.as_deref())?,
    )
}

/// The window's label as a registered window id.
fn window_id(state: &AppState, window: &Window) -> Result<WindowId> {
    let id = WindowId::parse(window.label())?;
    state
        .windows
        .lock()
        .expect("windows lock")
        .get(&id)
        .map(|_| id)
        .ok_or_else(|| Error::Invalid("Unknown window.".into()))
}

/// Turns the calling project window into a workspace of that project and the one at `path`, keeping its views.
#[tauri::command]
#[specta::specta]
pub fn workspaces_convert(
    app: AppHandle,
    state: State<'_, AppState>,
    window: Window,
    project_id: ProjectId,
    path: PathBuf,
) -> Result<Workspace> {
    let label = window_id(&state, &window)?;
    let project_id = ProjectId::parse(project_id.as_str())?;
    let root = existing(&path, "Choose a folder")?;
    let (home, other) = {
        let conn = state.db.lock().expect("db lock");
        let home =
            db::projects::get(&conn, &project_id)?.ok_or_else(|| Error::Invalid("Unknown project id.".into()))?;
        (home, db::find_or_create_project(&conn, &root)?)
    };
    let ws = state.workspaces.create(
        &format!("{} workspace", home.name),
        &[home.id.clone(), other.id.clone()],
    )?;
    merge_containers(&state.db, &[Container::Project { id: home.id.clone() }], &ws.id)?;
    let container = Container::Workspace { id: ws.id.clone() };
    {
        let mut windows = state.windows.lock().expect("windows lock");
        windows.set_container(&label, container.clone())?;
        windows.set_details(&label, ws.name.clone(), vec![home.id, other.id])?;
    }
    log_err("saving the windows", state.save_windows());
    log_err(
        "announcing the window",
        events::emit(
            &app,
            WindowContainerChanged {
                window_id: label,
                container: container.clone(),
            },
        ),
    );
    log_err("announcing the views", events::emit(&app, ViewsChanged { container }));
    changed(&app, ws)
}

/// A new workspace of every project shown in the open windows, their views merged into it; those windows close and
/// the workspace opens in a window of its own.
#[tauri::command]
#[specta::specta]
pub async fn workspaces_create_from_windows(
    app: AppHandle,
    state: State<'_, AppState>,
    window: Window,
    name: String,
) -> Result<Workspace> {
    let open: Vec<_> = state
        .windows
        .lock()
        .expect("windows lock")
        .list()
        .into_iter()
        .filter(|w| w.container.is_some())
        .collect();
    let mut projects: Vec<ProjectId> = Vec::new();
    for p in open.iter().flat_map(|w| w.project_ids.iter()) {
        if !projects.contains(p) {
            projects.push(p.clone());
        }
    }
    let ws = state.workspaces.create(&name, &projects)?;
    let sources: Vec<Container> = open.iter().filter_map(|w| w.container.clone()).collect();
    merge_containers(&state.db, &sources, &ws.id)?;
    let path = ws
        .file_path
        .clone()
        .ok_or_else(|| Error::Io("The workspace has no file.".into()))?;
    crate::commands::windows::open_workspace_window(&window, &state, &path)?;
    for w in &open {
        if let Some(win) = app.get_webview_window(w.window_id.as_str()) {
            log_err(
                "closing a merged window",
                win.close().map_err(|e| Error::Provider(e.to_string())),
            );
        }
    }
    changed(&app, ws)
}

#[tauri::command]
#[specta::specta]
pub fn workspaces_link_status(state: State<'_, AppState>, task_id: TaskId) -> Result<LinkStatus> {
    let conn = state.db.lock().expect("db lock");
    let task = db::tasks::require(&conn, &TaskId::parse(task_id.as_str())?)?;
    teamcode_core::workspace::launch::status_of(&conn, &task)
}
