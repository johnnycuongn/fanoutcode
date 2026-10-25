//! What the app does with one parsed hook event (§7.3, §6): session id, subagent attribution, the chip state, the
//! change-set attributions and their git probes, then `team-event` and the `hooks_tailer` subscribers.

use std::path::{Path, PathBuf};

use fanoutcode_core::changes::attributor::Probe;
use fanoutcode_core::cost::{ledger, LiveCost};
use fanoutcode_core::domain::{AgentState, Task, TaskId};
use fanoutcode_core::hooks::attribution::read_agent_meta;
use fanoutcode_core::hooks::plan_steps::{dispatch_from_tool_input, Dispatch};
use fanoutcode_core::hooks::{artifact, feed, HookEvent, TaskEvent};
use fanoutcode_core::memory::TaskFolder;
use fanoutcode_core::{db, git, pipeline, Result};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::events::{self, CostUpdated, TeamEvent};
use crate::observation;
use crate::observe;
use crate::state::AppState;

/// The raw payload of a log line (the third field), for what `HookEvent` does not carry (`tool_use_id`, paths).
fn payload(line: &str) -> Value {
    line.splitn(3, '\t')
        .nth(2)
        .and_then(|p| serde_json::from_str(p).ok())
        .unwrap_or(Value::Null)
}

/// A dispatch as its description reads: `Step 2, 3 · frontend: summary`, or the summary alone.
fn describe(d: &Dispatch) -> String {
    if d.steps.is_empty() {
        return d.summary.clone();
    }
    let steps: Vec<String> = d.steps.iter().map(u32::to_string).collect();
    format!("Step {} · {}: {}", steps.join(", "), d.role, d.summary)
}

/// Sets the task's state through the pipeline (a hand-back writes its timeline line), unless it already is.
pub fn set_state(app: &AppHandle, state: &AppState, id: &TaskId, next: AgentState) -> Result<()> {
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        if task.state == next || task.archived {
            return Ok(());
        }
        let now = chrono::Utc::now();
        let folder = TaskFolder::open(task.memory_dir.clone());
        pipeline::apply(&mut conn, &folder, &task, vec![pipeline::Effect::SetState(next)], now)?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)
}

/// Whether the Build artifact exists: files attributed since the stage was entered, or a worktree with changes.
fn files_changed(state: &AppState, task: &Task, repos: &[PathBuf]) -> Result<bool> {
    let since = task.stage_entered_at;
    let attributed =
        observe::with_attributor(state, &task.id, repos, |a| a.files().iter().any(|f| f.added_at > since))?;
    let tree = task.worktree_path.is_some()
        && state
            .change_sets
            .lock()
            .expect("change sets lock")
            .get(&task.id)
            .is_some_and(|s| s.repos.iter().any(|r| !r.files.is_empty()));
    Ok(attributed || tree)
}

/// Runs the attributor's probes: `git status` of every repo for the Bash window, then saves the attributions.
async fn run_probes(state: &AppState, id: &TaskId, repos: &[PathBuf], probes: Vec<Probe>) -> Result<()> {
    for probe in probes {
        let window = match probe {
            Probe::Snapshot { window } | Probe::Delta { window } => window,
        };
        for repo in repos {
            let Ok(status) = git::status(repo).await else { continue };
            observe::with_attributor(state, id, repos, |a| a.on_status(repo, window, &status))?;
        }
    }
    let files = observe::with_attributor(state, id, repos, |a| a.files().to_vec())?;
    observe::save_attributions(state, id, &files)
}

