//! Tails `<app_data>/events/<task-id>.jsonl` (§7.3): a non-recursive `notify` watch on the events folder, each file
//! read from its saved cursor, complete lines parsed and handed to `hook_feed`, the cursor saved after each batch.
//! At launch every log is replayed from its cursor before the first window opens. Plan 04 registers its
//! `note_hook_event` with `subscribe` before `HooksTailer::start` (A18); this module imports nothing from Plan 04.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use fanoutcode_core::domain::TaskId;
use fanoutcode_core::hooks::cursor::EventCursor;
use fanoutcode_core::hooks::parse::{parse_line, split_complete_lines};
use fanoutcode_core::hooks::subscribers::{HookSubscriber, Subscribers};
use fanoutcode_core::hooks::{event_log_path, HookEvent};
use fanoutcode_core::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::hook_feed;
use crate::observe::log_err;
use crate::state::AppState;

static SUBSCRIBERS: LazyLock<Subscribers> = LazyLock::new(Subscribers::default);

/// Registers `f` for every parsed hook event, live and replayed, once each and in log order (A18).
/// `f` is a `HookSubscriber`, i.e. `Box<dyn Fn(&TaskId, &HookEvent) + Send + Sync>`.
pub fn subscribe(f: HookSubscriber) {
    SUBSCRIBERS.subscribe(f);
}

/// Hands one event to every subscriber; `hook_feed` calls it after its own dispatch.
pub(crate) fn publish(task_id: &TaskId, ev: &HookEvent) {
    SUBSCRIBERS.publish(task_id, ev);
}

#[derive(Default)]
pub struct HooksTailer {
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// One pump per task at a time, so lines are handled once and in order.
    pumps: Mutex<HashMap<TaskId, Arc<tokio::sync::Mutex<()>>>>,
}

/// The task id of an events file, `None` for anything else.
fn task_of(path: &Path) -> Option<TaskId> {
    let name = path.file_name()?.to_str()?.strip_suffix(".jsonl")?;
    TaskId::parse(name).ok()
}

/// The bytes after `offset`, or nothing when the file is gone or shorter (rewritten).
fn read_from(path: &Path, offset: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Ok(mut f) = std::fs::File::open(path) {
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len >= offset && f.seek(SeekFrom::Start(offset)).is_ok() {
            let _ = f.read_to_end(&mut buf);
        }
    }
    buf
}

impl HooksTailer {
    /// Replays every log from its cursor, then watches the events folder. Call once from `setup`.
    pub fn start(app: &AppHandle) {
        let state = app.state::<AppState>();
        let dir = state.data_dir.join("events");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("Fanout Code: couldn't create the events folder: {e}");
            return;
        }
        let ids: Vec<TaskId> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().filter_map(|e| task_of(&e.path())).collect())
            .unwrap_or_default();
        tauri::async_runtime::block_on(async {
            for id in &ids {
                log_err("replaying a task's events", pump(app, id, false).await);
            }
        });
        let handle = app.clone();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                return;
            }
            for id in event.paths.iter().filter_map(|p| task_of(p)) {
                let app = handle.clone();
                tauri::async_runtime::spawn(
                    async move { log_err("reading a task's events", pump(&app, &id, true).await) },
                );
            }
        });
        match watcher {
            Ok(mut w) => {
                if let Err(e) = w.watch(&dir, RecursiveMode::NonRecursive) {
                    eprintln!("Fanout Code: couldn't watch the events folder: {e}");
                }
                *state.tailer.watcher.lock().expect("tailer lock") = Some(w);
            }
            Err(e) => eprintln!("Fanout Code: couldn't watch the events folder: {e}"),
        }
    }

    fn pump_lock(&self, id: &TaskId) -> Arc<tokio::sync::Mutex<()>> {
        self.pumps
            .lock()
            .expect("pumps lock")
            .entry(id.clone())
            .or_default()
            .clone()
    }
}

/// The longest one event's handling may take before the pump moves on to the next line.
const HANDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Reads the task's log from its cursor and handles each new complete line; `live` is false during launch replay
/// (no git probes for calls long finished).
async fn pump(app: &AppHandle, id: &TaskId, live: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let lock = state.tailer.pump_lock(id);
    let _one = lock.lock().await;
    let path: PathBuf = event_log_path(&state.data_dir, id);
    let mut cursor = EventCursor::load(&state.db.lock().expect("db lock"), id)?;
    let mut buf = read_from(&path, cursor.byte_offset);
    let consumed = buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1) as u64;
    let lines = split_complete_lines(&mut buf);
    if lines.is_empty() {
        return Ok(());
    }
    for line in lines {
        cursor.seq += 1;
        match parse_line(id, cursor.seq, &line) {
            Ok(ev) => match tokio::time::timeout(HANDLE_TIMEOUT, hook_feed::handle(app, ev, &line, live)).await {
                Ok(r) => {
                    log_err("handling a hook event", r);
                }
                // one stuck event must not hold every later one (and the task's chip) behind this pump's lock
                Err(_) => eprintln!(
                    "Fanout Code: gave up on event line {} of {id} after {HANDLE_TIMEOUT:?}",
                    cursor.seq
                ),
            },
            Err(e) => eprintln!("Fanout Code: skipped event line {} of {id}: {e}", cursor.seq),
        }
    }
    cursor.byte_offset += consumed;
    let conn = state.db.lock().expect("db lock");
    cursor.save(&conn, id)
}
