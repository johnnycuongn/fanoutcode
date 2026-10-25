//! Sessions in the shell (Plan 04): launching a task's primary session or a plain terminal on a PTY, feeding the
//! registry from the PTY threads, and reporting exits. Stage effects and hook events are in `effects`, stopping in
//! `stop`, the quit warning in `quit`; the logic itself lives in `teamcode_core::sessions`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use tauri::{AppHandle, Manager};
use teamcode_core::domain::{AgentKind, Project, ProjectId, Session, SessionId, Task, TaskId, WorktreeChoice};
use teamcode_core::hooks::{event_log_path, hook_specs, status_line_command, user_status_line};
use teamcode_core::roster::{self, RosterLevel, RosterScope};
use teamcode_core::sessions::{
    claude_launch, codex_launch, settings_path, shell_launch, ClaudeLaunch, GeneratedSettings, LaunchSpec,
};
use teamcode_core::{db, memory, worktree, Error, Result};

use crate::events::{self, SessionExited, SessionQueueChanged};
use crate::observe::log_err;
use crate::state::{scrollback_lines, AppState};

mod effects;
mod quit;
mod stop;

pub use effects::*;
pub use quit::*;
pub use stop::*;

/// Every session starts at this size; the pane's first fit resizes it.
pub const START_SIZE: (u16, u16) = (80, 24);

/// The task's primary session: the live one when there is one (unless `fresh`), otherwise a new launch of the
/// task's agent kind. A Claude Code launch writes the task's generated settings file first and resumes the last
/// Claude session unless `fresh`.
pub fn open_primary(app: &AppHandle, task_id: &TaskId, fresh: bool) -> Result<SessionId> {
    let state = app.state::<AppState>();
    let _one_at_a_time = state.session_open.lock().expect("session open lock");
    if !fresh {
        if let Some(id) = state.registry().live_for_task(task_id) {
            return Ok(id);
        }
    }
    let (launch, kind) = {
        let conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, task_id)?;
        let project = project_of(&conn, &task.project_id)?;
        (task_launch(&conn, &state, &task, &project, fresh)?, task.agent_kind)
    };
    let session = new_session(&state, Some(task_id.clone()), kind, &launch);
    let id = start(app, session, launch)?;
    let task = {
        let conn = state.db.lock().expect("db lock");
        let mut task = db::tasks::require(&conn, task_id)?;
        task.primary_session = Some(id.clone());
        task.updated_at = Utc::now();
        db::tasks::update(&conn, &task)?;
        log_err("announcing the session", events::emit_task_changed(app, &conn, &task));
        task
    };
    let (app2, tid) = (app.clone(), task.id.clone());
    tauri::async_runtime::spawn(async move {
        log_err(
            "watching the task's repos",
            crate::git_watcher::watch(&app2, &tid).await,
        );
    });
    Ok(id)
}

/// A plain terminal: a login shell in the project root, not tied to a task.
pub fn open_terminal(app: &AppHandle, project_id: &ProjectId) -> Result<SessionId> {
    let state = app.state::<AppState>();
    let root = project_of(&state.db.lock().expect("db lock"), project_id)?.root;
    let launch = shell_launch(&root, &login_shell());
    let session = new_session(&state, None, AgentKind::Shell, &launch);
    start(app, session, launch)
}

/// Spawns `launch` through the driver, registers the session and persists its row. The registry lock is held
/// across the spawn, so the PTY threads cannot report output or an exit before the session is registered.
pub fn start(app: &AppHandle, session: Session, launch: LaunchSpec) -> Result<SessionId> {
    let state = app.state::<AppState>();
    let id = session.id.clone();
    db::sessions::insert(&state.db.lock().expect("db lock"), &session)?;
    let lines = scrollback_lines(&state.db.lock().expect("db lock"))?;
    let (out_reg, out_id) = (state.sessions.clone(), id.clone());
    let on_output = Box::new(move |bytes: &[u8]| {
        out_reg
            .lock()
            .expect("sessions lock")
            .on_output(&out_id, bytes, Instant::now());
    });
    let (exit_app, exit_id) = (app.clone(), id.clone());
    let on_exit = Box::new(move |code: i32| exited(&exit_app, &exit_id, code));
    // the spawner resolves the program on the login shell's PATH; read it (once, cached) before taking the lock
    let _ = livespec_core::shell_env::login_shell_path();
    let mut registry = state.registry();
    registry.set_scrollback_lines(lines);
    match state
        .driver
        .start(&launch, START_SIZE, &*state.spawner, on_output, on_exit)
    {
        Ok(pty) => registry.insert(session, pty, START_SIZE),
        Err(e) => {
            drop(registry);
            log_err(
                "removing a failed session",
                db::sessions::delete(&state.db.lock().expect("db lock"), &id),
            );
            return Err(e);
        }
    }
    Ok(id)
}

