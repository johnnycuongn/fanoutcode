//! Worktree tasks working around each other in the shell (Plan 11, §6): "Bring <base> in" (the integrate prompt's
//! file list, an integrating session's `Stop`, the base re-recorded after a rebase), the footprint read from
//! `plan.md`, "Others working here" for the stage prompts, and the soft and new-task notes. The rules live in
//! `fanoutcode_core::overlap::{integrate, others}`. No `std` lock here is held across an `.await`.

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use fanoutcode_core::domain::{AgentState, HandbackReason, RepoBase, Stage, Task, TaskId};
use fanoutcode_core::hooks::{event_log_path, HookEvent, TaskEvent};
use fanoutcode_core::memory::{MemoryFile, TaskFolder};
use fanoutcode_core::overlap::integrate::{
    conflict_ctxs, integration_result, stop_effects, valid_branch, IntegrationResult, OtherIntent,
};
use fanoutcode_core::overlap::notes::{notes_path, push_note};
use fanoutcode_core::overlap::others;
use fanoutcode_core::overlap::prompts::ConflictCtx;
use fanoutcode_core::overlap::{footprint, store};
use fanoutcode_core::pipeline::{self, ctx::task_scope, handback::ends_with_sentinel, Effect};
use fanoutcode_core::settings::{keys, resolve, Integration};
use fanoutcode_core::{db, git, Result};
use rusqlite::Connection;
use tauri::AppHandle;

use crate::events;
use crate::observation;
use crate::observe::log_err;
use crate::state::AppState;

/// Another task's title, `spec.md` and `plan.md`; `None` when the task is gone.
fn intent_of(conn: &Connection, id: &TaskId) -> Option<OtherIntent> {
    let task = db::tasks::get(conn, id).ok()??;
    let folder = TaskFolder::open(task.memory_dir.clone());
    let read = |f: MemoryFile| folder.read(f).ok().flatten().unwrap_or_default();
    Some(OtherIntent {
        title: task.title,
        spec_md: read(MemoryFile::Spec),
        plan_md: read(MemoryFile::Plan),
    })
}

/// The files the integrate prompt lists for `task`, from its overlap rows, with each other task's intent.
pub fn contexts(state: &AppState, task: &TaskId) -> Result<Vec<ConflictCtx>> {
    let conn = state.db.lock().expect("db lock");
    let rows = store::list(&conn, task)?;
    Ok(conflict_ctxs(&rows, |id| intent_of(&conn, id)))
}

/// The project's `worktree_integration` at the task's scope.
pub fn mode_of(state: &AppState, task: &Task) -> Result<Integration> {
    let scope = task_scope(task);
    let values = db::settings::load(&state.db.lock().expect("db lock"), &scope)?;
    Ok(resolve::<keys::WorktreeIntegration>(&values, &scope))
}

/// Whether the task is integrating: marked so, or handed back as *Merge needs you* (the mark does not survive a
/// relaunch; the hand-back does), in which case it is marked again.
pub fn is_integrating(state: &AppState, task: &Task) -> Result<bool> {
    if state
        .integrating
        .lock()
        .expect("integrating lock")
        .contains_key(&task.id)
    {
        return Ok(true);
    }
    let merge = matches!(
        task.state,
        AgentState::NeedsYou {
            reason: HandbackReason::MergeNeedsYou { .. }
        }
    );
    if merge {
        let ctxs = contexts(state, &task.id)?;
        state
            .integrating
            .lock()
            .expect("integrating lock")
            .insert(task.id.clone(), ctxs);
    }
    Ok(merge)
}

/// Applies `effects` to the task (a `SetState` it is already in is dropped) and tells every window.
pub fn apply(app: &AppHandle, state: &AppState, id: &TaskId, effects: Vec<Effect>) -> Result<()> {
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        if task.archived {
            return Ok(());
        }
        let effects: Vec<Effect> = effects
            .into_iter()
            .filter(|e| !matches!(e, Effect::SetState(s) if *s == task.state))
            .collect();
        if effects.is_empty() {
            return Ok(());
        }
        let folder = TaskFolder::open(task.memory_dir.clone());
        pipeline::apply(&mut conn, &folder, &task, effects, Utc::now())?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)
}

/// After a rebase the recorded base of the task's tree no longer bounds its diff: it becomes the merge-base of the
/// base branch and `HEAD` (§6.3).
async fn rerecord_base(app: &AppHandle, state: &AppState, id: &TaskId, tree: &Path, base: &str) -> Result<()> {
    let commit = git::merge_base(tree, base, "HEAD").await?;
    let tree: PathBuf = tree.to_owned();
    observation::update_task(app, state, id, |t| {
        match t.base.iter_mut().find(|b| b.repo_root == tree) {
            Some(b) => b.commit = commit,
            None => t.base.push(RepoBase {
                repo_root: tree,
                commit,
            }),
        }
    })?;
    Ok(())
}

