//! The `sessions` table: one row per started session, kept after it exits so `sessions_get` can still describe it.
//! `attached_to` is live state only and never stored.

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{enum_from, enum_str, parse_time, ts};
use crate::domain::{Session, SessionId, TaskId};
use crate::Result;

pub fn insert(conn: &Connection, s: &Session) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, task_id, kind, cwd, driver, started_at, exited) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            s.id.as_str(),
            s.task_id.as_ref().map(|t| t.as_str()),
            enum_str(&s.kind)?,
            s.cwd.to_string_lossy(),
            enum_str(&s.driver)?,
            ts(&s.started_at),
            s.exited,
        ],
    )?;
    Ok(())
}

/// `id, task_id, kind, cwd, driver, started_at, exited` as stored.
type Columns = (String, Option<String>, String, String, String, String, Option<i32>);

fn from_row(r: &Row) -> rusqlite::Result<Columns> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
    ))
}

pub fn get(conn: &Connection, id: &SessionId) -> Result<Option<Session>> {
    let row = conn
        .query_row(
            "SELECT id, task_id, kind, cwd, driver, started_at, exited FROM sessions WHERE id = ?1",
            [id.as_str()],
            from_row,
        )
        .optional()?;
    let Some((id, task_id, kind, cwd, driver, started_at, exited)) = row else {
        return Ok(None);
    };
    Ok(Some(Session {
        id: SessionId::parse(&id)?,
        task_id: task_id.map(|t| TaskId::parse(&t)).transpose()?,
        kind: enum_from(&kind)?,
        cwd: cwd.into(),
        driver: enum_from(&driver)?,
        attached_to: None,
        started_at: parse_time(&started_at)?,
        exited,
    }))
}

pub fn mark_exited(conn: &Connection, id: &SessionId, code: i32) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET exited = ?2 WHERE id = ?1",
        params![id.as_str(), code],
    )?;
    Ok(())
}

/// Every row still marked running gets `code`; an Owned PTY cannot outlive the app, so the startup sweep calls this
/// with -1. Returns how many rows changed.
pub fn mark_all_exited(conn: &Connection, code: i32) -> Result<usize> {
    Ok(conn.execute("UPDATE sessions SET exited = ?1 WHERE exited IS NULL", [code])?)
}

/// A row whose spawn failed.
pub fn delete(conn: &Connection, id: &SessionId) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", [id.as_str()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::{AgentKind, DriverKind, Session, SessionId};

    fn session() -> Session {
        Session {
            id: SessionId::new(),
            task_id: None,
            kind: AgentKind::Shell,
            cwd: "/repo".into(),
            driver: DriverKind::Owned,
            attached_to: None,
            started_at: Utc::now(),
            exited: None,
        }
    }

    #[test]
    fn insert_get_and_mark_exited_round_trip() {
        let conn = open_in_memory().unwrap();
        let s = session();
        insert(&conn, &s).unwrap();
        assert_eq!(get(&conn, &s.id).unwrap(), Some(s.clone()));
        mark_exited(&conn, &s.id, 130).unwrap();
        assert_eq!(get(&conn, &s.id).unwrap().unwrap().exited, Some(130));
        delete(&conn, &s.id).unwrap();
        assert_eq!(get(&conn, &s.id).unwrap(), None);
    }

    #[test]
    fn mark_all_exited_touches_only_running_rows() {
        let conn = open_in_memory().unwrap();
        let (a, b) = (session(), session());
        insert(&conn, &a).unwrap();
        insert(&conn, &b).unwrap();
        mark_exited(&conn, &b.id, 0).unwrap();
        assert_eq!(mark_all_exited(&conn, -1).unwrap(), 1);
        assert_eq!(get(&conn, &a.id).unwrap().unwrap().exited, Some(-1));
        assert_eq!(get(&conn, &b.id).unwrap().unwrap().exited, Some(0));
    }
}
