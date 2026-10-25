//! Quitting with sessions alive (§7.1): an Owned PTY cannot outlive the app, so ⌘Q while an agent works asks first,
//! then every session gets the graceful stop before the app exits.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use fanoutcode_core::db;
use fanoutcode_core::sessions::STOP_GRACE;
use tauri::{AppHandle, ExitRequestApi, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

use super::stop_all_blocking;
use crate::state::AppState;

/// Set once the quit is confirmed and the sessions are being stopped, so the exit that follows goes through.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// `"<task title> (<stage>)"` for each working task session, `"Terminal in <project>"` for any other.
pub fn working_titles(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let working = state.registry().working(Instant::now());
    let conn = state.db.lock().expect("db lock");
    working
        .iter()
        .map(
            |s| match s.task_id.as_ref().and_then(|t| db::tasks::get(&conn, t).ok().flatten()) {
                Some(task) => format!("{} ({:?})", task.title, task.stage),
                None => {
                    let project = db::projects::list(&conn)
                        .unwrap_or_default()
                        .into_iter()
                        .find(|p| p.root == s.cwd)
                        .map(|p| p.name)
                        .unwrap_or_else(|| s.cwd.display().to_string());
                    format!("Terminal in {project}")
                }
            },
        )
        .collect()
}

/// `RunEvent::ExitRequested`. A programmatic exit (`code` set) passes; otherwise the exit is held, the developer
/// confirms when agents are working, and every session is stopped on a worker thread before `app.exit(0)`.
pub fn on_exit_requested(app: &AppHandle, code: Option<i32>, api: &ExitRequestApi) {
    let state = app.state::<AppState>();
    if code.is_some() || QUITTING.load(Ordering::SeqCst) || !state.driver.ends_on_quit() {
        return;
    }
    let live = state.registry().list().iter().any(|s| s.exited.is_none());
    if !live {
        return;
    }
    api.prevent_exit();
    let titles = working_titles(app);
    if titles.is_empty() {
        quit(app);
        return;
    }
    let app2 = app.clone();
    app.dialog()
        .message(format!(
            "{} agent(s) are still working:\n{}",
            titles.len(),
            titles.join("\n")
        ))
        .title("Quit Fanout Code?")
        .buttons(MessageDialogButtons::OkCancelCustom("Quit".into(), "Cancel".into()))
        .show(move |confirmed| {
            if confirmed {
                quit(&app2);
            }
        });
}

fn quit(app: &AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        stop_all_blocking(&app, STOP_GRACE);
        app.exit(0);
    });
}
