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

/// Writes `file` (already sanitised and reviewed or trusted) into record `target`: the record is created when absent,
/// projects are matched by absolute root (an unknown root becomes a new project), and the mirrored rows are
/// replaced. The record then points at `path`; when `rewritten` (the file's id belonged to another record) the
/// record is marked unsaved, or written through in app data, so the file gets its new id.
pub(crate) fn import(
    store: &WorkspaceStore,
    path: &Path,
    file: &WorkspaceFile,
    roots: &[PathBuf],
    target: &WorkspaceId,
    rewritten: bool,
    trust: super::import::DocTrust,
) -> Result<Workspace> {
    if !store.exists(target)? {
        store.insert_record(target, &file.name, path, file.policy)?;
    }
    let members = {
        let conn = store.db().lock().expect("db lock");
        let mut members = Vec::new();
        for (i, (fp, root)) in file.projects.iter().zip(roots.iter().cloned()).enumerate() {
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
    store.replace_from_file(target, file, &members)?;
    {
        let conn = store.db().lock().expect("db lock");
        let container = crate::domain::Container::Workspace { id: target.clone() };
        let existing = super::context_docs::list_rows(&conn, &container)?;
        let docs: Vec<_> = file
            .context_docs
            .iter()
            .map(|d| {
                let mut d = d.clone();
                d.trusted = match trust {
                    super::import::DocTrust::Confirmed => true,
                    super::import::DocTrust::Existing => existing.iter().any(|m| {
                        m.trusted
                            && m.kind == d.kind
                            && m.target == d.target
                            && m.label == d.label
                            && m.page_id == d.page_id
                    }),
                };
                d
            })
            .collect();
        super::context_docs::replace_rows(&conn, &container, &docs)?;
    }
    store.set_file_clean(target, path)?;
    store.after_open(target, path);
    if rewritten {
        return store.after_change(target);
    }
    store.get(target)
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
    super::import::open_file(store, &path)
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

/// Points a workspace whose file went missing at the file's new place. The file must carry this workspace's id and
/// add nothing to the record (no new folder, agent, instruction or reference); otherwise open it as a workspace of
/// its own, through the preview.
pub fn relocate_file(store: &WorkspaceStore, id: &WorkspaceId, new_path: &Path) -> Result<Workspace> {
    let parsed = WorkspaceFile::parse(&std::fs::read(new_path)?)?;
    if &parsed.id != id {
        return Err(Error::Precondition("That file belongs to a different workspace".into()));
    }
    let record = store.get(id)?;
    if !status(store, id)?.file_missing {
        return Err(Error::Precondition(
            "This workspace's file is still in place; open the other file as its own workspace.".into(),
        ));
    }
    let (file, _) = super::import_checks::sanitize(parsed, &record.name);
    if !super::import_checks::adds_nothing(store, &record, &file, new_path)? {
        return Err(Error::Precondition(
            "That file adds folders, agents or instructions; open it as a new workspace instead.".into(),
        ));
    }
    let roots: Vec<PathBuf> = file
        .absolute_project_paths(new_path)
        .iter()
        .map(|r| std::fs::canonicalize(r).unwrap_or_else(|_| r.clone()))
        .collect();
    import(
        store,
        new_path,
        &file,
        &roots,
        id,
        false,
        super::import::DocTrust::Existing,
    )
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
    use crate::workspace::import::{confirm_open, open_file, preview_open, OpenEnv};
    use crate::workspace::store::WorkspaceStore;
    use crate::workspace::test_support::{btree, fixture, store_with, store_with_both};
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

    fn env_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
        (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
    }

    fn open_reviewed(s: &WorkspaceStore, path: &Path) -> Result<Workspace> {
        let (home, cfg) = env_dirs();
        let env = OpenEnv {
            home_dir: home.path(),
            app_config: cfg.path(),
        };
        let p = preview_open(s, path, &env)?;
        confirm_open(s, &p.token, &env)
    }

    #[test]
    fn reopening_the_records_own_file_reuses_it_without_review() {
        let (_fx, s, ws) = store_with("Shop");
        let path = ws.file_path.clone().unwrap();
        let (home, cfg) = env_dirs();
        let p = preview_open(
            &s,
            &path,
            &OpenEnv {
                home_dir: home.path(),
                app_config: cfg.path(),
            },
        )
        .unwrap();
        assert!(!p.needs_confirmation && !p.blocked);
        let opened = open_file(&s, &path).unwrap();
        assert_eq!((opened.id, s.list().unwrap().len()), (ws.id, 1));
    }

    #[test]
    fn a_file_whose_id_matches_another_workspace_elsewhere_gets_a_new_record() {
        let (_fx, s, ws) = store_with("Shop");
        s.apply_workspace_settings(&ws.id, &btree(&[("jira", serde_json::json!({"site_url": "x"}))]))
            .unwrap();
        let before = s.get(&ws.id).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("Shop.teamcode-workspace.json");
        std::fs::copy(ws.file_path.as_ref().unwrap(), &copy).unwrap();
        assert!(
            matches!(open_file(&s, &copy), Err(Error::Precondition(_))),
            "an unknown path needs review"
        );
        let opened = open_reviewed(&s, &copy).unwrap();
        assert_ne!(opened.id, ws.id);
        assert_eq!(opened.file_path, Some(std::fs::canonicalize(&copy).unwrap()));
        assert!(opened.dirty, "the new id is written into the file on its next save");
        assert_eq!(s.get(&ws.id).unwrap(), before, "the existing record is untouched");
        let conn = s.db().lock().unwrap();
        assert!(
            crate::db::settings::get_raw(&conn, crate::settings::ScopeKind::Workspace, opened.id.as_str(), "jira")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn open_file_with_unknown_id_creates_record_after_review() {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ext.teamcode-workspace.json");
        std::fs::write(
            &path,
            file_json(
                "ws-e7",
                &[
                    ("api", fx.api.root.to_str().unwrap()),
                    ("web", fx.web.root.to_str().unwrap()),
                ],
            ),
        )
        .unwrap();
        let ws = open_reviewed(&s, &path).unwrap();
        assert_eq!((ws.id.as_str(), ws.projects.len()), ("ws-e7", 2));
        assert_eq!(
            ws.projects[0].project_id, fx.api.id,
            "the existing project is reused by root"
        );
        assert_eq!(s.fragments(&ws.id).unwrap()["plan"], "Both suites.");
        std::fs::remove_dir(&fx.web.root).unwrap();
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
    fn reload_of_a_file_that_gained_a_folder_needs_review() {
        let (fx, s, ws) = store_with("Shop");
        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("Shop.teamcode-workspace.json");
        let ws = s.save_as(&ws.id, &path).unwrap();
        std::fs::write(
            &path,
            file_json(
                ws.id.as_str(),
                &[
                    ("api", fx.api.root.to_str().unwrap()),
                    ("web", fx.web.root.to_str().unwrap()),
                ],
            ),
        )
        .unwrap();
        assert!(matches!(reload(&s, &ws.id, false), Err(Error::Precondition(_))));
        assert_eq!(
            open_reviewed(&s, &path).unwrap().id,
            ws.id,
            "after review the record at that path is reused"
        );
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
    fn relocate_file_after_the_file_moved() {
        let (_fx, s, ws) = store_with("Shop");
        let out = tempfile::tempdir().unwrap();
        let ws = s
            .save_as(&ws.id, &out.path().join("Shop.teamcode-workspace.json"))
            .unwrap();
        let moved = out.path().join("Moved.teamcode-workspace.json");
        std::fs::rename(ws.file_path.as_ref().unwrap(), &moved).unwrap();
        let ws2 = relocate_file(&s, &ws.id, &moved).unwrap();
        assert_eq!((ws2.id, ws2.file_path.as_deref()), (ws.id, Some(moved.as_path())));
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
