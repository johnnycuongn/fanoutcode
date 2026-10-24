//! The `views` table.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::{Container, LayoutNode, ProjectId, View, ViewId, WorkspaceId};
use crate::{Error, Result};

const COLUMNS: &str = "id, container_kind, container_id, name, \"order\", layout";

fn container_cols(c: &Container) -> (&'static str, &str) {
    match c {
        Container::Project { id } => ("Project", id.as_str()),
        Container::Workspace { id } => ("Workspace", id.as_str()),
    }
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<(String, String, String, String, u32, String)> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
}

fn view((id, kind, container_id, name, order, layout): (String, String, String, String, u32, String)) -> Result<View> {
    let container = match kind.as_str() {
        "Project" => Container::Project {
            id: ProjectId::parse(&container_id)?,
        },
        "Workspace" => Container::Workspace {
            id: WorkspaceId::parse(&container_id)?,
        },
        other => return Err(Error::Db(format!("Unknown container kind {other:?}"))),
    };
    let layout = serde_json::from_str(&layout).map_err(|e| Error::Db(format!("A view layout does not parse: {e}")))?;
    Ok(View {
        id: ViewId::parse(&id)?,
        container,
        name,
        order,
        layout,
    })
}

fn unknown(id: &ViewId) -> Error {
    Error::Invalid(format!("Unknown view {id}"))
}

fn changed(n: usize, id: &ViewId) -> Result<()> {
    if n == 0 {
        Err(unknown(id))
    } else {
        Ok(())
    }
}

pub fn insert(conn: &Connection, v: &View) -> Result<()> {
    let (kind, container_id) = container_cols(&v.container);
    conn.execute(
        &format!("INSERT INTO views ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
        params![
            v.id.as_str(),
            kind,
            container_id,
            v.name,
            v.order,
            serde_json::to_string(&v.layout)?
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &ViewId) -> Result<Option<View>> {
    let row = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM views WHERE id = ?1"),
            [id.as_str()],
            from_row,
        )
        .optional()?;
    row.map(view).transpose()
}

/// `Invalid("Unknown view <id>")` when there is no such view.
pub fn require(conn: &Connection, id: &ViewId) -> Result<View> {
    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// The container's views by `order`.
pub fn list(conn: &Connection, c: &Container) -> Result<Vec<View>> {
    let (kind, id) = container_cols(c);
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM views WHERE container_kind = ?1 AND container_id = ?2 ORDER BY \"order\", id"
    ))?;
    let rows = stmt.query_map([kind, id], from_row)?;
    rows.map(|r| view(r?)).collect()
}

pub fn update_layout(conn: &Connection, id: &ViewId, layout: &LayoutNode) -> Result<()> {
    let n = conn.execute(
        "UPDATE views SET layout = ?2 WHERE id = ?1",
        params![id.as_str(), serde_json::to_string(layout)?],
    )?;
    changed(n, id)
}

pub fn rename(conn: &Connection, id: &ViewId, name: &str) -> Result<()> {
    let n = conn.execute("UPDATE views SET name = ?2 WHERE id = ?1", params![id.as_str(), name])?;
    changed(n, id)
}

/// `order` becomes each id's position in `ids`; an id that is not one of the container's views is `Invalid` and
/// nothing changes.
pub fn reorder(conn: &Connection, c: &Container, ids: &[ViewId]) -> Result<()> {
    let existing: Vec<ViewId> = list(conn, c)?.into_iter().map(|v| v.id).collect();
    if let Some(stray) = ids.iter().find(|id| !existing.contains(id)) {
        return Err(unknown(stray));
    }
    let tx = conn.unchecked_transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE views SET \"order\" = ?2 WHERE id = ?1",
            params![id.as_str(), i as u32],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &ViewId) -> Result<()> {
    let n = conn.execute("DELETE FROM views WHERE id = ?1", [id.as_str()])?;
    changed(n, id)
}

/// One past the container's highest `order`; 0 for the first view.
pub fn next_order(conn: &Connection, c: &Container) -> Result<u32> {
    let (kind, id) = container_cols(c);
    Ok(conn.query_row(
        "SELECT coalesce(max(\"order\") + 1, 0) FROM views WHERE container_kind = ?1 AND container_id = ?2",
        [kind, id],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PaneId, SplitDirection};
    use crate::test_support::test_conn;
    use crate::views::layout::{leaves, split};
    use crate::views::new_view;

    #[test]
    fn views_round_trip_reorder_and_delete() {
        let c = test_conn();
        let pc = Container::Project {
            id: ProjectId::parse("p-1a-1").unwrap(),
        };
        let a = new_view(pc.clone(), "Alpha", next_order(&c, &pc).unwrap());
        insert(&c, &a).unwrap();
        let b = new_view(pc.clone(), "Beta", next_order(&c, &pc).unwrap());
        insert(&c, &b).unwrap();
        assert_eq!(
            list(&c, &pc)
                .unwrap()
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "Beta"]
        );
        reorder(&c, &pc, &[b.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(list(&c, &pc).unwrap()[0].id, b.id);
        let layout = split(
            &a.layout,
            &leaves(&a.layout)[0].0.clone(),
            SplitDirection::Vertical,
            PaneId::parse("pane-ff-1").unwrap(),
        )
        .unwrap();
        update_layout(&c, &a.id, &layout).unwrap();
        rename(&c, &a.id, "Gamma").unwrap();
        let got = get(&c, &a.id).unwrap().unwrap();
        assert_eq!((got.name.as_str(), leaves(&got.layout).len()), ("Gamma", 2));
        delete(&c, &a.id).unwrap();
        assert!(get(&c, &a.id).unwrap().is_none());
        assert!(matches!(reorder(&c, &pc, std::slice::from_ref(&a.id)), Err(Error::Invalid(_))));
    }
}
