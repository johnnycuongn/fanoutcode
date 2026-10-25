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
        ("mv /w/api/a /w/web/a", "/w/web", true), // moving a file out writes the root it leaves
        ("npm install --prefix=/w/api", "/w/web", true),
        ("find /w/web -exec rm {} +", "/w/web", false),
        ("xargs rm < list.txt", "/w/api", true),
    ];
    for (cmd, cwd, expect) in cases {
        assert_eq!(deny(cmd, cwd), *expect, "command: {cmd} (cwd {cwd})");
    }
}

#[test]
fn moving_a_protected_folder_is_denied() {
    for cmd in [
        "mv /w/api /tmp/x",
        "mv /w /tmp/y",
        "mv /w/api/src /tmp/z",
        "rm -rf /w",
        "rm -r /w/api/..",
        "chmod -R 000 /w",
        "find /w -name '*.ts' -delete",
        "cd /w && find . -delete",
    ] {
        assert!(deny(cmd, "/elsewhere"), "{cmd}");
    }
    // reads of an ancestor, and running inside one, are not writes to the root
    for cmd in ["ls /w", "cat /w/README", "cd /w && rm notes.txt", "cp /w/api/x /tmp/x"] {
        assert!(!deny(cmd, "/elsewhere"), "{cmd}");
    }
}

/// `<tmp>/data/app` is app data with its overlap folders, guarded through `run_guard` as a launch writes it.
pub(super) fn app_data_guard() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap().join("data");
    let app = base.join("app");
    let files: Vec<PathBuf> = ["grants", "claims"].iter().map(|d| app.join(d)).collect();
    for f in &files {
        std::fs::create_dir_all(f).unwrap();
    }
    let roots = write_roots_file(&app.join("guards"), &"t-1".parse_id(), &[], &files, Path::new("/h")).unwrap();
    (dir, app, roots)
}

trait ParseId {
    fn parse_id(&self) -> crate::domain::TaskId;
}
impl ParseId for str {
    fn parse_id(&self) -> crate::domain::TaskId {
        crate::domain::TaskId::parse(self).unwrap()
    }
}

fn guard_says_deny(roots: &Path, cmd: &str) -> bool {
    let input = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": cmd}, "cwd": "/"});
    let mut out = Vec::new();
    run_guard(&mut input.to_string().as_bytes(), &mut out, roots);
    String::from_utf8(out).unwrap().contains("\"deny\"")
}

#[test]
fn deleting_app_data_is_denied() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    let parent = app.parent().unwrap().display().to_string();
    for cmd in [
        format!("mv {a}/grants /tmp/x"),
        format!("mv {a} /tmp/y"),
        format!("rm -rf {a}"),
        format!("rm -r {a}/grants/.."),
        format!("rm -rf {parent}"),
        format!("rm -f {a}/claims/0123456789abcdef.json"),
        format!("mv {a}/claims/k.json /tmp/k.json"),
    ] {
        assert!(guard_says_deny(&roots, &cmd), "{cmd}");
    }
}

#[test]
fn writing_a_sibling_of_app_data_is_allowed() {
    let (_d, app, roots) = app_data_guard();
    let parent = app.parent().unwrap().display().to_string();
    for cmd in [
        format!("echo x > {parent}/other.txt"),
        format!("rm -rf {parent}/other"),
        format!("touch {parent}/app-backup"),
        format!("mkdir {parent}/app2"),
        format!("cp /tmp/x {parent}/notes.txt"),
        format!("ls {parent}"),
        format!("cat {}/grants/t-1.json", app.display()),
    ] {
        assert!(!guard_says_deny(&roots, &cmd), "{cmd}");
    }
}

#[test]
fn copying_into_app_data_is_denied_whatever_the_source() {
    let (_d, app, roots) = app_data_guard();
    let a = app.display();
    let parent = app.parent().unwrap().display().to_string();
    for cmd in [
        format!("cp -r x/ {a}"),
        format!("cp -rT x {a}/grants"),
        format!("mv x/. {a}"),
        format!("cp x {parent}/"),
        format!("rsync -a x/ {a}/claims/"),
        format!("cp -t {a}/grants x"),
        format!("cp --target-directory={a} x"),
        format!("install -D x {a}/grants/t-1.json"),
        format!("ln -sf /tmp/evil {a}/grants"),
        format!("tar -xf x.tar -C {a}"),
        format!("tar xf x.tar --directory={parent}"),
        format!("cd {a} && tar -xzf /tmp/x.tgz"),
        format!("unzip x.zip -d {a}"),
    ] {
        assert!(guard_says_deny(&roots, &cmd), "{cmd}");
    }
}

#[test]
fn copying_next_to_app_data_is_allowed() {
    let (_d, app, roots) = app_data_guard();
    let parent = app.parent().unwrap().display().to_string();
    for cmd in [
        format!("cp -r x/ {parent}/sibling"),
        format!("mv x {parent}/app-old"),
        format!("rsync -a x/ {parent}/backup/"),
        format!("tar -xf x.tar -C {parent}/sibling"),
        format!("cp {}/grants/t-1.json /tmp/copy.json", app.display()),
        format!("tar -tf {}/x.tar", app.display()),
    ] {
        assert!(!guard_says_deny(&roots, &cmd), "{cmd}");
    }
}

#[test]
fn dot_dot_through_a_symlink_is_resolved_like_the_shell() {
    let (d, app, roots) = app_data_guard();
    // `<tmp>/links/to-grants` points at app data's grants; `to-grants/..` is app data, as the shell reads it, though
    // its text cleans to `<tmp>/links`
    let links = std::fs::canonicalize(d.path()).unwrap().join("links");
    std::fs::create_dir_all(&links).unwrap();
    let link = links.join("to-grants");
    std::os::unix::fs::symlink(app.join("grants"), &link).unwrap();
    let l = link.display();
    for cmd in [
        format!("echo x > {l}/../grants/t-1.json"),
        format!("rm -rf {l}/../claims"),
        format!("cd {l}/.. && echo x > claims/k.json"),
    ] {
        assert!(guard_says_deny(&roots, &cmd), "{cmd}");
    }
    assert!(!guard_says_deny(
        &roots,
        &format!("echo x > {}/other.txt", links.display())
    ));
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
    fn the_fallback_protects_the_canonical_app_data() {
        let dir = tempfile::tempdir().unwrap();
        let real = std::fs::canonicalize(dir.path()).unwrap().join("real");
        std::fs::create_dir_all(real.join("guards")).unwrap();
        let alias = real.with_file_name("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        // the roots file the session was told to read, through the link, is gone
        let roots = alias.join("guards/t-1.guard.json");
        for data in [&alias, &real] {
            let mut out = Vec::new();
            let cmd = format!("echo x > {}/grants/t-1.json", data.display());
            run_guard(&mut hook_json(&cmd, "/w/web").as_slice(), &mut out, &roots);
            assert!(String::from_utf8(out).unwrap().contains("\"deny\""), "{cmd}");
        }
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
