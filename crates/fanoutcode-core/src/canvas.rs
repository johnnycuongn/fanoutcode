//! Saved Team-canvas positions per task (§10.7 "Live layout": a hand-arranged canvas comes back the same).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::domain::TaskId;
use crate::{Error, Result};

/// The longest node id stored; a file node id is `file:<absolute path>`.
pub const MAX_NODE_ID_LEN: usize = 1024;

/// Where one canvas node sits; `node_id` is the webview's `agent:<id>`, `step:<n>` or `file:<path>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CanvasPosition {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

/// The task's saved positions, ordered by node id.
pub fn positions_load(conn: &Connection, task_id: &TaskId) -> Result<Vec<CanvasPosition>> {
    let mut stmt = conn.prepare("SELECT node_id, x, y FROM canvas_positions WHERE task_id = ?1 ORDER BY node_id")?;
    let rows = stmt.query_map(params![task_id.as_str()], |r| {
        Ok(CanvasPosition {
            node_id: r.get(0)?,
            x: r.get(1)?,
            y: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Replaces every saved position of the task in one transaction. Validation runs first, so a rejected save leaves
/// the table untouched.
pub fn positions_save(conn: &Connection, task_id: &TaskId, positions: &[CanvasPosition]) -> Result<()> {
    for p in positions {
        if p.node_id.is_empty() || p.node_id.len() > MAX_NODE_ID_LEN {
            return Err(Error::Invalid("A canvas node id is empty or too long.".to_owned()));
        }
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(Error::Invalid("A canvas position is not a finite number.".to_owned()));
        }
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM canvas_positions WHERE task_id = ?1",
        params![task_id.as_str()],
    )?;
    {
        let mut insert =
            tx.prepare("INSERT OR REPLACE INTO canvas_positions (task_id, node_id, x, y) VALUES (?1, ?2, ?3, ?4)")?;
        for p in positions {
            insert.execute(params![task_id.as_str(), p.node_id, p.x, p.y])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::test_support::{insert_project, insert_task};

    fn conn() -> rusqlite::Connection {
        let c = db::open_in_memory().unwrap();
        insert_project(&c, "p-1");
        insert_task(&c, "t-0c4", "p-1");
        c
    }
    fn id() -> TaskId {
        TaskId::parse("t-0c4").unwrap()
    }
    fn p(node_id: &str, x: f64, y: f64) -> CanvasPosition {
        CanvasPosition {
            node_id: node_id.into(),
            x,
            y,
        }
    }

    #[test]
    fn positions_round_trip() {
        let c = conn();
        positions_save(&c, &id(), &[p("step:1", 10.0, 20.0), p("agent:main", 0.5, -3.0)]).unwrap();
        assert_eq!(
            positions_load(&c, &id()).unwrap(),
            vec![p("agent:main", 0.5, -3.0), p("step:1", 10.0, 20.0)]
        );
    }
    #[test]
    fn save_replaces_previous_rows() {
        let c = conn();
        positions_save(&c, &id(), &[p("step:1", 0.0, 0.0), p("step:2", 0.0, 0.0)]).unwrap();
        positions_save(&c, &id(), &[p("step:1", 1.0, 1.0)]).unwrap();
        assert_eq!(positions_load(&c, &id()).unwrap(), vec![p("step:1", 1.0, 1.0)]);
    }
    #[test]
    fn load_unknown_task_is_empty() {
        assert!(positions_load(&conn(), &id()).unwrap().is_empty());
    }
    #[test]
    fn rejects_empty_node_id_and_non_finite_coordinates() {
        let c = conn();
        assert!(matches!(
            positions_save(&c, &id(), &[p("", 0.0, 0.0)]),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            positions_save(&c, &id(), &[p("step:1", f64::NAN, 0.0)]),
            Err(Error::Invalid(_))
        ));
        assert!(positions_load(&c, &id()).unwrap().is_empty());
    }
}
