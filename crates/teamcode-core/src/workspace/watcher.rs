//! Watches workspace files saved outside app data (explicit-save mode) for edits made elsewhere: an editor, a sync
//! tool, a `git pull`. Each file's parent folder is watched (editors replace files atomically), events are
//! debounced 300 ms per workspace, and a change is reported only when the file's bytes differ from the last ones
//! seen or written by the app itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode, Watcher};

use crate::domain::WorkspaceId;
use crate::{Error, Result};

const DEBOUNCE: Duration = Duration::from_millis(300);
const TICK: Duration = Duration::from_millis(50);

type Callback = Box<dyn Fn(WorkspaceId) + Send + Sync>;

#[derive(Default)]
struct Watched {
    /// The canonical file path per workspace.
    paths: HashMap<WorkspaceId, PathBuf>,
    /// The bytes last seen on disk or written by the app.
    last: HashMap<WorkspaceId, Vec<u8>>,
    /// When the latest event of an unreported burst arrived.
    pending: HashMap<WorkspaceId, Instant>,
    /// Watched folders and how many files each holds.
    dirs: HashMap<PathBuf, usize>,
}

struct Inner {
    watched: Mutex<Watched>,
    on_external_change: Callback,
}

pub struct WorkspaceFileWatcher {
    inner: Arc<Inner>,
    watcher: Mutex<notify::RecommendedWatcher>,
}

/// `path` with its folder canonicalised, so `/var/…` and `/private/var/…` compare equal.
fn canonical_file(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned()).join(name),
        _ => path.to_owned(),
    }
}

fn notify_err(e: notify::Error) -> Error {
    Error::Io(format!("Couldn't watch the workspace file: {e}"))
}

impl WorkspaceFileWatcher {
    pub fn new(on_external_change: Callback) -> Result<Arc<Self>> {
        let inner = Arc::new(Inner {
            watched: Mutex::default(),
            on_external_change,
        });
        let weak = Arc::downgrade(&inner);
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let (Some(inner), Ok(ev)) = (weak.upgrade(), res) else {
                return;
            };
            if matches!(ev.kind, EventKind::Access(_)) {
                return;
            }
            let mut w = inner.watched.lock().expect("watcher lock");
            let hit: Vec<WorkspaceId> = w
                .paths
                .iter()
                .filter(|(_, p)| ev.paths.iter().any(|e| e == *p || canonical_file(e) == **p))
                .map(|(id, _)| id.clone())
                .collect();
            for id in hit {
                w.pending.insert(id, Instant::now());
            }
        })
        .map_err(notify_err)?;
        spawn_debouncer(Arc::downgrade(&inner));
        Ok(Arc::new(Self {
            inner,
            watcher: Mutex::new(watcher),
        }))
    }

    /// Starts watching `path` for workspace `id` (replacing an earlier path for it).
    pub fn watch(&self, id: WorkspaceId, path: &Path) -> Result<()> {
        self.unwatch(&id);
        let file = canonical_file(path);
        let dir = file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/"));
        let current = std::fs::read(&file).unwrap_or_default();
        let first = {
            let mut w = self.inner.watched.lock().expect("watcher lock");
            w.paths.insert(id.clone(), file);
            w.last.insert(id, current);
            let n = w.dirs.entry(dir.clone()).or_insert(0);
            *n += 1;
            *n == 1
        };
        if first {
            self.watcher
                .lock()
                .expect("notify lock")
                .watch(&dir, RecursiveMode::NonRecursive)
                .map_err(notify_err)?;
        }
        Ok(())
    }

    pub fn unwatch(&self, id: &WorkspaceId) {
        let drop_dir = {
            let mut w = self.inner.watched.lock().expect("watcher lock");
            w.last.remove(id);
            w.pending.remove(id);
            let Some(file) = w.paths.remove(id) else { return };
            let dir = file
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("/"));
            match w.dirs.get_mut(&dir) {
                Some(n) if *n > 1 => {
                    *n -= 1;
                    None
                }
                Some(_) => {
                    w.dirs.remove(&dir);
                    Some(dir)
                }
                None => None,
            }
        };
        if let Some(dir) = drop_dir {
            let _ = self.watcher.lock().expect("notify lock").unwatch(&dir);
        }
    }

    /// Records the bytes the app is about to write, so the event their write causes is not reported.
    pub fn note_own_write(&self, id: &WorkspaceId, bytes: &[u8]) {
        let mut w = self.inner.watched.lock().expect("watcher lock");
        if w.paths.contains_key(id) {
            w.last.insert(id.clone(), bytes.to_vec());
        }
    }
}

/// Reports each burst once it has been quiet for `DEBOUNCE`, if the file's bytes changed. Ends with the watcher.
fn spawn_debouncer(inner: Weak<Inner>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(TICK);
        let Some(inner) = inner.upgrade() else { return };
        let due: Vec<(WorkspaceId, PathBuf)> = {
            let mut w = inner.watched.lock().expect("watcher lock");
            let now = Instant::now();
            let ids: Vec<WorkspaceId> = w
                .pending
                .iter()
                .filter(|(_, at)| now.duration_since(**at) >= DEBOUNCE)
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| {
                    w.pending.remove(&id);
                    w.paths.get(&id).cloned().map(|p| (id, p))
                })
                .collect()
        };
        for (id, path) in due {
            let bytes = std::fs::read(&path).unwrap_or_default();
            let changed = {
                let mut w = inner.watched.lock().expect("watcher lock");
                match w.last.get(&id) {
                    Some(last) if *last == bytes => false,
                    _ => {
                        w.last.insert(id.clone(), bytes);
                        true
                    }
                }
            };
            if changed {
                (inner.on_external_change)(id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ws1() -> WorkspaceId {
        WorkspaceId::parse("ws-1").unwrap()
    }

    #[test]
    fn external_change_fires_callback_once_per_burst() {
        let (tx, rx) = std::sync::mpsc::channel();
        let w = WorkspaceFileWatcher::new(Box::new(move |id| tx.send(id).unwrap())).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("S.teamcode-workspace.json");
        std::fs::write(&p, b"{}").unwrap();
        w.watch(ws1(), &p).unwrap();
        std::fs::write(&p, b"{\"a\":1}").unwrap();
        std::fs::write(&p, b"{\"a\":2}").unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap().as_str(), "ws-1");
        assert!(rx.recv_timeout(Duration::from_millis(700)).is_err());
    }

    #[test]
    fn own_write_does_not_fire() {
        let (tx, rx) = std::sync::mpsc::channel();
        let w = WorkspaceFileWatcher::new(Box::new(move |id| tx.send(id).unwrap())).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("S.teamcode-workspace.json");
        std::fs::write(&p, b"{}").unwrap();
        w.watch(ws1(), &p).unwrap();
        w.note_own_write(&ws1(), b"{\"own\":true}");
        std::fs::write(&p, b"{\"own\":true}").unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }
}
