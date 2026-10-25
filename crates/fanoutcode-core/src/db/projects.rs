//! The `projects` table.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{parse_time, ts};
use crate::domain::{Project, ProjectColor, ProjectId};
use crate::{Error, Result};

/// The project colours, all swatch tokens from `domain::SWATCHES` and none a stage colour (spec §11.1).
pub const PROJECT_PALETTE: [&str; 8] = [
    "indigo", "cyan", "lime", "orange", "fuchsia", "yellow", "emerald", "rose",
];

const COLUMNS: &str = "id, name, root, color, memory_location, created_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<[String; 6]> {
    Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?])
}

fn project([id, name, root, color, memory_location, created_at]: [String; 6]) -> Result<Project> {
    Ok(Project {
        id: ProjectId::parse(&id)?,
        name,
        root: root.into(),
        color: ProjectColor(color),
        memory_location: serde_json::from_str(&memory_location)?,
        created_at: parse_time(&created_at)?,
    })
}

fn one(conn: &Connection, filter: &str, arg: &str) -> Result<Option<Project>> {
    let row = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM projects WHERE {filter} = ?1"),
            [arg],
            from_row,
        )
        .optional()?;
    row.map(project).transpose()
}

pub fn insert(conn: &Connection, p: &Project) -> Result<()> {
    conn.execute(
        &format!("INSERT INTO projects ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
        params![
            p.id.as_str(),
            p.name,
            p.root.to_string_lossy(),
            p.color.0,
            serde_json::to_string(&p.memory_location)?,
            ts(&p.created_at)
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &ProjectId) -> Result<Option<Project>> {
    one(conn, "id", id.as_str())
}

pub fn find_by_root(conn: &Connection, root: &Path) -> Result<Option<Project>> {
    one(conn, "root", &root.to_string_lossy())
}

/// Every project, oldest first.
pub fn list(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM projects ORDER BY created_at, id"))?;
    let rows = stmt.query_map([], from_row)?;
    rows.map(|r| project(r?)).collect()
}

/// Rewrites name, root, colour and memory location; `Invalid` for an unknown id.
pub fn update(conn: &Connection, p: &Project) -> Result<()> {
    let n = conn.execute(
        "UPDATE projects SET name = ?2, root = ?3, color = ?4, memory_location = ?5 WHERE id = ?1",
        params![
            p.id.as_str(),
            p.name,
            p.root.to_string_lossy(),
            p.color.0,
            serde_json::to_string(&p.memory_location)?
        ],
    )?;
    if n == 0 {
        return Err(Error::Invalid(format!("Unknown project {}", p.id)));
    }
    Ok(())
}

/// The least-used colour of `PROJECT_PALETTE`; ties go to palette order.
pub fn next_color(conn: &Connection) -> Result<ProjectColor> {
    let mut stmt = conn.prepare("SELECT color, count(*) FROM projects GROUP BY color")?;
    let used: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let uses = |c: &str| used.iter().find(|(u, _)| u == c).map_or(0, |(_, n)| *n);
    let best = PROJECT_PALETTE
        .iter()
        .min_by_key(|c| uses(c))
        .expect("the palette is not empty");
    Ok(ProjectColor((*best).to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MemoryLocation;
    use crate::test_support::{project_fixture, test_conn};

    #[test]
    fn project_round_trips_and_is_found_by_root() {
        let c = test_conn();
        let p = project_fixture(
            "p-1a-1",
            MemoryLocation::Folder {
                path: "/r/docs/tasks".into(),
            },
        );
        insert(&c, &p).unwrap();
        assert_eq!(get(&c, &p.id).unwrap(), Some(p.clone()));
        assert_eq!(find_by_root(&c, &p.root).unwrap(), Some(p.clone()));
        assert!(find_by_root(&c, Path::new("/elsewhere")).unwrap().is_none());
        let mut q = p.clone();
        q.name = "Renamed".into();
        q.memory_location = MemoryLocation::AppStorage;
        update(&c, &q).unwrap();
        assert_eq!(list(&c).unwrap(), vec![q]);
    }

    #[test]
    fn next_color_avoids_used_tokens() {
        let c = test_conn();
        assert_eq!(next_color(&c).unwrap(), ProjectColor("indigo".into()));
        let mut p = project_fixture("p-1a-1", MemoryLocation::AppStorage);
        p.color = ProjectColor("indigo".into());
        insert(&c, &p).unwrap();
        assert_eq!(next_color(&c).unwrap(), ProjectColor("cyan".into()));
    }
}
