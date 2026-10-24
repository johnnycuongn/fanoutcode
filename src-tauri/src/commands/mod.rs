//! IPC commands, one file per domain. Every command is `#[tauri::command] #[specta::specta]` and listed in
//! `specta::builder()`.

pub mod settings;
pub mod windows;
