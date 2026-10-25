//! "Bring <base> in" in the shell (Plan 11, §6.3): the integrate prompt's file list, the effects of an integrating
//! session's `Stop`, and the base re-recorded after a rebase. The rules live in `fanoutcode_core::overlap::integrate`.
//! No `std` lock here is held across an `.await`.

use std::path::{Path, PathBuf};

use chrono::Utc;
use fanoutcode_core::domain::{AgentState, HandbackReason, RepoBase, Task, TaskId};
use fanoutcode_core::hooks::{HookEvent, TaskEvent};
use fanoutcode_core::memory::{MemoryFile, TaskFolder};
use fanoutcode_core::overlap::integrate::{
    conflict_ctxs, integration_result, stop_effects, IntegrationResult, OtherIntent,
};
use fanoutcode_core::overlap::prompts::ConflictCtx;
use fanoutcode_core::overlap::store;
use fanoutcode_core::pipeline::{self, ctx::task_scope, Effect};
use fanoutcode_core::settings::{keys, resolve, Integration};
use fanoutcode_core::{db, git, Result};
use rusqlite::Connection;
use tauri::AppHandle;

use crate::events;
use crate::observation;
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
    if ev.agent_id.is_some() || !is_integrating(state, task)? {
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
    let base = task.base_branch.clone().unwrap_or_else(|| "the base branch".to_owned());
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
