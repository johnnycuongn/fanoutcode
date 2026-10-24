//! The Spec stage's LiveSpec switch (§6.1, A14): `livespec_analyze` returns at once and streams the run's events;
//! a new call replaces the task's run in flight; `livespec_cancel` stops it.

use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use teamcode_core::domain::TaskId;
use teamcode_core::helpers::livespec_level::{lane_plan, LanePlan};
use teamcode_core::helpers::livespec_run::{analyze, AnalyzeInput, LiveSpecEvent};
use teamcode_core::helpers::{briefs, tag_for};
use teamcode_core::memory::{MemoryFile, TaskFolder};
use teamcode_core::settings::{keys, resolve};
use teamcode_core::{db, pipeline, Error, Result};

use crate::observe;
use crate::provider;
use crate::state::AppState;

/// Starts the LiveSpec notes for the task's `spec.md` at the level its project chose; `Precondition` when Off.
#[tauri::command]
#[specta::specta]
pub async fn livespec_analyze(app: AppHandle, task_id: TaskId, on_event: Channel<LiveSpecEvent>) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    let state = app.state::<AppState>();
    let (task, _) = observe::task_and_home(&state, &id)?;
    let level = {
        let conn = state.db.lock().expect("db lock");
        let scope = pipeline::ctx::task_scope(&task);
        resolve::<keys::LivespecLevel>(&db::settings::load(&conn, &scope)?, &scope)
    };
    let settings = provider::settings(&app);
    let key = provider::typesafe_key_present(&app).await;
    let plan =
        lane_plan(level, &settings, key).ok_or_else(|| Error::Precondition("LiveSpec is off for this task.".into()))?;
    let (app2, id2) = (app.clone(), id.clone());
    let run = tokio::spawn(async move {
        if let Err(e) = run(&app2, &id2, plan, &on_event).await {
            let _ = on_event.send(LiveSpecEvent::Error { message: e.to_string() });
        }
    });
    state.livespec_runs.replace(id, run);
    Ok(())
}

async fn run(app: &AppHandle, id: &TaskId, plan: LanePlan, on_event: &Channel<LiveSpecEvent>) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, id)?;
    let draft = TaskFolder::open(task.memory_dir.clone())
        .read(MemoryFile::Spec)?
        .unwrap_or_default();
    let previous = state.livespec_last.lock().expect("livespec lock").get(id).cloned();
    let cached = state
        .livespec_indexes
        .lock()
        .expect("livespec lock")
        .get(&home)
        .cloned();
    let index = match cached {
        Some(i) => Some(i),
        None => {
            let root = home.clone();
            // an unreadable root analyses without an index
            let scanned = tokio::task::spawn_blocking(move || livespec_core::project::scan(&root).ok())
                .await
                .ok()
                .flatten()
                .map(Arc::new);
            if let Some(i) = &scanned {
                state
                    .livespec_indexes
                    .lock()
                    .expect("livespec lock")
                    .insert(home.clone(), i.clone());
            }
            scanned
        }
    };
    let brief = briefs::cached(&state.briefs, &home);
    let settings = provider::settings(app);
    let provider = provider::resolve(app, plan.tier).await?;
    let jev = if settings.jev_allowed() {
        provider::jev(app).await
    } else {
        None
    };
    let emit = |e: LiveSpecEvent| {
        let _ = on_event.send(e); // the window may be gone
    };
    let input = AnalyzeInput {
        draft: &draft,
        previous_draft: previous.as_deref(),
        index: index.as_deref(),
        brief: brief.as_deref(),
        plan: &plan,
        gate: settings.jev_gate_enabled(),
        rerank: settings.jev_rerank_enabled(),
    };
    let notes = analyze(
        input,
        &provider,
        jev.as_ref(),
        state.usage.clone(),
        tag_for(&home),
        &emit,
    )
    .await;
    if notes.is_some() {
        state
            .livespec_last
            .lock()
            .expect("livespec lock")
            .insert(id.clone(), draft);
    }
    let _ = state.usage.flush_if_dirty();
    Ok(())
}

/// Stops the task's LiveSpec run, if one is in flight.
#[tauri::command]
#[specta::specta]
pub fn livespec_cancel(state: State<'_, AppState>, task_id: TaskId) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    state.livespec_runs.cancel(&id);
    Ok(())
}
