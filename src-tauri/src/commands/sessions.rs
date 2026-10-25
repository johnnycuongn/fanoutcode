//! The `sessions_*` commands (Plan 04). Each re-parses its ids and delegates to `session_service` or the registry.
//! Launches run off the main thread, since the first one may wait for the login shell's PATH.

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use teamcode_core::domain::{PaneId, ProjectId, Session, SessionId, TaskId, WindowId};
use teamcode_core::sessions::Scrollback;
use teamcode_core::{db, Error, Result};

use crate::events::{self, SessionAttached};
use crate::pty::{ChannelSink, TerminalBytes};
use crate::session_service;
use crate::state::AppState;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::Io(format!("The session launch stopped: {e}")))?
}

/// The task's primary session, started (or resumed) when none is live.
#[tauri::command]
#[specta::specta]
pub async fn sessions_open(app: AppHandle, task_id: TaskId) -> Result<SessionId> {
    let id = TaskId::parse(task_id.as_str())?;
    blocking(move || session_service::open_primary(&app, &id, false)).await
}

/// A plain shell in the project root.
#[tauri::command]
#[specta::specta]
pub async fn sessions_terminal(app: AppHandle, project_id: ProjectId) -> Result<SessionId> {
    let id = ProjectId::parse(project_id.as_str())?;
    blocking(move || session_service::open_terminal(&app, &id)).await
}

fn ids(session: &SessionId, window: &WindowId, pane: &PaneId) -> Result<(SessionId, WindowId, PaneId)> {
    Ok((
        SessionId::parse(session.as_str())?,
        WindowId::parse(window.as_str())?,
        PaneId::parse(pane.as_str())?,
    ))
}

/// Attaches this pane: the scrollback is replayed on `on_data`, then live output follows. Another pane holding the
/// session is `attached-elsewhere:<window>:<pane>`.
#[tauri::command]
#[specta::specta]
pub fn sessions_attach(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: SessionId,
    window_id: WindowId,
    pane_id: PaneId,
    on_data: Channel<TerminalBytes>,
) -> Result<Scrollback> {
    let (s, w, p) = ids(&session_id, &window_id, &pane_id)?;
    let snap = state
        .registry()
        .attach(&s, w.clone(), p.clone(), Box::new(ChannelSink(on_data)))?;
    events::emit(
        &app,
        SessionAttached {
            session_id: s,
            window_id: w,
            pane_id: p,
        },
    )?;
    Ok(snap)
}

/// Moves the session to this pane from wherever it is attached.
#[tauri::command]
#[specta::specta]
pub fn sessions_bring(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: SessionId,
    window_id: WindowId,
    pane_id: PaneId,
    on_data: Channel<TerminalBytes>,
) -> Result<Scrollback> {
    let (s, w, p) = ids(&session_id, &window_id, &pane_id)?;
    let snap = state
        .registry()
        .bring(&s, w.clone(), p.clone(), Box::new(ChannelSink(on_data)))?;
    events::emit(
        &app,
        SessionAttached {
            session_id: s,
            window_id: w,
            pane_id: p,
        },
    )?;
    Ok(snap)
}

#[tauri::command]
#[specta::specta]
pub fn sessions_detach(state: State<'_, AppState>, session_id: SessionId) -> Result<()> {
    state.registry().detach(&SessionId::parse(session_id.as_str())?)
}

#[tauri::command]
#[specta::specta]
pub fn sessions_write(state: State<'_, AppState>, session_id: SessionId, bytes: Vec<u8>) -> Result<()> {
    state.registry().write(&SessionId::parse(session_id.as_str())?, &bytes)
}

#[tauri::command]
#[specta::specta]
pub fn sessions_resize(state: State<'_, AppState>, session_id: SessionId, cols: u16, rows: u16) -> Result<()> {
    state
        .registry()
        .resize(&SessionId::parse(session_id.as_str())?, cols, rows)
}

/// The pane has drawn `bytes` more bytes (flow control).
#[tauri::command]
#[specta::specta]
pub fn sessions_ack(state: State<'_, AppState>, session_id: SessionId, bytes: u32) -> Result<()> {
    state
        .registry()
        .ack(&SessionId::parse(session_id.as_str())?, bytes as usize)
}

/// A live session, else its stored row.
#[tauri::command]
#[specta::specta]
pub fn sessions_get(state: State<'_, AppState>, session_id: SessionId) -> Result<Session> {
    let id = SessionId::parse(session_id.as_str())?;
    if let Some(s) = state.registry().get(&id).cloned() {
        return Ok(s);
    }
    db::sessions::get(&state.db.lock().expect("db lock"), &id)?
        .ok_or_else(|| Error::Invalid(format!("Unknown session {id}")))
}

/// Every session this launch has started, exited ones included.
#[tauri::command]
#[specta::specta]
pub fn sessions_list(state: State<'_, AppState>) -> Vec<Session> {
    state.registry().list()
}
