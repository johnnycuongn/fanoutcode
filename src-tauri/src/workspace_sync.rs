//! Workspace file watching at runtime (Plan 10): every explicitly saved workspace is watched from launch, and the
//! watcher's reports become `workspace-file-changed` for the window to offer a reload.

use fanoutcode_core::workspace::file::is_in_app_data;
use tauri::{AppHandle, Manager};

use crate::events::{self, WorkspaceFileChanged};
use crate::observe::log_err;
use crate::state::AppState;

pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(rx) = state
        .workspace_file_changes
        .lock()
        .expect("watcher channel lock")
        .take()
    else {
        return;
    };
    match state.workspaces.list() {
        Ok(all) => {
            for ws in all {
                if let Some(path) = ws
                    .file_path
                    .filter(|p| !is_in_app_data(p, &state.data_dir) && p.is_file())
                {
                    log_err("watching a workspace file", state.workspace_watcher.watch(ws.id, &path));
                }
            }
        }
        Err(e) => eprintln!("Fanout Code: couldn't list the workspaces: {e}"),
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for workspace_id in rx {
            log_err(
                "announcing a workspace file change",
                events::emit(&app, WorkspaceFileChanged { workspace_id }),
            );
        }
    });
}
