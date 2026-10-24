//! Runtime windows, created in Rust only. Each is labelled with its id (`w-<n>`), so `tauri-plugin-window-state`
//! (which keys on labels) restores its position and size, and the capability glob `"windows": ["w-*"]` gives it IPC.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};
use teamcode_core::windows::WindowInfo;
use teamcode_core::{Error, Result};

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

/// Registered with `Builder::on_window_event`: a window the user closes leaves the registry and the saved list.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { .. } = event {
        let state = window.app_handle().state::<AppState>();
        if let Ok(id) = teamcode_core::ids::WindowId::parse(window.label()) {
            state.windows.lock().expect("windows lock").close(&id);
        }
        if let Err(e) = state.save_windows() {
            eprintln!("Team Code: couldn't save the open windows: {e}");
        }
    }
}
