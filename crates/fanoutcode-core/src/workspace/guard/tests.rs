//! Guard tests.

use super::*;

const P: &str = "/w/api"; // protected; home_dir = /Users/j; default cwd = /w/web
fn deny(cmd: &str, cwd: &str) -> bool {
    matches!(
        evaluate(
            &GuardInput {
                command: cmd.into(),
                cwd: cwd.into()
            },
            &[PathBuf::from(P)],
            Path::new("/Users/j")
        ),
        GuardDecision::Deny { .. }
    )
}

#[test]
fn guard_cases() {
    let cases: &[(&str, &str, bool)] = &[
        ("rm -rf /w/api/dist", "/w/web", true),
        ("cat /w/api/README.md", "/w/web", false),
        ("echo hi > /w/api/notes.txt", "/w/web", true),
        ("npm test 2>&1 | tee /w/web/out.log", "/w/web", false),
        ("npm test 2>/dev/null", "/w/api", false),
        ("cd /w/api && npm install", "/w/web", true),
        ("cd /w/api && git status", "/w/web", false),
        ("git -C /w/api commit -m x", "/w/web", true),
        ("git -C /w/api log --oneline", "/w/web", false),
        ("sed -i '' 's/a/b/' /w/api/src/x.ts", "/w/web", true),
        ("sed -n 1p /w/api/src/x.ts", "/w/web", false),
        ("cp /w/api/schema.json /w/web/schema.json", "/w/web", false),
        ("cp /w/web/schema.json /w/api/schema.json", "/w/web", true),
        ("grep -r foo /w/api", "/w/web", false),
        ("ls /w/api; rm /w/web/tmp", "/w/web", false),
        ("(cd /w/api; touch x)", "/w/web", true),
        ("echo \"rm -rf /w/api\"", "/w/web", false),
        ("cd /w/api/sub && echo x > out.txt", "/w/web", true),
        ("cd /w/api && git branch --list", "/w/web", false),
        ("cd /w/api && git branch -D old", "/w/web", true),
        ("find /w/api -name '*.log' -delete", "/w/web", true),
        ("pnpm --dir /w/api add zod", "/w/web", true),
        ("git stash", "/w/api", true),
        ("python3 /w/api/scripts/gen.py", "/w/web", false), // documented best-effort gap
    ];
    for (cmd, cwd, expect) in cases {
        assert_eq!(deny(cmd, cwd), *expect, "command: {cmd} (cwd {cwd})");
    }
}

#[test]
fn more_shapes() {
    let cases: &[(&str, &str, bool)] = &[
        ("FOO=1 sudo -u me rm /w/api/x", "/w/web", true),
        ("cd ../api && touch x", "/w/web", true),
        ("cd /w/api && cd - && touch x", "/w/web", true), // `cd -` leaves the running cwd (conservative)
        ("rm /w/apiary/x", "/w/web", false),
        ("echo x >> \"/w/api/log\"", "/w/web", true),
        ("perl -pi -e 's/a/b/' /w/api/x", "/w/web", true),
        ("git -C /w/api fetch", "/w/web", false),
        ("mv /w/api/a /w/web/a", "/w/web", false),
        ("npm install --prefix=/w/api", "/w/web", true),
        ("find /w/web -exec rm {} +", "/w/web", false),
        ("xargs rm < list.txt", "/w/api", true),
    ];
    for (cmd, cwd, expect) in cases {
        assert_eq!(deny(cmd, cwd), *expect, "command: {cmd} (cwd {cwd})");
    }
}

#[test]
fn nested_sh_c_is_unwrapped() {
    assert!(deny("sh -c 'rm -rf /w/api/dist'", "/w/web"));
    assert!(deny("bash -lc \"cd /w/api && npm i\"", "/w/web"));
    assert!(!deny("sh -c 'ls /w/api'", "/w/web"));
}

#[test]
fn tilde_spelling_of_protected_root_is_matched() {
    let d = evaluate(
        &GuardInput {
            command: "rm -rf ~/w/api/dist".into(),
            cwd: "/Users/j".into(),
        },
        &[PathBuf::from("/Users/j/w/api")],
        Path::new("/Users/j"),
    );
    assert!(matches!(
        d,
        GuardDecision::Deny {
            rule: GuardRule::Program("rm"),
            ..
        }
    ));
}

#[test]
fn deny_reason_names_root_and_rule() {
    let r = deny_reason(Path::new("/w/api"), &GuardRule::Git("commit"));
    assert!(r.starts_with("Fanout Code: /w/api is read-only in this workspace") && r.contains("git commit"));
}

mod protocol {
    use super::super::*;
    use crate::domain::TaskId;
    use crate::Error;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn tid(s: &str) -> TaskId {
        serde_json::from_value(json!(s)).unwrap() // unvalidated, as an id from elsewhere would arrive
    }
    fn hook_json(cmd: &str, cwd: &str) -> Vec<u8> {
        serde_json::to_vec(
            &json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":cmd},"cwd":cwd}),
        )
        .unwrap()
    }
    fn roots(dir: &Path) -> PathBuf {
        write_roots_file(dir, &tid("t-1"), &[PathBuf::from("/w/api")], &[], Path::new("/Users/j")).unwrap()
    }