/// Handles one event of task `ev.task_id`; `line` is its raw log line; `live` is false during launch replay.
pub async fn handle(app: &AppHandle, mut ev: TaskEvent, line: &str, live: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let id = ev.task_id.clone();
    let Ok((task, home)) = observe::task_and_home(&state, &id) else {
        // a log of a deleted task still reaches the subscribers
        crate::hooks_tailer::publish(&id, &ev.event);
        return Ok(());
    };
    let repos = observe::repos(&task, &home);
    let raw = payload(line);
    match &mut ev.event {
        HookEvent::SessionStart {
            session_id,
            transcript_path,
        } => {
            state.tailer_transcripts(&id, Some(transcript_path.clone()));
            ledger::register_transcript(&state.db.lock().expect("db lock"), transcript_path, &id, "")?;
            let sid = session_id.clone();
            if !sid.is_empty() && task.claude_session_id.as_deref() != Some(sid.as_str()) {
                observation::update_task(app, &state, &id, |t| t.claude_session_id = Some(sid))?;
            }
        }
        HookEvent::PreToolUse { tool, input } => {
            if let Some(d) = dispatch_from_tool_input(tool, input) {
                let tool_use_id = raw.get("tool_use_id").and_then(Value::as_str).unwrap_or("");
                state.with_correlator(&id, |c| c.note_dispatch(tool_use_id, d));
            }
        }
        HookEvent::SubagentStart {
            agent_id, description, ..
        } => {
            let meta = main_transcript(&state, &id).and_then(|t| read_agent_meta(&t, agent_id));
            let d = state.with_correlator(&id, |c| c.on_subagent_start(agent_id, meta.as_ref()));
            *description = meta.and_then(|m| m.description).or_else(|| d.as_ref().map(describe));
        }
        HookEvent::SubagentStop { agent_id } => {
            if let Some(path) = raw.get("agent_transcript_path").and_then(Value::as_str) {
                ledger::register_transcript(&state.db.lock().expect("db lock"), Path::new(path), &id, agent_id)?;
            }
        }
        HookEvent::StatusLine {
            cost_usd,
            context_used_pct,
            ..
        } => {
            let live = LiveCost {
                cost_usd: *cost_usd,
                context_used_pct: *context_used_pct,
                at: ev.at,
            };
            state.live_cost.lock().expect("live cost lock").insert(id.clone(), live);
            events::emit(app, CostUpdated { task_id: id.clone() })?;
        }
        _ => {}
    }
    // the ledger: each turn's (and each subagent's) transcript lines, under the task's current stage
    if matches!(ev.event, HookEvent::Stop { .. } | HookEvent::SubagentStop { .. }) && ingest_all(&state, &task)? {
        events::emit(app, CostUpdated { task_id: id.clone() })?;
    }
    // the chip: a Stop checks the stage's artifact first
    let artifact_changed = match &ev.event {
        HookEvent::Stop { .. } if ev.agent_id.is_none() => {
            let files = files_changed(&state, &task, &repos)?;
            artifact::artifact_changed(&task.memory_dir, task.stage, task.stage_entered_at, files)
        }
        _ => false,
    };
    let main_or_prompt = ev.agent_id.is_none() || matches!(ev.event, HookEvent::PermissionRequest { .. });
    // Plan 11: a Stop of a task bringing its base in is read by `overlap_integrate`, not the stage feed
    let integrating = crate::overlap_integrate::on_main_stop(app, &state, &task, &ev).await?;
    if main_or_prompt && !integrating {
        if let Some(next) = feed::next_state(task.stage, &ev.event, artifact_changed) {
            set_state(app, &state, &id, next)?;
        }
    }
    // the change set: edits attribute directly, Bash calls need a status before and after
    let probes = observe::with_attributor(&state, &id, &repos, |a| a.on_event(&ev))?;
    if live {
        run_probes(&state, &id, &repos, probes).await?;
    } else if matches!(ev.event, HookEvent::PostToolUse { .. }) {
        let files = observe::with_attributor(&state, &id, &repos, |a| a.files().to_vec())?;
        observe::save_attributions(&state, &id, &files)?;
    }
    crate::overlap_integrate::on_edit(&state, &task, &repos, &ev, live);
    crate::commands::activity::track(&state, &ev, &repos);
    if !matches!(ev.event, HookEvent::StatusLine { .. }) {
        events::emit(
            app,
            TeamEvent {
                task_id: id.clone(),
                event: ev.clone(),
            },
        )?;
    }
    crate::hooks_tailer::publish(&id, &ev.event);
    Ok(())
}

/// The task's main transcript: from this launch's `SessionStart`, else the ledger's registered one.
fn main_transcript(state: &AppState, id: &TaskId) -> Option<PathBuf> {
    state.tailer_transcripts(id, None).or_else(|| {
        let conn = state.db.lock().expect("db lock");
        ledger::transcripts_for(&conn, id)
            .into_iter()
            .find(|(_, agent)| agent.is_empty())
            .map(|(p, _)| p)
    })
}

/// Reads every transcript of the task from its cursor into the ledger; whether anything was added.
fn ingest_all(state: &AppState, task: &Task) -> Result<bool> {
    let conn = state.db.lock().expect("db lock");
    let mut added = 0;
    for (path, agent) in ledger::transcripts_for(&conn, &task.id) {
        added += ledger::ingest(&conn, &task.id, task.stage, &path, &agent)?;
    }
    Ok(added > 0)
}

/// The memory folder's plan, for `hooks_plan_steps`.
pub fn plan_text(memory_dir: &Path) -> Result<Option<String>> {
    TaskFolder::open(memory_dir.to_owned()).read(fanoutcode_core::memory::MemoryFile::Plan)
}
