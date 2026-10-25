//! Hookless sessions (§4): edits of Codex and shell sessions in a shared repo, attributed as `Observed`.

use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use fanoutcode_core::changes::{AttributedFile, ChangeSource, MAIN_AGENT};
use fanoutcode_core::domain::{AgentKind, TaskId};
use fanoutcode_core::git;
use fanoutcode_core::overlap::{claims, observed};
use fanoutcode_core::Result;
use tauri::{AppHandle, Manager};

use super::{active, save_observed, Active};
use crate::observe;
use crate::state::AppState;

/// A settled shared `repo`: what became dirty there that no task owns is attributed to the one hookless session
/// working there (`Observed`), or, with several working, kept as claims of *another session* (§4). Nothing is
/// attributed while a Claude session on the repo has a Bash call open (its own window will attribute it).
pub async fn observe_hookless(app: &AppHandle, repo: &Path) -> Result<()> {
    let status = git::status(repo).await?;
    let state = app.state::<AppState>();
    let all = active(&state)?;
    let on_repo: Vec<&Active> = all.iter().filter(|a| a.repos.iter().any(|r| r == repo)).collect();
    let prev = state
        .overlap
        .last_status
        .lock()
        .expect("last status lock")
        .insert(repo.to_owned(), status.clone());
    if on_repo.len() < 2 {
        return Ok(());
    }
    let working: Vec<TaskId> = state
        .registry()
        .working(Instant::now())
        .into_iter()
        .filter_map(|s| s.task_id)
        .collect();
    let hookless: Vec<&&Active> = on_repo
        .iter()
        .filter(|a| a.task.agent_kind != AgentKind::ClaudeCode && working.contains(&a.task.id))
        .collect();
    if hookless.is_empty() {
        return Ok(());
    }
    let mut owned = Vec::new();
    for a in &on_repo {
        let bash_open = observe::with_attributor(&state, &a.task.id, &a.repos, |x| {
            owned.extend_from_slice(x.files());
            x.has_open_bash_windows()
        })?;
        if bash_open && a.task.agent_kind == AgentKind::ClaudeCode {
            return Ok(());
        }
    }
    owned.extend(
        state
            .overlap
            .unattributed
            .lock()
            .expect("unattributed lock")
            .get(repo)
            .cloned()
            .unwrap_or_default(),
    );
    let fresh = claims::new_unowned_dirt(repo, prev.as_deref(), &status, &owned);
    if fresh.is_empty() {
        return Ok(());
    }
    let now = Utc::now();
    if let [one] = hookless.as_slice() {
        let files = observe::with_attributor(&state, &one.task.id, &one.repos, |x| {
            x.observe_dirty(repo, &fresh, now);
            x.files().to_vec()
        })?;
        return observe::save_attributions(&state, &one.task.id, &files);
    }
    let seen: Vec<AttributedFile> = fresh
        .into_iter()
        .map(|e| AttributedFile {
            repo_root: repo.to_owned(),
            path: e.path,
            agent_id: MAIN_AGENT.to_owned(),
            source: ChangeSource::Observed,
            added_at: now,
        })
        .collect();
    let list = {
        let mut all = state.overlap.unattributed.lock().expect("unattributed lock");
        let list = all.entry(repo.to_owned()).or_default();
        observed::merge(list, seen).then(|| list.clone())
    };
    match list {
        Some(list) => save_observed(&state, repo, &list),
        None => Ok(()),
    }
}
