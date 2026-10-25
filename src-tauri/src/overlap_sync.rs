//! Plan 11's shell sync (spec §5): the claims file of every repo rebuilt from the active tasks' attributions and a
//! fresh `git status`, each task's `task_overlaps` rows recomputed, the `ClaimDenied` stop recorded and pinned, and
//! the hookless sessions' edits attributed. Every decision lives in `fanoutcode_core::overlap`; this only gathers
//! its inputs and applies what it returns.
//!
//! The event log's `OverlapDecided` and `OverlapNoted` lines are a display mirror for the Team view: nothing here
//! reads them. A `ClaimDenied` line only opens pending decisions on, and pins, the task whose log it is in.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use chrono::Utc;
use fanoutcode_core::changes::{self, AttributedFile, ChangeSource, MAIN_AGENT};
use fanoutcode_core::domain::{AgentKind, AgentState, ProjectId, Task, TaskId};
use fanoutcode_core::git::{self, StatusEntry};
use fanoutcode_core::hooks::{HookEvent, TaskEvent};
use fanoutcode_core::overlap::claims::{self, ClaimsFile, TaskFiles};
use fanoutcode_core::overlap::compute::{self, OtherTask};
use fanoutcode_core::overlap::denial::{self, EditCall};
use fanoutcode_core::overlap::footprint::{self, FootprintRow};
use fanoutcode_core::overlap::{self, grants, notes, store};
use fanoutcode_core::pipeline::Effect;
use fanoutcode_core::{db, Result};
use tauri::{AppHandle, Manager};

use crate::events::{self, OverlapChanged};
use crate::hook_feed::set_state;
use crate::observe::{self, log_err};
use crate::session_service;
use crate::state::AppState;

/// How many recent edit calls per task are kept for a later `ClaimDenied` to take its snapshot from.
const RECENT_EDITS: usize = 16;

/// What the sync keeps between runs. Every lock is held briefly and never across an `.await`.
#[derive(Default)]
pub struct OverlapCache {
    /// Each task's latest edit tool calls, newest last.
    recent_edits: Mutex<HashMap<TaskId, VecDeque<EditCall>>>,
    /// Per repo, the dirty files seen while several hookless sessions worked there: claims of *another session*.
    unattributed: Mutex<HashMap<PathBuf, Vec<AttributedFile>>>,
    /// Per shared repo, the status `observe_hookless` saw last.
    last_status: Mutex<HashMap<PathBuf, Vec<StatusEntry>>>,
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

fn files_of(state: &AppState, a: &Active) -> Result<Vec<AttributedFile>> {
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
    let observed = {
        let mut all = state.overlap.unattributed.lock().expect("unattributed lock");
        let list = all.entry(repo.to_owned()).or_default();
        list.retain(|f| status.iter().any(|e| e.path == f.path));
        list.clone()
    };
    claims::add_unattributed(&mut file, &observed, &status);
    claims::write_claims(&state.data_dir, &file)?;
    for a in &owners {
        log_err("pruning a task's grants", prune_grants(&state, a, &file));
    }
    Ok(())
}

/// Drops the task's grants whose claims are gone, reading the claims of its other repos from disk.
fn prune_grants(state: &AppState, a: &Active, fresh: &ClaimsFile) -> Result<()> {
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

/// At launch: every repo's claims file from the stored attributions and a fresh status (a stale file never survives
/// a restart), every task's overlaps, and the pin of every task with an open decision.
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
        for r in &repos {
            log_err("rebuilding a repo's claims", rebuild_claims(&app, r).await);
        }
        for a in &all {
            log_err("rebuilding a task's overlaps", rebuild_overlaps(&app, &a.task.id).await);
            log_err("pinning a task", repin(&app, &state, &a.task.id));
        }
    });
}

/// Sets the task to the pin its open decisions call for (`overlap::pin_for`), keeping the state it had aside the
/// first time; with none open, releases the pin.
pub fn repin(app: &AppHandle, state: &AppState, id: &TaskId) -> Result<()> {
    let (task, reason) = {
        let conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        let rows = store::list(&conn, id)?;
        let mut titles = HashMap::new();
        for other in rows.iter().filter_map(|r| r.other_task.as_ref()) {
            if let Some(t) = db::tasks::get(&conn, other)? {
                titles.insert(other.clone(), t.title);
            }
        }
        (task, overlap::pin_for(&rows, &titles))
    };
    if task.archived {
        return Ok(());
    }
    match reason {
        Some(reason) => {
            if !overlap::is_pin_state(&task.state) {
                state
                    .pinned
                    .lock()
                    .expect("pinned lock")
                    .entry(id.clone())
                    .or_insert(task.state.clone());
            }
            set_state(app, state, id, AgentState::NeedsYou { reason })
        }
        None => {
            let kept = state.pinned.lock().expect("pinned lock").remove(id);
            if overlap::is_pin_state(&task.state) {
                // the state the feed derived meanwhile; after a relaunch none was kept, and the session is about to
                // be told to go ahead
                set_state(app, state, id, kept.unwrap_or(AgentState::Working))?;
            }
            Ok(())
        }
    }
}

