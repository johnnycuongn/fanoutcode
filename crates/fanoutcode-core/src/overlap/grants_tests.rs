use serde_json::json;

use super::*;
use crate::overlap::claims::Claim;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn at() -> DateTime<Utc> {
    "2026-10-25T10:00:00Z".parse().unwrap()
}

fn file_grant(other: Option<&str>, repo: &str, path: &str) -> Grant {
    Grant {
        other_task: other.map(id),
        scope: GrantScope::File {
            repo: repo.into(),
            path: path.into(),
        },
        granted_at: at(),
    }
}

fn all_grant(other: &str) -> Grant {
    Grant {
        other_task: Some(id(other)),
        scope: GrantScope::AllFiles,
        granted_at: at(),
    }
}

fn claims(repo: &str, held: &[(&str, Option<&str>)]) -> ClaimsFile {
    ClaimsFile {
        repo_root: repo.into(),
        written_at: at(),
        claims: held
            .iter()
            .map(|(path, task)| Claim {
                path: (*path).into(),
                task_id: task.map(id),
                title: "x".into(),
                stage: None,
                state: None,
                last_edit: at(),
                staged: false,
            })
            .collect(),
    }
}

#[test]
fn file_grant_covers_exactly_its_path() {
    let g = GrantsFile {
        grants: vec![file_grant(Some("t-b2"), "/r", "src/a.ts")],
    };
    let b = id("t-b2");
    assert!(g.covers(Path::new("/r"), "src/a.ts", Some(&b)));
    assert!(!g.covers(Path::new("/r"), "src/b.ts", Some(&b)));
    assert!(!g.covers(Path::new("/other"), "src/a.ts", Some(&b)));
    assert!(!g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-c3"))));
}

#[test]
fn all_files_grant_covers_only_the_named_task() {
    let g = GrantsFile {
        grants: vec![all_grant("t-b2")],
    };
    assert!(g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-b2"))));
    assert!(g.covers(Path::new("/s"), "lib/z.rs", Some(&id("t-b2"))));
    assert!(!g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-c3"))));
    assert!(!g.covers(Path::new("/r"), "src/a.ts", None));
}

#[test]
fn a_grant_with_no_other_task_matches_an_unattributed_claim() {
    let g = GrantsFile {
        grants: vec![file_grant(None, "/r", "src/gen.ts")],
    };
    assert!(g.covers(Path::new("/r"), "src/gen.ts", None));
    assert!(!g.covers(Path::new("/r"), "src/gen.ts", Some(&id("t-b2"))));
}

#[test]
fn add_replaces_the_same_grant() {
    let mut g = GrantsFile::default();
    g.add(all_grant("t-b2"));
    g.add(all_grant("t-b2"));
    g.add(file_grant(Some("t-b2"), "/r", "a"));
    assert_eq!(g.grants.len(), 2);
}

#[test]
fn prune_drops_grants_without_claims() {
    let mut g = GrantsFile {
        grants: vec![
            file_grant(Some("t-b2"), "/r", "src/a.ts"),    // still claimed by B: kept
            file_grant(Some("t-b2"), "/r", "src/gone.ts"), // committed: dropped
            file_grant(Some("t-c3"), "/r", "src/a.ts"),    // a.ts is B's, not C's: dropped
            file_grant(None, "/s", "gen.ts"),              // unattributed claim in /s: kept
            all_grant("t-b2"),                             // B claims something: kept
            all_grant("t-d4"),                             // D claims nothing: dropped
            file_grant(Some("t-b2"), "/r", "src/x.ts"),    // B claims src/x.ts in /s only: dropped
        ],
    };
    let r = claims("/r", &[("src/a.ts", Some("t-b2"))]);
    let s = claims("/s", &[("gen.ts", None), ("src/x.ts", Some("t-b2"))]);
    g.prune(&[&r, &s]);
    assert_eq!(
        g.grants,
        vec![
            file_grant(Some("t-b2"), "/r", "src/a.ts"),
            file_grant(None, "/s", "gen.ts"),
            all_grant("t-b2"),
        ]
    );
}

#[test]
fn a_command_grant_covers_no_edit_and_matches_its_command_trimmed() {
    let g = GrantsFile {
        grants: vec![Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::Command {
                repo: "/r".into(),
                command: "git stash".into(),
                paths: vec!["src/a.ts".into()],
            },
            granted_at: at(),
        }],
    };
    let (b, r) = (id("t-b2"), Path::new("/r"));
    let a = ["src/a.ts".to_owned()];
    assert!(!g.covers(r, "src/a.ts", Some(&b)));
    assert!(g.allows_command(" git stash\n", r, Some(&b)));
    assert!(!g.allows_command("git stash", Path::new("/s"), Some(&b)));
    assert!(!g.allows_command("git stash -u", r, Some(&b)));
    assert!(!g.allows_command("git stash", r, Some(&id("t-c3"))));
    assert!(!g.allows_command("git stash", r, None));
    let dir = tempfile::tempdir().unwrap();
    let path = write_grants(dir.path(), &id("t-a1"), &g).unwrap();
    assert!(!take_command(&path, "git reset --hard", r, Some(&b), &a, at()));
    assert_eq!(read_grants(&path), g, "another command spends nothing");
    assert!(take_command(&path, "git stash", r, Some(&b), &a, at()));
    assert!(read_grants(&path).grants.is_empty());
    assert!(path.with_file_name("t-a1.json.lock").exists());
}

#[test]
fn read_is_empty_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_grants(&dir.path().join("none.json")), GrantsFile::default());
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{\"grants\": [").unwrap();
    assert_eq!(read_grants(&bad), GrantsFile::default());
}