    #[test]
    fn run_guard_prints_deny_json_for_blocked_command() {
        let dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        assert_eq!(
            run_guard(
                &mut hook_json("rm -rf /w/api/x", "/w/web").as_slice(),
                &mut out,
                &roots(dir.path())
            ),
            0
        );
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
        assert!(v["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("/w/api"));
    }

    #[test]
    fn run_guard_prints_nothing_when_allowed_or_not_bash() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        let mut out = Vec::new();
        run_guard(&mut hook_json("ls /w/api", "/w/web").as_slice(), &mut out, &r);
        assert!(out.is_empty());
        let other =
            serde_json::to_vec(&json!({"tool_name":"Edit","tool_input":{"file_path":"/w/api/x"},"cwd":"/w/web"}))
                .unwrap();
        let mut out = Vec::new();
        run_guard(&mut other.as_slice(), &mut out, &r);
        assert!(out.is_empty());
    }

    #[test]
    fn run_guard_denies_shell_writes_to_the_overlap_files() {
        let dir = tempfile::tempdir().unwrap();
        let app_files: Vec<PathBuf> = ["grants", "claims", "notes", "events"]
            .iter()
            .map(|d| Path::new("/d").join(d))
            .collect();
        let r = write_roots_file(dir.path(), &tid("t-1"), &[], &app_files, Path::new("/Users/j")).unwrap();
        for cmd in [
            "echo x > /d/grants/t-a1.json",
            "rm /d/claims/0123456789abcdef.json",
            "cd /d/notes && touch t-a1.json",
            "sh -c 'cat x >> /d/events/t-a1.jsonl'",
        ] {
            let mut out = Vec::new();
            run_guard(&mut hook_json(cmd, "/w/web").as_slice(), &mut out, &r);
            let v: serde_json::Value = serde_json::from_slice(&out).unwrap_or_else(|_| panic!("denied: {cmd}"));
            assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny", "{cmd}");
            let reason = v["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap();
            assert!(
                reason.contains("Fanout Code's own records") && !reason.contains("linking policy"),
                "{reason}"
            );
        }
        let mut out = Vec::new();
        run_guard(
            &mut hook_json("cat /d/grants/t-a1.json", "/w/web").as_slice(),
            &mut out,
            &r,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_corrupt_roots_file_still_protects_app_data() {
        let dir = tempfile::tempdir().unwrap();
        let data = std::fs::canonicalize(dir.path()).unwrap();
        let roots = data.join("guards/t-1.guard.json");
        let run = |cmd: &str| {
            let mut out = Vec::new();
            run_guard(&mut hook_json(cmd, "/w/web").as_slice(), &mut out, &roots);
            String::from_utf8(out).unwrap().contains("\"deny\"")
        };
        let write = format!("echo x > {}/grants/t-1.json", data.display());
        // missing (the agent removed it), then corrupt (the agent emptied it)
        assert!(run(&write), "missing roots file");
        std::fs::create_dir_all(roots.parent().unwrap()).unwrap();
        std::fs::write(&roots, "").unwrap();
        assert!(run(&write), "empty roots file");
        std::fs::write(&roots, "{").unwrap();
        assert!(run(&write), "malformed roots file");
        assert!(run(&format!("rm {}", roots.display())));
        // anything else is allowed as before
        assert!(!run("echo x > /w/web/out.txt"));
        assert!(!run(&format!("cat {}/grants/t-1.json", data.display())));
    }

    #[test]
    fn run_guard_fails_open_on_bad_input_or_missing_roots() {
        let mut out = Vec::new();
        assert_eq!(
            run_guard(&mut b"not json".as_slice(), &mut out, Path::new("/nonexistent.json")),
            0
        );
        assert!(out.is_empty());
    }

    #[test]
    fn write_roots_file_validates_task_id() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            write_roots_file(dir.path(), &tid("../evil"), &[], &[], Path::new("/h")),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn guard_hook_spec_targets_bash_pre_tool_use_and_quotes_paths() {
        let s = guard_hook_spec(
            Path::new("/Applications/Fanout Code.app/Contents/MacOS/fanoutcode"),
            Path::new("/app/guards/t-1.guard.json"),
        );
        assert_eq!(
            s.command,
            "'/Applications/Fanout Code.app/Contents/MacOS/fanoutcode' guard '/app/guards/t-1.guard.json'"
        );
        assert_eq!(
            (s.event.as_str(), s.matcher.as_deref(), s.timeout_s),
            ("PreToolUse", Some("Bash"), Some(5))
        );
        assert!(guard_hook_spec(Path::new("/x/it's"), Path::new("/r"))
            .command
            .starts_with("'/x/it'\\''s'"));
    }

    #[test]
    fn prepare_guard_is_none_without_protected_roots_or_app_files() {
        let dir = tempfile::tempdir().unwrap();
        let exe = Path::new("/x/fanoutcode");
        assert!(prepare_guard(dir.path(), exe, &tid("t-1"), &[], &[], Path::new("/h"))
            .unwrap()
            .is_none());
        assert!(prepare_guard(
            dir.path(),
            exe,
            &tid("t-1"),
            &[PathBuf::from("/w/api")],
            &[],
            Path::new("/h")
        )
        .unwrap()
        .is_some());
        assert!(dir.path().join("guards/t-1.guard.json").exists());
        assert!(prepare_guard(
            dir.path(),
            exe,
            &tid("t-2"),
            &[],
            &[PathBuf::from("/d/grants")],
            Path::new("/h")
        )
        .unwrap()
        .is_some());
    }
}
