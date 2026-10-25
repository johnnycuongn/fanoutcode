//! The decision flow (§5.4, §5.5, §5.6): the developer's answer from `overlap_decide` applied, and a held decision
//! released once its claim is gone. What to write comes from `overlap::decide`; this writes it. Decisions only ever
//! come from here and from `task_overlaps`, never from a log line.

use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use fanoutcode_core::changes;
use fanoutcode_core::domain::{Task, TaskId};
use fanoutcode_core::hooks::event_log_path;
use fanoutcode_core::memory::{IndexEntry, TaskFolder, TimelineKind};
use fanoutcode_core::overlap::claims::{ClaimsFile, ANOTHER_SESSION};
use fanoutcode_core::overlap::decide::{covered_pending, decided_line, outcome, released_holds};
use fanoutcode_core::overlap::prompts::go_ahead;
use fanoutcode_core::overlap::{grants, notes, store, Decision, OverlapRow, PendingDecision, Relation};
use fanoutcode_core::pipeline::{self, Effect};
use fanoutcode_core::{db, Error, Result};
use tauri::{AppHandle, Manager};

use super::{deliver_notes_if_idle, emit_overlap_changed, rebuild_claims, rebuild_overlaps, repin};
use crate::events;
use crate::git_watcher;
use crate::observe::{self, log_err};
use crate::session_service;
use crate::state::AppState;

/// Writes a `TimelineKind::Overlap` line into the task's timeline and `index.md`, and tells the windows.
fn timeline(app: &AppHandle, state: &AppState, id: &TaskId, detail: String, now: DateTime<Utc>) -> Result<()> {
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        let entry = IndexEntry {
            at: now,
            stage: task.stage,
            kind: TimelineKind::Overlap { detail },
        };
        let folder = TaskFolder::open(task.memory_dir.clone());
        pipeline::apply(&mut conn, &folder, &task, vec![Effect::WriteIndex(entry)], now)?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)
}

