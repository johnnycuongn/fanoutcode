//! `WorkspaceStore`: the workspace record in SQLite (the source of truth) and its file mirror. While the file lives
//! in app data every change is written through; once saved elsewhere a change only sets `dirty` until `save`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use super::file::{default_file_path, is_in_app_data, WorkspaceFile, WorkspaceParts, FILE_SUFFIX};
use super::rows;
use super::watcher::WorkspaceFileWatcher;
use crate::db::{self, enum_str, Db};
use crate::domain::{LinkPolicy, Project, ProjectId, Stage, Workspace, WorkspaceId, WorkspaceProject};
use crate::memory::store::write_atomic;
use crate::{Error, Result};

pub struct WorkspaceStore {
    db: Db,
    app_data: PathBuf,
    watcher: Option<Arc<WorkspaceFileWatcher>>,
}

impl WorkspaceStore {
    pub fn new(db: Db, app_data: PathBuf, watcher: Option<Arc<WorkspaceFileWatcher>>) -> Self {
        Self { db, app_data, watcher }
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn app_data(&self) -> &Path {
        &self.app_data
    }

    pub fn watcher(&self) -> Option<&Arc<WorkspaceFileWatcher>> {
        self.watcher.as_ref()
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().expect("db lock")
    }

    /// A new workspace (id prefix `ws`) holding `project_ids` in that order, its file in app data.
    pub fn create(&self, name: &str, project_ids: &[ProjectId]) -> Result<Workspace> {
        let name = clean_name(name)?;
        let id = WorkspaceId::new();
        {
            let conn = self.conn();
            for p in project_ids {
                require_project(&conn, p)?;
            }
            let path = default_file_path(&self.app_data, &id, &name)?;
            rows::insert(&conn, &id, &name, Some(&path), LinkPolicy::ReadAndModifyOnRequest)?;
            let members: Vec<WorkspaceProject> = dedup(project_ids)
                .into_iter()
                .enumerate()
                .map(|(i, p)| WorkspaceProject {
                    project_id: p,
                    order: i as u32,
                    protected: false,
                })
                .collect();
            rows::set_members(&conn, &id, &members)?;
        }
        self.after_change(&id)
    }

    pub fn get(&self, id: &WorkspaceId) -> Result<Workspace> {
        rows::get(&self.conn(), id)
    }

    pub fn exists(&self, id: &WorkspaceId) -> Result<bool> {
        rows::exists(&self.conn(), id)
    }

    pub fn list(&self) -> Result<Vec<Workspace>> {
        let conn = self.conn();
        rows::list_ids(&conn)?.iter().map(|id| rows::get(&conn, id)).collect()
    }

    /// Renames the workspace; a file in app data moves to the new default name.
    pub fn rename(&self, id: &WorkspaceId, name: &str) -> Result<Workspace> {
        let name = clean_name(name)?;
        let old = self.get(id)?;
        rows::set_column(&self.conn(), id, "name", &name)?;
        if let Some(path) = old.file_path.filter(|p| is_in_app_data(p, &self.app_data)) {
            let new_path = default_file_path(&self.app_data, id, &name)?;
            if new_path != path {
                rows::set_column(&self.conn(), id, "file_path", &new_path.to_string_lossy().into_owned())?;
                let _ = std::fs::remove_file(&path);
            }
        }
        self.after_change(id)
    }

    /// Appends the project with the next `order`; a no-op when it is already a member.
    pub fn add_project(&self, id: &WorkspaceId, project_id: &ProjectId) -> Result<Workspace> {
        {
            let conn = self.conn();
            require_project(&conn, project_id)?;
            let mut ws = rows::get(&conn, id)?;
            if ws.projects.iter().any(|m| &m.project_id == project_id) {
                return Ok(ws);
            }
            let order = ws.projects.iter().map(|m| m.order + 1).max().unwrap_or(0);
            ws.projects.push(WorkspaceProject {
                project_id: project_id.clone(),
                order,
                protected: false,
            });
            rows::set_members(&conn, id, &ws.projects)?;
        }
        self.after_change(id)
    }

    pub fn remove_project(&self, id: &WorkspaceId, project_id: &ProjectId) -> Result<Workspace> {
        {
            let conn = self.conn();
            let mut ws = rows::get(&conn, id)?;
            ws.projects.retain(|m| &m.project_id != project_id);
            rows::set_members(&conn, id, &renumber(ws.projects))?;
        }
        self.after_change(id)
    }

    /// `order` must name every member exactly once.
    pub fn reorder_projects(&self, id: &WorkspaceId, order: &[ProjectId]) -> Result<Workspace> {
        {
            let conn = self.conn();
            let ws = rows::get(&conn, id)?;
            let mut wanted: Vec<&ProjectId> = order.iter().collect();
            let mut have: Vec<&ProjectId> = ws.projects.iter().map(|m| &m.project_id).collect();
            wanted.sort();
            have.sort();
            if wanted != have {
                return Err(Error::Invalid(
                    "The new order must list every project of the workspace once.".into(),
                ));
            }
            let members = order
                .iter()
                .filter_map(|p| ws.projects.iter().find(|m| &m.project_id == p).cloned())
                .collect();
            rows::set_members(&conn, id, &renumber(members))?;
        }
        self.after_change(id)
    }

    /// The policy, and which members are protected targets (every other member is not).
    pub fn set_policy(&self, id: &WorkspaceId, policy: LinkPolicy, protected: &[ProjectId]) -> Result<Workspace> {
        {
            let conn = self.conn();
            let mut ws = rows::get(&conn, id)?;
            for m in &mut ws.projects {
                m.protected = protected.contains(&m.project_id);
            }
            rows::set_members(&conn, id, &ws.projects)?;
            rows::set_column(&conn, id, "policy", &enum_str(&policy)?)?;
        }
        self.after_change(id)
    }

    /// The instructions appended to `stage`'s prompt in this workspace (`"spec"` … `"commit"`); `None` or blank
    /// removes them.
    pub fn set_fragment(&self, id: &WorkspaceId, stage: &str, text: Option<&str>) -> Result<Workspace> {
        match Stage::parse(stage) {
            Some(s) if s.has_agent_prompt() => {}
            _ => return Err(Error::Invalid(format!("Unknown stage {stage}"))),
        }
        self.set_map_entry(id, "fragments_json", stage, text)
    }

    pub fn fragments(&self, id: &WorkspaceId) -> Result<BTreeMap<String, String>> {
        rows::map_column(&self.conn(), id, "fragments_json")
    }

    /// A workspace-level agent definition (markdown with front matter); `None` or blank removes it.
    pub fn set_agent(&self, id: &WorkspaceId, name: &str, markdown: Option<&str>) -> Result<Workspace> {
        if !valid_agent_name(name) {
            return Err(Error::Invalid(
                "Agent names use lower-case letters, digits and dashes.".into(),
            ));
        }
        self.set_map_entry(id, "agents_json", name, markdown)
    }

    pub fn agents(&self, id: &WorkspaceId) -> Result<BTreeMap<String, String>> {
        rows::map_column(&self.conn(), id, "agents_json")
    }

    fn set_map_entry(&self, id: &WorkspaceId, column: &str, key: &str, text: Option<&str>) -> Result<Workspace> {
        {
            let conn = self.conn();
            let mut map = rows::map_column(&conn, id, column)?;
            match text.map(str::trim).filter(|t| !t.is_empty()) {
                Some(t) => map.insert(key.to_owned(), t.to_owned()),
                None => map.remove(key),
            };
            rows::set_map_column(&conn, id, column, &map)?;
        }
        self.after_change(id)
    }

    /// The workspace-scope settings rows the file mirrors (every key but `jira`).
    pub fn workspace_settings(&self, id: &WorkspaceId) -> Result<BTreeMap<String, serde_json::Value>> {
        rows::settings(&self.conn(), id)
    }

    /// Upserts workspace-scope settings rows (`null` removes one); `jira` is ignored.
    pub fn apply_workspace_settings(
        &self,
        id: &WorkspaceId,
        settings: &BTreeMap<String, serde_json::Value>,
    ) -> Result<()> {
        rows::get(&self.conn(), id)?;
        rows::upsert_settings(&self.conn(), id, settings)?;
        self.after_change(id).map(|_| ())
    }

    /// A record with a given id (opening a file whose id is unknown here); no file is written.
    pub(crate) fn insert_record(
        &self,
        id: &WorkspaceId,
        name: &str,
        file_path: &Path,
        policy: LinkPolicy,
    ) -> Result<()> {
        rows::insert(&self.conn(), id, name, Some(file_path), policy)
    }

    /// Replaces the mirrored rows (the record's settings, fragments, agents) from a file, without writing it back.
    pub(crate) fn replace_from_file(
        &self,
        id: &WorkspaceId,
        file: &WorkspaceFile,
        members: &[WorkspaceProject],
    ) -> Result<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        rows::set_column(&tx, id, "name", &file.name)?;
        rows::set_column(&tx, id, "policy", &enum_str(&file.policy)?)?;
        rows::set_members(&tx, id, members)?;
        rows::set_map_column(&tx, id, "fragments_json", &file.prompt_fragments)?;
        rows::set_map_column(&tx, id, "agents_json", &file.agents)?;
        rows::clear_settings(&tx, id)?;
        rows::upsert_settings(&tx, id, &file.settings)?;
        tx.commit()?;
        Ok(())
    }

