//! The `tasks` table: the full `Task` in `json`, mirrored into the columns the queries filter and sort on.

use rusqlite::{params, Connection, OptionalExtension};

use super::{enum_str, ts};
use crate::domain::{Container, ProjectId, Task, TaskId, WorkspaceId};
use crate::{Error, Result};

/// Which tasks a container holds: a project's own, or those of every project in a workspace.
fn container_filter(c: &Container) -> (&'static str, &str) {
    match c {
        Container::Project { id } => ("project_id = ?1", id.as_str()),
        Container::Workspace { id } => (
            "project_id IN (SELECT project_id FROM workspace_projects WHERE workspace_id = ?1)",
            id.as_str(),
        ),
    }
}

fn task_from_json(json: String) -> Result<Task> {
    serde_json::from_str(&json).map_err(|e| Error::Db(format!("A task row does not parse: {e}")))
}

pub fn insert(conn: &Connection, t: &Task) -> Result<()> {
    conn.execute(
        "INSERT INTO tasks (id, project_id, workspace_id, title, stage, state, priority, flagged, hidden, archived,
             memory_dir, json, created_at, updated_at, stage_entered_at, state_entered_at, links_bound_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            t.id.as_str(),
            t.project_id.as_str(),
            t.workspace_id.as_ref().map(|w| w.as_str()),
            t.title,
            enum_str(&t.stage)?,
            serde_json::to_string(&t.state)?,
            enum_str(&t.priority)?,
            t.flagged,
            t.hidden,
            t.archived,
            t.memory_dir.to_string_lossy(),
            serde_json::to_string(t)?,
            ts(&t.created_at),
            ts(&t.updated_at),
            ts(&t.stage_entered_at),
            ts(&t.state_entered_at),
            t.links_bound_at.as_ref().map(ts),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &TaskId) -> Result<Option<Task>> {
    let json: Option<String> = conn
        .query_row("SELECT json FROM tasks WHERE id = ?1", [id.as_str()], |r| r.get(0))
        .optional()?;
    json.map(task_from_json).transpose()
}

/// `Invalid("Unknown task <id>")` when there is no such task.
pub fn require(conn: &Connection, id: &TaskId) -> Result<Task> {
    get(conn, id)?.ok_or_else(|| Error::Invalid(format!("Unknown task {id}")))
}

/// The container's tasks, archived included, oldest first.
pub fn list(conn: &Connection, c: &Container) -> Result<Vec<Task>> {
    let (filter, id) = container_filter(c);
    let mut stmt = conn.prepare(&format!(
        "SELECT json FROM tasks WHERE {filter} ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([id], |r| r.get::<_, String>(0))?;
    rows.map(|r| task_from_json(r?)).collect()
}

/// Rewrites the whole row from `t` (including `updated_at`); `Invalid` for an unknown id.
pub fn update(conn: &Connection, t: &Task) -> Result<()> {
    let n = conn.execute(
        "UPDATE tasks SET project_id = ?2, workspace_id = ?3, title = ?4, stage = ?5, state = ?6, priority = ?7,
             flagged = ?8, hidden = ?9, archived = ?10, memory_dir = ?11, json = ?12, created_at = ?13,
             updated_at = ?14, stage_entered_at = ?15, state_entered_at = ?16, links_bound_at = ?17
         WHERE id = ?1",
        params![
            t.id.as_str(),
            t.project_id.as_str(),
            t.workspace_id.as_ref().map(|w| w.as_str()),
            t.title,
            enum_str(&t.stage)?,
            serde_json::to_string(&t.state)?,
            enum_str(&t.priority)?,
            t.flagged,
            t.hidden,
            t.archived,
            t.memory_dir.to_string_lossy(),
            serde_json::to_string(t)?,
            ts(&t.created_at),
            ts(&t.updated_at),
            ts(&t.stage_entered_at),
            ts(&t.state_entered_at),
            t.links_bound_at.as_ref().map(ts),
        ],
    )?;
    if n == 0 {
        return Err(Error::Invalid(format!("Unknown task {}", t.id)));
    }
    Ok(())
}

/// Tasks in the container waiting on the developer, archived ones excluded.
pub fn needs_you_count(conn: &Connection, c: &Container) -> Result<u32> {
    let (filter, id) = container_filter(c);
    Ok(conn.query_row(
        &format!(
            "SELECT count(*) FROM tasks WHERE {filter} AND archived = 0 AND json_extract(state, '$.type') = 'NeedsYou'"
        ),
        [id],
        |r| r.get(0),
    )?)
}

pub fn workspace_contains(conn: &Connection, ws: &WorkspaceId, p: &ProjectId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM workspace_projects WHERE workspace_id = ?1 AND project_id = ?2)",
        [ws.as_str(), p.as_str()],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentState, HandbackReason, RepoBase, TaskColor};
    use crate::test_support::{insert_project, insert_workspace_with, task_fixture, test_conn};

    #[test]
    fn task_round_trips_every_field() {
        let c = test_conn();
        insert_project(&c, "p-1a-1");
        let mut t = task_fixture("t-1a-1", "p-1a-1");
        t.state = AgentState::NeedsYou {
            reason: HandbackReason::TestFailed { exit_code: 3 },
        };
        t.state_entered_at = "2026-10-24T05:20:00Z".parse().unwrap();
        t.links_bound_at = Some("2026-10-24T05:21:00Z".parse().unwrap());
        t.base = vec![RepoBase {
            repo_root: "/r".into(),
            commit: "abc".into(),
        }];
        t.linked_roots = vec!["/api".into()];
        t.color = Some(TaskColor("violet".into()));
        t.jira_key = Some("TC-1".into());
        insert(&c, &t).unwrap();
        assert_eq!(get(&c, &t.id).unwrap(), Some(t.clone()));
        let raw: String = c
            .query_row("SELECT state FROM tasks WHERE id = ?1", [t.id.as_str()], |r| r.get(0))
            .unwrap();
        assert_eq!(
            raw,
            r#"{"type":"NeedsYou","reason":{"type":"TestFailed","exit_code":3}}"#
        );
        assert!(matches!(
            require(&c, &TaskId::parse("t-ff-9").unwrap()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn list_by_project_and_by_workspace_membership() {
        let c = test_conn();
        insert_project(&c, "p-1a-1");
        insert_project(&c, "p-1a-2");
        insert_project(&c, "p-1a-3");
        insert_workspace_with(&c, "ws-1a-1", &["p-1a-1", "p-1a-2"]);
        for (t, p) in [("t-1a-1", "p-1a-1"), ("t-1a-2", "p-1a-2"), ("t-1a-3", "p-1a-3")] {
            insert(&c, &task_fixture(t, p)).unwrap();
        }
        assert_eq!(
            list(
                &c,
                &Container::Project {
                    id: ProjectId::parse("p-1a-1").unwrap()
                }
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            list(
                &c,
                &Container::Workspace {
                    id: WorkspaceId::parse("ws-1a-1").unwrap()
                }
            )
            .unwrap()
            .len(),
            2
        );
        let ws = WorkspaceId::parse("ws-1a-1").unwrap();
        assert!(workspace_contains(&c, &ws, &ProjectId::parse("p-1a-1").unwrap()).unwrap());
        assert!(!workspace_contains(&c, &ws, &ProjectId::parse("p-1a-3").unwrap()).unwrap());
    }

    #[test]
    fn needs_you_count_skips_archived() {
        let c = test_conn();
        insert_project(&c, "p-1a-1");
        let mut a = task_fixture("t-1a-1", "p-1a-1");
        a.state = AgentState::NeedsYou {
            reason: HandbackReason::StageDone,
        };
        insert(&c, &a).unwrap();
        let mut b = task_fixture("t-1a-2", "p-1a-1");
        b.state = AgentState::NeedsYou {
            reason: HandbackReason::PermissionNeeded,
        };
        b.archived = true;
        insert(&c, &b).unwrap();
        assert_eq!(
            needs_you_count(
                &c,
                &Container::Project {
                    id: a.project_id.clone()
                }
            )
            .unwrap(),
            1
        );
    }
}
