//! The SQL behind `WorkspaceStore`: the `workspaces` and `workspace_projects` rows and the workspace-scope
//! settings rows the file mirrors.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{enum_from, enum_str};
use crate::domain::{LinkPolicy, ProjectId, Workspace, WorkspaceId, WorkspaceProject};
use crate::{Error, Result};

/// The settings key the file never carries: the Jira site names a keychain account on this machine.
pub const UNMIRRORED_SETTING: &str = "jira";

pub fn insert(
    conn: &Connection,
    id: &WorkspaceId,
    name: &str,
    file_path: Option<&Path>,
    policy: LinkPolicy,
) -> Result<()> {
    conn.execute(
        "INSERT INTO workspaces (id, name, file_path, policy, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id.as_str(),
            name,
            file_path.map(|p| p.to_string_lossy().into_owned()),
            enum_str(&policy)?,
            chrono::Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

pub fn exists(conn: &Connection, id: &WorkspaceId) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM workspaces WHERE id = ?1)",
        [id.as_str()],
        |r| r.get(0),
    )?)
}

/// The record, or `Error::Invalid("Unknown workspace.")`.
pub fn get(conn: &Connection, id: &WorkspaceId) -> Result<Workspace> {
    let row: Option<(String, Option<String>, bool, String)> = conn
        .query_row(
            "SELECT name, file_path, dirty, policy FROM workspaces WHERE id = ?1",
            [id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let (name, file_path, dirty, policy) = row.ok_or_else(|| Error::Invalid("Unknown workspace.".into()))?;
    Ok(Workspace {
        id: id.clone(),
        name,
        file_path: file_path.map(PathBuf::from),
        dirty,
        projects: members(conn, id)?,
        policy: enum_from(&policy)?,
    })
}

pub fn list_ids(conn: &Connection) -> Result<Vec<WorkspaceId>> {
    let mut stmt = conn.prepare("SELECT id FROM workspaces ORDER BY name, id")?;
    let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
    ids.map(|id| WorkspaceId::parse(&id?)).collect()
}

fn members(conn: &Connection, id: &WorkspaceId) -> Result<Vec<WorkspaceProject>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, \"order\", protected FROM workspace_projects WHERE workspace_id = ?1 ORDER BY \"order\"",
    )?;
    let rows = stmt.query_map([id.as_str()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, bool>(2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (project_id, order, protected) = row?;
        out.push(WorkspaceProject {
            project_id: ProjectId::parse(&project_id)?,
            order,
            protected,
        });
    }
    Ok(out)
}

/// Updates one text-ish column of the row and bumps `updated_at`.
pub fn set_column(conn: &Connection, id: &WorkspaceId, column: &str, value: &dyn rusqlite::ToSql) -> Result<()> {
    debug_assert!(["name", "file_path", "dirty", "policy", "fragments_json", "agents_json"].contains(&column));
    conn.execute(
        &format!("UPDATE workspaces SET {column} = ?1, updated_at = ?2 WHERE id = ?3"),
        params![value, chrono::Utc::now().to_rfc3339(), id.as_str()],
    )?;
    Ok(())
}

/// Replaces the member rows with `members` (order and protection as given).
pub fn set_members(conn: &Connection, id: &WorkspaceId, members: &[WorkspaceProject]) -> Result<()> {
    conn.execute("DELETE FROM workspace_projects WHERE workspace_id = ?1", [id.as_str()])?;
    for m in members {
        conn.execute(
            "INSERT INTO workspace_projects (workspace_id, project_id, \"order\", protected) VALUES (?1, ?2, ?3, ?4)",
            params![id.as_str(), m.project_id.as_str(), m.order, m.protected],
        )?;
    }
    Ok(())
}

/// A JSON map column (`fragments_json`, `agents_json`).
pub fn map_column(conn: &Connection, id: &WorkspaceId, column: &str) -> Result<BTreeMap<String, String>> {
    debug_assert!(["fragments_json", "agents_json"].contains(&column));
    let json: String = conn
        .query_row(
            &format!("SELECT {column} FROM workspaces WHERE id = ?1"),
            [id.as_str()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::Invalid("Unknown workspace.".into()))?;
    Ok(serde_json::from_str(&json).unwrap_or_default())
}

pub fn set_map_column(conn: &Connection, id: &WorkspaceId, column: &str, map: &BTreeMap<String, String>) -> Result<()> {
    set_column(conn, id, column, &serde_json::to_string(map)?)
}

/// The workspace-scope settings rows, minus `jira`.
pub fn settings(conn: &Connection, id: &WorkspaceId) -> Result<BTreeMap<String, serde_json::Value>> {
    let mut stmt = conn.prepare("SELECT key, json FROM settings WHERE scope_kind = 'workspace' AND scope_id = ?1")?;
    let rows = stmt.query_map([id.as_str()], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (key, json) = row?;
        if key == UNMIRRORED_SETTING {
            continue;
        }
        if let Ok(v) = serde_json::from_str(&json) {
            out.insert(key, v);
        }
    }
    Ok(out)
}

/// Upserts each row; `null` removes it. `jira` is never written from here.
pub fn upsert_settings(
    conn: &Connection,
    id: &WorkspaceId,
    settings: &BTreeMap<String, serde_json::Value>,
) -> Result<()> {
    use crate::settings::ScopeKind;
    for (key, value) in settings.iter().filter(|(k, _)| *k != UNMIRRORED_SETTING) {
        crate::db::settings::set(conn, ScopeKind::Workspace, id.as_str(), key, value)?;
    }
    Ok(())
}
