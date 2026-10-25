use super::*;
use crate::domain::{LinkPolicy, Stage};
use crate::pipeline::ctx::{build_prompt_ctx, build_stage_prompts};
use crate::workspace::context_docs::ContextDocKind;
use crate::workspace::store::WorkspaceStore;
use crate::workspace::test_support::{fixture, Fixture};

/// A workspace of api and web, and a Claude Code task in web created there.
fn setup() -> (Fixture, WorkspaceStore, Workspace, Task) {
    let fx = fixture();
    let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
    let ws = s.create("Shop", &[fx.api.id.clone(), fx.web.id.clone()]).unwrap();
    let conn = fx.db.lock().unwrap();
    let mut t = crate::test_support::task_fixture("t-1", fx.web.id.as_str());
    t.workspace_id = Some(ws.id.clone());
    db::tasks::insert(&conn, &t).unwrap();
    drop(conn);
    (fx, s, ws, t)
}

#[test]
fn first_start_binds_once_and_records_links() {
    let (fx, _s, _ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    let (bound, needs_briefs) = bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
    assert_eq!(bound.linked_roots, vec![fx.api.root.clone()]);
    assert!(bound.links_bound_at.is_some() && needs_briefs);
    assert_eq!(load_links(&conn, &t.id).unwrap()[0].project_id, Some(fx.api.id.clone()));
    let (again, _) = bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
    assert_eq!(again.links_bound_at, bound.links_bound_at);
}

#[test]
fn protection_deny_rules_and_guard_follow_the_policy() {
    let (fx, s, ws, t) = setup();
    bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
    let exe = Path::new("/x/fanoutcode");
    let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap();
    let none = launch_extras(
        &fx.db.lock().unwrap(),
        &task,
        fx.app_data.path(),
        exe,
        Path::new("/h"),
        true,
        true,
    )
    .unwrap();
    assert!(none.deny.iter().all(|r| !r.contains(&*fx.api.root.to_string_lossy())));
    s.set_policy(&ws.id, LinkPolicy::ReadOnly, &[]).unwrap();
    let ro = launch_extras(
        &fx.db.lock().unwrap(),
        &task,
        fx.app_data.path(),
        exe,
        Path::new("/h"),
        true,
        true,
    )
    .unwrap();
    assert!(ro.deny.contains(&format!("Edit(/{}/**)", fx.api.root.display())));
    assert!(ro.guard.is_some());
}

#[test]
fn generated_settings_deny_edits_to_the_overlap_files() {
    let (fx, _s, _ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap(); // nothing linked, nothing protected
    let data = fx.app_data.path();
    let canonical = std::fs::canonicalize(data).unwrap();
    // whatever the overlap settings say: the hook trusts these files, so the agent never writes them
    for (ask, git) in [(true, true), (false, false)] {
        let extras = launch_extras(
            &conn,
            &task,
            data,
            Path::new("/x/fanoutcode"),
            Path::new("/h"),
            ask,
            git,
        )
        .unwrap();
        for dir in ["grants", "claims", "notes", "events", "guards", "claude-settings"] {
            for base in [data, canonical.as_path()] {
                let rule = format!("Edit(/{}/**)", base.join(dir).display());
                assert!(extras.deny.contains(&rule), "{rule} in {:?}", extras.deny);
            }
        }
        // and the Bash guard runs to refuse shell writes there, though no linked root is protected
        let guard = extras.guard.expect("the Bash guard");
        assert_eq!(guard.matcher.as_deref(), Some("Bash"));
    }
    let mut shell = task.clone();
    shell.agent_kind = AgentKind::Shell;
    let none = launch_extras(&conn, &shell, data, Path::new("/x"), Path::new("/h"), true, true).unwrap();
    assert!(none.deny.is_empty() && none.guard.is_none());
}

/// Runs the task's Bash guard (the roots file `launch_extras` wrote under `app_data`) on `command`.
fn guard_denies(app_data: &Path, task: &TaskId, command: &str) -> bool {
    let roots = app_data.join("guards").join(format!("{task}.guard.json"));
    let input = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}, "cwd": "/"});
    let mut out = Vec::new();
    crate::workspace::guard::run_guard(&mut input.to_string().as_bytes(), &mut out, &roots);
    String::from_utf8(out).unwrap().contains("\"deny\"")
}