    /// Points the record at `path` with nothing unsaved.
    pub(crate) fn set_file_clean(&self, id: &WorkspaceId, path: &Path) -> Result<()> {
        let conn = self.conn();
        rows::set_column(&conn, id, "file_path", &path.to_string_lossy().into_owned())?;
        rows::set_column(&conn, id, "dirty", &false)
    }

    /// The file this record would be saved as now.
    pub fn snapshot(&self, id: &WorkspaceId) -> Result<WorkspaceFile> {
        let conn = self.conn();
        let ws = rows::get(&conn, id)?;
        let path = match ws.file_path.clone() {
            Some(p) => p,
            None => default_file_path(&self.app_data, id, &ws.name)?,
        };
        let projects = member_projects(&conn, &ws)?;
        let settings = rows::settings(&conn, id)?;
        let fragments = rows::map_column(&conn, id, "fragments_json")?;
        let agents = rows::map_column(&conn, id, "agents_json")?;
        let docs = super::context_docs::list_rows(&conn, &crate::domain::Container::Workspace { id: id.clone() })?;
        Ok(WorkspaceFile::from_parts(
            &WorkspaceParts {
                workspace: &ws,
                projects: &projects,
                settings: &settings,
                prompt_fragments: &fragments,
                agents: &agents,
                context_docs: &docs,
            },
            &path,
        ))
    }

