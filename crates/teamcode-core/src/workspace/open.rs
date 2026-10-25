//! Opening a workspace by its file, reload, status, and relocating a moved file or project.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file::{is_in_app_data, WorkspaceFile};
use super::store::WorkspaceStore;
use crate::db;
use crate::domain::{MemoryLocation, Project, ProjectId, Workspace, WorkspaceId, WorkspaceProject};
use crate::{Error, Result};

/// What the settings screen and the window warn about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceStatus {
    pub file_missing: bool,
    /// Members whose root folder no longer exists.
    pub missing_projects: Vec<ProjectId>,
    /// The file lives outside app data, so changes wait for `save`.
    pub explicit_save: bool,
}

/// Loads the file at `path` into the database: the record with the file's id is updated (or created with that id),
/// projects are matched by absolute root (an unknown root becomes a new project even when its folder is missing),
/// and the mirrored rows are replaced. The record then points at `path` with nothing unsaved.
pub fn open_file(store: &WorkspaceStore, path: &Path) -> Result<Workspace> {
    let bytes = std::fs::read(path)
        .map_err(|_| Error::Precondition("That workspace file no longer exists. Relocate or remove it.".into()))?;
    let file = WorkspaceFile::parse(&bytes)?;
    if !store.exists(&file.id)? {
        store.insert_record(&file.id, &file.name, path, file.policy)?;
    }
    let members = {
        let conn = store.db().lock().expect("db lock");
        let mut members = Vec::new();
        for (i, (fp, root)) in file.projects.iter().zip(file.absolute_project_paths(path)).enumerate() {
            let root = std::fs::canonicalize(&root).unwrap_or(root);
            let project = match db::projects::find_by_root(&conn, &root)? {
                Some(p) => p,
                None => {
                    let p = Project {
                        id: ProjectId::new(),
                        name: fp.name.clone(),
                        root,
                        color: fp.color.clone(),
                        memory_location: MemoryLocation::AppStorage,
                        created_at: chrono::Utc::now(),
                    };
                    db::projects::insert(&conn, &p)?;
                    p
                }
            };
            if members.iter().all(|m: &WorkspaceProject| m.project_id != project.id) {
                members.push(WorkspaceProject {
                    project_id: project.id,
                    order: i as u32,
                    protected: fp.protected,
                });
            }
        }
        members
    };
    store.replace_from_file(&file.id, &file, &members)?;
    store.set_file_clean(&file.id, path)?;
    store.after_open(&file.id, path);
    store.get(&file.id)
}

/// Re-reads the record's file. Refuses while there are unsaved changes unless `discard_unsaved`.
pub fn reload(store: &WorkspaceStore, id: &WorkspaceId, discard_unsaved: bool) -> Result<Workspace> {
    let ws = store.get(id)?;
    if ws.dirty && !discard_unsaved {
        return Err(Error::Precondition(
            "Unsaved workspace changes; save or discard them first".into(),
        ));
    }
    let path = ws
        .file_path
        .ok_or_else(|| Error::Precondition("This workspace has no file yet; save it first".into()))?;
    open_file(store, &path)
}

pub fn status(store: &WorkspaceStore, id: &WorkspaceId) -> Result<WorkspaceStatus> {
    let ws = store.get(id)?;
    let missing_projects = {
        let conn = store.db().lock().expect("db lock");
        let mut out = Vec::new();
        for m in &ws.projects {
            let gone = db::projects::get(&conn, &m.project_id)?.is_none_or(|p| !p.root.is_dir());
            if gone {
                out.push(m.project_id.clone());
            }
        }
        out
    };
    let path: Option<PathBuf> = ws.file_path;
    Ok(WorkspaceStatus {
        file_missing: path.as_ref().is_none_or(|p| !p.is_file()),
        missing_projects,
        explicit_save: path.as_ref().is_some_and(|p| !is_in_app_data(p, store.app_data())),
    })
}

/// Points the workspace at its moved file; the file must carry this workspace's id.
pub fn relocate_file(store: &WorkspaceStore, id: &WorkspaceId, new_path: &Path) -> Result<Workspace> {
    let file = WorkspaceFile::parse(&std::fs::read(new_path)?)?;
    if &file.id != id {
        return Err(Error::Precondition("That file belongs to a different workspace".into()));
    }
    open_file(store, new_path)
}

