//! The Test tab's commands (§6.5, A15). A run belongs to `AppState.test_runs`, not to the caller, so a closed
//! window never kills it; `tests_run` starts one or attaches to the one in flight, replaying what it printed.

use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use teamcode_core::domain::{ProjectId, TaskId};
use teamcode_core::settings::{keys, resolve, Scope};
use teamcode_core::testrun::{self, Attached, LiveOutput, TestCommand};
use teamcode_core::{db, pipeline, Error, Result};

use crate::events::{self, TestFinished};
use crate::observe::{self, log_err};
use crate::state::AppState;

/// The home command at the task's scope in the task's cwd, then the linked projects' commands added to it.
fn commands(state: &AppState, task_id: &TaskId) -> Result<Vec<TestCommand>> {
    let (task, home) = observe::task_and_home(state, task_id)?;
    let conn = state.db.lock().expect("db lock");
    let scope = pipeline::ctx::task_scope(&task);
    let values = db::settings::load(&conn, &scope)?;
    let Some(home_cmd) = resolve::<keys::TestCommand>(&values, &scope).filter(|c| !c.trim().is_empty()) else {
        return Err(Error::Precondition(
            "Set a test command in project settings before using the Test stage".into(),
        ));
    };
    let mut all = vec![TestCommand {
        cwd: task.worktree_path.clone().unwrap_or(home),
        command: home_cmd,
    }];
    all.extend(testrun::commands_for(&conn, &task.id)?);
    Ok(all)
}

/// The run in flight for the task, or a new one started now (inserted before the lock is released, so two calls
/// arriving together start one run).
pub(crate) fn start_or_attach(app: &AppHandle, task_id: &TaskId) -> Result<Arc<LiveOutput>> {
    let state = app.state::<AppState>();
    let mut runs = state.test_runs.lock().expect("test runs lock");
    if let Some((_, out)) = runs.get(task_id) {
        return Ok(out.clone());
    }
    let cmds = commands(&state, task_id)?;
    let out = LiveOutput::new();
    let (app2, id, out2) = (app.clone(), task_id.clone(), out.clone());
    let handle = tauri::async_runtime::spawn(async move {
        let result = testrun::run(&cmds, &mut |b| out2.push(&b)).await;
        let code = match result {
            Ok(r) => {
                let code = r.exit_code;
                app2.state::<AppState>()
                    .last_tests
                    .lock()
                    .expect("last test lock")
                    .insert(id.clone(), r);
                code
            }
            Err(e) => {
                out2.push(format!("{e}\n").as_bytes());
                -1
            }
        };
        finished(&app2, &id, code);
        app2.state::<AppState>()
            .test_runs
            .lock()
            .expect("test runs lock")
            .remove(&id);
        out2.finish(code);
    });
    runs.insert(task_id.clone(), (handle, out.clone()));
    Ok(out)
}

/// The run's end, once: `test-finished`, then the pipeline (§6.5: advance on green, else needs you).
fn finished(app: &AppHandle, id: &TaskId, code: i32) {
    log_err(
        "reporting the test result",
        events::emit(
            app,
            TestFinished {
                task_id: id.clone(),
                code,
            },
        ),
    );
    let state = app.state::<AppState>();
    log_err(
        "applying the test result",
        crate::commands::tasks::run(app, &state, id, |task, tctx| {
            pipeline::on_test_finished(task, code, tctx)
        }),
    );
}

/// Sends the replay, then every live chunk, until the run ends; a closed window ends only this caller's loop.
async fn forward(attached: Attached, on_output: &Channel<Vec<u8>>) -> i32 {
    let Attached {
        replay,
        mut live,
        mut done,
    } = attached;
    let mut open = replay.is_empty() || on_output.send(replay).is_ok();
    loop {
        tokio::select! {
            biased;
            chunk = live.recv() => match chunk {
                Some(c) => open = open && on_output.send(c).is_ok(),
                None => break,
            },
            _ = done.wait_for(|c| c.is_some()) => {
                while let Ok(c) = live.try_recv() {
                    open = open && on_output.send(c).is_ok();
                }
                break;
            }
        }
    }
    let code = done.borrow().unwrap_or(-1);
    code
}

/// Starts the task's tests, or attaches to the run in flight; streams its output and returns its exit code.
#[tauri::command]
#[specta::specta]
pub async fn tests_run(app: AppHandle, task_id: TaskId, on_output: Channel<Vec<u8>>) -> Result<i32> {
    let id = TaskId::parse(task_id.as_str())?;
    let out = start_or_attach(&app, &id)?;
    Ok(forward(out.attach(), &on_output).await)
}

/// Stops the run in flight (the child dies with it); attached callers return -1 and the card needs you.
#[tauri::command]
#[specta::specta]
pub fn tests_stop(app: AppHandle, state: State<'_, AppState>, task_id: TaskId) -> Result<()> {
    let id = TaskId::parse(task_id.as_str())?;
    let Some((handle, out)) = state.test_runs.lock().expect("test runs lock").remove(&id) else {
        return Ok(());
    };
    handle.abort();
    out.push(b"\nStopped.\n");
    finished(&app, &id, -1);
    out.finish(-1);
    Ok(())
}

/// Adds a linked project's test command (its own project-scope `test_command`, run in its root) to the task.
#[tauri::command]
#[specta::specta]
pub fn tests_add_linked(state: State<'_, AppState>, task_id: TaskId, project_id: ProjectId) -> Result<TestCommand> {
    let id = TaskId::parse(task_id.as_str())?;
    let pid = ProjectId::parse(project_id.as_str())?;
    let conn = state.db.lock().expect("db lock");
    let task = db::tasks::require(&conn, &id)?;
    let project = db::projects::get(&conn, &pid)?.ok_or_else(|| Error::Invalid("Unknown project id.".into()))?;
    let scope = Scope {
        project: Some(pid),
        ..Scope::default()
    };
    let command = resolve::<keys::TestCommand>(&db::settings::load(&conn, &scope)?, &scope);
    let c = testrun::linked_test_command(&task, &project, command.as_deref())?;
    testrun::save_command(&conn, &task.id, &c)?;
    Ok(c)
}
