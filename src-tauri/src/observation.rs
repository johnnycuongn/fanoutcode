//! What the app does on its own after a task changes: keep its repos watched, record its base when it enters Build,
//! and create its worktree (`Effect::CreateWorktree`). Plan 05's runners for `effects::dispatch`.

use fanoutcode_core::domain::{RepoBase, Stage, Task, TaskId, WorktreeConvention};
use fanoutcode_core::memory::TaskFolder;
use fanoutcode_core::{db, git, pipeline, worktree, Error, Result};
use tauri::{AppHandle, Manager};

use crate::events;
use crate::git_watcher;
use crate::observe::{self, log_err};
use crate::state::AppState;

/// Re-reads the task, lets `edit` change it, persists it (and `index.md`) and tells every window.
pub fn update_task(app: &AppHandle, state: &AppState, id: &TaskId, edit: impl FnOnce(&mut Task)) -> Result<Task> {
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let mut task = db::tasks::require(&conn, id)?;
        edit(&mut task);
        task.updated_at = chrono::Utc::now();
        let folder = TaskFolder::open(task.memory_dir.clone());
        pipeline::apply(&mut conn, &folder, &task, Vec::new(), task.updated_at)?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)?;
    Ok(applied.task)
}

/// Archived: stop watching. Otherwise watch its repos (a no-op when unchanged) and, on entering Build without a
/// base, record HEAD of each repo as the base (§9).
pub fn on_task_applied(app: &AppHandle, state: &AppState, task: &Task) {
    if task.archived {
        state.git_watcher.unwatch(&task.id);
        return;
    }
    let (app, id) = (app.clone(), task.id.clone());
    let needs_base = task.stage == Stage::Build && task.base.is_empty();
    tauri::async_runtime::spawn(async move {
        log_err("watching a task", git_watcher::watch(&app, &id).await);
        if needs_base {
            log_err("recording the task's base", record_base(&app, &id).await);
        }
    });
}

async fn record_base(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, id)?;
    let mut bases = Vec::new();
    for repo in observe::repos(&task, &home) {
        // a repo that is not there (or not git) has no base; the others still get theirs
        if let Ok(commit) = git::head(&repo).await {
            bases.push(RepoBase {
                repo_root: repo,
                commit,
            });
        }
    }
    update_task(app, &state, id, |t| {
        if t.base.is_empty() {
            t.base = bases;
        }
    })?;
    Ok(())
}

/// `Effect::CreateWorktree`: the managed kind is created now; Claude Code's own is only named (it creates it at
/// launch). The task records the path and branch, and, for a managed worktree created here, the branch it was
/// created from as `base_branch` (Plan 11, §6.3; nothing for Claude Code's convention). Runs git on its own thread,
/// so it works inside or outside a runtime.
pub fn create_worktree(app: &AppHandle, state: &AppState, task: &Task) -> Result<Task> {
    let (_, home) = observe::task_and_home(state, &task.id)?;
    let (convention, dir) = worktree::settings_for(&state.db.lock().expect("db lock"), task, &home)?;
    let t = task.clone();
    let managed = convention == WorktreeConvention::FanoutCodeManaged;
    let (path, branch, base_branch) = std::thread::spawn(move || -> Result<_> {
        let creating = managed && !worktree::managed_path(&dir, &worktree::slug_of(&t)).exists();
        // the branch the new worktree forks from: HEAD of the home checkout, read before `git worktree add`
        let base_branch = if creating {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(git::current_branch(&home))
                .ok()
                .flatten()
        } else {
            None
        };
        let (path, branch) = worktree::prepare(&t, &home, convention, &dir)?;
        Ok((path, branch, base_branch))
    })
    .join()
    .map_err(|_| Error::Git("Creating the worktree failed".into()))??;
    update_task(app, state, &task.id, |t| {
        t.worktree_path = Some(path);
        if branch.is_some() {
            t.branch = branch;
        }
        if t.base_branch.is_none() {
            t.base_branch = base_branch;
        }
    })
}
