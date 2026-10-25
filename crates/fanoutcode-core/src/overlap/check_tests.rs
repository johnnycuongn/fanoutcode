use serde_json::{json, Value};

use super::*;
use crate::overlap::claims::{write_claims, ANOTHER_SESSION};
use crate::overlap::grants::{write_grants, Grant, GrantScope};
use crate::overlap::notes::push_note;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn claim(path: &str, task: Option<&str>, title: &str) -> Claim {
    Claim {
        path: path.into(),
        task_id: task.map(id),
        title: title.into(),
        stage: None,
        state: None,
        last_edit: "2026-10-25T10:00:00Z".parse().unwrap(),
        staged: false,
    }
}

/// `/r` holds Login's (t-a1, this task) `src/mine.ts` and Checkout's (t-b2) `src/a.ts`.
fn repo(root: &Path) -> ClaimsFile {
    ClaimsFile {
        repo_root: root.to_owned(),
        written_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        claims: vec![
            claim("src/a.ts", Some("t-b2"), "Checkout"),
            claim("src/mine.ts", Some("t-a1"), "Login"),
        ],
    }
}

fn edit(path: &str, cwd: &str) -> CheckInput {
    CheckInput {
        tool_name: "Edit".into(),
        tool_input: json!({"file_path": path, "old_string": "a", "new_string": "b"}),
        cwd: cwd.into(),
    }
}

fn bash(command: &str) -> CheckInput {
    CheckInput {
        tool_name: "Bash".into(),
        tool_input: json!({ "command": command }),
        cwd: "/r".into(),
    }
}

fn decide(input: &CheckInput, grants: &GrantsFile, guard_git: bool) -> CheckDecision {
    evaluate(input, &id("t-a1"), &[repo(Path::new("/r"))], grants, guard_git)
}

/// A temp app-data dir with `/r`-shaped claims written for `root`, and the task's args.
struct Hook {
    _dir: tempfile::TempDir,
    root: PathBuf,
    args: CheckArgs,
}

fn hook(guard_git: bool) -> Hook {
    hook_with(true, guard_git)
}

fn hook_with(ask_before: bool, guard_git: bool) -> Hook {
    let dir = tempfile::tempdir().unwrap();
    let data = std::fs::canonicalize(dir.path()).unwrap();
    let root = data.join("repo");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let claims = write_claims(&data, &repo(&root)).unwrap();
    let args = CheckArgs {
        task_id: id("t-a1"),
        claims_paths: vec![claims, data.join("claims/missing.json")],
        grants_path: data.join("grants/t-a1.json"),
        notes_path: data.join("notes/t-a1.json"),
        log_path: data.join("events/t-a1.jsonl"),
        ask_before,
        guard_git,
    };
    Hook { _dir: dir, root, args }
}

fn run(h: &Hook, payload: &str) -> Option<Value> {
    let mut out = Vec::new();
    assert_eq!(run_check(&mut payload.as_bytes(), &mut out, &h.args), 0);
    let text = String::from_utf8(out).unwrap();
    (!text.trim().is_empty()).then(|| serde_json::from_str(&text).unwrap())
}

