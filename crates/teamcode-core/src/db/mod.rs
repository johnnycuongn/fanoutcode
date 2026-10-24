//! SQLite at `<app_data>/teamcode.db`, migrated at open from `migrations/00NN_<name>.sql` (NN = owning plan, A8).

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::domain::{
    Container, LinkPolicy, MemoryLocation, Project, ProjectId, ThemePreference, Workspace, WorkspaceId,
    WorkspaceProject, WorktreeConvention,
};
use crate::{Error, Result};

pub mod projects;
pub mod settings;
pub mod tasks;
pub mod timeline;
pub mod views;

/// `(version, name, sql)`, ascending. Versions have gaps by design (the owning plan's number); each plan adds its
/// file and one line here. An applied file is never edited.
pub const MIGRATIONS: &[(u32, &str, &str)] = &[
    (1, "initial", include_str!("../../migrations/0001_initial.sql")),
    (3, "core", include_str!("../../migrations/0003_core.sql")),
];

/// Opens (creating parent folders on first launch), enables WAL and foreign keys, then migrates.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// A migrated in-memory database, for tests in every plan.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

/// Applies every entry of `MIGRATIONS` absent from `schema_migrations`; returns the current version.
pub fn migrate(conn: &mut Connection) -> Result<u32> {
    apply(conn, MIGRATIONS)
}

/// Each pending file runs in its own transaction with its `schema_migrations` row, so a failing file leaves the
/// database at the previous version with nothing from that file applied.
pub(crate) fn apply(conn: &mut Connection, list: &[(u32, &str, &str)]) -> Result<u32> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL)",
    )?;
    for (version, name, sql) in list {
        let applied: Option<u32> = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1",
                [version],
                |r| r.get(0),
            )
            .optional()?;
        if applied.is_some() {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .map_err(|e| Error::Db(format!("Migration {version:04}_{name} failed: {e}")))?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![version, name, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }
    user_version(conn)
}

/// The highest applied migration version; 0 when nothing is applied.
pub fn user_version(conn: &Connection) -> Result<u32> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(0);
    }
    Ok(
        conn.query_row("SELECT coalesce(max(version), 0) FROM schema_migrations", [], |r| {
            r.get(0)
        })?,
    )
}

/// An app-scope setting row (`scope_kind = 'app'`, `scope_id = ''`); `None` when unset.
pub fn app_setting<T: DeserializeOwned>(conn: &Connection, key: &str) -> Result<Option<T>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT json FROM settings WHERE scope_kind = 'app' AND scope_id = '' AND key = ?1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    json.map(|j| serde_json::from_str(&j).map_err(Error::from)).transpose()
}

/// Upserts an app-scope setting row.
pub fn set_app_setting<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (scope_kind, scope_id, key, json) VALUES ('app', '', ?1, ?2)
         ON CONFLICT (scope_kind, scope_id, key) DO UPDATE SET json = excluded.json",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

/// True until `first_run_complete` is `true`.
pub fn first_run_pending(conn: &Connection) -> Result<bool> {
    Ok(app_setting::<bool>(conn, "first_run_complete")? != Some(true))
}

/// Records the two first-run answers (§4.4) and marks first run complete, through the settings rows.
pub fn complete_first_run(conn: &Connection, convention: WorktreeConvention, theme: ThemePreference) -> Result<()> {
    use crate::settings::ScopeKind::App;
    let tx = conn.unchecked_transaction()?;
    settings::set(&tx, App, "", "worktree_convention", &serde_json::to_value(convention)?)?;
    settings::set(&tx, App, "", "theme", &serde_json::to_value(theme)?)?;
    settings::set(&tx, App, "", "first_run_complete", &serde_json::Value::Bool(true))?;
    tx.commit()?;
    Ok(())
}

/// A `projects` or `workspaces` row with the container's id exists.
pub fn container_exists(conn: &Connection, container: &Container) -> Result<bool> {
    let (table, id) = match container {
        Container::Project { id } => ("projects", id.as_str()),
        Container::Workspace { id } => ("workspaces", id.as_str()),
    };
    Ok(conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE id = ?1)"),
        [id],
        |r| r.get(0),
    )?)
}

