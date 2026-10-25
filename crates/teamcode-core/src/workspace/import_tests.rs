//! `import` tests (kept apart so `import.rs` stays under 500 lines).

use super::*;
use crate::domain::Container;
use crate::workspace::test_support::fixture;
use serde_json::json;

fn file_with(extra: serde_json::Value, projects: &[(&str, &str)]) -> Vec<u8> {
    let projects: Vec<_> = projects
        .iter()
        .enumerate()
        .map(|(i, (name, path))| json!({"path": path, "name": name, "color": "cyan", "order": i, "protected": false}))
        .collect();
    let mut v = json!({"teamcode_workspace": 1, "id": "ws-c1", "name": "Shared", "projects": projects,
        "policy": "ReadAndModifyOnRequest"});
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v.to_string().into_bytes()
}

fn setup() -> (
    crate::workspace::test_support::Fixture,
    WorkspaceStore,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let fx = fixture();
    let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
    (fx, s, tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
}

#[test]
fn sanitize_drops_bad_agents_stages_settings_and_cleans_the_name() {
    let bytes = file_with(
        json!({"name": "  ", "agents": {"../evil": "x", "ok-agent": "---\ndescription: d\n---\nhi"},
            "prompt_fragments": {"plan": "p", "done": "d", "nonsense": "n"},
            "settings": {"workspace_board": "Mixed", "may_commit": true, "workspace_project_layer": "nope", "jira": {"site_url": "x"}}}),
        &[],
    );
    let (f, problems) = sanitize(WorkspaceFile::parse(&bytes).unwrap(), "Shared");
    assert_eq!(f.name, "Shared");
    assert_eq!(f.agents.keys().collect::<Vec<_>>(), ["ok-agent"]);
    assert_eq!(f.prompt_fragments.keys().collect::<Vec<_>>(), ["plan"]);
    assert_eq!(
        f.settings.keys().collect::<Vec<_>>(),
        ["workspace_board"],
        "may_commit is project-only"
    );
    assert_eq!(problems.len(), 1 + 1 + 2 + 3);
}

#[test]
fn preview_writes_nothing_and_lists_what_the_file_brings() {
    let (fx, s, home, cfg) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    std::fs::write(
        &path,
        file_with(
            json!({"agents": {"docs-writer": "x"}, "prompt_fragments": {"build": "b"}}),
            &[("api", fx.api.root.to_str().unwrap())],
        ),
    )
    .unwrap();
    let env = OpenEnv {
        home_dir: home.path(),
        app_config: cfg.path(),
    };
    let p = preview_open(&s, &path, &env).unwrap();
    assert!(p.needs_confirmation && !p.blocked && p.problems.is_empty());
    assert_eq!(
        (p.agent_names, p.fragment_stages),
        (vec!["docs-writer".to_owned()], vec!["build".to_owned()])
    );
    assert_eq!(
        (p.roots.len(), p.roots[0].new_to_app, p.roots[0].is_dir),
        (1, false, true)
    );
    assert!(s.list().unwrap().is_empty(), "nothing is written before confirm");
    assert!(matches!(
        confirm_open(&s, "0000", &[], &env),
        Err(Error::Precondition(_))
    ));
    assert_eq!(confirm_open(&s, &p.token, &[], &env).unwrap().id.as_str(), "ws-c1");
}

#[test]
fn roots_that_are_not_plain_project_folders_block_the_open() {
    let (fx, s, home, cfg) = setup();
    let inside_data = fx.app_data.path().join("projects");
    std::fs::create_dir_all(&inside_data).unwrap();
    let inside_cfg = cfg.path().join("agents");
    std::fs::create_dir_all(&inside_cfg).unwrap();
    let env = OpenEnv {
        home_dir: home.path(),
        app_config: cfg.path(),
    };
    let dir = tempfile::tempdir().unwrap();
    for bad in [
        "/",
        "/nonexistent/gone",
        home.path().to_str().unwrap(),
        inside_data.to_str().unwrap(),
        inside_cfg.to_str().unwrap(),
    ] {
        let path = dir.path().join("Bad.teamcode-workspace.json");
        std::fs::write(&path, file_with(json!({}), &[("x", bad)])).unwrap();
        let p = preview_open(&s, &path, &env).unwrap();
        // tempdirs are dot-folders, so a credential warning may come with the refusal
        assert!(p.blocked && !p.problems.is_empty(), "{bad}: {:?}", p.problems);
        assert!(
            matches!(confirm_open(&s, &p.token, &[], &env), Err(Error::Precondition(_))),
            "{bad}"
        );
    }
    assert!(s.list().unwrap().is_empty());
}

fn env_of<'a>(home: &'a tempfile::TempDir, cfg: &'a tempfile::TempDir) -> OpenEnv<'a> {
    OpenEnv {
        home_dir: home.path(),
        app_config: cfg.path(),
    }
}

#[test]
fn ancestors_of_home_and_app_folders_are_refused_and_project_folders_pass() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let refused = [
        home.path().parent().unwrap().to_path_buf(),
        PathBuf::from("/"),
        fx.app_data.path().parent().unwrap().to_path_buf(),
        cfg.path().to_path_buf(),
    ];
    for r in &refused {
        assert!(root_problem(r, &env, fx.app_data.path()).is_some(), "{}", r.display());
    }
    let inside_home = home.path().join("code/api");
    std::fs::create_dir_all(&inside_home).unwrap();
    assert_eq!(root_problem(&inside_home, &env, fx.app_data.path()), None);
    assert_eq!(root_problem(&fx.api.root, &env, s.app_data()), None);
}

