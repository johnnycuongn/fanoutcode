//! A command grant tells the other task only when the command runs (§5.6).

use super::*;
use crate::domain::TaskId;
use crate::overlap::CommandKind;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn now() -> DateTime<Utc> {
    "2026-10-25T10:00:00Z".parse().unwrap()
}

fn command_row(other: Option<&str>) -> OverlapRow {
    OverlapRow {
        task_id: id("t-a1"),
        other_task: other.map(id),
        repo_root: "/r".into(),
        path: "src/a.ts".into(),
        relation: Relation::Claimed,
        pending: Some(PendingDecision::Command {
            command: "git add -A".into(),
            kind: CommandKind::Commits,
            paths: vec!["src/a.ts".into()],
        }),
        held: false,
        noted_out: false,
        noted_in: false,
        decided_at: None,
        updated_at: now(),
    }
}

fn paths() -> Vec<String> {
    vec!["src/a.ts".into()]
}

#[test]
fn told_command_is_sent_when_the_command_runs_not_when_granted() {
    let out = outcome(
        Decision::OnTopFile,
        &command_row(Some("t-b2")),
        "Checkout",
        "Login",
        now(),
    );
    assert_eq!(out.other_prompt, None, "nothing is told at decide time");
    assert_eq!(out.other_timeline, None);
    let record = out.granted_command.expect("the app records the grant it wrote");
    assert_eq!(record.paths, paths());
    let mut granted = vec![record];
    let told = command_ran(
        &mut granted,
        Path::new("/r"),
        " git add -A ",
        &paths(),
        Some(&id("t-b2")),
    )
    .expect("a matching run tells the other task");
    assert_eq!(told.other_task, id("t-b2"));
    assert_eq!(
        told.other_prompt,
        told_command("Checkout", "git add -A", CommandKind::Commits, &paths())
    );
    assert_eq!(told.other_timeline, "Checkout ran `git add -A` over your changes");
    assert_eq!(told.my_timeline, "ran `git add -A` over Login's changes");
    assert!(granted.is_empty(), "the record is used up");
    // an unattributed session has nobody to tell, so nothing is recorded
    let anon = outcome(Decision::OnTopFile, &command_row(None), "Checkout", "x", now());
    assert_eq!(anon.granted_command, None);
}

#[test]
fn a_command_ran_line_without_a_matching_grant_does_nothing() {
    let b = id("t-b2");
    let r = Path::new("/r");
    // forged: the app wrote no grant at all
    assert_eq!(command_ran(&mut Vec::new(), r, "git add -A", &paths(), Some(&b)), None);
    let record = outcome(
        Decision::OnTopFile,
        &command_row(Some("t-b2")),
        "Checkout",
        "Login",
        now(),
    )
    .granted_command
    .unwrap();
    let mut granted = vec![record];
    let other_paths = vec!["src/b.ts".to_owned()];
    for (repo, command, paths, other) in [
        (Path::new("/s"), "git add -A", &paths(), Some(&b)),
        (r, "git add -A && git commit -m x", &paths(), Some(&b)),
        (r, "git add -A", &other_paths, Some(&b)),
        (r, "git add -A", &paths(), Some(&id("t-c3"))),
        (r, "git add -A", &paths(), None),
    ] {
        assert_eq!(
            command_ran(&mut granted, repo, command, paths, other),
            None,
            "{command} {paths:?}"
        );
    }
    assert_eq!(granted.len(), 1, "a mismatch leaves the record");
}
