//! "Bring <base> in" from a Done card (Plan 11, §6.3). The app runs no git that changes anything: these commands
//! inject prompts into the task's session and read answers from git; `overlap_integrate::on_main_stop` reads the
//! session's `Stop`.

use fanoutcode_core::domain::{AgentState, Stage, Task, TaskId};
use fanoutcode_core::overlap::integrate::{abort_prompt, resume_prompt, valid_branch};
use fanoutcode_core::overlap::prompts::{self, ConflictCtx};
use fanoutcode_core::pipeline::Effect;
use fanoutcode_core::{db, git, Error, Result};
use tauri::{AppHandle, Manager, State};

use crate::observation;
use crate::observe;
use crate::overlap_integrate::{apply, contexts, is_integrating, mode_of};
use crate::session_service;
use crate::state::AppState;

fn task_of(state: &AppState, id: &TaskId) -> Result<Task> {
    db::tasks::require(&state.db.lock().expect("db lock"), &TaskId::parse(id.as_str())?)
}

/// Sets the task Working and types `prompt` into its session (resuming one when none is live).
fn send(app: &AppHandle, state: &AppState, task: &Task, prompt: String) -> Result<()> {
    apply(app, state, &task.id, vec![Effect::SetState(AgentState::Working)])?;
    session_service::apply_effects(app, task, None, &[Effect::InjectPrompt(prompt)]);
    Ok(())
}

fn require_integrating(state: &AppState, task: &Task) -> Result<()> {
    if is_integrating(state, task)? {
        Ok(())
    } else {
        Err(Error::Precondition(
            "This task is not bringing its base branch in.".into(),
        ))
    }
}

/// **Bring <base> in**: lists the task's overlapping files with the other tasks' intent, marks it integrating and
/// asks its session to merge (or rebase onto) the base branch.
#[tauri::command]
#[specta::specta]
pub fn overlap_integrate(app: AppHandle, state: State<'_, AppState>, task_id: TaskId) -> Result<()> {
    let task = task_of(&state, &task_id)?;
    if task.stage != Stage::Done || task.worktree_path.is_none() {
        return Err(Error::Precondition(
            "Bring the base branch in from a Done task that has its own worktree.".into(),
        ));
    }
    // a stored name that is not a safe branch name is asked for again, never put in the prompt
    let Some(base) = task.base_branch.clone().filter(|b| valid_branch(b)) else {
        return Err(Error::Precondition("Confirm the base branch first.".into()));
    };
    let ctxs = contexts(&state, &task.id)?;
    let prompt = prompts::integrate(&base, mode_of(&state, &task)?, &ctxs);
    state
        .integrating
        .lock()
        .expect("integrating lock")
        .insert(task.id.clone(), ctxs);
    state.integration_aborts.lock().expect("aborts lock").remove(&task.id);
    send(&app, &state, &task, prompt)
}

/// Stores the base branch the developer confirmed.
#[tauri::command]
#[specta::specta]
pub fn overlap_set_base(app: AppHandle, state: State<'_, AppState>, task_id: TaskId, branch: String) -> Result<Task> {
    let task = task_of(&state, &task_id)?;
    let branch = branch.trim().to_owned();
    if !valid_branch(&branch) {
        return Err(Error::Invalid(format!("“{branch}” is not a branch name.")));
    }
    observation::update_task(&app, &state, &task.id, |t| t.base_branch = Some(branch))
}

/// The base to suggest: the stored one, else the home repo's default branch (`origin/HEAD`, `main`, `master`).
#[tauri::command]
#[specta::specta]
pub async fn overlap_base_suggestion(app: AppHandle, task_id: TaskId) -> Result<Option<String>> {
    let state = app.state::<AppState>();
    let (task, home) = observe::task_and_home(&state, &TaskId::parse(task_id.as_str())?)?;
    if let Some(stored) = task.base_branch.filter(|b| valid_branch(b)) {
        return Ok(Some(stored));
    }
    git::default_branch(&home).await
}

/// **Resume** on the merge hand-back: continue the integration with the developer's instructions.
#[tauri::command]
#[specta::specta]
pub fn overlap_resume(app: AppHandle, state: State<'_, AppState>, task_id: TaskId, instruction: String) -> Result<()> {
    let task = task_of(&state, &task_id)?;
    require_integrating(&state, &task)?;
    if instruction.trim().is_empty() {
        return Err(Error::Invalid("Say how the integration should continue.".into()));
    }
    let prompt = resume_prompt(&instruction, mode_of(&state, &task)?);
    send(&app, &state, &task, prompt)
}

/// **Abort** on the merge hand-back: undo the merge or rebase; its sentinel records no integration.
#[tauri::command]
#[specta::specta]
pub fn overlap_abort(app: AppHandle, state: State<'_, AppState>, task_id: TaskId) -> Result<()> {
    let task = task_of(&state, &task_id)?;
    require_integrating(&state, &task)?;
    let base = (task.base_branch.clone().filter(|b| valid_branch(b))).unwrap_or_else(|| "the base branch".to_owned());
    let prompt = abort_prompt(&base, mode_of(&state, &task)?);
    state
        .integration_aborts
        .lock()
        .expect("aborts lock")
        .insert(task.id.clone());
    send(&app, &state, &task, prompt)
}

/// The files the integrate prompt listed, with the other tasks' intent; rebuilt from the overlap rows when the app
/// was relaunched since.
#[tauri::command]
#[specta::specta]
pub fn overlap_integration_context(state: State<'_, AppState>, task_id: TaskId) -> Result<Vec<ConflictCtx>> {
    let id = TaskId::parse(task_id.as_str())?;
    if let Some(ctxs) = state.integrating.lock().expect("integrating lock").get(&id) {
        return Ok(ctxs.clone());
    }
    contexts(&state, &id)
}
