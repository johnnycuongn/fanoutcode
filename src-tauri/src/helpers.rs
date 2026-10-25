//! Runs `Effect::RunHelper(kind)` (§7.4): one slot per `(task, kind)`, a new run replacing (aborting) the old, as
//! LiveSpec does. Naming runs on the fast tier; the spec watcher and commit messages on the deep tier. The prompts,
//! parsing and rules live in `fanoutcode_core::helpers`.

use std::time::Duration;

use chrono::Utc;
use fanoutcode_core::db;
use fanoutcode_core::domain::Stage;
use fanoutcode_core::domain::TaskId;
use fanoutcode_core::domain::{AgentState, HandbackReason};
use fanoutcode_core::helpers::commit_messages::{self, RepoCommits};
use fanoutcode_core::helpers::spec_watcher::{self, SpecWatchOutcome};
use fanoutcode_core::helpers::{briefs, call, naming, tag_for, HelperKind};
use fanoutcode_core::memory::slug::title_from_intent;
use fanoutcode_core::memory::{IndexEntry, TimelineKind};
use fanoutcode_core::memory::{MemoryFile, TaskFolder};
use fanoutcode_core::pipeline::{self, Effect};
use fanoutcode_core::Result;
use livespec_core::llm::Usage;
use livespec_core::settings::Tier;
use tauri::{AppHandle, Manager};

use crate::events::{self, CostUpdated, SpecWatcher};
use crate::observe::{self, log_err};
use crate::state::AppState;
use crate::{git_watcher, hook_feed, observation, provider};

/// The tier a helper runs on.
pub fn tier_for(kind: HelperKind) -> Tier {
    match kind {
        HelperKind::Naming => Tier::Fast,
        _ => Tier::Deep,
    }
}

/// Starts the helper for the task, replacing any run of the same kind still in flight.
pub fn run(app: &AppHandle, task_id: &TaskId, kind: HelperKind) {
    run_after(app, task_id, kind, Duration::ZERO);
}