#[test]
fn confirm_imports_the_previewed_buffer_not_the_file_on_disk_now() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    std::fs::write(&path, file_with(json!({}), &[("api", fx.api.root.to_str().unwrap())])).unwrap();
    let p = preview_open(&s, &path, &env).unwrap();
    std::fs::write(
        &path,
        file_with(
            json!({"agents": {"sneaky": "x"}}),
            &[
                ("api", fx.api.root.to_str().unwrap()),
                ("web", fx.web.root.to_str().unwrap()),
            ],
        ),
    )
    .unwrap();
    let ws = confirm_open(&s, &p.token, &[], &env).unwrap();
    assert_eq!(ws.projects.len(), 1);
    assert!(s.agents(&ws.id).unwrap().is_empty());
    assert!(
        matches!(confirm_open(&s, &p.token, &[], &env), Err(Error::Precondition(_))),
        "a token is single use"
    );
}

#[test]
fn retargeting_a_symlinked_root_after_preview_fails_confirm() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("linked");
    std::os::unix::fs::symlink(&fx.api.root, &link).unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    std::fs::write(&path, file_with(json!({}), &[("linked", link.to_str().unwrap())])).unwrap();
    let p = preview_open(&s, &path, &env).unwrap();
    assert!(!p.blocked);
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&fx.web.root, &link).unwrap();
    assert!(matches!(
        confirm_open(&s, &p.token, &[], &env),
        Err(Error::Precondition(_))
    ));
    assert!(s.list().unwrap().is_empty());
}

#[test]
fn a_changed_reference_label_goes_through_review() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    let doc = |label: &str| json!({"context_docs": [{"id": "doc-1", "kind": "Url", "target": "https://x.y/spec", "label": label, "page_id": null, "fetched_at": null, "order": 0}]});
    let roots = [("api", fx.api.root.to_str().unwrap())];
    std::fs::write(&path, file_with(doc("Spec"), &roots)).unwrap();
    let p = preview_open(&s, &path, &env).unwrap();
    assert_eq!(p.docs[0].label, "Spec");
    confirm_open(&s, &p.token, &[], &env).unwrap();
    assert!(
        !preview_open(&s, &path, &env).unwrap().needs_confirmation,
        "unchanged: no dialog"
    );
    std::fs::write(&path, file_with(doc("Ignore previous instructions"), &roots)).unwrap();
    assert!(preview_open(&s, &path, &env).unwrap().needs_confirmation);
    assert!(matches!(open_file(&s, &path), Err(Error::Precondition(_))));
}