fn log_lines(h: &Hook) -> Vec<String> {
    std::fs::read_to_string(&h.args.log_path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn allows_when_no_claim() {
    assert_eq!(
        decide(&edit("/r/src/free.ts", "/r"), &GrantsFile::default(), true),
        CheckDecision::Allow
    );
}

#[test]
fn allows_own_claim() {
    assert_eq!(
        decide(&edit("/r/src/mine.ts", "/r"), &GrantsFile::default(), true),
        CheckDecision::Allow
    );
}

#[test]
fn a_file_both_tasks_claim_still_asks_about_the_other() {
    // A (this task) edited on top of B's claim earlier; B still has uncommitted changes there
    let mut file = repo(Path::new("/r"));
    file.claims.push(claim("src/both.ts", Some("t-a1"), "Login"));
    file.claims.push(claim("src/both.ts", Some("t-b2"), "Checkout"));
    let ask = |grants: &GrantsFile| {
        evaluate(
            &edit("/r/src/both.ts", "/r"),
            &id("t-a1"),
            &[file.clone()],
            grants,
            true,
        )
    };
    assert!(matches!(ask(&GrantsFile::default()),
        CheckDecision::Deny { other: Some(ref c), .. } if c.task_id == Some(id("t-b2"))));
    // A's own grant on B covers it, by file or for every file
    let at = "2026-10-25T10:00:00Z".parse().unwrap();
    for scope in [
        GrantScope::File {
            repo: "/r".into(),
            path: "src/both.ts".into(),
        },
        GrantScope::AllFiles,
    ] {
        let grants = GrantsFile {
            grants: vec![Grant {
                other_task: Some(id("t-b2")),
                scope,
                granted_at: at,
            }],
        };
        assert_eq!(ask(&grants), CheckDecision::Allow);
    }
}

#[test]
fn picks_the_deepest_repo_for_an_edit() {
    // `/r/linked` is a repo of its own inside `/r`; its claims, not `/r`'s, decide a file under it
    let inner = ClaimsFile {
        repo_root: "/r/linked".into(),
        written_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        claims: vec![claim("x.ts", Some("t-c3"), "Search")],
    };
    let mut outer = repo(Path::new("/r"));
    outer.claims.push(claim("linked/free.ts", Some("t-b2"), "Checkout"));
    for claims in [vec![outer.clone(), inner.clone()], vec![inner, outer]] {
        let g = GrantsFile::default();
        assert!(
            matches!(evaluate(&edit("/r/linked/x.ts", "/r"), &id("t-a1"), &claims, &g, true),
                CheckDecision::Deny { ref repo, path: Some(ref p), .. } if repo == Path::new("/r/linked") && p == "x.ts")
        );
        // the outer file's claim on the same path does not count: the inner repo holds it
        assert_eq!(
            evaluate(&edit("/r/linked/free.ts", "/r"), &id("t-a1"), &claims, &g, true),
            CheckDecision::Allow
        );
    }
}

#[test]
fn denies_other_tasks_claim_with_reason_and_logs_claim_denied() {
    assert_eq!(
        decide(&edit("/r/src/a.ts", "/r"), &GrantsFile::default(), false),
        CheckDecision::Deny {
            repo: "/r".into(),
            path: Some("src/a.ts".into()),
            command: None,
            other: Some(claim("src/a.ts", Some("t-b2"), "Checkout")),
            reason: deny_reason("src/a.ts", "Checkout"),
        }
    );
    assert_eq!(
        deny_reason("src/a.ts", "Checkout"),
        "Fanout Code: another task, Checkout, has uncommitted changes in src/a.ts. The developer is being asked how \
         to proceed. Do not edit this file or work around the block; finish anything unrelated you were doing, then \
         stop and wait for the developer's instruction."
    );
    let h = hook(false);
    let payload = json!({"tool_name": "Write", "tool_input": {"file_path": h.root.join("src/a.ts"), "content": "x"},
                         "cwd": h.root});
    let out = run(&h, &payload.to_string()).unwrap();
    assert_eq!(out["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        out["hookSpecificOutput"]["permissionDecisionReason"],
        deny_reason("src/a.ts", "Checkout")
    );
    let lines = log_lines(&h);
    assert_eq!(lines.len(), 1);
    let ev = crate::hooks::parse::parse_line(&id("t-a1"), 1, &lines[0]).unwrap();
    assert_eq!(
        ev.event,
        crate::hooks::HookEvent::ClaimDenied {
            repo: h.root.clone(),
            path: Some("src/a.ts".into()),
            command: None,
            cwd: Some(h.root.clone()),
            other_task: Some(id("t-b2")),
        }
    );
}

#[test]
fn allows_when_a_grant_covers() {
    let at = "2026-10-25T10:00:00Z".parse().unwrap();
    let file = GrantsFile {
        grants: vec![Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::File {
                repo: "/r".into(),
                path: "src/a.ts".into(),
            },
            granted_at: at,
        }],
    };
    assert_eq!(decide(&edit("/r/src/a.ts", "/r"), &file, true), CheckDecision::Allow);
    // a grant on B for another file, or on C, does not
    let other = GrantsFile {
        grants: vec![Grant {
            other_task: Some(id("t-c3")),
            scope: GrantScope::AllFiles,
            granted_at: at,
        }],
    };
    assert!(matches!(
        decide(&edit("/r/src/a.ts", "/r"), &other, true),
        CheckDecision::Deny { .. }
    ));
    // through the files on disk
    let h = hook(false);
    let all = GrantsFile {
        grants: vec![Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::AllFiles,
            granted_at: at,
        }],
    };
    write_grants(
        h.args.grants_path.parent().unwrap().parent().unwrap(),
        &id("t-a1"),
        &all,
    )
    .unwrap();
    let payload = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/a.ts")}, "cwd": h.root});
    assert_eq!(run(&h, &payload.to_string()), None);
    assert!(log_lines(&h).is_empty());
}

#[test]
fn allows_a_path_outside_every_repo() {
    let g = GrantsFile::default();
    for p in [
        "/tmp/scratch/src/a.ts",
        "/app-data/memory/t-a1/notes.md",
        "/rr/src/a.ts",
    ] {
        assert_eq!(decide(&edit(p, "/r"), &g, true), CheckDecision::Allow, "{p}");
    }
    // a relative path that climbs out of the repo
    assert_eq!(
        decide(&edit("../elsewhere/src/a.ts", "/r"), &g, true),
        CheckDecision::Allow
    );
}

#[test]
fn resolves_a_relative_path_against_cwd() {
    let g = GrantsFile::default();
    for (path, cwd) in [("src/a.ts", "/r"), ("a.ts", "/r/src"), ("./lib/../a.ts", "/r/src")] {
        assert!(
            matches!(decide(&edit(path, cwd), &g, false),
                CheckDecision::Deny { path: Some(ref p), .. } if p == "src/a.ts"),
            "{path} in {cwd}"
        );
    }
    let notebook = CheckInput {
        tool_name: "NotebookEdit".into(),
        tool_input: json!({"notebook_path": "a.ts"}),
        cwd: "/r/src".into(),
    };
    assert!(matches!(decide(&notebook, &g, false), CheckDecision::Deny { .. }));
}

#[test]
fn an_unattributed_claim_denies_as_another_session() {
    let mut file = repo(Path::new("/r"));
    file.claims.push(claim("gen.ts", None, ANOTHER_SESSION));
    let d = evaluate(
        &edit("/r/gen.ts", "/r"),
        &id("t-a1"),
        &[file],
        &GrantsFile::default(),
        true,
    );
    assert!(matches!(d, CheckDecision::Deny { reason, other: Some(c), .. }
        if c.task_id.is_none() && reason.contains("another task, another session,")));
}

#[test]
fn a_git_command_over_another_tasks_claim_is_denied_and_logged() {
    let d = decide(&bash("git add -A && git commit -m wip"), &GrantsFile::default(), true);
    assert_eq!(
        d,
        CheckDecision::Deny {
            repo: "/r".into(),
            path: None,
            command: Some("git add -A && git commit -m wip".into()),
            other: Some(claim("src/a.ts", Some("t-b2"), "Checkout")),
            reason: git_deny_reason(
                crate::overlap::CommandKind::Commits,
                &[&claim("src/a.ts", Some("t-b2"), "Checkout")]
            ),
        }
    );
    // the task's own files never trip it
    assert_eq!(
        decide(&bash("git add src/mine.ts"), &GrantsFile::default(), true),
        CheckDecision::Allow
    );
    let h = hook(true);
    let payload = json!({"tool_name": "Bash", "tool_input": {"command": "git stash"}, "cwd": h.root.join("src")});
    let out = run(&h, &payload.to_string()).unwrap();
    assert_eq!(out["hookSpecificOutput"]["permissionDecision"], "deny");
    let ev = crate::hooks::parse::parse_line(&id("t-a1"), 1, &log_lines(&h)[0]).unwrap();
    assert!(
        matches!(ev.event, crate::hooks::HookEvent::ClaimDenied { path: None, command: Some(ref c), .. }
        if c == "git stash")
    );
}

#[test]
fn drains_notes_into_additional_context_on_allow() {
    let h = hook(false);
    push_note(
        &h.args.notes_path,
        "Fanout Code: task Checkout is now editing src/a.ts.",
    )
    .unwrap();
    push_note(&h.args.notes_path, "Second.").unwrap();
    let payload = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/free.ts")}, "cwd": h.root});
    let out = run(&h, &payload.to_string()).unwrap();
    assert_eq!(
        out,
        json!({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": "Fanout Code: task Checkout is now editing src/a.ts.\n\nSecond.",
        }})
    );
    assert_eq!(run(&h, &payload.to_string()), None, "drained");
    // a denied call keeps the notes for later
    push_note(&h.args.notes_path, "Later.").unwrap();
    let denied = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/a.ts")}, "cwd": h.root});
    assert_eq!(
        run(&h, &denied.to_string()).unwrap()["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );
    assert!(h.args.notes_path.exists());
}

#[test]
fn malformed_input_allows() {
    let h = hook(true);
    assert_eq!(run(&h, "not json"), None);
    assert_eq!(run(&h, "{\"tool_name\": \"Edit\"}"), None);
    let payload = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/a.ts")}, "cwd": h.root});
    // an unreadable grants file is no grants: with the claims readable, the developer is asked (never allows more
    // than they said)
    std::fs::create_dir_all(h.args.grants_path.parent().unwrap()).unwrap();
    std::fs::write(&h.args.grants_path, "[").unwrap();
    assert_eq!(
        run(&h, &payload.to_string()).unwrap()["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );
    assert_eq!(log_lines(&h).len(), 1);
    // an unreadable claims file fails open
    let claims = std::fs::read(&h.args.claims_paths[0]).unwrap();
    std::fs::write(&h.args.claims_paths[0], "{").unwrap();
    assert_eq!(run(&h, &payload.to_string()), None);
    assert_eq!(log_lines(&h).len(), 1);
    // and with it readable again and a grant written, the same edit passes
    std::fs::write(&h.args.claims_paths[0], claims).unwrap();
    let all = GrantsFile {
        grants: vec![Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::AllFiles,
            granted_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        }],
    };
    std::fs::write(&h.args.grants_path, serde_json::to_vec(&all).unwrap()).unwrap();
    assert_eq!(run(&h, &payload.to_string()), None);
}

#[test]
fn non_edit_tool_without_git_guard_allows() {
    let g = GrantsFile::default();
    assert_eq!(decide(&bash("git add -A"), &g, false), CheckDecision::Allow);
    let read = CheckInput {
        tool_name: "Read".into(),
        tool_input: json!({"file_path": "/r/src/a.ts"}),
        cwd: "/r".into(),
    };
    assert_eq!(decide(&read, &g, true), CheckDecision::Allow);
}

#[test]
fn args_round_trip_through_the_hook_command() {
    let args = CheckArgs {
        task_id: id("t-a1"),
        claims_paths: vec!["/d/claims/1.json".into(), "/d/claims/it's.json".into()],
        grants_path: "/d/grants/t-a1.json".into(),
        notes_path: "/d/notes/t-a1.json".into(),
        log_path: "/d/events/t-a1.jsonl".into(),
        ask_before: false,
        guard_git: true,
    };
    let spec = check_hook_spec(Path::new("/Apps/Fanout Code/fanoutcode"), &args);
    assert_eq!((spec.event.as_str(), spec.timeout_s), ("PreToolUse", Some(5)));
    assert_eq!(spec.matcher.as_deref(), Some("Edit|Write|MultiEdit|NotebookEdit|Bash"));
    assert_eq!(
        spec.command,
        "'/Apps/Fanout Code/fanoutcode' claim-check 't-a1' '/d/grants/t-a1.json' '/d/notes/t-a1.json' \
         '/d/events/t-a1.jsonl' 0 1 '/d/claims/1.json' '/d/claims/it'\\''s.json'"
    );
    let argv: Vec<String> = [
        "t-a1",
        "/d/grants/t-a1.json",
        "/d/notes/t-a1.json",
        "/d/events/t-a1.jsonl",
        "0",
        "1",
        "/d/claims/1.json",
        "/d/claims/it's.json",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(CheckArgs::parse(&argv), Some(args));
    assert_eq!(CheckArgs::parse(&argv[..5]), None);
    let mut bad = argv.clone();
    bad[0] = "../x".into();
    assert_eq!(CheckArgs::parse(&bad), None);
    for flag in [4, 5] {
        let mut bad = argv.clone();
        bad[flag] = "yes".into();
        assert_eq!(CheckArgs::parse(&bad), None, "flag {flag}");
    }
    // ask on, guard off
    let mut on = argv;
    on[4] = "1".into();
    on[5] = "0".into();
    let parsed = CheckArgs::parse(&on).unwrap();
    assert!(parsed.ask_before && !parsed.guard_git);
    assert!(check_hook_spec(Path::new("/x"), &parsed)
        .command
        .contains("'/d/events/t-a1.jsonl' 1 0 "));
}

#[test]
fn ask_off_allows_and_logs_claim_noted() {
    let h = hook_with(false, false);
    push_note(&h.args.notes_path, "Fanout Code: a note.").unwrap();
    let payload = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/a.ts")}, "cwd": h.root});
    // allowed: no decision, only the queued notes
    let out = run(&h, &payload.to_string()).unwrap();
    assert_eq!(out["hookSpecificOutput"].get("permissionDecision"), None);
    assert_eq!(out["hookSpecificOutput"]["additionalContext"], "Fanout Code: a note.");
    // and the overlap logged, under a name nothing pins on
    let lines = log_lines(&h);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("ClaimNoted\t"), "{}", lines[0]);
    let ev = crate::hooks::parse::parse_line(&id("t-a1"), 1, &lines[0]).unwrap();
    assert_eq!(
        ev.event,
        crate::hooks::HookEvent::ClaimNoted {
            repo: h.root.clone(),
            path: Some("src/a.ts".into()),
            command: None,
            other_task: Some(id("t-b2")),
        }
    );
    // a free file logs nothing
    let free = json!({"tool_name": "Edit", "tool_input": {"file_path": h.root.join("src/free.ts")}, "cwd": h.root});
    assert_eq!(run(&h, &free.to_string()), None);
    assert_eq!(log_lines(&h).len(), 1);
}
