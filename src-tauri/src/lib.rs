//! Tauri shell: wires `teamcode-core` to the webview. Kept thin on purpose; logic that doesn't need Tauri lives in
//! the core crate.

pub mod state;
pub mod windows;

use tauri::Manager;
use teamcode_core::db;
use teamcode_core::windows::SavedWindow;

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Restores each window's position and size by label (= window id).
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .on_window_event(windows::on_window_event)
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            app.manage(AppState::new(&data_dir, &config_dir)?);
            let state = app.state::<AppState>();
            let open = {
                let conn = state.db.lock().expect("db lock");
                let saved = db::app_setting::<Vec<SavedWindow>>(&conn, "open_windows").unwrap_or_else(|e| {
                    eprintln!("Team Code: couldn't read the saved windows: {e}");
                    None
                });
                let mut registry = state.windows.lock().expect("windows lock");
                registry.restore(saved.unwrap_or_default(), |c| db::container_exists(&conn, c).unwrap_or(false))
            };
            // One window failing to build must not stop the rest; with none on screen the app would run
            // invisible, so then the last error ends the launch.
            let mut built = 0;
            let mut last_error = None;
            for info in &open {
                match windows::build(app.handle(), info) {
                    Ok(_) => built += 1,
                    Err(e) => {
                        eprintln!("Team Code: couldn't open window {}: {e}", info.window_id);
                        state.windows.lock().expect("windows lock").close(&info.window_id);
                        last_error = Some(e);
                    }
                }
            }
            if let Err(e) = state.save_windows() {
                eprintln!("Team Code: couldn't save the open windows: {e}");
            }
            match last_error {
                Some(e) if built == 0 => Err(e.into()),
                _ => Ok(()),
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run Team Code");
}
