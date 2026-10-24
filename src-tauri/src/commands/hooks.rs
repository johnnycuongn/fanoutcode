//! Hook commands for the Team canvas (A6): the task's whole event log, and its `plan.md` steps.

use tauri::State;
use teamcode_core::domain::TaskId;
use teamcode_core::hooks::attribution::{read_agent_meta, Correlator};
use teamcode_core::hooks::parse::read_all;
use teamcode_core::hooks::plan_steps::{dispatch_from_tool_input, parse_plan, PlanDoc};
use teamcode_core::hooks::{event_log_path, HookEvent, TaskEvent};
use teamcode_core::{db, Error, Result};

use crate::hook_feed;
use crate::state::AppState;

/// Every event of the task's log, in order, with subagent descriptions filled in as the live feed fills them.
#[tauri::command]
#[specta::specta]
pub async fn hooks_list(state: State<'_, AppState>, task_id: TaskId) -> Result<Vec<TaskEvent>> {
    let id = TaskId::parse(task_id.as_str())?;
    db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    let path = event_log_path(&state.data_dir, &id);
    tokio::task::spawn_blocking(move || {
        let mut events = read_all(&path, &id)?;
        let (mut transcript, mut correlator) = (None, Correlator::default());
        for e in &mut events {
            match &mut e.event {
                HookEvent::SessionStart { transcript_path, .. } => transcript = Some(transcript_path.clone()),
                HookEvent::PreToolUse { tool, input } => {
                    if let Some(d) = dispatch_from_tool_input(tool, input) {
                        correlator.note_dispatch("", d);
                    }
                }
                HookEvent::SubagentStart {
                    agent_id, description, ..
                } => {
                    let meta = transcript.as_ref().and_then(|t| read_agent_meta(t, agent_id));
                    let d = correlator.on_subagent_start(agent_id, None);
                    *description = meta
                        .and_then(|m| m.description)
                        .or_else(|| d.map(|d| d.summary).filter(|s| !s.is_empty()));
                }
                _ => {}
            }
        }
        Ok(events)
    })
    .await
    .map_err(|e| Error::Io(format!("Reading the events failed: {e}")))?
}

/// The task's `plan.md` as steps, dependencies and team; empty when there is no plan yet.
#[tauri::command]
#[specta::specta]
pub fn hooks_plan_steps(state: State<'_, AppState>, task_id: TaskId) -> Result<PlanDoc> {
    let id = TaskId::parse(task_id.as_str())?;
    let task = db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    Ok(hook_feed::plan_text(&task.memory_dir)?
        .map(|t| parse_plan(&t))
        .unwrap_or_default())
}
