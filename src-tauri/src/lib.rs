//! Tauri shell: wires `teamcode-core` to the webview. Kept thin on purpose; logic that doesn't need Tauri lives in
//! the core crate.

pub mod commands;
pub mod effects;
pub mod events;
pub mod git_watcher;
pub mod helpers;
pub mod hook_feed;
pub mod hooks_tailer;
pub mod jira_transitions;
pub mod linking;
pub mod observation;
pub mod observe;
pub mod provider;
pub mod pty;
pub mod secrets;
pub mod session_service;
pub mod specta;
pub mod state;
pub mod windows;
pub mod workspace_sync;

use tauri::Manager;
use teamcode_core::db;
use teamcode_core::windows::SavedWindow;

use state::AppState;

pub fn run() {
    let specta = specta::builder();
    tauri::Builder::default()
        .invoke_handler(specta.invoke_handler())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
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
        .on_window_event(|window, event| {
            windows::on_window_event(window, event);
            // a closed window's panes are gone; their sessions keep running, detached (Review Focus 2)
            if let tauri::WindowEvent::Destroyed = event {
                if let Ok(id) = teamcode_core::ids::WindowId::parse(window.label()) {
                    window.app_handle().state::<AppState>().registry().detach_window(&id);
                }
            }
        })
        .setup(move |app| {
            specta.mount_events(app);
            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            app.manage(AppState::new(&data_dir, &config_dir)?);
            // an Owned PTY cannot outlive the app: every session row still marked running is stale
            if let Err(e) = db::sessions::mark_all_exited(&app.state::<AppState>().db.lock().expect("db lock"), -1) {
                eprintln!("Team Code: couldn't sweep the stale sessions: {e}");
            }
            // the shipped subagents, written once; an edited or deleted-and-restored file is never replaced
            if let Err(e) = teamcode_core::roster::install_shipped(&config_dir) {
                eprintln!("Team Code: couldn't install the shipped agents: {e}");
            }
            // Plan 04's note_hook_event, subscribed before the replay (A18); the tailer never names session_service
            let handle = app.handle().clone();
            hooks_tailer::subscribe(Box::new(move |task_id, ev| {
                session_service::note_hook_event(&handle, task_id, ev)
            }));
            hooks_tailer::HooksTailer::start(app.handle());
            session_service::start_codex_ticker(app.handle());
            // the first launch would otherwise wait for the login shell's PATH
            std::thread::spawn(livespec_core::shell_env::login_shell_path);
            git_watcher::GitWatcher::start(app.handle());
            workspace_sync::start(app.handle());
            jira_transitions::install(app.handle());
            let state = app.state::<AppState>();
            let open = {
                let conn = state.db.lock().expect("db lock");
                let saved = db::app_setting::<Vec<SavedWindow>>(&conn, "open_windows").unwrap_or_else(|e| {
                    eprintln!("Team Code: couldn't read the saved windows: {e}");
                    None
                });
                let mut registry = state.windows.lock().expect("windows lock");
                registry.restore(saved.unwrap_or_default(), |c| {
                    db::container_exists(&conn, c).unwrap_or(false)
                })
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
        .build(tauri::generate_context!())
        .expect("failed to build Team Code")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                session_service::on_exit_requested(app, code, &api);
            }
        });
}