    /// Writes the file where the record points and clears `dirty`.
    pub fn save(&self, id: &WorkspaceId) -> Result<Workspace> {
        let ws = self.ensure_path(id)?;
        let path = ws.file_path.expect("ensure_path sets a path");
        self.write_file(id, &path)?;
        rows::set_column(&self.conn(), id, "dirty", &false)?;
        self.get(id)
    }

    /// Saves to `path` (the suffix appended when missing) and points the record there. A previous file in app data
    /// is removed; a path in app data switches back to write-through.
    pub fn save_as(&self, id: &WorkspaceId, path: &Path) -> Result<Workspace> {
        let path = with_suffix(path);
        let old = self.get(id)?.file_path;
        self.write_file(id, &path)?;
        self.set_file_clean(id, &path)?;
        if let Some(old) = old.filter(|o| o != &path && is_in_app_data(o, &self.app_data)) {
            let _ = std::fs::remove_file(&old);
            if let Some(dir) = old.parent() {
                let _ = std::fs::remove_dir(dir); // only when empty
            }
        }
        self.after_save_as(id, &path);
        self.get(id)
    }

    /// Hook for the watcher (Task 4).
    fn after_save_as(&self, _id: &WorkspaceId, _path: &Path) {}

    /// Hook for the watcher (Task 4): a file opened outside app data is watched.
    pub(crate) fn after_open(&self, id: &WorkspaceId, path: &Path) {
        self.after_save_as(id, path)
    }

