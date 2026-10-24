//! The `settings` table: one row per `(scope_kind, scope_id, key)` holding the value as JSON (ADR 0002).

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::settings::{Scope, ScopeKind, ScopeValues};
use crate::Result;

fn layer(conn: &Connection, kind: ScopeKind, id: &str) -> Result<HashMap<String, Value>> {
    let mut stmt = conn.prepare("SELECT key, json FROM settings WHERE scope_kind = ?1 AND scope_id = ?2")?;
    let rows = stmt.query_map([kind.as_str(), id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let (key, json) = row?;
        // a row that does not parse is skipped, so resolution falls through to the next layer
        if let Ok(v) = serde_json::from_str(&json) {
            out.insert(key, v);
        }
    }
    Ok(out)
}

/// The app rows, and the rows of each layer whose id is present in `scope`.
pub fn load(conn: &Connection, scope: &Scope) -> Result<ScopeValues> {
    let opt = |kind, id: Option<&str>| id.map_or_else(|| Ok(HashMap::new()), |id| layer(conn, kind, id));
    Ok(ScopeValues {
        app: layer(conn, ScopeKind::App, "")?,
        workspace: opt(ScopeKind::Workspace, scope.workspace.as_ref().map(|w| w.as_str()))?,
        project: opt(ScopeKind::Project, scope.project.as_ref().map(|p| p.as_str()))?,
        task: opt(ScopeKind::Task, scope.task.as_ref().map(|t| t.as_str()))?,
    })
}

/// Upserts the row; `Value::Null` deletes it (the layer then inherits).
pub fn set(conn: &Connection, kind: ScopeKind, id: &str, key: &str, value: &Value) -> Result<()> {
    if value.is_null() {
        conn.execute(
            "DELETE FROM settings WHERE scope_kind = ?1 AND scope_id = ?2 AND key = ?3",
            params![kind.as_str(), id, key],
        )?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO settings (scope_kind, scope_id, key, json) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (scope_kind, scope_id, key) DO UPDATE SET json = excluded.json",
        params![kind.as_str(), id, key, value.to_string()],
    )?;
    Ok(())
}

pub fn get_raw(conn: &Connection, kind: ScopeKind, id: &str, key: &str) -> Result<Option<Value>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT json FROM settings WHERE scope_kind = ?1 AND scope_id = ?2 AND key = ?3",
            params![kind.as_str(), id, key],
            |r| r.get(0),
        )
        .optional()?;
    Ok(json.and_then(|j| serde_json::from_str(&j).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ProjectId;
    use crate::test_support::test_conn;

    #[test]
    fn settings_rows_load_per_layer_and_null_deletes() {
        let c = test_conn();
        set(&c, ScopeKind::App, "", "detail_level", &serde_json::json!("Medium")).unwrap();
        set(&c, ScopeKind::Project, "p-1a-1", "may_commit", &serde_json::json!(true)).unwrap();
        set(&c, ScopeKind::Project, "p-1a-2", "may_commit", &serde_json::json!(true)).unwrap();
        let scope = Scope {
            workspace: None,
            project: Some(ProjectId::parse("p-1a-1").unwrap()),
            task: None,
        };
        let v = load(&c, &scope).unwrap();
        assert_eq!(v.app["detail_level"], "Medium");
        assert_eq!(v.project["may_commit"], true);
        assert!(v.workspace.is_empty());
        set(&c, ScopeKind::App, "", "detail_level", &serde_json::Value::Null).unwrap();
        assert_eq!(get_raw(&c, ScopeKind::App, "", "detail_level").unwrap(), None);
    }
}