/// A main-agent `Stop` of a task bringing its base in: Done (the sentinel on a clean tree) → Idle with the
/// integration in the timeline, and after a rebase the base re-recorded; Conflicted → *Merge needs you* with the
/// unmerged paths; otherwise Working. Returns whether it handled the event (the stage feed then skips it).
pub async fn on_main_stop(app: &AppHandle, state: &AppState, task: &Task, ev: &TaskEvent) -> Result<bool> {
    let HookEvent::Stop {
        last_assistant_message, ..
    } = &ev.event
    else {
        return Ok(false);
    };
    if ev.agent_id.is_some() {
        return Ok(false);
    }
    if task.stage == Stage::Plan && ends_with_sentinel(Stage::Plan, last_assistant_message) {
        log_err("recording the plan's footprint", record_plan_footprint(state, task));
    }
    if !is_integrating(state, task)? {
        return Ok(false);
    }
    let Some(tree) = task.worktree_path.clone() else {
        state.integrating.lock().expect("integrating lock").remove(&task.id);
        return Ok(false);
    };
    let result = integration_result(last_assistant_message, git::has_conflicts(&tree).await?);
    let conflicted = match result {
        IntegrationResult::Conflicted => git::conflicted_paths(&tree).await.unwrap_or_default(),
        _ => Vec::new(),
    };
    let aborted = state.integration_aborts.lock().expect("aborts lock").contains(&task.id);
    let base = (task.base_branch.clone().filter(|b| valid_branch(b))).unwrap_or_else(|| "the base branch".to_owned());
    if result == IntegrationResult::Done {
        state.integrating.lock().expect("integrating lock").remove(&task.id);
        state.integration_aborts.lock().expect("aborts lock").remove(&task.id);
        if !aborted && mode_of(state, task)? == Integration::Rebase {
            rerecord_base(app, state, &task.id, &tree, &base).await?;
        }
    }
    let effects = stop_effects(result, &base, aborted, &tree, conflicted, task.stage, Utc::now());
    apply(app, state, &task.id, effects)?;
    Ok(true)
}

/// A file the app reads for a prompt or a branch name: `None` when missing, unreadable or over 1 MB.
fn read_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    (meta.is_file() && meta.len() <= 1 << 20).then(|| std::fs::read_to_string(path).ok())?
}

/// The plan's `## Files` as the task's footprint for its home repo (§6.1), on the Plan stage's sentinel.
fn record_plan_footprint(state: &AppState, task: &Task) -> Result<()> {
    let Some(plan) = TaskFolder::open(task.memory_dir.clone()).read(MemoryFile::Plan)? else {
        return Ok(());
    };
    let conn = state.db.lock().expect("db lock");
    let Some(project) = db::projects::get(&conn, &task.project_id)? else {
        return Ok(());
    };
    let mut roots: Vec<PathBuf> = task.worktree_path.iter().cloned().collect();
    roots.push(project.root.clone());
    let entries = others::plan_entries(&plan, &roots);
    footprint::replace_plan(&conn, &task.id, &project.root, &entries, Utc::now())
}

/// "Others working here" for the task's Plan and Build prompts (§6.1); `""` on any failure (logged).
pub fn others_section(state: &AppState, id: &TaskId) -> String {
    // the change sets are copied first: this lock is never taken while the database lock is held
    let sets = state.change_sets.lock().expect("change sets lock").clone();
    let conn = state.db.lock().expect("db lock");
    let section = db::tasks::get(&conn, id).and_then(|me| match me {
        Some(me) => others::others_section(
            &conn,
            &me,
            |t| sets.get(t).map(|s| s.repos.clone()).unwrap_or_default(),
            read_small,
        ),
        None => Ok(String::new()),
    });
    log_err("listing the others working here", section).unwrap_or_default()
}

/// Appends one app-written line to a task's event log (display only: the shell never acts on it).
fn append_log(state: &AppState, task: &TaskId, line: &str) -> Result<()> {
    let path = event_log_path(&state.data_dir, task);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

/// A live `PostToolUse` edit: the soft notes it owes (§6.2), into each session's note inbox (the claim-check hook
/// hands them over as `additionalContext` on the next tool call), with an `OverlapNoted` line in both event logs.
pub fn on_edit(state: &AppState, task: &Task, repos: &[PathBuf], ev: &TaskEvent, live: bool) {
    if !live {
        return;
    }
    let Some((tree, path)) = others::edited_file(&ev.event, repos) else {
        return;
    };
    let notes = {
        let conn = state.db.lock().expect("db lock");
        others::soft_notes(&conn, &task.id, &tree, &path, &read_small, Utc::now())
    };
    for note in log_err("finding the soft notes of an edit", notes).unwrap_or_default() {
        log_err(
            "sending a soft note",
            push_note(&notes_path(&state.data_dir, &note.to), &note.text),
        );
        for (to, line) in note.log_lines(Utc::now()) {
            log_err("logging a soft note", append_log(state, &to, &line));
        }
    }
}

/// A task that just entered Plan tells every other active task of its project (§6.1, last paragraph).
pub fn on_stage_entered(state: &AppState, before: Stage, task: &Task) {
    if before == Stage::Plan || task.stage != Stage::Plan || task.archived {
        return;
    }
    let notes = {
        let conn = state.db.lock().expect("db lock");
        others::new_task_notes(&conn, task, &read_small)
    };
    for (to, note) in log_err("finding the new-task notes", notes).unwrap_or_default() {
        log_err(
            "sending a new-task note",
            push_note(&notes_path(&state.data_dir, &to), &note),
        );
    }
}
