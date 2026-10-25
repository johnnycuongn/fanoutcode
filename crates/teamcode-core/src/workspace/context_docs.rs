//! Context docs: files, folders, URLs and Confluence pages listed per project or workspace (§12).

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{enum_from, parse_time};
use crate::domain::Container;
use crate::Result;

/// What a context doc points at. Serialises as `"File"`, `"Folder"`, `"Url"`, `"Confluence"` (A16), the strings
/// stored in `context_docs.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ContextDocKind {
    File,
    Folder,
    Url,
    Confluence,
}

/// One reference listed in prompts under "Project references" or "Workspace references".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContextDoc {
    pub id: String,
    pub kind: ContextDocKind,
    pub target: String,
    pub label: String,
    pub page_id: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub order: u32,
}

/// `(scope_kind, scope_id)` of a container in the `context_docs` table.
pub(crate) fn scope_of(container: &Container) -> (&'static str, &str) {
    match container {
        Container::Project { id } => ("project", id.as_str()),
        Container::Workspace { id } => ("workspace", id.as_str()),
    }
}

/// The container's docs by `order`.
pub(crate) fn list_rows(conn: &Connection, container: &Container) -> Result<Vec<ContextDoc>> {
    let (kind, id) = scope_of(container);
    let mut stmt = conn.prepare(
        "SELECT id, kind, target, label, page_id, fetched_at, ord FROM context_docs
         WHERE scope_kind = ?1 AND scope_id = ?2 ORDER BY ord, id",
    )?;
    let rows = stmt.query_map([kind, id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, u32>(6)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, kind, target, label, page_id, fetched_at, order) = row?;
        out.push(ContextDoc {
            id,
            kind: enum_from(&kind)?,
            target,
            label,
            page_id,
            fetched_at: fetched_at.as_deref().map(parse_time).transpose()?,
            order,
        });
    }
    Ok(out)
}

/// Replaces the container's docs with `docs` in their order, each under a fresh id (ids from a file are never
/// trusted to be unique across containers).
pub(crate) fn replace_rows(conn: &Connection, container: &Container, docs: &[ContextDoc]) -> Result<()> {
    let (kind, id) = scope_of(container);
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM context_docs WHERE scope_kind = ?1 AND scope_id = ?2",
        [kind, id],
    )?;
    for (i, d) in docs.iter().enumerate() {
        tx.execute(
            "INSERT INTO context_docs (id, scope_kind, scope_id, kind, target, label, page_id, fetched_at, ord)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
            rusqlite::params![
                crate::ids::new_id("doc"),
                kind,
                id,
                crate::db::enum_str(&d.kind)?,
                d.target,
                d.label,
                d.page_id,
                i as u32
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}
