//! `WorkspaceStore` tests (kept apart so `store.rs` stays under 500 lines).

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
        Some(outside.path().join("shop.fanoutcode-workspace.json").as_path())
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

#[test]
fn save_if_dirty_writes_only_unsaved_changes() {
    let (fx, s, ws) = store_with("Shop");
    assert!(!s.save_if_dirty(&ws.id).unwrap(), "written through in app data");
    let out = tempfile::tempdir().unwrap();
    let ws = s.save_as(&ws.id, &out.path().join("Shop")).unwrap();
    s.add_project(&ws.id, &fx.web.id).unwrap();
    assert!(s.save_if_dirty(&ws.id).unwrap());
    let path = s.get(&ws.id).unwrap().file_path.unwrap();
    assert_eq!(
        WorkspaceFile::parse(&std::fs::read(path).unwrap())
            .unwrap()
            .projects
            .len(),
        2
    );
    assert!(!s.get(&ws.id).unwrap().dirty && !s.save_if_dirty(&ws.id).unwrap());
}
