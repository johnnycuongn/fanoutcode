//! Plan 11's shell sync (spec §5): the claims file of every repo rebuilt from the active tasks' attributions and a
//! fresh `git status`, each task's `task_overlaps` rows recomputed, the `ClaimDenied` stop recorded and pinned, and
//! the hookless sessions' edits attributed. Every decision lives in `fanoutcode_core::overlap`; this only gathers
//! its inputs and applies what it returns.
//!
//! The event log's `OverlapDecided` and `OverlapNoted` lines are a display mirror for the Team view: nothing here
//! reads them. A `ClaimDenied` line only opens pending decisions on, and pins, the task whose log it is in.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use fanoutcode_core::changes::{self, AttributedFile};
use fanoutcode_core::domain::{ProjectId, Task, TaskId};
use fanoutcode_core::git::{self, StatusEntry};
use fanoutcode_core::overlap::claims::{self, ClaimsFile, TaskFiles};
use fanoutcode_core::overlap::compute::{self, OtherTask};
use fanoutcode_core::overlap::denial::EditCall;
use fanoutcode_core::overlap::footprint::{self, FootprintRow};
use fanoutcode_core::overlap::observed;
use fanoutcode_core::overlap::{grants, store};
use fanoutcode_core::{db, Result};
use tauri::{AppHandle, Manager};

use crate::events::{self, OverlapChanged};
use crate::observe::{self, log_err};
use crate::state::AppState;

mod decide;
mod hookless;
mod pin;

pub use decide::{decide, release_holds};
pub use hookless::observe_hookless;
pub use pin::{apply_derived, clear_task, deliver_notes_if_idle, note_edit_call, on_claim_denied, repin};

/// How many recent edit calls per task are kept for a later `ClaimDenied` to take its snapshot from.
pub(super) const RECENT_EDITS: usize = 16;

/// What the sync keeps between runs. Every lock is held briefly and never across an `.await`.
#[derive(Default)]
pub struct OverlapCache {
    /// Each task's latest edit tool calls, newest last.
    pub(super) recent_edits: Mutex<HashMap<TaskId, VecDeque<EditCall>>>,
    /// Per repo, the dirty files no task owns (seen while several hookless sessions worked there, or left by an
    /// archived task): claims of *another session*, mirrored to `<app_data>/claims/<repo_key>.observed.json` on every
    /// change (`save_observed`) and read back by `start`.
    pub(super) unattributed: Mutex<HashMap<PathBuf, Vec<AttributedFile>>>,
    /// Per shared repo, the status `observe_hookless` saw last.
    pub(super) last_status: Mutex<HashMap<PathBuf, Vec<StatusEntry>>>,
    /// Per task, the lock every read-modify-write of its grants file in the shell holds (`grants_lock`), so a prune
    /// and a decision never drop each other's write. The claim-check hook runs in its own process and is not
    /// covered: its rewrite of the file can still race one of these.
    grants_locks: Mutex<HashMap<TaskId, Arc<Mutex<()>>>>,
}

/// The task's grants-file lock (see `OverlapCache::grants_locks`); hold the returned guard's mutex while reading,
/// changing and writing the file.
pub(super) fn grants_lock(state: &AppState, id: &TaskId) -> Arc<Mutex<()>> {
    state
        .overlap
        .grants_locks
        .lock()
        .expect("grants locks lock")
        .entry(id.clone())
        .or_default()
        .clone()
}

/// An active (not archived) task with its home root and repos.
pub(crate) struct Active {
    pub task: Task,
    pub home: PathBuf,
    pub repos: Vec<PathBuf>,
}

/// Every active task whose project is known.
pub(crate) fn active(state: &AppState) -> Result<Vec<Active>> {
    let conn = state.db.lock().expect("db lock");
    let mut roots: HashMap<ProjectId, Option<PathBuf>> = HashMap::new();
    let mut out = Vec::new();
    for task in db::tasks::list_all(&conn)?.into_iter().filter(|t| !t.archived) {
        let root = match roots.get(&task.project_id) {
            Some(r) => r.clone(),
            None => {
                let r = db::projects::get(&conn, &task.project_id)?.map(|p| p.root);
                roots.insert(task.project_id.clone(), r.clone());
                r
            }
        };
        if let Some(home) = root {
            let repos = changes::repos_of(&task, &home);
            out.push(Active { task, home, repos });
        }
    }
    Ok(out)
}

pub(super) fn files_of(state: &AppState, a: &Active) -> Result<Vec<AttributedFile>> {
    observe::with_attributor(state, &a.task.id, &a.repos, |x| x.files().to_vec())
}

/// The repos of `task` another active task also writes (the same tree: a home root without a worktree, or a linked
/// root).
pub fn shared_repos_of(state: &AppState, task: &Task) -> Result<Vec<PathBuf>> {
    let all = active(state)?;
    let Some(me) = all.iter().find(|a| a.task.id == task.id) else {
        return Ok(Vec::new());
    };
    Ok(me
        .repos
        .iter()
        .filter(|r| all.iter().any(|a| a.task.id != task.id && a.repos.contains(r)))
        .cloned()
        .collect())
}