    /// Ends every mutation: writes through while the file is in app data, else marks the record dirty.
    pub(crate) fn after_change(&self, id: &WorkspaceId) -> Result<Workspace> {
        let ws = self.ensure_path(id)?;
        let path = ws.file_path.clone().expect("ensure_path sets a path");
        if is_in_app_data(&path, &self.app_data) {
            self.write_file(id, &path)?;
            rows::set_column(&self.conn(), id, "dirty", &false)?;
        } else {
            rows::set_column(&self.conn(), id, "dirty", &true)?;
        }
        self.get(id)
    }

    /// An older row without a path gets its default one.
    fn ensure_path(&self, id: &WorkspaceId) -> Result<Workspace> {
        let ws = self.get(id)?;
        if ws.file_path.is_some() {
            return Ok(ws);
        }
        let path = default_file_path(&self.app_data, id, &ws.name)?;
        rows::set_column(&self.conn(), id, "file_path", &path.to_string_lossy().into_owned())?;
        self.get(id)
    }

    fn write_file(&self, id: &WorkspaceId, path: &Path) -> Result<()> {
        let bytes = self.snapshot(id)?.to_json().into_bytes();
        if let Some(w) = &self.watcher {
            w.note_own_write(id, &bytes);
        }
        write_atomic(path, &bytes)
    }
}

/// The `Project` of every member, in member order; a member whose row is gone is skipped.
pub(crate) fn member_projects(conn: &Connection, ws: &Workspace) -> Result<Vec<Project>> {
    let mut out = Vec::new();
    for m in &ws.projects {
        if let Some(p) = db::projects::get(conn, &m.project_id)? {
            out.push(p);
        }
    }
    Ok(out)
}

fn require_project(conn: &Connection, id: &ProjectId) -> Result<Project> {
    db::projects::get(conn, id)?.ok_or_else(|| Error::Invalid("Unknown project id.".into()))
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("A workspace needs a name.".into()));
    }
    Ok(name.to_owned())
}

