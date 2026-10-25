//! The git guard's half of the claim-check hook (`check_bash`, §5.6): only an exact, single-use `Command` grant lets a
//! command over another task's claims through; edit grants never do.

use serde_json::json;

use super::*;
use crate::overlap::claims::write_claims;
use crate::overlap::grants::{take_command, with_grants_lock, write_grants, Grant, GrantScope};
use crate::overlap::CommandKind;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn claim(path: &str, task: &str, title: &str) -> Claim {
    Claim {
        path: path.into(),
        task_id: Some(id(task)),
        title: title.into(),
        stage: None,
        state: None,
        last_edit: "2026-10-25T10:00:00Z".parse().unwrap(),
        staged: false,
    }
}

/// `root` holds this task's (t-a1) `src/mine.ts` and Checkout's (t-b2) `src/a.ts`.
fn repo(root: &Path) -> ClaimsFile {
    ClaimsFile {
        repo_root: root.to_owned(),
        written_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        claims: vec![
            claim("src/a.ts", "t-b2", "Checkout"),
            claim("src/mine.ts", "t-a1", "Login"),
        ],
    }
}

fn bash(command: &str) -> CheckInput {
    CheckInput {
        tool_name: "Bash".into(),
        tool_input: json!({ "command": command }),
        cwd: "/r".into(),
    }
}

fn decide(command: &str, grants: &GrantsFile) -> CheckDecision {
    evaluate(&bash(command), &id("t-a1"), &[repo(Path::new("/r"))], grants, true)
}

fn grant(other: &str, scope: GrantScope) -> GrantsFile {
    GrantsFile {
        grants: vec![Grant {
            other_task: Some(id(other)),
            scope,
            granted_at: Utc::now(),
        }],
    }
}

fn command_grant_in(other: &str, command: &str, repo: &Path) -> GrantsFile {
    grant(
        other,
        GrantScope::Command {
            repo: repo.to_owned(),
            command: command.into(),
            paths: vec!["src/a.ts".into()],
        },
    )
}

fn command_grant(other: &str, command: &str) -> GrantsFile {
    command_grant_in(other, command, Path::new("/r"))
}

#[test]
fn edit_grants_never_allow_git_commands() {
    let file = grant(
        "t-b2",
        GrantScope::File {
            repo: "/r".into(),
            path: "src/a.ts".into(),
        },
    );
    let all = grant("t-b2", GrantScope::AllFiles);
    for command in [
        "git stash",
        "git reset --hard",
        "git clean -fd",
        "git checkout -- src/a.ts",
        "git restore src/a.ts",
        "git commit -am wip",
    ] {
        for grants in [&file, &all] {
            assert!(
                matches!(decide(command, grants), CheckDecision::Deny { .. }),
                "{command} with {grants:?}"
            );
        }
    }
}

/// A temp app-data dir with the claims of `repo/` written and the task's args, the git guard on.
fn hook() -> (tempfile::TempDir, PathBuf, CheckArgs) {
    let dir = tempfile::tempdir().unwrap();
    let data = std::fs::canonicalize(dir.path()).unwrap();
    let root = data.join("repo");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let claims = write_claims(&data, &repo(&root)).unwrap();
    let args = CheckArgs {
        task_id: id("t-a1"),
        claims_paths: vec![claims],
        grants_path: data.join("grants/t-a1.json"),
        notes_path: data.join("notes/t-a1.json"),
        log_path: data.join("events/t-a1.jsonl"),
        ask_before: true,
        guard_git: true,
    };
    (dir, root, args)
}

fn denied(args: &CheckArgs, root: &Path, command: &str) -> bool {
    let payload = json!({"tool_name": "Bash", "tool_input": {"command": command}, "cwd": root});
    let mut out = Vec::new();
    assert_eq!(run_check(&mut payload.to_string().as_bytes(), &mut out, args), 0);
    let text = String::from_utf8(out).unwrap();
    !text.trim().is_empty()
        && serde_json::from_str::<serde_json::Value>(&text).unwrap()["hookSpecificOutput"]["permissionDecision"]
            == "deny"
}

#[test]
fn command_grant_allows_exactly_that_command_once() {
    assert_eq!(
        decide("git stash", &command_grant("t-b2", "git stash")),
        CheckDecision::Allow
    );
    // the command is matched trimmed, on either side
    assert_eq!(
        decide("  git stash\n", &command_grant("t-b2", " git stash ")),
        CheckDecision::Allow
    );
    // through the files on disk: allowed once, then the grant is spent
    let (_dir, root, args) = hook();
    let data = args.grants_path.parent().unwrap().parent().unwrap().to_owned();
    let mut grants = command_grant_in("t-b2", "git stash", &root);
    grants.grants.push(Grant {
        other_task: Some(id("t-b2")),
        scope: GrantScope::File {
            repo: root.clone(),
            path: "src/a.ts".into(),
        },
        granted_at: "2026-10-25T10:00:00Z".parse().unwrap(),
    });
    write_grants(&data, &id("t-a1"), &grants).unwrap();
    assert!(
        denied(&args, &root, "git reset --hard"),
        "another command is still stopped"
    );
    assert!(!denied(&args, &root, "git stash"), "the granted command runs");
    let left = read_grants(&args.grants_path);
    assert_eq!(left.grants.len(), 1, "the edit grant stays: {left:?}");
    assert!(matches!(left.grants[0].scope, GrantScope::File { .. }));
    assert!(denied(&args, &root, "git stash"), "a second run is asked about again");
}

