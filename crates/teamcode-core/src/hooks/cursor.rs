//! The `event_cursors` table: how far into each task's event log the tailer has read, so no line is parsed twice.

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::TaskId;
use crate::Result;

/// The byte offset just past the last complete line read, and that line's number.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventCursor {
    pub byte_offset: u64,
    pub seq: u64,
}

impl EventCursor {
    /// The saved cursor; the start of the file when there is none.
    pub fn load(conn: &Connection, task_id: &TaskId) -> Result<EventCursor> {
        Ok(conn
            .query_row(
                "SELECT byte_offset, seq FROM event_cursors WHERE task_id = ?1",
                [task_id.as_str()],
                |r| {
                    Ok(EventCursor {
                        byte_offset: r.get::<_, i64>(0)? as u64,
                        seq: r.get::<_, i64>(1)? as u64,
                    })
                },
            )
            .optional()?
            .unwrap_or_default())
    }

    pub fn save(&self, conn: &Connection, task_id: &TaskId) -> Result<()> {
        conn.execute(
            "INSERT INTO event_cursors (task_id, byte_offset, seq) VALUES (?1, ?2, ?3)
             ON CONFLICT (task_id) DO UPDATE SET byte_offset = excluded.byte_offset, seq = excluded.seq",
            params![task_id.as_str(), self.byte_offset as i64, self.seq as i64],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    #[test]
    fn cursor_resumes_after_the_last_complete_line() {
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        assert_eq!(
            EventCursor::load(&conn, &tid()).unwrap(),
            EventCursor { byte_offset: 0, seq: 0 }
        );
        EventCursor {
            byte_offset: 120,
            seq: 3,
        }
        .save(&conn, &tid())
        .unwrap();
        assert_eq!(
            EventCursor::load(&conn, &tid()).unwrap(),
            EventCursor {
                byte_offset: 120,
                seq: 3
            }
        );
    }
}
