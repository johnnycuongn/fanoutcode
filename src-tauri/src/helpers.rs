//! Runs `Effect::RunHelper(kind)` (§7.4): one slot per `(task, kind)`, a new run replacing (aborting) the old, as
//! LiveSpec does. Naming runs on the fast tier; the spec watcher and commit messages on the deep tier. The prompts,
//! parsing and rules live in `teamcode_core::helpers`.

use livespec_core::llm::Usage;
use livespec_core::settings::Tier;
use tauri::{AppHandle, Manager};
use teamcode_core::domain::TaskId;
use teamcode_core::helpers::{briefs, naming, tag_for, HelperKind};
use teamcode_core::memory::slug::title_from_intent;
use teamcode_core::memory::{MemoryFile, TaskFolder};
use teamcode_core::Result;

use crate::observe::{self, log_err};
use crate::state::AppState;
use crate::{observation, provider};

/// The tier a helper runs on.
pub fn tier_for(kind: HelperKind) -> Tier {
    match kind {
        HelperKind::Naming => Tier::Fast,
        _ => Tier::Deep,
    }
}

/// Starts the helper for the task, replacing any run of the same kind still in flight.
pub fn run(app: &AppHandle, task_id: &TaskId, kind: HelperKind) {
    let (app2, id) = (app.clone(), task_id.clone());
    let handle = tauri::async_runtime::spawn(async move {
        log_err(&format!("the {kind:?} helper"), job(&app2, &id, kind).await);
    });
    let state = app.state::<AppState>();
    let old = state
        .helper_runs
        .lock()
        .expect("helper runs lock")
        .insert((task_id.clone(), kind), handle);
    if let Some(old) = old {
        old.abort();
    }
}

async fn job(app: &AppHandle, id: &TaskId, kind: HelperKind) -> Result<()> {
    match kind {
        HelperKind::Naming => naming(app, id).await,
        HelperKind::ProjectBrief => project_briefs(app, id).await,
        // the spec watcher, commit messages and commit runners land with their own tasks of this plan
        HelperKind::SpecWatcher | HelperKind::CommitMessages | HelperKind::GitCommit => Ok(()),
    }
}

/// Names the task from `intent.md`, renaming it only while its title is still the derived one (the developer's
/// own title always wins).
async fn naming(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, id)?;
    let intent = TaskFolder::open(task.memory_dir.clone())
        .read(MemoryFile::Intent)?
        .unwrap_or_default();
    if intent.trim().is_empty() {
        return Ok(());
    }
    let provider = provider::resolve(app, tier_for(HelperKind::Naming)).await?;
    let (title, usage) = naming::run_with_usage(&provider, state.usage.clone(), tag_for(&home), &intent).await;
    note_usage(app, id, &usage);
    let Some(title) = title else { return Ok(()) };
    observation::update_task(app, &state, id, |t| {
        if t.title == title_from_intent(&intent) || t.title == naming::fallback(&intent) {
            t.title = title;
        }
    })?;
    Ok(())
}

/// Builds the brief of every project linked to the task that has none yet.
async fn project_briefs(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, _) = observe::task_and_home(&state, id)?;
    for root in &task.linked_roots {
        if briefs::cached(&state.briefs, root).is_some() {
            continue;
        }
        let provider = provider::resolve(app, Tier::Deep).await?;
        briefs::ensure(&state.briefs, root, &provider, state.usage.clone()).await?;
        note_usage(app, id, &[]);
    }
    Ok(())
}

/// After a helper call: LiveSpec's ledger is flushed.
pub fn note_usage(app: &AppHandle, _task_id: &TaskId, _usage: &[Usage]) {
    let state = app.state::<AppState>();
    if let Err(e) = state.usage.flush_if_dirty() {
        eprintln!("Team Code: couldn't save the usage ledger: {e}");
    }
}