/// As `run`, after `delay`; a newer call in the meantime replaces it, so this also debounces.
pub fn run_after(app: &AppHandle, task_id: &TaskId, kind: HelperKind, delay: Duration) {
    let (app2, id) = (app.clone(), task_id.clone());
    let handle = tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        if let Err(e) = job(&app2, &id, kind).await {
            eprintln!("Fanout Code: the {kind:?} helper: {e}");
            log_err("handing back a failed helper", hand_back_failure(&app2, &id, kind, &e));
        }
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

/// A failed commit-message draft moves the card off "working" (`pipeline::on_helper_failed`).
fn hand_back_failure(app: &AppHandle, id: &TaskId, kind: HelperKind, e: &fanoutcode_core::Error) -> Result<()> {
    let state = app.state::<AppState>();
    let task = db::tasks::require(&state.db.lock().expect("db lock"), id)?;
    match pipeline::on_helper_failed(&task, kind, &e.to_string()) {
        Some(next) => hook_feed::set_state(app, &state, id, next),
        None => Ok(()),
    }
}

async fn job(app: &AppHandle, id: &TaskId, kind: HelperKind) -> Result<()> {
    match kind {
        HelperKind::Naming => naming(app, id).await,
        HelperKind::ProjectBrief => project_briefs(app, id).await,
        HelperKind::SpecWatcher => spec_watcher(app, id).await,
        HelperKind::CommitMessages => commit_messages(app, id).await,
        HelperKind::GitCommit => commit_all(app, id).await,
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

/// During Review: rewrites `spec.md` from the diff when the watcher finds it out of step, logs the reason on the
/// timeline and emits `spec-watcher`. The app writes the file, and only over the text the watcher read.
async fn spec_watcher(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, _) = observe::task_and_home(&state, id)?;
    if task.stage != Stage::Review {
        return Ok(());
    }
    let folder = TaskFolder::open(task.memory_dir.clone());
    let Some(spec) = folder.read(MemoryFile::Spec)?.filter(|s| !s.trim().is_empty()) else {
        return Ok(());
    };
    let (task, set, _) = git_watcher::refresh(app, id).await?;
    let provider = provider::resolve(app, tier_for(HelperKind::SpecWatcher)).await?;
    let (outcome, usage) = spec_watcher::run_with_usage(&provider, state.usage.clone(), &task, &set, &spec).await?;
    note_usage(app, id, &usage);
    let at = Utc::now();
    let (Some(ev), SpecWatchOutcome::Rewrite { spec_md, reason }) =
        (spec_watcher::event_for(id, at, &outcome), outcome)
    else {
        return Ok(());
    };
    if folder.read(MemoryFile::Spec)?.as_deref() != Some(spec.as_str()) {
        return Ok(()); // the developer edited it meanwhile; their text wins
    }
    folder.write(MemoryFile::Spec, &spec_md)?;
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        let entry = IndexEntry {
            at,
            stage: task.stage,
            kind: TimelineKind::Spec { reason },
        };
        pipeline::apply(&mut conn, &folder, &task, vec![Effect::WriteIndex(entry)], at)?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)?;
    events::emit(
        app,
        SpecWatcher {
            task_id: ev.task_id,
            at: ev.at,
            reason: ev.reason,
        },
    )
}

/// On entering Commit: one draft per repo with task files, written to `commits.md`, then *Message drafted*.
async fn commit_messages(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, set, _) = git_watcher::refresh(app, id).await?;
    if task.stage != Stage::Commit {
        return Ok(());
    }
    let provider = provider::resolve(app, tier_for(HelperKind::CommitMessages)).await?;
    let mut all = Vec::new();
    for (repo, diff) in spec_watcher::diffs(&set).await {
        let files: Vec<String> = set
            .repos
            .iter()
            .filter(|r| r.repo_root.display().to_string() == repo)
            .flat_map(|r| r.files.iter().map(|f| f.path.clone()))
            .collect();
        let root = std::path::PathBuf::from(&repo);
        let req = commit_messages::request(&root, &diff, &files, &provider);
        let out = call(&provider, &req, state.usage.clone(), tag_for(&root)).await?;
        note_usage(app, id, &out.usage);
        all.push(RepoCommits {
            commits: commit_messages::parse(&out.text, &root),
            repo_root: root,
        });
    }
    TaskFolder::open(task.memory_dir.clone()).write(MemoryFile::Commits, &commit_messages::render(&all))?;
    let current = observe::task_and_home(&state, id)?.0;
    if current.stage == Stage::Commit {
        hook_feed::set_state(
            app,
            &state,
            id,
            AgentState::NeedsYou {
                reason: HandbackReason::StageDone,
            },
        )?;
    }
    Ok(())
}

/// The Commit action (only with "Fanout Code may commit"): every draft in `commits.md`, in order.
async fn commit_all(app: &AppHandle, id: &TaskId) -> Result<()> {
    let state = app.state::<AppState>();
    let (task, _) = observe::task_and_home(&state, id)?;
    let md = TaskFolder::open(task.memory_dir.clone())
        .read(MemoryFile::Commits)?
        .unwrap_or_default();
    for repo in commit_messages::parse_commits_md(&md) {
        for i in 0..repo.commits.len() {
            crate::commands::git::commit_one(app, id, &repo.repo_root, i).await?;
        }
    }
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

/// After a helper call: LiveSpec's ledger is flushed, and the usage lands in the task's cost under `helpers`, in
/// the stage the task is in now.
pub fn note_usage(app: &AppHandle, task_id: &TaskId, usage: &[Usage]) {
    let state = app.state::<AppState>();
    if let Err(e) = state.usage.flush_if_dirty() {
        eprintln!("Fanout Code: couldn't save the usage ledger: {e}");
    }
    if usage.is_empty() {
        return;
    }
    let recorded = {
        let conn = state.db.lock().expect("db lock");
        db::tasks::get(&conn, task_id).ok().flatten().map(|t| {
            usage
                .iter()
                .try_for_each(|u| fanoutcode_core::cost::ledger::record_helper(&conn, task_id, t.stage, u))
        })
    };
    if let Some(r) = recorded {
        if log_err("recording a helper's cost", r).is_some() {
            log_err(
                "reporting a cost change",
                events::emit(
                    app,
                    CostUpdated {
                        task_id: task_id.clone(),
                    },
                ),
            );
        }
    }
}
