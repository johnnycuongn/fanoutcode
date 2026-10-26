//! The overlap pin (§5.4): open decisions from `ClaimDenied`, the needs-you pin and its release, the clearing on
//! Start fresh session and Archive, and note delivery to an idle session.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use fanoutcode_core::changes;
use fanoutcode_core::domain::{AgentState, Task, TaskId};
use fanoutcode_core::hooks::{HookEvent, TaskEvent};
use fanoutcode_core::overlap::claims;
use fanoutcode_core::overlap::denial::{self, EditCall};
use fanoutcode_core::overlap::{self, grants, notes, store};
use fanoutcode_core::pipeline::Effect;
use fanoutcode_core::{db, Result};
use tauri::AppHandle;

use super::{emit_overlap_changed, orphan_files, rebuild_around, RECENT_EDITS};
use crate::hook_feed::set_state;
use crate::observe::{self, log_err};
use crate::session_service;
use crate::state::AppState;

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
/// repo must be one of the task's and the path inside it; a named other task that is archived or unknown is recorded
/// as *another session*, and a line naming the task itself is ignored.
pub fn on_claim_denied(app: &AppHandle, state: &AppState, ev: &TaskEvent) -> Result<()> {
    let HookEvent::ClaimDenied {
        repo,
        path,
        command,
        cwd,
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
    // an archived or unknown other task is *another session*: its changes are still what stopped the session, and
    // a stopped session must always have a decision to wait on
    let other = match other_task {
        Some(o) if o == id => return Ok(()),
        Some(o) => db::tasks::get(&state.db.lock().expect("db lock"), o)?
            .filter(|t| !t.archived)
            .map(|t| t.id),
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
        cwd.as_deref(),
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
/// born needs-you (Review Focus 5); an archived task's note inbox goes too and its claims leave the claims files,
/// its dirty files in shared repos staying claimed as *another session* (`orphan_files`).
pub fn clear_task(app: &AppHandle, state: &AppState, task: &Task) -> Result<()> {
    store::clear_pending(&state.db.lock().expect("db lock"), &task.id)?;
    state.pinned.lock().expect("pinned lock").remove(&task.id);
    super::commands::forget(state, &task.id);
    let grants_file = grants::grants_path(&state.data_dir, &task.id);
    grants::with_grants_lock(&grants_file, || {
        remove_if_there(&grants_file);
        Ok(())
    })?;
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
            log_err(
                "keeping an archived task's dirty files claimed",
                orphan_files(&app, &task).await,
            );
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
    // the session's answer to these notes must not replace the hand-back the developer is looking at
    state
        .notice_turns
        .lock()
        .expect("notice turns lock")
        .entry(task_id.clone())
        .or_insert_with(|| task.state.clone());
    session_service::apply_effects(app, &task, None, &[Effect::InjectPrompt(inbox.join("\n\n"))]);
    Ok(())
}