/// `overlap-changed` for `task_id` with its fresh counts.
pub fn emit_overlap_changed(app: &AppHandle, state: &AppState, task_id: &TaskId) -> Result<()> {
    let rows = store::list(&state.db.lock().expect("db lock"), task_id)?;
    let pending = rows.iter().filter(|r| r.pending.is_some()).count();
    events::emit(
        app,
        OverlapChanged {
            task_id: task_id.clone(),
            count: u32::try_from(rows.len()).unwrap_or(u32::MAX),
            pending: u32::try_from(pending).unwrap_or(u32::MAX),
        },
    )
}

/// Rewrites `repo`'s claims file from its active tasks' attributions and its status (plus the unattributed files
/// still dirty), then prunes the grants of every task working there.
pub async fn rebuild_claims(app: &AppHandle, repo: &Path) -> Result<()> {
    if !git::is_repo(repo).await {
        return Ok(());
    }
    let status = git::status(repo).await?;
    let state = app.state::<AppState>();
    let all = active(&state)?;
    let owners: Vec<&Active> = all.iter().filter(|a| a.repos.iter().any(|r| r == repo)).collect();
    let files: Vec<Vec<AttributedFile>> = owners.iter().map(|a| files_of(&state, a)).collect::<Result<_>>()?;
    let tasks: Vec<TaskFiles<'_>> = owners
        .iter()
        .zip(&files)
        .map(|(a, f)| TaskFiles {
            task: &a.task,
            files: f,
        })
        .collect();
    let mut file = claims::build_claims(repo, &tasks, &status, Utc::now());
    let (seen, changed) = {
        let mut all = state.overlap.unattributed.lock().expect("unattributed lock");
        let list = all.entry(repo.to_owned()).or_default();
        let before = list.len();
        list.retain(|f| status.iter().any(|e| e.path == f.path));
        (list.clone(), list.len() != before)
    };
    if changed {
        save_observed(&state, repo, &seen)?;
    }
    claims::add_unattributed(&mut file, &seen, &status);
    claims::write_claims(&state.data_dir, &file)?;
    for a in &owners {
        log_err("pruning a task's grants", prune_grants(&state, a, &file));
    }
    // a hold is over only once its file is clean (whoever claims it), never because a claim left the file
    let attributed = |r: &fanoutcode_core::overlap::OverlapRow| {
        owners.iter().zip(&files).any(|(a, f)| {
            Some(&a.task.id) == r.other_task.as_ref() && f.iter().any(|x| x.repo_root == repo && x.path == r.path)
        })
    };
    for a in &owners {
        log_err(
            "releasing held decisions",
            release_holds(app, &state, &a.task.id, repo, &status, attributed),
        );
    }
    Ok(())
}

/// Writes `repo`'s list of *another session*'s files to disk, so a relaunch still knows them.
pub(super) fn save_observed(state: &AppState, repo: &Path, list: &[AttributedFile]) -> Result<()> {
    observed::write_observed(&state.data_dir, repo, list)
}

/// An archived task's dirty files in the repos other active tasks still work in become claims of *another
/// session*: its changes are still on disk, so they keep stopping other sessions and keep holds held until clean.
pub(super) async fn orphan_files(app: &AppHandle, task: &Task) -> Result<()> {
    let state = app.state::<AppState>();
    let (_, home) = observe::task_and_home(&state, &task.id)?;
    let repos = changes::repos_of(task, &home);
    let all = active(&state)?;
    let files = observe::with_attributor(&state, &task.id, &repos, |x| x.files().to_vec())?;
    for repo in repos.iter().filter(|r| all.iter().any(|a| a.repos.contains(r))) {
        if !git::is_repo(repo).await {
            continue;
        }
        let status = git::status(repo).await?;
        let left = observed::orphaned(repo, &files, &status);
        let list = {
            let mut all = state.overlap.unattributed.lock().expect("unattributed lock");
            let list = all.entry(repo.clone()).or_default();
            observed::merge(list, left).then(|| list.clone())
        };
        if let Some(list) = list {
            save_observed(&state, repo, &list)?;
        }
    }
    Ok(())
}

/// Drops the task's grants whose claims are gone, reading the claims of its other repos from disk.
fn prune_grants(state: &AppState, a: &Active, fresh: &ClaimsFile) -> Result<()> {
    let lock = grants_lock(state, &a.task.id);
    let _held = lock.lock().expect("grants lock");
    let path = grants::grants_path(&state.data_dir, &a.task.id);
    let mut held = grants::read_grants(&path);
    if held.grants.is_empty() {
        return Ok(());
    }
    let others: Vec<ClaimsFile> = a
        .repos
        .iter()
        .filter(|r| **r != fresh.repo_root)
        .filter_map(|r| claims::read_claims(&claims::claims_path(&state.data_dir, r)))
        .collect();
    let all: Vec<&ClaimsFile> = std::iter::once(fresh).chain(&others).collect();
    let before = held.clone();
    held.prune(&all);
    if held != before {
        grants::write_grants(&state.data_dir, &a.task.id, &held)?;
    }
    Ok(())
}

