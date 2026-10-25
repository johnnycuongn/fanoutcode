//! A claim-check hook whose arguments no longer parse (a session started by an older Fanout Code) fails closed for
//! edits and git commands.

use serde_json::json;

use super::*;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn answer(payload: serde_json::Value) -> Option<serde_json::Value> {
    let mut out = Vec::new();
    assert_eq!(run_unparsed(&mut payload.to_string().as_bytes(), &mut out), 0);
    let text = String::from_utf8(out).unwrap();
    (!text.trim().is_empty()).then(|| serde_json::from_str(&text).unwrap())
}

#[test]
fn old_argument_shapes_deny_edits_and_git_commands_and_allow_the_rest() {
    assert_eq!(CheckArgs::parse(&["t-a1".into(), "/g.json".into()]), None);
    for tool in EDIT_TOOLS {
        let out = answer(json!({"tool_name": tool, "tool_input": {"file_path": "/r/a.ts"}, "cwd": "/r"}))
            .unwrap_or_else(|| panic!("{tool} is denied"));
        assert_eq!(out["hookSpecificOutput"]["permissionDecision"], "deny");
        assert_eq!(out["hookSpecificOutput"]["permissionDecisionReason"], STALE_HOOK_REASON);
    }
    assert_eq!(
        STALE_HOOK_REASON,
        "Fanout Code updated: restart this task's session to pick up the new hook."
    );
    for command in ["git add -A", "ls && git -C /r commit -am x", "sh -c 'git stash'"] {
        let out = answer(json!({"tool_name": "Bash", "tool_input": {"command": command}, "cwd": "/r"}));
        assert_eq!(
            out.unwrap()["hookSpecificOutput"]["permissionDecision"],
            "deny",
            "{command}"
        );
    }
    for payload in [
        json!({"tool_name": "Bash", "tool_input": {"command": "npm test"}, "cwd": "/r"}),
        json!({"tool_name": "Read", "tool_input": {"file_path": "/r/a.ts"}, "cwd": "/r"}),
        json!({"tool_name": "Grep", "tool_input": {"pattern": "git"}}),
    ] {
        assert_eq!(answer(payload.clone()), None, "{payload}");
    }
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
