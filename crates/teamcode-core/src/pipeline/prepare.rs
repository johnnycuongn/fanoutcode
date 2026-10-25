//! The first half of every stage action: read the task, build its transition context and compute the effects.
//! When those effects start the task's first session, its links are bound first (spec §4.1) and the effects are
//! computed again, so the first stage prompt already carries the Linked projects section (§4.3, §6.8).

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::{ctx, Effect, TestRun, TransitionCtx};
use crate::domain::{Project, Task, TaskId};
use crate::{db, workspace, Error, Result};

/// The task the effects were computed for (bound when this action binds it), the effects, and whether a linked
/// project still lacks a brief (the shell then runs the brief helper).
#[derive(Debug)]
pub struct Prepared {
    pub task: Task,
    pub effects: Vec<Effect>,
    pub needs_briefs: bool,
}

fn project_of(conn: &Connection, task: &Task) -> Result<Project> {
    db::projects::get(conn, &task.project_id)?
        .ok_or_else(|| Error::Invalid(format!("Unknown project {}", task.project_id)))
}

/// Computes `compute`'s effects for task `id`. Call with the database lock held, then `apply` the result.
pub fn prepare_action(
    conn: &Connection,
    id: &TaskId,
    app_config: &Path,
    app_data: &Path,
    now: DateTime<Utc>,
    last_test: Option<TestRun>,
    compute: impl Fn(&Task, &TransitionCtx) -> Result<Vec<Effect>>,
) -> Result<Prepared> {
    let task = db::tasks::require(conn, id)?;
    let project = project_of(conn, &task)?;
    let build =
        |task: &Task| ctx::build_transition_ctx(conn, task, &project, app_config, app_data, now, last_test.clone());
    let effects = compute(&task, &build(&task)?)?;
    let starts = effects.iter().any(|e| matches!(e, Effect::StartSession));
    if task.links_bound_at.is_some() || !starts {
        return Ok(Prepared {
            task,
            effects,
            needs_briefs: false,
        });
    }
    let (task, needs_briefs) = workspace::launch::bind_if_first_start(conn, app_data, id, None)?;
    let effects = compute(&task, &build(&task)?)?;
    Ok(Prepared {
        task,
        effects,
        needs_briefs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{transition, StageAction};
    use crate::workspace::store::WorkspaceStore;
    use crate::workspace::test_support::fixture;

    fn injected(effects: &[Effect]) -> &str {
        effects
            .iter()
            .find_map(|e| match e {
                Effect::InjectPrompt(p) => Some(p.as_str()),
                _ => None,
            })
            .expect("a prompt")
    }

    #[test]
    fn the_first_plan_prompt_of_a_workspace_task_lists_its_linked_projects() {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let ws = s.create("Shop", &[fx.api.id.clone(), fx.web.id.clone()]).unwrap();
        let conn = fx.db.lock().unwrap();
        let mut t = crate::test_support::task_fixture("t-1", fx.web.id.as_str());
        t.workspace_id = Some(ws.id.clone());
        db::tasks::insert(&conn, &t).unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let prepared = prepare_action(
            &conn,
            &t.id,
            cfg.path(),
            fx.app_data.path(),
            Utc::now(),
            None,
            |task, c| transition(task, StageAction::Approve, c),
        )
        .unwrap();
        assert!(prepared.task.links_bound_at.is_some() && prepared.needs_briefs);
        let plan = injected(&prepared.effects);
        assert!(plan.contains("## Linked projects"), "{plan}");
        assert!(plan.contains(&fx.api.root.display().to_string()), "{plan}");
        assert!(!plan.contains("This task has no linked projects"), "{plan}");
    }

    #[test]
    fn an_action_that_starts_no_session_leaves_the_links_unbound() {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let ws = s.create("Shop", &[fx.api.id.clone(), fx.web.id.clone()]).unwrap();
        let conn = fx.db.lock().unwrap();
        let mut t = crate::test_support::task_fixture("t-1", fx.web.id.as_str());
        t.workspace_id = Some(ws.id.clone());
        db::tasks::insert(&conn, &t).unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let prepared = prepare_action(
            &conn,
            &t.id,
            cfg.path(),
            fx.app_data.path(),
            Utc::now(),
            None,
            |task, c| transition(task, StageAction::Archive, c),
        )
        .unwrap();
        assert!(prepared.task.links_bound_at.is_none() && !prepared.needs_briefs);
    }
}
