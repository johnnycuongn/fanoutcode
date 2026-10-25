//! `AppState`: what every window shares. Every lock is `std` and is never held across `.await`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use livespec_core::local::LocalServer;
use livespec_core::project::BriefStore;
use livespec_core::project::ProjectIndex;
use livespec_core::usage::UsageLedger;
use rusqlite::Connection;
use teamcode_core::activity::ActivityTracker;
use teamcode_core::changes::attributor::Attributor;
use teamcode_core::changes::ChangeSet;
use teamcode_core::cost::LiveCost;
use teamcode_core::domain::TaskId;
use teamcode_core::helpers::livespec_run::RunSlots;
use teamcode_core::helpers::HelperKind;
use teamcode_core::hooks::attribution::Correlator;
use teamcode_core::pipeline::TestRun;
use teamcode_core::sessions::{OwnedDriver, PtySpawner, SessionDriver, SessionRegistry};
use teamcode_core::settings::{keys, resolve, Scope};
use teamcode_core::testrun::{LiveOutput, TestResult};
use teamcode_core::windows::WindowRegistry;
use teamcode_core::{db, Result};

use crate::git_watcher::GitWatcher;
use crate::hooks_tailer::HooksTailer;
use crate::pty::PortablePtySpawner;

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
    /// LiveSpec's usage ledger, `<app_data>/usage.json`: every helper and notes call is metered here.
    pub usage: Arc<UsageLedger>,
    /// LiveSpec's project briefs, `<app_data>/briefs`.
    pub briefs: BriefStore,
    /// One helper run per task and kind; a new one aborts the old.
    pub helper_runs: Mutex<HashMap<(TaskId, HelperKind), tauri::async_runtime::JoinHandle<()>>>,
    /// The managed local model server, when the Local provider is used.
    pub local_server: tokio::sync::Mutex<Option<LocalServer>>,
    pub http: reqwest::Client,
    /// Each task's latest status-line numbers.
    pub live_cost: Mutex<HashMap<TaskId, LiveCost>>,
    /// The LiveSpec run in flight per task; a new one replaces it.
    pub livespec_runs: RunSlots<TaskId>,
    /// The last draft each task's notes were written for (the Jev gate's "previous").
    pub livespec_last: Mutex<HashMap<TaskId, String>>,
    /// Scanned project indexes, by home root, scanned on first use.
    pub livespec_indexes: Mutex<HashMap<PathBuf, Arc<ProjectIndex>>>,
    /// Each task's board-card activity, fed every hook event.
    pub activity: Mutex<HashMap<TaskId, ActivityTracker>>,
    /// Every live session (Plan 04). Its lock is taken by the PTY threads for each frame, so hold it briefly and never
    /// while taking `db`.
    pub sessions: Arc<Mutex<SessionRegistry>>,
    pub spawner: Arc<dyn PtySpawner>,
    pub driver: Arc<dyn SessionDriver>,
    /// Serialises session launches, so two callers opening one task's session get the same one.
    pub session_open: Mutex<()>,
    /// Each Codex session's idle state at the last tick, so a change updates the task once.
    pub codex_idle: Mutex<HashMap<teamcode_core::domain::SessionId, bool>>,
}

impl AppState {
    /// Opens (and on first launch creates) `<app_data>/teamcode.db`.
    pub fn new(data_dir: &Path, config_dir: &Path) -> Result<Self> {
        let conn = db::open(&data_dir.join("teamcode.db"))?;
        let lines = scrollback_lines(&conn)?;
        Ok(Self {
            db: Mutex::new(conn),
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
            usage: Arc::new(UsageLedger::new(data_dir.join("usage.json"))),
            briefs: BriefStore::new(data_dir.join("briefs")),
            helper_runs: Mutex::default(),
            local_server: tokio::sync::Mutex::default(),
            http: reqwest::Client::new(),
            live_cost: Mutex::default(),
            livespec_runs: RunSlots::default(),
            livespec_last: Mutex::default(),
            livespec_indexes: Mutex::default(),
            activity: Mutex::default(),
            sessions: Arc::new(Mutex::new(SessionRegistry::new(lines))),
            spawner: Arc::new(PortablePtySpawner),
            driver: Arc::new(OwnedDriver),
            session_open: Mutex::new(()),
            codex_idle: Mutex::default(),
        })
    }

    /// Locks the session registry.
    pub fn registry(&self) -> std::sync::MutexGuard<'_, SessionRegistry> {
        self.sessions.lock().expect("sessions lock")
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

/// The app-level `scrollback_lines` setting.
pub fn scrollback_lines(conn: &Connection) -> Result<usize> {
    let values = db::settings::load(conn, &Scope::app())?;
    Ok(resolve::<keys::ScrollbackLines>(&values, &Scope::app()) as usize)
}