/// Timestamps in columns: RFC 3339 with nanoseconds and `Z`, fixed width so text order is time order.
pub(crate) fn ts(t: &chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

/// A unit enum's serde string, the Rust variant name (`"Spec"`), for a text column (A16).
pub(crate) fn enum_str<T: Serialize>(v: &T) -> Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(Error::Db(format!("Not a unit enum: {other}"))),
    }
}

/// The inverse of `enum_str`.
pub(crate) fn enum_from<T: DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|e| Error::Db(format!("Bad stored value {s:?}: {e}")))
}

pub(crate) fn parse_time(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|e| Error::Db(format!("Bad timestamp {s:?}: {e}")))
}

/// The project rooted at `root`, created on first open: named after the folder, given the least-used project colour
/// (`projects::next_color`), memory in app storage. The caller canonicalises `root`.
pub fn find_or_create_project(conn: &Connection, root: &Path) -> Result<Project> {
    if !root.is_dir() {
        return Err(Error::Precondition(
            "That folder no longer exists. Relocate or remove it.".into(),
        ));
    }
    if let Some(existing) = projects::find_by_root(conn, root)? {
        return Ok(existing);
    }
    let project = Project {
        id: ProjectId::new(),
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
        root: root.to_owned(),
        color: projects::next_color(conn)?,
        memory_location: MemoryLocation::AppStorage,
        created_at: chrono::Utc::now(),
    };
    projects::insert(conn, &project)?;
    Ok(project)
}

