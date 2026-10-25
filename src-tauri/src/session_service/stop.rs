//! Ending sessions: the graceful stop (§7.1) one at a time, and every session at quit.

use std::time::{Duration, Instant};

use fanoutcode_core::domain::SessionId;
use fanoutcode_core::sessions::{stop, SystemClock};
use fanoutcode_core::{Error, Result};
use tauri::{AppHandle, Manager};

use crate::observe::log_err;
use crate::state::AppState;

/// Stops the session on a spawned thread and returns at once.
pub fn stop(app: &AppHandle, id: &SessionId) -> Result<()> {
    let state = app.state::<AppState>();
    if state.registry().get(id).is_none() {
        return Err(Error::Invalid(format!("Unknown session {id}")));
    }
    let (app, id) = (app.clone(), id.clone());
    std::thread::spawn(move || log_err("stopping a session", stop_blocking(&app, &id)));
    Ok(())
}

/// The graceful stop on this thread: the exit keys, up to `STOP_GRACE`, then kill. The PTY is taken out of the
/// registry while it runs (so the PTY threads never wait on it); the session's exit is still reported once, by the
/// PTY's own waiter. An exited session is already stopped.
pub fn stop_blocking(app: &AppHandle, id: &SessionId) -> Result<()> {
    let state = app.state::<AppState>();
    let (kind, pty) = {
        let mut registry = state.registry();
        let Some(session) = registry.get(id).cloned() else {
            return Err(Error::Invalid(format!("Unknown session {id}")));
        };
        if session.exited.is_some() {
            return Ok(());
        }
        (session.kind, registry.take_pty(id))
    };
    let Some(mut pty) = pty else {
        return Err(Error::Precondition("The session is already stopping.".into()));
    };
    stop::run(&state.driver.graceful_exit(kind), pty.as_mut(), &SystemClock);
    Ok(())
}

/// Stops every live session in parallel and waits at most `grace` (plus the kill second) for them.
pub fn stop_all_blocking(app: &AppHandle, grace: Duration) {
    let state = app.state::<AppState>();
    let live: Vec<SessionId> = state
        .registry()
        .list()
        .into_iter()
        .filter(|s| s.exited.is_none())
        .map(|s| s.id)
        .collect();
    let handles: Vec<_> = live
        .into_iter()
        .map(|id| {
            let app = app.clone();
            std::thread::spawn(move || log_err("stopping a session at quit", stop_blocking(&app, &id)))
        })
        .collect();
    let deadline = Instant::now() + grace + Duration::from_secs(1);
    for h in handles {
        while !h.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