fn dedup(ids: &[ProjectId]) -> Vec<ProjectId> {
    let mut out: Vec<ProjectId> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

fn renumber(mut members: Vec<WorkspaceProject>) -> Vec<WorkspaceProject> {
    for (i, m) in members.iter_mut().enumerate() {
        m.order = i as u32;
    }
    members
}

fn valid_agent_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `path` with `.teamcode-workspace.json` appended unless it already ends so.
pub fn with_suffix(path: &Path) -> PathBuf {
    if path.to_string_lossy().ends_with(FILE_SUFFIX) {
        path.to_owned()
    } else {
        let mut s = path.as_os_str().to_owned();
        s.push(FILE_SUFFIX);
        PathBuf::from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::test_support::{btree, fixture, store_with, store_with_both};
    use serde_json::json;

    #[test]
    fn create_writes_file_in_app_data() {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let ws = s.create("Shop", std::slice::from_ref(&fx.api.id)).unwrap();
        let path = ws.file_path.clone().unwrap();
        assert_eq!(path, default_file_path(fx.app_data.path(), &ws.id, "Shop").unwrap());
        assert!(ws.id.as_str().starts_with("ws-"));
        assert!(!ws.dirty && WorkspaceFile::parse(&std::fs::read(&path).unwrap()).unwrap().id == ws.id);
    }

    #[test]
    fn mutation_in_app_data_writes_through() {
        let (fx, s, ws) = store_with("Shop");
        let ws = s.add_project(&ws.id, &fx.web.id).unwrap();
        assert!(!ws.dirty);
        assert_eq!(
            WorkspaceFile::parse(&std::fs::read(ws.file_path.unwrap()).unwrap())
                .unwrap()
                .projects
                .len(),
            2
        );
    }

    #[test]
    fn save_as_switches_to_explicit_mode() {
        let (fx, s, ws) = store_with("Shop");
        let outside = tempfile::tempdir().unwrap();
        let old = ws.file_path.clone().unwrap();
        let ws = s.save_as(&ws.id, &outside.path().join("shop")).unwrap(); // suffix appended
        assert_eq!(
            ws.file_path.as_deref(),
            Some(outside.path().join("shop.teamcode-workspace.json").as_path())
        );
        assert!(!old.exists() && !ws.dirty);
        let ws = s.add_project(&ws.id, &fx.web.id).unwrap();
        assert!(ws.dirty);
        assert_eq!(
            WorkspaceFile::parse(&std::fs::read(ws.file_path.as_ref().unwrap()).unwrap())
                .unwrap()
                .projects
                .len(),
            1
        );
        let ws = s.save(&ws.id).unwrap();
        assert!(!ws.dirty);
        assert_eq!(
            WorkspaceFile::parse(&std::fs::read(ws.file_path.unwrap()).unwrap())
                .unwrap()
                .projects
                .len(),
            2
        );
    }

    #[test]
    fn set_policy_marks_protected_projects() {
        let (fx, s, ws) = store_with_both("Shop");
        let ws = s
            .set_policy(
                &ws.id,
                LinkPolicy::ReadAndModifyOnRequest,
                std::slice::from_ref(&fx.web.id),
            )
            .unwrap();
        let web = ws.projects.iter().find(|p| p.project_id == fx.web.id).unwrap();
        assert!(
            web.protected
                && !ws
                    .projects
                    .iter()
                    .find(|p| p.project_id == fx.api.id)
                    .unwrap()
                    .protected
        );
        assert!(s.snapshot(&ws.id).unwrap().projects.iter().any(|p| p.protected));
        let ws = s.set_policy(&ws.id, LinkPolicy::ReadOnly, &[]).unwrap();
        assert_eq!(ws.policy, LinkPolicy::ReadOnly);
    }

    #[test]
    fn fragments_and_agents_persist_and_mirror() {
        let (_fx, s, ws) = store_with("Shop");
        s.set_fragment(&ws.id, "plan", Some("Prefer the api crate's naming."))
            .unwrap();
        s.set_agent(&ws.id, "docs-writer", Some("---\nname: docs-writer\n---\nWrites docs."))
            .unwrap();
        let f = s.snapshot(&ws.id).unwrap();
        assert_eq!(f.prompt_fragments["plan"], "Prefer the api crate's naming.");
        assert!(f.agents.contains_key("docs-writer"));
        assert!(s
            .set_fragment(&ws.id, "plan", None)
            .and_then(|_| s.fragments(&ws.id))
            .unwrap()
            .is_empty());
        assert!(matches!(
            s.set_fragment(&ws.id, "done", Some("x")),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(s.set_agent(&ws.id, "../x", Some("x")), Err(Error::Invalid(_))));
    }

    #[test]
    fn settings_rows_mirror_into_file_except_jira() {
        let (_fx, s, ws) = store_with("Shop");
        s.apply_workspace_settings(
            &ws.id,
            &btree(&[("workspace_board", json!("mixed")), ("jira", json!({"site_url":"x"}))]),
        )
        .unwrap();
        let f = s.snapshot(&ws.id).unwrap();
        assert_eq!(f.settings.get("workspace_board"), Some(&json!("mixed")));
        assert!(!f.settings.contains_key("jira"));
    }

    #[test]
    fn reorder_and_remove_renumber_members() {
        let (fx, s, ws) = store_with_both("Shop");
        assert!(matches!(
            s.reorder_projects(&ws.id, std::slice::from_ref(&fx.web.id)),
            Err(Error::Invalid(_))
        ));
        let ws = s
            .reorder_projects(&ws.id, &[fx.web.id.clone(), fx.api.id.clone()])
            .unwrap();
        assert_eq!(ws.projects[0].project_id, fx.web.id);
        let ws = s.remove_project(&ws.id, &fx.web.id).unwrap();
        assert_eq!((ws.projects.len(), ws.projects[0].order), (1, 0));
    }
}
