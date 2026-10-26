//! Git commands for the Changes tab (§6.4, §9). Every task id is re-parsed, every repo must be one of the task's,
//! and every path goes through `changes::checked_path`.

use std::path::PathBuf;

use fanoutcode_core::changes::{self, FileDiff, RepoChanges};
use fanoutcode_core::domain::TaskId;
use fanoutcode_core::git::{self, FileStatus};
use fanoutcode_core::helpers::commit_messages::{self, RepoCommits};
use fanoutcode_core::memory::store::write_atomic;
use fanoutcode_core::memory::{IndexEntry, MemoryFile, TaskFolder, TimelineKind};
use fanoutcode_core::pipeline::{self, Effect};
use fanoutcode_core::settings::{keys, resolve};
use fanoutcode_core::Result;
use fanoutcode_core::{db, Error};
use tauri::{AppHandle, State};

use crate::events;
use crate::git_watcher;
use crate::observe;
use crate::state::AppState;

/// The task, its repo as named by the webview (validated), and the file's absolute path.
fn target(
    state: &AppState,
    task_id: &TaskId,
    repo: &std::path::Path,
    path: &str,
) -> Result<(fanoutcode_core::domain::Task, PathBuf, PathBuf)> {
    let id = TaskId::parse(task_id.as_str())?;
    let (task, home) = observe::task_and_home(state, &id)?;
    let repo = observe::checked_repo(&observe::repos(&task, &home), repo)?;
    let abs = changes::checked_path(&repo, path)?;
    Ok((task, repo, abs))
}

/// The task's change set as the watcher last built it. A pure read: it never emits `git-changes`, which every
/// change view answers by calling this again.
#[tauri::command]
#[specta::specta]
pub async fn git_changes(app: AppHandle, task_id: TaskId) -> Result<Vec<RepoChanges>> {
    let id = TaskId::parse(task_id.as_str())?;
    Ok(git_watcher::cached(&app, &id).await?.repos)
}

/// Both sides of one file: `before` at the repo's base (empty when absent there), `after` the working tree.
#[tauri::command]
#[specta::specta]
pub async fn git_diff(state: State<'_, AppState>, task_id: TaskId, repo: PathBuf, path: String) -> Result<FileDiff> {
    let (task, repo, abs) = target(&state, &task_id, &repo, &path)?;
    let entry = git::status(&repo).await?.into_iter().find(|e| e.path == path);
    let base = match git::head(&repo).await {
        Ok(head) => git::effective_base(&task, &repo, &head),
        Err(_) => None,
    };
    let before_path = entry
        .as_ref()
        .and_then(|e| e.orig_path.clone())
        .unwrap_or_else(|| path.clone());
    let before_bytes = match &base {
        Some(b) => git::show_at(&repo, b, &before_path).await?.unwrap_or_default(),
        None => String::new(),
    };
    let after_bytes = tokio::fs::read(&abs).await.unwrap_or_default();
    let binary = before_bytes.contains('\0') || after_bytes.contains(&0);
    Ok(FileDiff {
        repo_root: repo,
        status: entry.map_or(FileStatus::Modified, |e| e.status),
        before: if binary { String::new() } else { before_bytes },
        after: if binary {
            String::new()
        } else {
            String::from_utf8_lossy(&after_bytes).into_owned()
        },
        path,
        binary,
    })
}

/// The working-tree file's text.
#[tauri::command]
#[specta::specta]
pub async fn git_file_read(state: State<'_, AppState>, task_id: TaskId, repo: PathBuf, path: String) -> Result<String> {
    let (_, _, abs) = target(&state, &task_id, &repo, &path)?;
    Ok(tokio::fs::read_to_string(&abs).await?)
}

/// Saves the working-tree side from the diff editor (atomic); the watcher recomputes the diff.
#[tauri::command]
#[specta::specta]
pub fn git_file_write(
    state: State<'_, AppState>,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
    contents: String,
) -> Result<()> {
    let (_, _, abs) = target(&state, &task_id, &repo, &path)?;
    write_atomic(&abs, contents.as_bytes())
}