/// Whether `b` is worth recomputing when `a` changed: the same project, a shared repo, or a link between them.
fn related(a: &Active, b: &Active) -> bool {
    a.task.project_id == b.task.project_id
        || a.repos.iter().any(|r| b.repos.contains(r))
        || a.task.linked_roots.contains(&b.home)
        || b.task.linked_roots.contains(&a.home)
}

/// Recomputes `task_id`'s overlap rows (`compute::overlaps_for`), stores them and emits `overlap-changed`. The rows
/// are read, computed and written under one database lock, so a denial or a decision in between is never lost.
pub async fn rebuild_overlaps(app: &AppHandle, task_id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let all = active(&state)?;
    let Some(me) = all.iter().find(|a| a.task.id == *task_id) else {
        return Ok(());
    };
    let mut status_by_repo = Vec::new();
    for r in &me.repos {
        if git::is_repo(r).await {
            if let Ok(s) = git::status(r).await {
                status_by_repo.push((r.clone(), s));
            }
        }
    }
    let files: Vec<Vec<AttributedFile>> = all.iter().map(|a| files_of(&state, a)).collect::<Result<_>>()?;
    let claims: Vec<ClaimsFile> = me
        .repos
        .iter()
        .filter_map(|r| claims::read_claims(&claims::claims_path(&state.data_dir, r)))
        .collect();
    {
        let conn = state.db.lock().expect("db lock");
        let ids: Vec<TaskId> = all.iter().map(|a| a.task.id.clone()).collect();
        let prints = footprint::footprints_of(&conn, &ids)?;
        let mut by_task: HashMap<&TaskId, Vec<FootprintRow>> = HashMap::new();
        for p in &prints {
            by_task.entry(&p.task_id).or_default().push(p.clone());
        }
        let views: Vec<OtherTask<'_>> = all
            .iter()
            .zip(&files)
            .map(|(a, f)| OtherTask {
                task: &a.task,
                files: f,
                footprint: by_task.get(&a.task.id).map_or(&[][..], Vec::as_slice),
                branch: None,
                home_root: &a.home,
            })
            .collect();
        let mine = views.iter().find(|v| v.task.id == *task_id).expect("me is active");
        let existing = store::list(&conn, task_id)?;
        let rows = compute::overlaps_for(mine, &views, &claims, &status_by_repo, &existing, Utc::now());
        let keep: Vec<(PathBuf, String, Option<TaskId>)> = rows
            .iter()
            .map(|r| (r.repo_root.clone(), r.path.clone(), r.other_task.clone()))
            .collect();
        store::remove_absent(&conn, task_id, &keep)?;
        for r in rows.iter().filter(|r| !existing.contains(r)) {
            store::upsert(&conn, r)?;
        }
    }
    emit_overlap_changed(app, &state, task_id)
}

/// Rebuilds the claims of every repo of `task`, then the overlaps of it and of every task related to it.
pub async fn rebuild_around(app: &AppHandle, task: &Task) -> Result<()> {
    let state = app.state::<AppState>();
    let all = active(&state)?;
    // an archived task is no longer active, but its repos still need their claims rebuilt
    let (_, home) = observe::task_and_home(&state, &task.id)?;
    let me = Active {
        task: task.clone(),
        repos: changes::repos_of(task, &home),
        home,
    };
    for r in &me.repos {
        log_err("rebuilding a repo's claims", rebuild_claims(app, r).await);
    }
    let ids: Vec<TaskId> = all
        .iter()
        .filter(|a| a.task.id == task.id || related(&me, a))
        .map(|a| a.task.id.clone())
        .collect();
    for id in ids {
        log_err("rebuilding a task's overlaps", rebuild_overlaps(app, &id).await);
    }
    Ok(())
}

/// At launch: every repo's claims file from the stored attributions, the saved files of *another session* and a
/// fresh status (a stale file never survives a restart), every task's overlaps, and the pin of every task with an open decision.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let Some(all) = log_err("listing the active tasks", active(&state)) else {
            return;
        };
        let mut repos: Vec<PathBuf> = Vec::new();
        for r in all.iter().flat_map(|a| &a.repos) {
            if !repos.contains(r) {
                repos.push(r.clone());
            }
        }
        {
            let mut seen = state.overlap.unattributed.lock().expect("unattributed lock");
            for r in &repos {
                seen.insert(r.clone(), observed::read_observed(&state.data_dir, r));
            }
        }
        for r in &repos {
            log_err("rebuilding a repo's claims", rebuild_claims(&app, r).await);
        }
        for a in &all {
            log_err("rebuilding a task's overlaps", rebuild_overlaps(&app, &a.task.id).await);
            log_err("pinning a task", repin(&app, &state, &a.task.id));
        }
    });
}
