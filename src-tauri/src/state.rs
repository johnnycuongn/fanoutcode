//! `AppState`: what every window shares. Every lock is `std` and is never held across `.await`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use teamcode_core::windows::WindowRegistry;
use teamcode_core::{db, Result};

pub struct AppState {
    /// `<app_data>/teamcode.db`, migrated at open.
    pub db: Mutex<Connection>,
    /// Which window shows which container.
    pub windows: Mutex<WindowRegistry>,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl AppState {
    /// Opens (and on first launch creates) `<app_data>/teamcode.db`.
    pub fn new(data_dir: &Path, config_dir: &Path) -> Result<Self> {
        Ok(Self {
            db: Mutex::new(db::open(&data_dir.join("teamcode.db"))?),
            windows: Mutex::new(WindowRegistry::new()),
            data_dir: data_dir.to_owned(),
            config_dir: config_dir.to_owned(),
        })
    }

    /// Persists the open windows (app setting `open_windows`) for the next launch.
    pub fn save_windows(&self) -> Result<()> {
        let saved = self.windows.lock().expect("windows lock").to_saved(); // released before the db lock
        db::set_app_setting(&self.db.lock().expect("db lock"), "open_windows", &saved)
    }
}