/// Pulls a file from "Other changes" into the task's set, then rebuilds the set (the tree did not change, so the
/// watcher would not).
#[tauri::command]
#[specta::specta]
pub async fn git_include_other(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: TaskId,
    repo: PathBuf,
    path: String,
) -> Result<()> {
    let (task, repo, _) = target(&state, &task_id, &repo, &path)?;
    let (_, home) = observe::task_and_home(&state, &task.id)?;
    let repos = observe::repos(&task, &home);
    let files = observe::with_attributor(&state, &task.id, &repos, |a| {
        a.include(&repo, &path);
        a.files().to_vec()
    })?;
    observe::save_attributions(&state, &task.id, &files)?;
    git_watcher::refresh(&app, &task.id).await.map(|_| ())
}

/// How many of the task's files are not committed yet, by the Commit stage's own Done check (§6.6): "Mark done"
/// asks first, and a Done task says so.
#[tauri::command]
#[specta::specta]
pub async fn git_uncommitted(state: State<'_, AppState>, task_id: TaskId) -> Result<u32> {
    let id = TaskId::parse(task_id.as_str())?;
    let (task, home) = observe::task_and_home(&state, &id)?;
    let repos = observe::repos(&task, &home);
    let files = observe::with_attributor(&state, &id, &repos, |a| a.files().to_vec())?;
    let last = state.change_sets.lock().expect("change sets lock").get(&id).cloned();
    let mut pending = 0;
    for (repo, paths) in changes::done_paths(&repos, &files, last.as_ref(), &[]) {
        pending += git::pending_paths(&repo, &paths).await?.len();
    }
    Ok(u32::try_from(pending).unwrap_or(u32::MAX))
}

/// The drafts in the task's `commits.md`, by repo (Plan 08's Commit tab).
#[tauri::command]
#[specta::specta]
pub fn git_drafts(state: State<'_, AppState>, task_id: TaskId) -> Result<Vec<RepoCommits>> {
    let id = TaskId::parse(task_id.as_str())?;
    let task = db::tasks::require(&state.db.lock().expect("db lock"), &id)?;
    let md = TaskFolder::open(task.memory_dir)
        .read(MemoryFile::Commits)?
        .unwrap_or_default();
    Ok(commit_messages::parse_commits_md(&md))
}

/// Commits draft `index` of `repo` (only with "Fanout Code may commit"), logs it on the timeline, rebuilds the
/// change set; the new sha.
pub(crate) async fn commit_one(app: &AppHandle, id: &TaskId, repo: &std::path::Path, index: usize) -> Result<String> {
    let state = tauri::Manager::state::<AppState>(app);
    let (task, home) = observe::task_and_home(&state, id)?;
    let may_commit = {
        let conn = state.db.lock().expect("db lock");
        let scope = pipeline::ctx::task_scope(&task);
        resolve::<keys::MayCommit>(&db::settings::load(&conn, &scope)?, &scope)
    };
    commit_messages::commit_guard(may_commit)?;
    let repo = observe::checked_repo(&observe::repos(&task, &home), repo)?;
    let folder = TaskFolder::open(task.memory_dir.clone());
    let md = folder.read(MemoryFile::Commits)?.unwrap_or_default();
    let draft = commit_messages::parse_commits_md(&md)
        .into_iter()
        .find(|r| r.repo_root == repo)
        .and_then(|r| r.commits.into_iter().nth(index))
        .ok_or_else(|| Error::Invalid("There is no such drafted commit.".into()))?;
    for f in &draft.files {
        changes::checked_path(&repo, f)?;
    }
    let sha = commit_messages::commit_files(&repo, &draft).await?;
    let applied = {
        let mut conn = state.db.lock().expect("db lock");
        let task = db::tasks::require(&conn, id)?;
        let now = chrono::Utc::now();
        let entry = IndexEntry {
            at: now,
            stage: task.stage,
            kind: TimelineKind::Commit {
                repo: repo.display().to_string(),
                sha: sha.clone(),
            },
        };
        pipeline::apply(&mut conn, &folder, &task, vec![Effect::WriteIndex(entry)], now)?
    };
    events::emit_task_changed(app, &state.db.lock().expect("db lock"), &applied.task)?;
    git_watcher::refresh(app, id).await?;
    Ok(sha)
}

/// The Commit tab's Commit button for one draft; refused unless "Fanout Code may commit" is on.
#[tauri::command]
#[specta::specta]
pub async fn git_commit(app: AppHandle, task_id: TaskId, repo: PathBuf, index: u32) -> Result<String> {
    let id = TaskId::parse(task_id.as_str())?;
    commit_one(&app, &id, &repo, index as usize).await
}