/// The chip feed's next state through the pin (`overlap::pinned`): applied when nothing is pending, else kept aside
/// for when the last decision is made, and the pin re-asserted.
pub fn apply_derived(app: &AppHandle, state: &AppState, id: &TaskId, next: AgentState) -> Result<()> {
    let pending = store::pending(&state.db.lock().expect("db lock"), id)?.len();
    match overlap::pinned(u32::try_from(pending).unwrap_or(u32::MAX), Some(next.clone())) {
        Some(s) => set_state(app, state, id, s),
        None => {
            state.pinned.lock().expect("pinned lock").insert(id.clone(), next);
            repin(app, state, id)
        }
    }
}

/// A `ClaimDenied` line of the task's own log: opens its pending decisions (`denial::denial_rows`) and pins it. The
/// repo must be one of the task's, the path inside it, and a named other task must exist and be active; anything
/// else is ignored.
pub fn on_claim_denied(app: &AppHandle, state: &AppState, ev: &TaskEvent) -> Result<()> {
    let HookEvent::ClaimDenied {
        repo,
        path,
        command,
        other_task,
    } = &ev.event
    else {
        return Ok(());
    };
    let id = &ev.task_id;
    let (task, home) = observe::task_and_home(state, id)?;
    if task.archived {
        return Ok(());
    }
    let repo = observe::checked_repo(&observe::repos(&task, &home), repo)?;
    if let Some(p) = path {
        changes::checked_path(&repo, p)?;
    }
    let other = match other_task {
        Some(o) => match db::tasks::get(&state.db.lock().expect("db lock"), o)? {
            Some(t) if !t.archived && t.id != *id => Some(t.id),
            _ => return Ok(()),
        },
        None => None,
    };
    let claims = claims::read_claims(&claims::claims_path(&state.data_dir, &repo));
    let grants = grants::read_grants(&grants::grants_path(&state.data_dir, id));
    let recent: Vec<EditCall> = state
        .overlap
        .recent_edits
        .lock()
        .expect("recent edits lock")
        .get(id)
        .map(|d| d.iter().cloned().collect())
        .unwrap_or_default();
    let rows = denial::denial_rows(
        id,
        &repo,
        path.as_deref(),
        command.as_deref(),
        other.as_ref(),
        claims.as_ref(),
        &grants,
        &recent,
    );
    if rows.is_empty() {
        return Ok(());
    }
    {
        let conn = state.db.lock().expect("db lock");
        for d in &rows {
            store::set_pending(
                &conn,
                id,
                &repo,
                &d.path,
                d.other_task.as_ref(),
                Some(d.pending.clone()),
            )?;
        }
    }
    repin(app, state, id)?;
    emit_overlap_changed(app, state, id)
}

/// An edit tool's `PreToolUse`: remembered for a later denial, and given as the snapshot to an open decision on its
/// file that is still waiting for one (the denial's line came first).
pub fn note_edit_call(app: &AppHandle, state: &AppState, id: &TaskId, call: EditCall) -> Result<()> {
    {
        let mut all = state.overlap.recent_edits.lock().expect("recent edits lock");
        let recent = all.entry(id.clone()).or_default();
        recent.push_back(call.clone());
        while recent.len() > RECENT_EDITS {
            recent.pop_front();
        }
    }
    let filled = {
        let conn = state.db.lock().expect("db lock");
        let mut filled = false;
        for row in store::pending(&conn, id)? {
            if denial::awaits_snapshot(&row) && denial::writes(&call, &row.repo_root, &row.path) {
                let pending = overlap::PendingDecision::Edit {
                    tool: call.tool.clone(),
                    input: call.input.clone(),
                };
                store::set_pending(
                    &conn,
                    id,
                    &row.repo_root,
                    &row.path,
                    row.other_task.as_ref(),
                    Some(pending),
                )?;
                filled = true;
            }
        }
        filled
    };
    if filled {
        emit_overlap_changed(app, state, id)?;
    }
    Ok(())
}

