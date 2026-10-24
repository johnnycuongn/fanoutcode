//! `AppState`: what every window shares. Every lock is `std` and is never held across `.await`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use teamcode_core::changes::attributor::Attributor;
use teamcode_core::changes::ChangeSet;
use teamcode_core::domain::TaskId;
use teamcode_core::hooks::attribution::Correlator;
use teamcode_core::pipeline::TestRun;
use teamcode_core::testrun::{LiveOutput, TestResult};
use teamcode_core::windows::WindowRegistry;
use teamcode_core::{db, Result};

use crate::git_watcher::GitWatcher;
use crate::hooks_tailer::HooksTailer;

/// A test run in flight: its task (aborting it kills the child) and its shared output.
pub type TestRunSlot = (tauri::async_runtime::JoinHandle<()>, Arc<LiveOutput>);

pub struct AppState {
    /// `<app_data>/teamcode.db`, migrated at open.
    pub db: Mutex<Connection>,
    /// Which window shows which container.
    pub windows: Mutex<WindowRegistry>,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    /// Each task's change-set attributions (Plan 05), loaded from `task_files` on first use.
    pub attributors: Mutex<HashMap<TaskId, Attributor>>,
    /// Each task's last built change set (the board's `+/-` and Done detection read it).
    pub change_sets: Mutex<HashMap<TaskId, ChangeSet>>,
    pub git_watcher: GitWatcher,
    /// Each task's subagent dispatches, matched to `SubagentStart`.
    pub correlators: Mutex<HashMap<TaskId, Correlator>>,
    /// Each task's main transcript, from its `SessionStart`.
    pub transcripts: Mutex<HashMap<TaskId, PathBuf>>,
    pub tailer: HooksTailer,
    /// The test run in flight per task, owned here so a closed window never kills it.
    pub test_runs: Mutex<HashMap<TaskId, TestRunSlot>>,
    /// Each task's last finished run.
    pub last_tests: Mutex<HashMap<TaskId, TestResult>>,
}

impl AppState {
    /// Opens (and on first launch creates) `<app_data>/teamcode.db`.
    pub fn new(data_dir: &Path, config_dir: &Path) -> Result<Self> {
        Ok(Self {
            db: Mutex::new(db::open(&data_dir.join("teamcode.db"))?),
            windows: Mutex::new(WindowRegistry::new()),
            data_dir: data_dir.to_owned(),
            config_dir: config_dir.to_owned(),
            attributors: Mutex::default(),
            change_sets: Mutex::default(),
            git_watcher: GitWatcher::default(),
            correlators: Mutex::default(),
            transcripts: Mutex::default(),
            tailer: HooksTailer::default(),
            test_runs: Mutex::default(),
            last_tests: Mutex::default(),
        })
    }

    /// Persists the open windows (app setting `open_windows`) for the next launch.
    pub fn save_windows(&self) -> Result<()> {
        let saved = self.windows.lock().expect("windows lock").to_saved(); // released before the db lock
        db::set_app_setting(&self.db.lock().expect("db lock"), "open_windows", &saved)
    }

    /// Runs `f` on the task's correlator.
    pub fn with_correlator<R>(&self, task: &TaskId, f: impl FnOnce(&mut Correlator) -> R) -> R {
        f(self
            .correlators
            .lock()
            .expect("correlators lock")
            .entry(task.clone())
            .or_default())
    }

    /// Records the task's main transcript when `set` is given; returns the one known.
    pub fn tailer_transcripts(&self, task: &TaskId, set: Option<PathBuf>) -> Option<PathBuf> {
        let mut all = self.transcripts.lock().expect("transcripts lock");
        if let Some(p) = set {
            all.insert(task.clone(), p);
        }
        all.get(task).cloned()
    }

    /// The task's last test run, for Fix failing tests and the Test prompt.
    pub fn last_test(&self, task: &TaskId) -> Option<TestRun> {
        self.last_tests
            .lock()
            .expect("last test lock")
            .get(task)
            .map(TestResult::as_test_run)
    }
}