#[test]
fn guard_denies_writes_to_its_own_roots_file() {
    let (fx, _s, _ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap();
    let data = fx.app_data.path();
    launch_extras(&conn, &task, data, Path::new("/x"), Path::new("/h"), true, true).unwrap();
    let roots = data.join("guards").join(format!("{}.guard.json", task.id));
    assert!(roots.exists());
    for cmd in [
        format!("echo '{{}}' > '{}'", roots.display()),
        format!("rm -f '{}'", roots.display()),
        format!("echo x > '{}/claude-settings/t-1.json'", data.display()),
    ] {
        assert!(guard_denies(data, &task.id, &cmd), "{cmd}");
    }
    assert!(!guard_denies(data, &task.id, &format!("cat '{}'", roots.display())));
}

#[test]
fn every_db_sidecar_is_protected() {
    let (fx, _s, _ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap();
    let data = fx.app_data.path();
    let canonical = std::fs::canonicalize(data).unwrap();
    let extras = launch_extras(&conn, &task, data, Path::new("/x"), Path::new("/h"), true, true).unwrap();
    // one prefix rule per path form covers the database and every sidecar, present or future
    for base in [data, canonical.as_path()] {
        let rule = format!("Edit(/{}/fanoutcode.db*)", base.display());
        assert!(extras.deny.contains(&rule), "{rule} in {:?}", extras.deny);
    }
    for name in [
        "fanoutcode.db",
        "fanoutcode.db-journal",
        "fanoutcode.db-wal",
        "fanoutcode.db-shm",
        "fanoutcode.db.bak",
    ] {
        for cmd in [
            format!("rm -f '{}'", data.join(name).display()),
            format!("echo x > '{}'", canonical.join(name).display()),
            format!("cp /tmp/evil.db '{}'", data.join(name).display()),
            format!("cd '{}' && rm -f {name}", data.display()),
        ] {
            assert!(guard_denies(data, &task.id, &cmd), "{cmd}");
        }
    }
    let db = data.join("fanoutcode.db");
    assert!(!guard_denies(
        data,
        &task.id,
        &format!("cat '{}' > /tmp/backup.db", db.display())
    ));
}

#[test]
fn the_guard_covers_both_forms_of_a_symlinked_app_data() {
    let (fx, _s, _ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap();
    let base = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(base.path()).unwrap().join("real");
    std::fs::create_dir(&real).unwrap();
    let alias = real.with_file_name("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    launch_extras(&conn, &task, &alias, Path::new("/x"), Path::new("/h"), true, true).unwrap();
    for data in [&alias, &real] {
        let cmd = format!("echo x > '{}/grants/t-a1.json'", data.display());
        assert!(guard_denies(&alias, &task.id, &cmd), "{cmd}");
    }
}

#[test]
fn a_task_whose_workspace_is_gone_keeps_its_links_protected() {
    let (fx, _s, ws, t) = setup();
    let conn = fx.db.lock().unwrap();
    bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
    // a record removed out from under the task (the foreign key would normally keep it)
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    conn.execute("DELETE FROM workspaces WHERE id = ?1", [ws.id.as_str()])
        .unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap();
    assert_eq!(protected_for(&conn, &task).unwrap(), vec![fx.api.root.clone()]);
}

#[test]
fn prompts_carry_policy_protection_references_and_stage_instructions() {
    let (fx, s, ws, t) = setup();
    bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
    s.set_policy(
        &ws.id,
        LinkPolicy::ReadAndModifyOnRequest,
        std::slice::from_ref(&fx.api.id),
    )
    .unwrap();
    s.set_fragment(&ws.id, "plan", Some("Run both suites.")).unwrap();
    let docs = crate::workspace::context_docs::ContextDocs::new(fx.db.clone());
    docs.add(
        &Container::Workspace { id: ws.id.clone() },
        ContextDocKind::Url,
        "https://x.y/spec",
        None,
    )
    .unwrap();
    let conn = fx.db.lock().unwrap();
    let task = db::tasks::require(&conn, &t.id).unwrap();
    let mut ctx = build_prompt_ctx(&conn, &task, &fx.web, fx.app_data.path(), fx.app_data.path()).unwrap();
    assert!(ctx.linked[0].protected);
    let prompts = build_stage_prompts(&mut ctx, fx.app_data.path(), fx.app_data.path()).unwrap();
    assert!(prompts.plan.contains("Run both suites.") && !prompts.build.contains("Run both suites."));
    assert!(prompts.plan.contains("url: https://x.y/spec — not fetched yet"));
    assert!(prompts.plan.contains("(protected: read only)"));
    let _ = Stage::Plan;
}

#[test]
fn launch_extras_adds_the_claim_check_hook() {
    let (fx, _s, _ws, t) = setup();
    bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
    let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap();
    let exe = Path::new("/x/fanoutcode");
    let extras = |ask: bool, git: bool| {
        launch_extras(
            &fx.db.lock().unwrap(),
            &task,
            fx.app_data.path(),
            exe,
            Path::new("/h"),
            ask,
            git,
        )
        .unwrap()
    };
    let spec = extras(true, false).claim_check.expect("a claim-check hook");
    assert!(
        spec.command.starts_with("'/x/fanoutcode' claim-check 't-1' "),
        "{}",
        spec.command
    );
    assert_eq!(spec.matcher.as_deref(), Some("Edit|Write|MultiEdit|NotebookEdit|Bash"));
    assert!(
        spec.command.contains(" 1 0 "),
        "ask on, git guard off: {}",
        spec.command
    );
    // the home root's claims file, then the linked root's
    let data = fx.app_data.path();
    let tail = format!(
        " '{}' '{}'",
        claims_path(data, &fx.web.root).display(),
        claims_path(data, &fx.api.root).display()
    );
    assert!(spec.command.ends_with(&tail), "{}", spec.command);
    assert!(spec
        .command
        .contains(&format!("'{}'", grants_path(data, &task.id).display())));
    assert!(spec
        .command
        .contains(&format!("'{}'", notes_path(data, &task.id).display())));
    assert!(spec
        .command
        .contains(&format!("'{}'", event_log_path(data, &task.id).display())));
    // with "Ask before editing" off the edit tools stay matched, so the hook logs the overlap (§8)
    let git_only = extras(false, true).claim_check.expect("the git guard alone");
    assert_eq!(
        git_only.matcher.as_deref(),
        Some("Edit|Write|MultiEdit|NotebookEdit|Bash")
    );
    assert!(git_only.command.contains(" 0 1 "), "{}", git_only.command);
    // both off: still there, to log overlaps and deliver notes; it never denies
    let neither = extras(false, false)
        .claim_check
        .expect("the hook for logging and notes");
    assert!(neither.command.contains(" 0 0 "), "{}", neither.command);
    let mut shell = task.clone();
    shell.agent_kind = AgentKind::Shell;
    let none = launch_extras(&fx.db.lock().unwrap(), &shell, data, exe, Path::new("/h"), true, true).unwrap();
    assert_eq!(none.claim_check, None);
}

#[test]
fn a_task_gets_only_its_own_projects_and_workspaces_cache() {
    let (fx, _s, ws, t) = setup();
    bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
    for c in [
        Container::Project { id: fx.api.id.clone() },
        Container::Project { id: fx.web.id.clone() },
        Container::Workspace { id: ws.id.clone() },
    ] {
        std::fs::create_dir_all(context_dir(fx.app_data.path(), &c).unwrap()).unwrap();
    }
    let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap(); // home: web
    let extras = launch_extras(
        &fx.db.lock().unwrap(),
        &task,
        fx.app_data.path(),
        Path::new("/x"),
        Path::new("/h"),
        false,
        false,
    )
    .unwrap();
    let api_cache = context_dir(fx.app_data.path(), &Container::Project { id: fx.api.id.clone() }).unwrap();
    assert!(extras.add_dirs.iter().all(|d| !d.starts_with(&api_cache)));
    assert_eq!(extras.add_dirs.len(), 2, "web's own cache and the workspace's");
}