#[test]
fn command_grant_does_not_cover_a_different_command() {
    let g = command_grant("t-b2", "git stash");
    for command in [
        "git stash -u",
        "git reset --hard",
        "git commit -am wip",
        "git stash && git clean -fd",
    ] {
        assert!(matches!(decide(command, &g), CheckDecision::Deny { .. }), "{command}");
    }
    // a grant on another task's claims does not cover Checkout's
    assert!(matches!(
        decide("git stash", &command_grant("t-c3", "git stash")),
        CheckDecision::Deny { .. }
    ));
}

#[test]
fn command_grant_is_scoped_to_its_repo() {
    assert!(matches!(
        decide(
            "git stash",
            &command_grant_in("t-b2", "git stash", Path::new("/elsewhere"))
        ),
        CheckDecision::Deny { .. }
    ));
    assert_eq!(
        decide("git stash", &command_grant("t-b2", "git stash")),
        CheckDecision::Allow
    );
}

#[test]
fn a_command_grant_is_used_up_before_the_command_is_allowed() {
    let (_dir, root, args) = hook();
    let data = args.grants_path.parent().unwrap().parent().unwrap().to_owned();
    write_grants(&data, &id("t-a1"), &command_grant_in("t-b2", "git stash", &root)).unwrap();
    let b = id("t-b2");
    let (a, now) = (["src/a.ts".to_owned()], Utc::now());
    assert!(take_command(&args.grants_path, " git stash", &root, Some(&b), &a, now));
    assert!(
        read_grants(&args.grants_path).grants.is_empty(),
        "spent on disk before the hook answers"
    );
    assert!(
        !take_command(&args.grants_path, "git stash", &root, Some(&b), &a, now),
        "only once"
    );
    // nothing to take: a grant for another repo or task is not this one
    write_grants(&data, &id("t-a1"), &command_grant_in("t-b2", "git stash", &root)).unwrap();
    assert!(!take_command(
        &args.grants_path,
        "git stash",
        Path::new("/elsewhere"),
        Some(&b),
        &a,
        now
    ));
    assert!(!take_command(
        &args.grants_path,
        "git stash",
        &root,
        Some(&id("t-c3")),
        &a,
        now
    ));
    assert_eq!(read_grants(&args.grants_path).grants.len(), 1);
    assert!(!denied(&args, &root, "git stash"));
    assert!(read_grants(&args.grants_path).grants.is_empty());
}

#[cfg(unix)]
#[test]
fn a_failed_consume_denies() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, root, args) = hook();
    let data = args.grants_path.parent().unwrap().parent().unwrap().to_owned();
    write_grants(&data, &id("t-a1"), &command_grant_in("t-b2", "git stash", &root)).unwrap();
    // the lock file exists, but the grants file cannot be rewritten: the grant cannot be spent
    with_grants_lock(&args.grants_path, || Ok(())).unwrap();
    let dir = args.grants_path.parent().unwrap().to_owned();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let still_denied = denied(&args, &root, "git stash");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(still_denied, "a grant that could not be spent allows nothing");
    assert_eq!(read_grants(&args.grants_path).grants.len(), 1);
}

#[test]
fn git_deny_reason_names_each_files_task_once_and_what_the_command_does() {
    let (a, b, c) = (
        claim("src/a.ts", "t-b2", "Login"),
        claim("src/b.ts", "t-c3", "Billing"),
        claim("src/c.ts", "t-b2", "Login"),
    );
    let tail = "The developer is being asked how to proceed. Do not run it or work around the block; finish anything \
unrelated you were doing, then stop and wait for the developer's instruction.";
    assert_eq!(
        git_deny_reason(CommandKind::Commits, &[&a, &b, &c]),
        format!(
            "Fanout Code: this git command would commit other tasks' uncommitted changes into this task's commit: \
Login has src/a.ts, src/c.ts; Billing has src/b.ts. {tail}"
        )
    );
    assert_eq!(
        git_deny_reason(CommandKind::Discards, &[&b]),
        format!(
            "Fanout Code: this git command would discard other tasks' uncommitted changes from the working tree: \
Billing has src/b.ts. {tail}"
        )
    );
    // the hook's reason for a command over Checkout's claim
    assert!(matches!(
        decide("git stash", &GrantsFile::default()),
        CheckDecision::Deny { reason, .. } if reason == git_deny_reason(
            CommandKind::Discards, &[&claim("src/a.ts", "t-b2", "Checkout")])
    ));
}

#[test]
fn a_spent_command_grant_logs_command_ran_and_a_stale_one_denies() {
    let (_dir, root, args) = hook();
    let data = args.grants_path.parent().unwrap().parent().unwrap().to_owned();
    write_grants(&data, &id("t-a1"), &command_grant_in("t-b2", "git stash", &root)).unwrap();
    assert!(!denied(&args, &root, "git stash"));
    let log = std::fs::read_to_string(&args.log_path).unwrap();
    let line = log
        .lines()
        .find(|l| l.starts_with("CommandRan\t"))
        .expect("a CommandRan line");
    let payload: serde_json::Value = serde_json::from_str(line.splitn(3, '\t').nth(2).unwrap()).unwrap();
    assert_eq!(
        payload,
        json!({"repo": root, "command": "git stash", "kind": "Discards", "paths": ["src/a.ts"], "other_task": "t-b2"})
    );
    // granted eleven minutes ago: not spent, denied, nothing logged as run
    let mut old = command_grant_in("t-b2", "git stash", &root);
    old.grants[0].granted_at = Utc::now() - chrono::Duration::minutes(11);
    write_grants(&data, &id("t-a1"), &old).unwrap();
    assert!(denied(&args, &root, "git stash"));
    assert_eq!(read_grants(&args.grants_path), old);
    let ran = std::fs::read_to_string(&args.log_path)
        .unwrap()
        .matches("CommandRan\t")
        .count();
    assert_eq!(ran, 1);
}
