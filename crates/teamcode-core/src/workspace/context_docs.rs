//! Context docs: files, folders, URLs and Confluence pages listed per project or workspace (§12).

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::{enum_from, parse_time};
use crate::domain::Container;
use crate::{Error, Result};

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
    /// Added by the developer in the app, or confirmed in a workspace import preview. A doc read from a file is
    /// never trusted until confirmed; only a trusted doc may be fetched from a private network.
    #[serde(default)]
    pub trusted: bool,
}

/// `<app_data>/context`, where fetched reference text is cached (and added to `--add-dir`).
pub fn context_dir(app_data: &std::path::Path) -> std::path::PathBuf {
    app_data.join("context")
}

/// `<app_data>/context/<doc-id>.md`; the id must be a valid `doc-…` id.
pub fn text_cache_path(app_data: &std::path::Path, doc_id: &str) -> Result<std::path::PathBuf> {
    if !(crate::ids::valid_id(doc_id) && doc_id.starts_with("doc-")) {
        return Err(crate::Error::Invalid("Unknown reference id.".into()));
    }
    Ok(context_dir(app_data).join(format!("{doc_id}.md")))
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
        "SELECT id, kind, target, label, page_id, fetched_at, ord, trusted FROM context_docs
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
            r.get::<_, bool>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, kind, target, label, page_id, fetched_at, order, trusted) = row?;
        out.push(ContextDoc {
            id,
            kind: enum_from(&kind)?,
            target,
            label,
            page_id,
            fetched_at: fetched_at.as_deref().map(parse_time).transpose()?,
            order,
            trusted,
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
            "INSERT INTO context_docs (id, scope_kind, scope_id, kind, target, label, page_id, fetched_at, ord, trusted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9)",
            rusqlite::params![
                crate::ids::new_id("doc"),
                kind,
                id,
                crate::db::enum_str(&d.kind)?,
                d.target,
                d.label,
                d.page_id,
                i as u32,
                d.trusted
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub use super::context_fetch::fetch_text;

/// The last path component; a URL's host and path; a Confluence page's title from its URL.
pub fn label_for(kind: &ContextDocKind, target: &str) -> String {
    let last = |t: &str| t.trim_end_matches('/').rsplit('/').next().unwrap_or(t).to_owned();
    match kind {
        ContextDocKind::File | ContextDocKind::Folder => last(target),
        ContextDocKind::Url => match url::Url::parse(target) {
            Ok(u) => format!("{}{}", u.host_str().unwrap_or(""), u.path().trim_end_matches('/')),
            Err(_) => target.to_owned(),
        },
        ContextDocKind::Confluence => {
            let path = url::Url::parse(target)
                .map(|u| u.path().to_owned())
                .unwrap_or_else(|_| target.to_owned());
            last(&path).replace(['-', '+'], " ")
        }
    }
}

/// The page id in a Confluence page URL.
fn confluence_page_id(url: &str) -> Option<String> {
    crate::jira::confluence::page_id_from_url(url)
}

/// The context docs of projects and workspaces.
pub struct ContextDocs {
    db: crate::db::Db,
}

impl ContextDocs {
    pub fn new(db: crate::db::Db) -> Self {
        Self { db }
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().expect("db lock")
    }

    pub fn list(&self, container: &Container) -> Result<Vec<ContextDoc>> {
        list_rows(&self.conn(), container)
    }

    /// Adds a reference at the end. File and folder targets must be absolute paths, URLs http or https, and a
    /// Confluence target a page URL (its page id is kept).
    pub fn add(
        &self,
        container: &Container,
        kind: ContextDocKind,
        target: &str,
        label: Option<&str>,
    ) -> Result<ContextDoc> {
        let target = target.trim();
        let mut page_id = None;
        match kind {
            ContextDocKind::File | ContextDocKind::Folder => {
                if !std::path::Path::new(target).is_absolute() {
                    return Err(Error::Invalid(
                        "A file or folder reference needs an absolute path.".into(),
                    ));
                }
            }
            ContextDocKind::Url | ContextDocKind::Confluence => {
                // the developer adds it, so a private (intranet) host is fine; local and link-local never are
                super::url_guard::check_lexical(target, true)?;
                if kind == ContextDocKind::Confluence {
                    page_id = Some(
                        confluence_page_id(target).ok_or_else(|| Error::Invalid("Not a Confluence page URL".into()))?,
                    );
                }
            }
        }
        let label = label
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map_or_else(|| label_for(&kind, target), str::to_owned);
        let conn = self.conn();
        let (scope_kind, scope_id) = scope_of(container);
        let order: u32 = conn.query_row(
            "SELECT coalesce(max(ord) + 1, 0) FROM context_docs WHERE scope_kind = ?1 AND scope_id = ?2",
            [scope_kind, scope_id],
            |r| r.get(0),
        )?;
        let doc = ContextDoc {
            id: crate::ids::new_id("doc"),
            kind,
            target: target.to_owned(),
            label,
            page_id,
            fetched_at: None,
            order,
            trusted: true,
        };
        conn.execute(
            "INSERT INTO context_docs (id, scope_kind, scope_id, kind, target, label, page_id, fetched_at, ord, trusted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, 1)",
            rusqlite::params![
                doc.id,
                scope_kind,
                scope_id,
                crate::db::enum_str(&doc.kind)?,
                doc.target,
                doc.label,
                doc.page_id,
                doc.order
            ],
        )?;
        Ok(doc)
    }

    pub fn remove(&self, container: &Container, id: &str) -> Result<()> {
        let (scope_kind, scope_id) = scope_of(container);
        self.conn().execute(
            "DELETE FROM context_docs WHERE id = ?1 AND scope_kind = ?2 AND scope_id = ?3",
            [id, scope_kind, scope_id],
        )?;
        Ok(())
    }

    /// Replaces the container's docs (each under a fresh id).
    pub fn replace_all(&self, container: &Container, docs: &[ContextDoc]) -> Result<()> {
        replace_rows(&self.conn(), container, docs)
    }

    pub fn set_fetched(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
        self.conn().execute(
            "UPDATE context_docs SET fetched_at = ?1 WHERE id = ?2",
            [crate::db::ts(&at), id.to_owned()],
        )?;
        Ok(())
    }

    /// The doc and the container it belongs to.
    pub fn get_with_container(&self, id: &str) -> Result<(Container, ContextDoc)> {
        let conn = self.conn();
        let scope: Option<(String, String)> = conn
            .query_row(
                "SELECT scope_kind, scope_id FROM context_docs WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (kind, sid) = scope.ok_or_else(|| Error::Invalid("Unknown reference id.".into()))?;
        let container = match kind.as_str() {
            "project" => Container::Project {
                id: crate::ids::ProjectId::parse(&sid)?,
            },
            _ => Container::Workspace {
                id: crate::ids::WorkspaceId::parse(&sid)?,
            },
        };
        let doc = list_rows(&conn, &container)?
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| Error::Invalid("Unknown reference id.".into()))?;
        Ok((container, doc))
    }

    pub fn get(&self, id: &str) -> Result<ContextDoc> {
        self.get_with_container(id).map(|(_, d)| d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::WorkspaceId;
    use crate::ids::valid_id;
    use crate::workspace::test_support::fixture;
    use crate::Error;
    use std::path::{Path, PathBuf};

    fn doc(id: &str, kind: ContextDocKind, target: &str, fetched: Option<DateTime<Utc>>) -> ContextDoc {
        ContextDoc {
            id: id.into(),
            kind,
            target: target.into(),
            label: label_for(&kind, target),
            page_id: None,
            fetched_at: fetched,
            order: 0,
            trusted: false,
        }
    }
    fn ws1() -> Container {
        Container::Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
        }
    }

    #[test]
    fn add_file_doc_labels_with_file_name_and_orders() {
        let fx = fixture();
        let d = ContextDocs::new(fx.db.clone());
        let c = Container::Project { id: fx.api.id.clone() };
        let a = d.add(&c, ContextDocKind::File, "/w/api/docs/arch.md", None).unwrap();
        let b = d
            .add(&c, ContextDocKind::Url, "https://example.com/spec/v2", None)
            .unwrap();
        assert_eq!(
            (a.label.as_str(), b.label.as_str(), a.order, b.order),
            ("arch.md", "example.com/spec/v2", 0, 1)
        );
        assert!(a.id.starts_with("doc-") && valid_id(&a.id));
        assert_eq!(d.get(&a.id).unwrap(), a);
    }

    #[test]
    fn list_is_scoped_by_container() {
        let fx = fixture();
        let d = ContextDocs::new(fx.db.clone());
        d.add(
            &Container::Project { id: fx.api.id.clone() },
            ContextDocKind::Folder,
            "/w/api/adr",
            None,
        )
        .unwrap();
        d.add(&ws1(), ContextDocKind::Url, "https://x.y/z", None).unwrap();
        assert_eq!(d.list(&Container::Project { id: fx.api.id.clone() }).unwrap().len(), 1);
        assert_eq!(d.list(&ws1()).unwrap().len(), 1);
    }

    #[test]
    fn replace_all_swaps_workspace_docs() {
        let fx = fixture();
        let d = ContextDocs::new(fx.db.clone());
        d.add(&ws1(), ContextDocKind::Url, "https://a/1", None).unwrap();
        d.replace_all(&ws1(), &[doc("doc-9", ContextDocKind::Url, "https://b/2", None)])
            .unwrap();
        assert_eq!(
            d.list(&ws1())
                .unwrap()
                .iter()
                .map(|x| x.target.as_str())
                .collect::<Vec<_>>(),
            vec!["https://b/2"]
        );
    }

    #[test]
    fn relative_file_and_bad_confluence_url_are_invalid() {
        let fx = fixture();
        let d = ContextDocs::new(fx.db.clone());
        let c = Container::Project { id: fx.api.id.clone() };
        assert!(matches!(
            d.add(&c, ContextDocKind::File, "docs/arch.md", None),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            d.add(
                &c,
                ContextDocKind::Confluence,
                "https://acme.atlassian.net/wiki/home",
                None
            ),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            d.add(&c, ContextDocKind::Url, "file:///etc/passwd", None),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn text_cache_path_validates_id() {
        assert_eq!(
            text_cache_path(Path::new("/app"), "doc-ab12").unwrap(),
            PathBuf::from("/app/context/doc-ab12.md")
        );
        assert!(text_cache_path(Path::new("/app"), "../x").is_err());
    }

    #[test]
    fn remove_and_set_fetched() {
        let fx = fixture();
        let d = ContextDocs::new(fx.db.clone());
        let a = d.add(&ws1(), ContextDocKind::Url, "https://a/1", Some("Spec")).unwrap();
        assert_eq!(a.label, "Spec");
        let at = Utc::now();
        d.set_fetched(&a.id, at).unwrap();
        assert!(d.get(&a.id).unwrap().fetched_at.is_some());
        d.remove(&ws1(), &a.id).unwrap();
        assert!(d.list(&ws1()).unwrap().is_empty());
        assert!(matches!(d.get(&a.id), Err(Error::Invalid(_))));
    }
}
