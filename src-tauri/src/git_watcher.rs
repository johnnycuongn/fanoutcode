//! Watches each task's repos (§6.6, §9): one `notify` watcher for the app, a 500 ms debounce per task, then
//! `git status` and `numstat`, the change set rebuilt and persisted, `git-changes` emitted, and in Commit the Done
//! check. The logic it runs lives in `fanoutcode_core::{git, changes}`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use fanoutcode_core::changes::{self, ChangeSet, RepoSnapshot};
use fanoutcode_core::domain::{Stage, Task, TaskId};
use fanoutcode_core::git::{self, WatchHit};
use fanoutcode_core::helpers::spec_watcher;
use fanoutcode_core::helpers::HelperKind;
use fanoutcode_core::pipeline::StageAction;
use fanoutcode_core::{db, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::events::{self, GitChanges};
use crate::observe::{self, log_err};
use crate::state::AppState;

/// Quiet time after the last tree change before the change set is rebuilt.
pub const DEBOUNCE: Duration = Duration::from_millis(500);

struct Watched {
    repos: Vec<PathBuf>,
    /// Git directories outside the repos (a worktree's own and the common one): their refs, HEAD and index.
    git_dirs: Vec<PathBuf>,
    memory_dir: PathBuf,
    generation: u64,
    /// Hits since the last rebuild, for the spec watcher's trigger.
    hits: Vec<WatchHit>,
}

#[derive(Default)]
pub struct GitWatcher {
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// Watched path → how many tasks use it.
    paths: Mutex<HashMap<PathBuf, usize>>,
    tasks: Mutex<HashMap<TaskId, Watched>>,
}

/// A worktree's git directory and the common one, when they sit outside `repo`.
async fn outside_git_dirs(repo: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for flag in ["--absolute-git-dir", "--git-common-dir"] {
        let Ok(out) = git::run(repo, &["rev-parse", flag]).await else {
            continue;
        };
        let dir = repo.join(out.trim());
        let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
        if !dir.starts_with(repo) && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

impl GitWatcher {
    /// Creates the app's watcher. Call once from `setup`, after `AppState` is managed.
    pub fn start(app: &AppHandle) {
        let handle = app.clone();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            let state = handle.state::<AppState>();
            for path in &event.paths {
                state.git_watcher.on_path(&handle, path);
            }
        });
        match watcher {
            Ok(w) => {
                *app.state::<AppState>()
                    .git_watcher
                    .watcher
                    .lock()
                    .expect("watcher lock") = Some(w)
            }
            Err(e) => eprintln!("Fanout Code: couldn't watch the repositories: {e}"),
        }
        // every non-archived task, as at the last launch
        let tasks = {
            let state = app.state::<AppState>();
            let conn = state.db.lock().expect("db lock");
            db::tasks::list_all(&conn).unwrap_or_default()
        };
        for task in tasks.into_iter().filter(|t| !t.archived) {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { log_err("watching a task", watch(&app, &task.id).await) });
        }
    }

    fn add_path(&self, path: &Path) {
        let mut paths = self.paths.lock().expect("paths lock");
        let n = paths.entry(path.to_owned()).or_insert(0);
        if *n == 0 {
            if let Some(w) = self.watcher.lock().expect("watcher lock").as_mut() {
                if let Err(e) = w.watch(path, RecursiveMode::Recursive) {
                    eprintln!("Fanout Code: couldn't watch {}: {e}", path.display());
                }
            }
        }
        *n += 1;
    }

    fn remove_path(&self, path: &Path) {
        let mut paths = self.paths.lock().expect("paths lock");
        if let Some(n) = paths.get_mut(path) {
            *n -= 1;
            if *n == 0 {
                paths.remove(path);
                if let Some(w) = self.watcher.lock().expect("watcher lock").as_mut() {
                    let _ = w.unwatch(path);
                }
            }
        }
    }

    /// The repos the task is watched with, if it is.
    pub fn watched_repos(&self, task_id: &TaskId) -> Option<Vec<PathBuf>> {
        self.tasks
            .lock()
            .expect("tasks lock")
            .get(task_id)
            .map(|w| w.repos.clone())
    }

    /// Stops watching the task (archived).
    pub fn unwatch(&self, task_id: &TaskId) {
        let Some(old) = self.tasks.lock().expect("tasks lock").remove(task_id) else {
            return;
        };
        for p in old.repos.iter().chain(&old.git_dirs) {
            self.remove_path(p);
        }
    }

    /// The hits gathered since the last rebuild, cleared.
    pub fn take_hits(&self, task_id: &TaskId) -> Vec<WatchHit> {
        self.tasks
            .lock()
            .expect("tasks lock")
            .get_mut(task_id)
            .map(|w| std::mem::take(&mut w.hits))
            .unwrap_or_default()
    }

    fn on_path(&self, app: &AppHandle, path: &Path) {
        let mut due = Vec::new();
        {
            let mut tasks = self.tasks.lock().expect("tasks lock");
            for (id, w) in tasks.iter_mut() {
                let hit = if let Some(dir) = w.git_dirs.iter().find(|d| path.starts_with(d)) {
                    git::git_dir_hit(path, dir)
                } else if let Some(repo) = changes::repo_for(path, &w.repos) {
                    git::watch_filter(path, repo, &w.memory_dir)
                } else {
                    WatchHit::None
                };
                match hit {
                    WatchHit::None => {}
                    WatchHit::MemoryOnly => w.hits.push(hit),
                    WatchHit::Tree => {
                        w.hits.push(hit);
                        w.generation += 1;
                        due.push((id.clone(), w.generation));
                    }
                }
            }
        }
        for (id, generation) in due {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(DEBOUNCE).await;
                let state = app.state::<AppState>();
                let current = state
                    .git_watcher
                    .tasks
                    .lock()
                    .expect("tasks lock")
                    .get(&id)
                    .map(|w| w.generation);
                if current == Some(generation) {
                    log_err("refreshing a task's changes", on_settled(&app, &id).await);
                }
            });
        }
    }
}

