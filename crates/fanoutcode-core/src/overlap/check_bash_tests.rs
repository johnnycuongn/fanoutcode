//! The git guard's half of the claim-check hook (`check_bash`, §5.6): only an exact, single-use `Command` grant lets a
//! command over another task's claims through; edit grants never do.

use serde_json::json;

use super::*;
use crate::overlap::claims::write_claims;
use crate::overlap::grants::{write_grants, Grant, GrantScope};

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
            granted_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        }],
    }
}

fn command_grant(other: &str, command: &str) -> GrantsFile {
    grant(
        other,
        GrantScope::Command {
            command: command.into(),
        },
    )
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
    let mut grants = command_grant("t-b2", "git stash");
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
