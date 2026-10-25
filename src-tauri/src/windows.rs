//! Runtime windows, created in Rust only. Each is labelled with its id (`w-<n>`), so `tauri-plugin-window-state`
//! (which keys on labels) restores its position and size, and the capability glob `"windows": ["w-*"]` gives it IPC.

use fanoutcode_core::domain::{Container, WorkspaceId};
use fanoutcode_core::windows::WindowInfo;
use fanoutcode_core::{Error, Result};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};

use crate::events::{self, WorkspaceChanged};
use crate::state::AppState;

/// Builds the webview window for `info`. Call it from `setup` or an `async` command only: on Windows, building a
/// window in a synchronous command or an event handler deadlocks (WebView2).
pub fn build(app: &AppHandle, info: &WindowInfo) -> Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, info.window_id.as_str(), WebviewUrl::App("index.html".into()))
        .title(&info.title)
        .inner_size(1280.0, 820.0)
        .min_inner_size(860.0, 560.0)
        .build()
        .map_err(|e| Error::Provider(format!("Couldn't open a window: {e}")))
}

/// Writes the workspace's unsaved changes to its file and tells the other windows (the dirty dot clears).
fn save_on_close(app: &AppHandle, state: &AppState, id: &WorkspaceId) {
    match state.workspaces.save_if_dirty(id) {
        Ok(true) => {
            let saved = state.workspaces.get(id).map(|workspace| WorkspaceChanged { workspace });
            if let Err(e) = saved.and_then(|e| events::emit(app, e)) {
                eprintln!("Fanout Code: couldn't announce the saved workspace: {e}");
            }
        }
        Ok(false) => {}
        Err(e) => eprintln!("Fanout Code: couldn't save the workspace file: {e}"),
    }
}

/// Registered with `Builder::on_window_event`: a window the user closes saves its workspace's unsaved changes
/// (§4.2), then leaves the registry and the saved list.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { .. } = event {
        let state = window.app_handle().state::<AppState>();
        if let Ok(id) = fanoutcode_core::ids::WindowId::parse(window.label()) {
            let mut windows = state.windows.lock().expect("windows lock");
            let container = windows.get(&id).and_then(|w| w.container);
            windows.close(&id);
            drop(windows);
            if let Some(Container::Workspace { id: ws }) = container {
                save_on_close(window.app_handle(), &state, &ws);
            }
        }
        if let Err(e) = state.save_windows() {
            eprintln!("Fanout Code: couldn't save the open windows: {e}");
        }
    }
}