/// Watches the task's repos (re-watching when they changed). Plan 04 calls this after `launch_plan`.
pub async fn watch(app: &AppHandle, task_id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, task_id)?;
    let repos = observe::repos(&task, &home);
    if state.git_watcher.watched_repos(task_id).as_ref() == Some(&repos) {
        return Ok(());
    }
    let mut git_dirs = Vec::new();
    for repo in &repos {
        for d in outside_git_dirs(repo).await {
            if !git_dirs.contains(&d) {
                git_dirs.push(d);
            }
        }
    }
    state.git_watcher.unwatch(task_id);
    for p in repos.iter().chain(&git_dirs) {
        if p.exists() {
            state.git_watcher.add_path(p);
        }
    }
    state.git_watcher.tasks.lock().expect("tasks lock").insert(
        task_id.clone(),
        Watched {
            repos,
            git_dirs,
            memory_dir: task.memory_dir.clone(),
            generation: 0,
            hits: Vec::new(),
        },
    );
    Ok(())
}

/// The status of every repo of the task, with its effective base.
pub async fn snapshots(task: &Task, repos: &[PathBuf]) -> Result<Vec<RepoSnapshot>> {
    let mut snaps = Vec::new();
    for repo in repos {
        if !git::is_repo(repo).await {
            continue;
        }
        let base = match git::head(repo).await {
            Ok(head) => git::effective_base(task, repo, &head),
            Err(_) => None,
        };
        snaps.push(RepoSnapshot {
            repo_root: repo.clone(),
            status: git::status(repo).await?,
            numstat: git::numstat(repo, base.as_deref()).await?,
            base,
            whole_tree_is_task: task.worktree_path.as_deref() == Some(repo.as_path()),
        });
    }
    Ok(snaps)
}

/// Rebuilds the task's change set: open Bash windows get the fresh status, the attributions are saved, the set is
/// cached and `git-changes` emitted. Returns the set and the one it replaced.
pub async fn refresh(app: &AppHandle, task_id: &TaskId) -> Result<(Task, ChangeSet, Option<ChangeSet>)> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, task_id)?;
    let repos = observe::repos(&task, &home);
    let snaps = snapshots(&task, &repos).await?;
    let files = observe::with_attributor(&state, task_id, &repos, |a| {
        for w in a.open_windows() {
            for s in &snaps {
                a.on_status(&s.repo_root, w, &s.status);
            }
        }
        a.files().to_vec()
    })?;
    observe::save_attributions(&state, task_id, &files)?;
    let set = changes::build(task_id, &snaps, &files);
    let previous = state
        .change_sets
        .lock()
        .expect("change sets lock")
        .insert(task_id.clone(), set.clone());
    events::emit(
        app,
        GitChanges {
            task_id: task_id.clone(),
        },
    )?;
    Ok((task, set, previous))
}

/// A debounced tree change: rebuild, then in Commit check whether everything the task touched is committed.
async fn on_settled(app: &AppHandle, task_id: &TaskId) -> Result<()> {
    let (task, _set, previous) = refresh(app, task_id).await?;
    let state = app.state::<AppState>();
    let hits = state.git_watcher.take_hits(task_id);
    if spec_watcher::should_trigger(task.stage, &hits) {
        crate::helpers::run_after(
            app,
            task_id,
            HelperKind::SpecWatcher,
            Duration::from_millis(spec_watcher::DEBOUNCE_MS),
        );
    }
    if task.stage == Stage::Commit {
        let repos = state.git_watcher.watched_repos(task_id).unwrap_or_default();
        let files = observe::with_attributor(&state, task_id, &repos, |a| a.files().to_vec())?;
        let pending = changes::done_paths(&repos, &files, previous.as_ref());
        if pending.is_empty() {
            return Ok(());
        }
        for (repo, paths) in &pending {
            if !git::commit_done(repo, paths).await? {
                return Ok(());
            }
        }
        crate::commands::tasks::run_action(app, &state, task_id, StageAction::Advance)?;
    }
    Ok(())
}
