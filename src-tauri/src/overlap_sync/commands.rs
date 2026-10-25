//! A stopped command's grant after the go-ahead (§5.6): the other task is told once the claim-check hook reports
//! spending it (`CommandRan`) and only when that line matches a grant the app recorded itself, since every event-log
//! line is agent-writable; an unspent grant goes at the session's next main-agent `Stop` after the go-ahead.

use chrono::{DateTime, Utc};
use fanoutcode_core::domain::TaskId;
use fanoutcode_core::hooks::{HookEvent, TaskEvent};
use fanoutcode_core::overlap::decide::{command_ran, GrantedCommand};
use fanoutcode_core::overlap::{grants, notes};
use fanoutcode_core::Result;
use tauri::AppHandle;

use super::decide::timeline;
use super::deliver_notes_if_idle;
use crate::state::AppState;

/// Records a `Command` grant the app just wrote for `task_id`.
pub(super) fn record_grant(state: &AppState, task_id: &TaskId, granted: GrantedCommand) {
    state
        .overlap
        .granted_commands
        .lock()
        .expect("granted commands lock")
        .entry(task_id.clone())
        .or_default()
        .push(granted);
}

/// The go-ahead for a granted command was typed into `task_id`'s session at `at`.
pub(super) fn note_go_ahead(state: &AppState, task_id: &TaskId, at: DateTime<Utc>) {
    state
        .overlap
        .command_go_aheads
        .lock()
        .expect("command go-aheads lock")
        .insert(task_id.clone(), at);
}

/// Forgets every recorded grant and go-ahead of `task_id` (Start fresh session, Archive).
pub(super) fn forget(state: &AppState, task_id: &TaskId) {
    state
        .overlap
        .granted_commands
        .lock()
        .expect("granted commands lock")
        .remove(task_id);
    state
        .overlap
        .command_go_aheads
        .lock()
        .expect("command go-aheads lock")
        .remove(task_id);
}

/// A `CommandRan` line of the task's own log: when it matches a grant the app recorded, the other task's inbox gets
/// `told_command` and both timelines a line; otherwise nothing happens.
pub fn on_command_ran(app: &AppHandle, state: &AppState, ev: &TaskEvent) -> Result<()> {
    let HookEvent::CommandRan {
        repo,
        command,
        paths,
        other_task,
        ..
    } = &ev.event
    else {
        return Ok(());
    };
    let told = {
        let mut all = state.overlap.granted_commands.lock().expect("granted commands lock");
        let Some(granted) = all.get_mut(&ev.task_id) else {
            return Ok(());
        };
        command_ran(granted, repo, command, paths, other_task.as_ref())
    };
    let Some(told) = told else {
        return Ok(());
    };
    let now = Utc::now();
    timeline(app, state, &ev.task_id, told.my_timeline, now)?;
    timeline(app, state, &told.other_task, told.other_timeline, now)?;
    notes::push_note(
        &notes::notes_path(&state.data_dir, &told.other_task),
        &told.other_prompt,
    )?;
    deliver_notes_if_idle(app, state, &told.other_task)
}

/// A main-agent `Stop` at `at`: once it comes after a command go-ahead, the `Command` grants that go-ahead covered and
/// the session did not spend are dropped (`grants::stale_command_grants`), with their records.
pub fn drop_stale_command_grants(state: &AppState, task_id: &TaskId, at: DateTime<Utc>) -> Result<()> {
    let delivered = {
        let mut all = state.overlap.command_go_aheads.lock().expect("command go-aheads lock");
        match all.get(task_id).copied() {
            Some(d) if at > d => {
                all.remove(task_id);
                d
            }
            _ => return Ok(()),
        }
    };
    let path = grants::grants_path(&state.data_dir, task_id);
    grants::with_grants_lock(&path, || {
        let mut held = grants::read_grants(&path);
        let stale = grants::stale_command_grants(&held, delivered);
        if stale.is_empty() {
            return Ok(());
        }
        held.grants.retain(|g| !stale.contains(g));
        grants::write_grants(&state.data_dir, task_id, &held).map(|_| ())
    })?;
    if let Some(granted) = state
        .overlap
        .granted_commands
        .lock()
        .expect("granted commands lock")
        .get_mut(task_id)
    {
        granted.retain(|g| g.granted_at > delivered);
    }
    Ok(())
}