/// Upserts the workspace's `name` and `file_path`; its projects come from `workspace_projects` (Plan 10 fills them
/// from the rest of the file).
pub fn find_or_create_workspace(
    conn: &Connection,
    id: &WorkspaceId,
    name: &str,
    file_path: &Path,
) -> Result<Workspace> {
    conn.execute(
        "INSERT INTO workspaces (id, name, file_path, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, file_path = excluded.file_path,
             updated_at = excluded.updated_at",
        params![
            id.as_str(),
            name,
            file_path.to_string_lossy(),
            chrono::Utc::now().to_rfc3339()
        ],
    )?;
    let policy: String = conn.query_row("SELECT policy FROM workspaces WHERE id = ?1", [id.as_str()], |r| {
        r.get(0)
    })?;
    let mut stmt = conn.prepare(
        "SELECT project_id, \"order\", protected FROM workspace_projects WHERE workspace_id = ?1 ORDER BY \"order\"",
    )?;
    let rows = stmt.query_map([id.as_str()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, bool>(2)?))
    })?;
    let mut projects = Vec::new();
    for row in rows {
        let (project_id, order, protected) = row?;
        projects.push(WorkspaceProject {
            project_id: ProjectId::parse(&project_id)?,
            order,
            protected,
        });
    }
    Ok(Workspace {
        id: id.clone(),
        name: name.to_owned(),
        file_path: Some(file_path.to_owned()),
        dirty: false,
        projects,
        policy: serde_json::from_value::<LinkPolicy>(serde_json::Value::String(policy))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Container, ProjectId, ThemePreference, WorkspaceId, WorktreeConvention};

    const TABLES: [&str; 11] = [
        "projects",
        "workspaces",
        "workspace_projects",
        "tasks",
        "task_timeline",
        "sessions",
        "views",
        "settings",
        "cost_buckets",
        "jira_links",
        "canvas_positions",
    ];

    #[test]
    fn migrations_apply_twice_without_error() {
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        migrate(&mut c).unwrap();
        migrate(&mut c).unwrap();
        let cols: Vec<String> = c
            .prepare("PRAGMA table_info(tasks)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for want in [
            "stage",
            "state",
            "memory_dir",
            "json",
            "stage_entered_at",
            "state_entered_at",
            "links_bound_at",
        ] {
            assert!(cols.iter().any(|c| c == want), "{want}");
        }
    }

    #[test]
    fn open_creates_parent_folders_and_every_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("deep/er/teamcode.db")).unwrap();
        for t in TABLES {
            conn.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get::<_, i64>(0))
                .unwrap_or_else(|e| panic!("{t}: {e}"));
        }
        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.last().unwrap().0);
    }

    #[test]
    fn reopening_applies_nothing_and_foreign_keys_are_on() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("teamcode.db");
        open(&path).unwrap();
        let conn = open(&path).unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            MIGRATIONS.len() as i64
        );
        assert_eq!(
            conn.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn migrations_are_numbered_by_owning_plan_and_strictly_increase() {
        assert_eq!((MIGRATIONS[0].0, MIGRATIONS[0].1), (1, "initial"));
        for w in MIGRATIONS.windows(2) {
            assert!(w[0].0 < w[1].0, "{} then {}", w[0].0, w[1].0);
        }
        for (v, name, _) in MIGRATIONS {
            assert!(
                *v >= 1 && !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{v}_{name}"
            );
        }
    }

    #[test]
    fn a_gap_in_versions_still_applies_the_later_file() {
        let mut conn = open_in_memory().unwrap();
        let later = [(5u32, "later", "CREATE TABLE later (id INTEGER);")];
        assert_eq!(apply(&mut conn, &later).unwrap(), 5);
        assert_eq!(apply(&mut conn, &later).unwrap(), 5, "re-running applies nothing");
        conn.query_row("SELECT count(*) FROM later", [], |r| r.get::<_, i64>(0))
            .unwrap();
    }

    #[test]
    fn a_failing_migration_rolls_back_whole() {
        let mut conn = open_in_memory().unwrap();
        let broken = [(99u32, "broken", "CREATE TABLE half (id INTEGER); THIS IS NOT SQL;")];
        assert!(apply(&mut conn, &broken).is_err());
        assert!(
            conn.query_row("SELECT count(*) FROM half", [], |r| r.get::<_, i64>(0))
                .is_err(),
            "nothing from the failed file stays"
        );
        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.last().unwrap().0);
    }

    #[test]
    fn app_settings_upsert_and_round_trip() {
        let conn = open_in_memory().unwrap();
        assert_eq!(app_setting::<bool>(&conn, "first_run_complete").unwrap(), None);
        set_app_setting(&conn, "theme", &ThemePreference::Dark).unwrap();
        set_app_setting(&conn, "theme", &ThemePreference::Light).unwrap();
        assert_eq!(
            app_setting::<ThemePreference>(&conn, "theme").unwrap(),
            Some(ThemePreference::Light)
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM settings WHERE scope_kind='app'", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn first_run_is_pending_until_completed() {
        let conn = open_in_memory().unwrap();
        assert!(first_run_pending(&conn).unwrap());
        complete_first_run(&conn, WorktreeConvention::TeamCodeManaged, ThemePreference::System).unwrap();
        assert!(!first_run_pending(&conn).unwrap());
        assert_eq!(
            app_setting::<WorktreeConvention>(&conn, "worktree_convention").unwrap(),
            Some(WorktreeConvention::TeamCodeManaged)
        );
    }

    #[test]
    fn container_exists_is_false_for_unknown_ids() {
        let conn = open_in_memory().unwrap();
        assert!(!container_exists(
            &conn,
            &Container::Project {
                id: ProjectId::parse("p-404").unwrap()
            }
        )
        .unwrap());
    }

    #[test]
    fn find_or_create_project_is_idempotent_by_root_and_picks_the_least_used_colour() {
        let conn = open_in_memory().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let p1 = find_or_create_project(&conn, a.path()).unwrap();
        let again = find_or_create_project(&conn, a.path()).unwrap();
        assert_eq!(p1, again);
        assert_eq!(p1.name, a.path().file_name().unwrap().to_string_lossy());
        assert_eq!(p1.color.0, "indigo");
        assert_eq!(find_or_create_project(&conn, b.path()).unwrap().color.0, "cyan");
        assert!(matches!(
            find_or_create_project(&conn, Path::new("/nonexistent/repo")),
            Err(Error::Precondition(_))
        ));
        assert!(container_exists(&conn, &Container::Project { id: p1.id }).unwrap());
    }

    #[test]
    fn find_or_create_workspace_upserts_name_and_path() {
        let conn = open_in_memory().unwrap();
        let id = WorkspaceId::parse("ws-1").unwrap();
        let w = find_or_create_workspace(&conn, &id, "Acme", Path::new("/a/acme.teamcode-workspace.json")).unwrap();
        let w2 = find_or_create_workspace(&conn, &id, "Acme 2", Path::new("/b/acme.teamcode-workspace.json")).unwrap();
        assert_eq!(w.id, w2.id);
        assert_eq!(w2.name, "Acme 2");
        assert_eq!(
            w2.file_path.as_deref(),
            Some(Path::new("/b/acme.teamcode-workspace.json"))
        );
        assert!(container_exists(&conn, &Container::Workspace { id }).unwrap());
    }
}
