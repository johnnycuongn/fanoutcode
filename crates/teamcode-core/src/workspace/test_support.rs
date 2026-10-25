//! Fixtures for the workspace tests: a shared migrated database, an app-data folder and two projects rooted in
//! real temp folders.
#![allow(dead_code)]

use std::collections::BTreeMap;

use tempfile::TempDir;

use super::store::WorkspaceStore;
use crate::db::{self, Db};
use crate::domain::{MemoryLocation, Project, ProjectColor, ProjectId, Workspace};

pub struct Fixture {
    pub db: Db,
    pub app_data: TempDir,
    pub api: Project,
    pub web: Project,
    _roots: TempDir,
}

fn project(id: &str, root: std::path::PathBuf, color: &str) -> Project {
    Project {
        id: ProjectId::parse(id).unwrap(),
        name: root.file_name().unwrap().to_string_lossy().into_owned(),
        root,
        color: ProjectColor(color.into()),
        memory_location: MemoryLocation::AppStorage,
        created_at: chrono::Utc::now(),
    }
}

/// Projects `api` (`p-a1`) and `web` (`p-b2`), each rooted in a real folder.
pub fn fixture() -> Fixture {
    let conn = db::open_in_memory().unwrap();
    let roots = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(roots.path()).unwrap();
    let (api_root, web_root) = (base.join("api"), base.join("web"));
    std::fs::create_dir_all(&api_root).unwrap();
    std::fs::create_dir_all(&web_root).unwrap();
    let api = project("p-a1", api_root, "indigo");
    let web = project("p-b2", web_root, "cyan");
    db::projects::insert(&conn, &api).unwrap();
    db::projects::insert(&conn, &web).unwrap();
    Fixture {
        db: db::shared(conn),
        app_data: tempfile::tempdir().unwrap(),
        api,
        web,
        _roots: roots,
    }
}

/// A store and a workspace named `name` holding `api` only.
pub fn store_with(name: &str) -> (Fixture, WorkspaceStore, Workspace) {
    let fx = fixture();
    let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
    let ws = s.create(name, std::slice::from_ref(&fx.api.id)).unwrap();
    (fx, s, ws)
}

/// As `store_with`, holding `api` then `web`.
pub fn store_with_both(name: &str) -> (Fixture, WorkspaceStore, Workspace) {
    let fx = fixture();
    let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
    let ws = s.create(name, &[fx.api.id.clone(), fx.web.id.clone()]).unwrap();
    (fx, s, ws)
}

pub fn btree(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<String, serde_json::Value> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect()
}