#[test]
fn write_then_read_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let file = GrantsFile {
        grants: vec![file_grant(Some("t-b2"), "/r", "src/a.ts"), all_grant("t-c3")],
    };
    let path = write_grants(dir.path(), &id("t-a1"), &file).unwrap();
    assert_eq!(path, dir.path().join("grants/t-a1.json"));
    assert_eq!(read_grants(&path), file);
}

#[test]
fn grant_scope_is_internally_tagged() {
    for scope in [
        GrantScope::AllFiles,
        GrantScope::File {
            repo: "/r".into(),
            path: "a.ts".into(),
        },
        GrantScope::Command {
            repo: "/r".into(),
            command: "git stash".into(),
            paths: vec!["src/a.ts".into()],
        },
    ] {
        let v = serde_json::to_value(&scope).unwrap();
        assert_eq!(serde_json::from_value::<GrantScope>(v).unwrap(), scope);
    }
    assert_eq!(
        serde_json::to_value(GrantScope::AllFiles).unwrap(),
        json!({"type": "AllFiles"})
    );
    assert_eq!(
        serde_json::to_value(file_grant(None, "/r", "a.ts")).unwrap()["scope"],
        json!({"type": "File", "repo": "/r", "path": "a.ts"})
    );
}

fn command_grant(paths: &[&str], granted_at: DateTime<Utc>) -> Grant {
    Grant {
        other_task: Some(id("t-b2")),
        scope: GrantScope::Command {
            repo: "/r".into(),
            command: "git stash".into(),
            paths: paths.iter().map(|p| (*p).to_owned()).collect(),
        },
        granted_at,
    }
}

fn minutes(n: i64) -> chrono::Duration {
    chrono::Duration::minutes(n)
}

#[test]
fn a_command_grant_expires() {
    let dir = tempfile::tempdir().unwrap();
    let g = GrantsFile {
        grants: vec![command_grant(&["src/a.ts"], at())],
    };
    let path = write_grants(dir.path(), &id("t-a1"), &g).unwrap();
    let (b, r, a) = (id("t-b2"), Path::new("/r"), ["src/a.ts".to_owned()]);
    assert!(
        !take_command(
            &path,
            "git stash",
            r,
            Some(&b),
            &a,
            at() + minutes(10) + chrono::Duration::seconds(1)
        ),
        "older than ten minutes: spent nothing"
    );
    assert_eq!(read_grants(&path), g, "an expired grant is left for the shell to drop");
    assert!(take_command(&path, "git stash", r, Some(&b), &a, at() + minutes(10)));
    assert!(read_grants(&path).grants.is_empty());
}

#[test]
fn a_command_grant_fails_when_the_other_tasks_files_changed() {
    let dir = tempfile::tempdir().unwrap();
    let g = GrantsFile {
        grants: vec![command_grant(&["src/a.ts"], at())],
    };
    let path = write_grants(dir.path(), &id("t-a1"), &g).unwrap();
    let (b, r) = (id("t-b2"), Path::new("/r"));
    let now = at() + minutes(1);
    for changed in [
        vec!["src/a.ts".to_owned(), "src/b.ts".to_owned()],
        vec!["src/b.ts".to_owned()],
        vec![],
    ] {
        assert!(
            !take_command(&path, "git stash", r, Some(&b), &changed, now),
            "{changed:?}"
        );
        assert_eq!(read_grants(&path), g);
    }
    assert!(take_command(
        &path,
        "git stash",
        r,
        Some(&b),
        &["src/a.ts".to_owned()],
        now
    ));
}

#[test]
fn stale_command_grants_are_the_command_grants_the_go_ahead_covered() {
    let fresh = command_grant(&["src/a.ts"], at() + minutes(5));
    let old = command_grant(&["src/a.ts"], at());
    let edit = file_grant(Some("t-b2"), "/r", "src/a.ts");
    let g = GrantsFile {
        grants: vec![old.clone(), edit, fresh],
    };
    // the go-ahead was typed a minute after the first grant: only that one is stale at the next Stop
    assert_eq!(stale_command_grants(&g, at() + minutes(1)), vec![old]);
    assert!(stale_command_grants(&g, at() - minutes(1)).is_empty());
}