/// The PTY's exit: the registry drops the queue and the pane's sink, the row records the code, every window hears.
fn exited(app: &AppHandle, id: &SessionId, code: i32) {
    let state = app.state::<AppState>();
    state.registry().on_exit(id, code);
    state.codex_idle.lock().expect("codex idle lock").remove(id);
    log_err(
        "recording a session exit",
        db::sessions::mark_exited(&state.db.lock().expect("db lock"), id, code),
    );
    let exit = SessionExited {
        session_id: id.clone(),
        code: Some(code),
    };
    log_err("announcing a session exit", events::emit(app, exit));
    queue_changed(app, id, 0);
}

/// `session-queue-changed`.
pub fn queue_changed(app: &AppHandle, id: &SessionId, pending: usize) {
    let event = SessionQueueChanged {
        session_id: id.clone(),
        pending: pending as u32,
    };
    log_err("announcing the prompt queue", events::emit(app, event));
}

fn new_session(state: &AppState, task_id: Option<TaskId>, kind: AgentKind, launch: &LaunchSpec) -> Session {
    Session {
        id: SessionId::new(),
        task_id,
        kind,
        cwd: launch.cwd.clone(),
        driver: state.driver.kind(),
        attached_to: None,
        started_at: Utc::now(),
        exited: None,
    }
}

fn project_of(conn: &rusqlite::Connection, id: &ProjectId) -> Result<Project> {
    db::projects::get(conn, id)?.ok_or_else(|| Error::Invalid(format!("Unknown project {id}")))
}

/// The launch for the task's agent kind.
fn task_launch(
    conn: &rusqlite::Connection,
    state: &AppState,
    task: &Task,
    project: &Project,
    fresh: bool,
) -> Result<LaunchSpec> {
    let (convention, _) = worktree::settings_for(conn, task, &project.root)?;
    let worktree_cwd = || -> Result<PathBuf> {
        match task.worktree {
            WorktreeChoice::Off => Ok(project.root.clone()),
            WorktreeChoice::On => task
                .worktree_path
                .clone()
                .ok_or_else(|| Error::Precondition("worktree not created".into())),
        }
    };
    match task.agent_kind {
        AgentKind::ClaudeCode => {
            let settings = settings_path(&state.data_dir, &task.id)?;
            generated_settings(&state.data_dir, task).write(&settings)?;
            let agents = roster::resolve(&roster_scope(state, project));
            claude_launch(&ClaudeLaunch {
                task,
                project_root: &project.root,
                settings_path: &settings,
                agents: &agents,
                convention,
                fresh,
            })
        }
        AgentKind::Codex => Ok(codex_launch(&worktree_cwd()?)),
        AgentKind::Shell => Ok(shell_launch(&worktree_cwd()?, &login_shell())),
    }
}

/// Hooks and the status line from Plan 05, the protected-root deny rules from Plan 10.
fn generated_settings(app_data: &Path, task: &Task) -> GeneratedSettings {
    let user = home_claude_settings().and_then(|text| user_status_line(&text));
    GeneratedSettings {
        hooks: hook_specs(&task.id, app_data),
        status_line: Some(status_line_command(
            &event_log_path(app_data, &task.id),
            user.as_deref(),
        )),
        permission_deny: protected_deny_rules(task),
        bg_isolation_none: false,
    }
}

/// `workspace::protected_deny_rules(&Task)` is Plan 10's.
fn protected_deny_rules(_task: &Task) -> Vec<String> {
    Vec::new() // STUB(plan 10)
}

/// The app's and the project's agents; the workspace level is Plan 10's.
fn roster_scope(state: &AppState, project: &Project) -> RosterScope {
    let load =
        |dir: PathBuf, level| log_err("reading the agent roster", roster::load_dir(&dir, level)).unwrap_or_default();
    RosterScope {
        app: load(roster::agents_dir_app(&state.config_dir), RosterLevel::App),
        workspace: Vec::new(), // STUB(plan 10)
        project: load(
            roster::agents_dir_project(&memory::root_for(project, &state.data_dir)),
            RosterLevel::Project,
        ),
    }
}

/// The developer's `~/.claude/settings.json`, for their own status-line command.
fn home_claude_settings() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    std::fs::read_to_string(Path::new(&home).join(".claude").join("settings.json")).ok()
}

/// `$SHELL`, else zsh (the macOS default).
pub fn login_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/zsh".into())
}