/// Appends one line to the task's event log (the Team view's mirror of a decision).
fn append_log(state: &AppState, id: &TaskId, line: &str) -> Result<()> {
    let path = event_log_path(&state.data_dir, id);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

fn inject(app: &AppHandle, state: &AppState, id: &TaskId, prompt: String) -> Result<()> {
    let task = db::tasks::require(&state.db.lock().expect("db lock"), id)?;
    session_service::apply_effects(app, &task, None, &[Effect::InjectPrompt(prompt)]);
    Ok(())
}

/// Moves a replaced path from `other`'s change set into `me`'s (`Manual`, as the developer put it there).
fn take_over(state: &AppState, me: &TaskId, other: &TaskId, repo: &Path, path: &str) -> Result<()> {
    let (o_task, o_home) = observe::task_and_home(state, other)?;
    let o_repos = observe::repos(&o_task, &o_home);
    let files = observe::with_attributor(state, other, &o_repos, |a| {
        a.remove(repo, path);
        a.files().to_vec()
    })?;
    observe::save_attributions(state, other, &files)?;
    let (m_task, m_home) = observe::task_and_home(state, me)?;
    let m_repos = observe::repos(&m_task, &m_home);
    let files = observe::with_attributor(state, me, &m_repos, |a| {
        a.include(repo, path);
        a.files().to_vec()
    })?;
    observe::save_attributions(state, me, &files)
}

/// The developer's answer to the open decision on `path` of `repo` against `other` (§5.4): the grant, the relation
/// (or the Hold), every other decision the grant answers, the timeline and log lines of both tasks, the taken-over
/// path moved, the pin re-evaluated, the go-ahead typed into this session and the note queued for the other one.
pub async fn decide(
    app: &AppHandle,
    task_id: &TaskId,
    repo: &Path,
    path: &str,
    other: Option<&TaskId>,
    decision: Decision,
) -> Result<()> {
    let state = app.state::<AppState>();
    let (me, home) = observe::task_and_home(&state, task_id)?;
    let repo = observe::checked_repo(&observe::repos(&me, &home), repo)?;
    changes::checked_path(&repo, path)?;
    let now = Utc::now();
    let (row, other_task): (OverlapRow, Option<Task>) = {
        let conn = state.db.lock().expect("db lock");
        let row = store::pending(&conn, task_id)?
            .into_iter()
            .find(|r| r.repo_root == repo && r.path == path && r.other_task.as_ref() == other)
            .ok_or_else(|| Error::Precondition("That file has no decision waiting.".into()))?;
        let other_task = match other {
            Some(o) => db::tasks::get(&conn, o)?,
            None => None,
        };
        (row, other_task)
    };
    if decision == Decision::Replace && matches!(row.pending, Some(PendingDecision::Command { .. })) {
        return Err(Error::Invalid("A stopped command can only proceed or hold.".into()));
    }
    let other_title = other_task
        .as_ref()
        .map_or(ANOTHER_SESSION, |t| t.title.as_str())
        .to_owned();
    let out = outcome(decision, &row, &me.title, &other_title, now);

    if let Some(g) = out.grant.clone() {
        // the claim-check hook spends command grants under the same lock
        let path = grants::grants_path(&state.data_dir, task_id);
        grants::with_grants_lock(&path, || {
            let mut held = grants::read_grants(&path);
            held.add(g);
            grants::write_grants(&state.data_dir, task_id, &held).map(|_| ())
        })?;
    }
    {
        let conn = state.db.lock().expect("db lock");
        let key = (task_id, repo.as_path(), path, row.other_task.as_ref());
        if out.hold {
            store::set_held(&conn, key.0, key.1, key.2, key.3, true)?;
        } else {
            store::set_relation(&conn, key.0, key.1, key.2, key.3, out.relation, Some(now))?;
            store::set_pending(&conn, key.0, key.1, key.2, key.3, None)?;
            // an AllFiles grant answers this task's other open decisions on the same task at once
            let grants = grants::read_grants(&grants::grants_path(&state.data_dir, task_id));
            let rows = store::list(&conn, task_id)?;
            for r in covered_pending(&rows, &grants, &row) {
                let other = r.other_task.as_ref();
                store::set_relation(
                    &conn,
                    task_id,
                    &r.repo_root,
                    &r.path,
                    other,
                    Relation::GrantedOnTop,
                    Some(now),
                )?;
                store::set_pending(&conn, task_id, &r.repo_root, &r.path, other, None)?;
            }
        }
    }
    timeline(app, &state, task_id, out.my_timeline.clone(), now)?;
    append_log(&state, task_id, &decided_line(&repo, path, other, decision, now))?;
    if let Some(o) = &other_task {
        if let Some(text) = out.other_timeline.clone() {
            timeline(app, &state, &o.id, text, now)?;
        }
        append_log(&state, &o.id, &decided_line(&repo, path, Some(task_id), decision, now))?;
        if let Some(relation) = out.other_relation {
            // the mirror row: the other task's Overlap tab shows the path taken over by this one
            let conn = state.db.lock().expect("db lock");
            store::set_relation(&conn, &o.id, &repo, path, Some(task_id), relation, Some(now))?;
        }
        if out.taken_over {
            take_over(&state, task_id, &o.id, &repo, path)?;
        }
    }
    repin(app, &state, task_id)?;
    emit_overlap_changed(app, &state, task_id)?;
    if let Some(p) = out.my_prompt.clone() {
        inject(app, &state, task_id, p)?;
    }
    if let (Some(o), Some(p)) = (&other_task, out.other_prompt.clone()) {
        notes::push_note(&notes::notes_path(&state.data_dir, &o.id), &p)?;
        deliver_notes_if_idle(app, &state, &o.id)?;
    }
    let (app, me_id, other_id) = (app.clone(), task_id.clone(), other_task.map(|t| t.id));
    let taken = out.taken_over;
    tauri::async_runtime::spawn(async move {
        after_decision(&app, &repo, &me_id, other_id.as_ref(), taken).await;
    });
    Ok(())
}

/// The claims, overlaps (and, after a takeover, the change sets) of both tasks, once a decision is written.
async fn after_decision(app: &AppHandle, repo: &Path, me: &TaskId, other: Option<&TaskId>, taken: bool) {
    log_err("rebuilding a repo's claims", rebuild_claims(app, repo).await);
    for id in std::iter::once(me).chain(other) {
        if taken {
            log_err(
                "refreshing a task's changes",
                git_watcher::refresh(app, id).await.map(|_| ()),
            );
        }
        log_err("rebuilding a task's overlaps", rebuild_overlaps(app, id).await);
    }
}

/// A rebuilt claims file: every held decision of `task_id` whose claim left it is closed, and the session is told
/// the file is now free (the `Hold` go-ahead, §5.4 answer 4), with a timeline line.
pub fn release_holds(app: &AppHandle, state: &AppState, task_id: &TaskId, file: &ClaimsFile) -> Result<()> {
    let freed: Vec<(OverlapRow, String)> = {
        let conn = state.db.lock().expect("db lock");
        let rows = store::pending(&conn, task_id)?;
        let mut freed = Vec::new();
        for r in released_holds(&rows, file) {
            store::set_pending(&conn, task_id, &r.repo_root, &r.path, r.other_task.as_ref(), None)?;
            let title = match &r.other_task {
                Some(o) => db::tasks::get(&conn, o)?.map(|t| t.title),
                None => None,
            };
            freed.push((r.clone(), title.unwrap_or_else(|| ANOTHER_SESSION.to_owned())));
        }
        freed
    };
    if freed.is_empty() {
        return Ok(());
    }
    let now = Utc::now();
    let mut prompts = Vec::new();
    for (r, title) in &freed {
        timeline(
            app,
            state,
            task_id,
            format!("{} is free: {title} let go of it", r.path),
            now,
        )?;
        prompts.push(go_ahead(Decision::Hold, title, &r.path));
    }
    repin(app, state, task_id)?;
    emit_overlap_changed(app, state, task_id)?;
    inject(app, state, task_id, prompts.join("\n\n"))
}