/// Start fresh session or Archive: the task's open decisions, its pin and its grants go, so the new session is not
/// born needs-you (Review Focus 5); an archived task's note inbox goes too and its claims leave the claims files.
pub fn clear_task(app: &AppHandle, state: &AppState, task: &Task) -> Result<()> {
    store::clear_pending(&state.db.lock().expect("db lock"), &task.id)?;
    state.pinned.lock().expect("pinned lock").remove(&task.id);
    remove_if_there(&grants::grants_path(&state.data_dir, &task.id));
    if task.archived {
        remove_if_there(&notes::notes_path(&state.data_dir, &task.id));
        state
            .overlap
            .recent_edits
            .lock()
            .expect("recent edits lock")
            .remove(&task.id);
        let (app, task) = (app.clone(), task.clone());
        tauri::async_runtime::spawn(async move {
            log_err("rebuilding after an archive", rebuild_around(&app, &task).await);
        });
    }
    emit_overlap_changed(app, state, &task.id)
}

fn remove_if_there(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("Fanout Code: couldn't remove {}: {e}", path.display());
        }
    }
}

/// On a main-agent `Stop`, or after a note was pushed: when the task's live session is idle and no decision of its
/// own is open (it was told to wait for the developer), drains its note inbox and types the notes in. A working
/// session hears them through the claim-check hook's `additionalContext` instead.
pub fn deliver_notes_if_idle(app: &AppHandle, state: &AppState, task_id: &TaskId) -> Result<()> {
    if !store::pending(&state.db.lock().expect("db lock"), task_id)?.is_empty() {
        return Ok(());
    }
    let idle = {
        let registry = state.registry();
        match registry.live_for_task(task_id) {
            None => false,
            Some(sid) => !registry.working(Instant::now()).iter().any(|s| s.id == sid),
        }
    };
    if !idle {
        return Ok(());
    }
    let inbox = notes::drain_notes(&notes::notes_path(&state.data_dir, task_id));
    if inbox.is_empty() {
        return Ok(());
    }
    let task = db::tasks::require(&state.db.lock().expect("db lock"), task_id)?;
    session_service::apply_effects(app, &task, None, &[Effect::InjectPrompt(inbox.join("\n\n"))]);
    Ok(())
}

/// A settled shared `repo`: what became dirty there that no task owns is attributed to the one hookless session
/// working there (`Observed`), or, with several working, kept as claims of *another session* (§4). Nothing is
/// attributed while a Claude session on the repo has a Bash call open (its own window will attribute it).
pub async fn observe_hookless(app: &AppHandle, repo: &Path) -> Result<()> {
    let status = git::status(repo).await?;
    let state = app.state::<AppState>();
    let all = active(&state)?;
    let on_repo: Vec<&Active> = all.iter().filter(|a| a.repos.iter().any(|r| r == repo)).collect();
    let prev = state
        .overlap
        .last_status
        .lock()
        .expect("last status lock")
        .insert(repo.to_owned(), status.clone());
    if on_repo.len() < 2 {
        return Ok(());
    }
    let working: Vec<TaskId> = state
        .registry()
        .working(Instant::now())
        .into_iter()
        .filter_map(|s| s.task_id)
        .collect();
    let hookless: Vec<&&Active> = on_repo
        .iter()
        .filter(|a| a.task.agent_kind != AgentKind::ClaudeCode && working.contains(&a.task.id))
        .collect();
    if hookless.is_empty() {
        return Ok(());
    }
    let mut owned = Vec::new();
    for a in &on_repo {
        let bash_open = observe::with_attributor(&state, &a.task.id, &a.repos, |x| {
            owned.extend_from_slice(x.files());
            x.has_open_bash_windows()
        })?;
        if bash_open && a.task.agent_kind == AgentKind::ClaudeCode {
            return Ok(());
        }
    }
    owned.extend(
        state
            .overlap
            .unattributed
            .lock()
            .expect("unattributed lock")
            .get(repo)
            .cloned()
            .unwrap_or_default(),
    );
    let fresh = claims::new_unowned_dirt(repo, prev.as_deref(), &status, &owned);
    if fresh.is_empty() {
        return Ok(());
    }
    let now = Utc::now();
    if let [one] = hookless.as_slice() {
        let files = observe::with_attributor(&state, &one.task.id, &one.repos, |x| {
            x.observe_dirty(repo, &fresh, now);
            x.files().to_vec()
        })?;
        return observe::save_attributions(&state, &one.task.id, &files);
    }
    let mut all = state.overlap.unattributed.lock().expect("unattributed lock");
    let list = all.entry(repo.to_owned()).or_default();
    for e in fresh {
        if !list.iter().any(|f| f.path == e.path) {
            list.push(AttributedFile {
                repo_root: repo.to_owned(),
                path: e.path,
                agent_id: MAIN_AGENT.to_owned(),
                source: ChangeSource::Observed,
                added_at: now,
            });
        }
    }
    Ok(())
}