/// Moves a member's root to `new_root`, which no other project may already own.
pub fn relocate_project(
    store: &WorkspaceStore,
    id: &WorkspaceId,
    project_id: &ProjectId,
    new_root: &Path,
) -> Result<Workspace> {
    let ws = store.get(id)?;
    if !ws.projects.iter().any(|m| &m.project_id == project_id) {
        return Err(Error::Invalid("That project is not in this workspace.".into()));
    }
    {
        let conn = store.db().lock().expect("db lock");
        if let Some(other) = db::projects::find_by_root(&conn, new_root)?.filter(|p| &p.id != project_id) {
            return Err(Error::Precondition(format!(
                "{} is already open as {}",
                new_root.display(),
                other.name
            )));
        }
        let mut project =
            db::projects::get(&conn, project_id)?.ok_or_else(|| Error::Invalid("Unknown project id.".into()))?;
        project.root = new_root.to_owned();
        db::projects::update(&conn, &project)?;
    }
    store.after_change(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::store::WorkspaceStore;
    use crate::workspace::test_support::{fixture, store_with, store_with_both};
    use crate::Error;

    fn file_json(id: &str, projects: &[(&str, &str)]) -> String {
        let projects: Vec<_> = projects
            .iter()
            .enumerate()
            .map(|(i, (name, path))| {
                serde_json::json!({"path": path, "name": name, "color": "cyan", "order": i, "protected": false})
            })
            .collect();
        serde_json::json!({"teamcode_workspace": 1, "id": id, "name": "Ext", "projects": projects, "policy": "ReadOnly",
            "prompt_fragments": {"plan": "Both suites."}})
        .to_string()
    }

    #[test]
    fn open_file_with_known_id_reuses_record_and_repoints_path() {
        let (_fx, s, ws) = store_with("Shop");
        let dir = tempfile::tempdir().unwrap();
        let moved = dir.path().join("Shop.teamcode-workspace.json");
        std::fs::copy(ws.file_path.as_ref().unwrap(), &moved).unwrap();
        let opened = open_file(&s, &moved).unwrap();
        assert_eq!((opened.id, opened.file_path.as_deref()), (ws.id, Some(moved.as_path())));
        assert_eq!(s.list().unwrap().len(), 1);
        assert_eq!(opened.projects.len(), 1);
    }

    #[test]
    fn open_file_with_unknown_id_creates_record_and_missing_projects() {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ext.teamcode-workspace.json");
        std::fs::write(
            &path,
            file_json(
                "ws-e7",
                &[("api", fx.api.root.to_str().unwrap()), ("gone", "/nonexistent/gone")],
            ),
        )
        .unwrap();
        let ws = open_file(&s, &path).unwrap();
        assert_eq!((ws.id.as_str(), ws.projects.len()), ("ws-e7", 2));
        assert_eq!(
            ws.projects[0].project_id, fx.api.id,
            "the existing project is reused by root"
        );
        assert_eq!(s.fragments(&ws.id).unwrap()["plan"], "Both suites.");
        let st = status(&s, &ws.id).unwrap();
        assert_eq!(
            (st.file_missing, st.missing_projects.len(), st.explicit_save),
            (false, 1, true)
        );
    }

    #[test]
    fn status_reports_missing_file() {
        let (_fx, s, ws) = store_with("Shop");
        let out = tempfile::tempdir().unwrap();
        let ws = s
            .save_as(&ws.id, &out.path().join("Shop.teamcode-workspace.json"))
            .unwrap();
        std::fs::remove_file(ws.file_path.unwrap()).unwrap();
        assert!(status(&s, &ws.id).unwrap().file_missing);
    }

    #[test]
    fn reload_refuses_when_dirty_unless_discard() {
        let (fx, s, ws) = store_with("Shop");
        let out = tempfile::tempdir().unwrap();
        let ws = s
            .save_as(&ws.id, &out.path().join("Shop.teamcode-workspace.json"))
            .unwrap();
        s.add_project(&ws.id, &fx.web.id).unwrap(); // dirty, file still has 1 project
        assert!(matches!(reload(&s, &ws.id, false), Err(Error::Precondition(_))));
        assert_eq!(reload(&s, &ws.id, true).unwrap().projects.len(), 1);
    }

    #[test]
    fn relocate_file_rejects_other_workspace() {
        let (_fx, s, a) = store_with("A");
        let b = s.create("B", &[]).unwrap();
        assert!(matches!(
            relocate_file(&s, &a.id, b.file_path.as_ref().unwrap()),
            Err(Error::Precondition(_))
        ));
    }

    #[test]
    fn relocate_project_rejects_root_of_another_project() {
        let (fx, s, ws) = store_with_both("Shop");
        assert!(matches!(
            relocate_project(&s, &ws.id, &fx.api.id, &fx.web.root),
            Err(Error::Precondition(_))
        ));
        let new_root = tempfile::tempdir().unwrap();
        relocate_project(&s, &ws.id, &fx.api.id, new_root.path()).unwrap();
        assert!(s
            .snapshot(&ws.id)
            .unwrap()
            .absolute_project_paths(ws.file_path.as_ref().unwrap())
            .contains(&new_root.path().to_path_buf()));
    }
}