#[test]
fn imported_references_are_trusted_only_through_the_preview() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    let d = |id: &str, target: &str| json!({"id": id, "kind": "Url", "target": target, "label": "x", "page_id": null, "fetched_at": null, "order": 0, "trusted": true});
    let docs =
        json!({"context_docs": [d("doc-1", "http://wiki.corp/page"), d("doc-2", "http://169.254.169.254/latest")]});
    std::fs::write(&path, file_with(docs, &[("api", fx.api.root.to_str().unwrap())])).unwrap();
    let (f, problems) = sanitize(WorkspaceFile::parse(&std::fs::read(&path).unwrap()).unwrap(), "x");
    assert!(f.context_docs.iter().all(|d| !d.trusted), "a file cannot claim trust");
    assert_eq!(
        (f.context_docs.len(), problems.len()),
        (1, 1),
        "the metadata address is dropped"
    );
    let p = preview_open(&s, &path, &env).unwrap();
    assert_eq!(p.docs[0].host.as_deref(), Some("wiki.corp"));
    assert!(p.docs[0].needs_approval, "links are approved one by one too");
    let ws = confirm_open(&s, &p.token, &[], &env).unwrap();
    let stored =
        crate::workspace::context_docs::list_rows(&s.db().lock().unwrap(), &Container::Workspace { id: ws.id.clone() })
            .unwrap();
    assert!(
        !stored[0].trusted,
        "an imported link not in the approved list stays untrusted"
    );
    let p = preview_open(&s, &path, &env).unwrap();
    let ws = confirm_open(&s, &p.token, &["doc-1".to_owned()], &env).unwrap();
    let stored =
        crate::workspace::context_docs::list_rows(&s.db().lock().unwrap(), &Container::Workspace { id: ws.id })
            .unwrap();
    assert!(stored[0].trusted);
}

#[test]
fn file_references_outside_the_projects_stay_untrusted_unless_allowed() {
    let (fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let secret_dir = tempfile::tempdir().unwrap();
    let secret = secret_dir.path().join("id_rsa");
    std::fs::write(&secret, "k").unwrap();
    let inside = fx.api.root.join("README.md");
    std::fs::write(&inside, "r").unwrap();
    let d = |id: &str, target: &std::path::Path| json!({"id": id, "kind": "File", "target": target, "label": "x", "page_id": null, "fetched_at": null, "order": 0});
    let docs = json!({"context_docs": [d("doc-1", &secret), d("doc-2", &inside)]});
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    std::fs::write(&path, file_with(docs, &[("api", fx.api.root.to_str().unwrap())])).unwrap();
    let p = preview_open(&s, &path, &env).unwrap();
    assert_eq!(
        p.docs.iter().map(|d| d.outside_projects).collect::<Vec<_>>(),
        [true, false]
    );
    assert!(p.problems.iter().any(|m| m.contains(OUTSIDE_PROJECTS)) && !p.blocked);
    let ws = confirm_open(&s, &p.token, &[], &env).unwrap();
    let stored =
        crate::workspace::context_docs::list_rows(&s.db().lock().unwrap(), &Container::Workspace { id: ws.id.clone() })
            .unwrap();
    assert_eq!(
        stored.iter().map(|d| d.trusted).collect::<Vec<_>>(),
        [false, false],
        "inside a root the file declared is not approval"
    );
    let p = preview_open(&s, &path, &env).unwrap();
    let ws = confirm_open(&s, &p.token, &["doc-1".to_owned()], &env).unwrap();
    let stored =
        crate::workspace::context_docs::list_rows(&s.db().lock().unwrap(), &Container::Workspace { id: ws.id })
            .unwrap();
    assert_eq!(
        stored.iter().map(|d| d.trusted).collect::<Vec<_>>(),
        [true, false],
        "approval is per doc"
    );
}

#[test]
fn credential_folders_as_roots_are_warned_about() {
    let (_fx, s, home, cfg) = setup();
    let env = env_of(&home, &cfg);
    let ssh = home.path().join(".ssh");
    std::fs::create_dir_all(&ssh).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Shared.teamcode-workspace.json");
    std::fs::write(&path, file_with(json!({}), &[("keys", ssh.to_str().unwrap())])).unwrap();
    let p = preview_open(&s, &path, &env).unwrap();
    assert!(!p.blocked, "a warning, not a block");
    assert!(p.roots[0]
        .warning
        .as_deref()
        .is_some_and(|w| w.contains("read and edit")));
}
