use serde_json::json;

use super::*;
use crate::domain::TaskId;
use crate::git::FileStatus;
use crate::hooks::parse::parse_line;
use crate::hooks::HookEvent;
use crate::overlap::prompts::command_free;
use crate::overlap::CommandKind;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn now() -> DateTime<Utc> {
    "2026-10-25T10:00:00Z".parse().unwrap()
}

fn row(path: &str, other: Option<&str>, pending: Option<PendingDecision>, held: bool) -> OverlapRow {
    OverlapRow {
        task_id: id("t-a1"),
        other_task: other.map(id),
        repo_root: "/r".into(),
        path: path.into(),
        relation: Relation::Claimed,
        pending,
        held,
        noted_out: false,
        noted_in: false,
        decided_at: None,
        updated_at: now(),
    }
}

fn edit() -> Option<PendingDecision> {
    Some(PendingDecision::Edit {
        tool: "Edit".into(),
        input: json!({"file_path": "/r/src/a.ts"}),
    })
}

#[test]
fn on_top_file_grants_the_path_and_prompts_both() {
    let out = outcome(
        Decision::OnTopFile,
        &row("src/a.ts", Some("t-b2"), edit(), false),
        "Checkout",
        "Login",
        now(),
    );
    assert_eq!(
        out.grant,
        Some(Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::File {
                repo: "/r".into(),
                path: "src/a.ts".into()
            },
            granted_at: now(),
        })
    );
    assert_eq!(out.relation, Relation::GrantedOnTop);
    assert_eq!(out.my_prompt, Some(go_ahead(Decision::OnTopFile, "Login", "src/a.ts")));
    assert_eq!(
        out.other_prompt,
        Some(told(Decision::OnTopFile, "Checkout", "src/a.ts"))
    );
    assert_eq!(out.my_timeline, "on top of Login in src/a.ts");
    assert!(out.other_timeline.is_some());
    assert!(!out.taken_over && !out.hold);
}

#[test]
fn on_top_all_grants_all_files() {
    let out = outcome(
        Decision::OnTopAll,
        &row("src/a.ts", Some("t-b2"), edit(), false),
        "Checkout",
        "Login",
        now(),
    );
    assert_eq!(out.grant.unwrap().scope, GrantScope::AllFiles);
    assert_eq!(out.relation, Relation::GrantedOnTop);
    assert_eq!(out.my_prompt, Some(go_ahead(Decision::OnTopAll, "Login", "src/a.ts")));
    assert_eq!(out.other_prompt, Some(told(Decision::OnTopAll, "Checkout", "src/a.ts")));
}

#[test]
fn replace_marks_taken_over() {
    let out = outcome(
        Decision::Replace,
        &row("src/a.ts", Some("t-b2"), edit(), false),
        "Checkout",
        "Login",
        now(),
    );
    assert!(matches!(out.grant.unwrap().scope, GrantScope::File { .. }));
    assert_eq!(out.relation, Relation::TakenOver);
    assert!(out.taken_over);
    assert_eq!(out.other_prompt, Some(told(Decision::Replace, "Checkout", "src/a.ts")));
    assert_eq!(out.other_timeline.as_deref(), Some("Checkout took over src/a.ts"));
}

#[test]
fn replace_writes_a_taken_over_by_row_on_the_other_task() {
    let out = outcome(
        Decision::Replace,
        &row("src/a.ts", Some("t-b2"), edit(), false),
        "Checkout",
        "Login",
        now(),
    );
    assert_eq!(out.other_relation, Some(Relation::TakenOverBy));
    // no other task (an unattributed session), no row to mirror; the on-top answers mirror nothing
    let anon = outcome(
        Decision::Replace,
        &row("src/a.ts", None, edit(), false),
        "Checkout",
        "x",
        now(),
    );
    assert_eq!(anon.other_relation, None);
    for d in [Decision::OnTopFile, Decision::OnTopAll, Decision::Hold] {
        let out = outcome(
            d,
            &row("src/a.ts", Some("t-b2"), edit(), false),
            "Checkout",
            "Login",
            now(),
        );
        assert_eq!(out.other_relation, None);
    }
}

#[test]
fn hold_grants_nothing_and_keeps_pending() {
    let r = row("src/a.ts", Some("t-b2"), edit(), false);
    let out = outcome(Decision::Hold, &r, "Checkout", "Login", now());
    assert_eq!(out.grant, None);
    assert_eq!(out.relation, r.relation);
    assert!(out.hold && !out.taken_over);
    assert_eq!(
        (out.my_prompt, out.other_prompt, out.other_timeline),
        (None, None, None)
    );
    assert_eq!(out.my_timeline, "holding for Login on src/a.ts");
}

#[test]
fn a_command_proceed_answers_that_commands_rows_on_the_same_task_only() {
    let cmd = |c: &str| {
        Some(PendingDecision::Command {
            command: c.into(),
            kind: CommandKind::Discards,
            paths: vec![],
        })
    };
    let decided = row("src/a.ts", Some("t-b2"), cmd("git stash"), false);
    let rows = [
        decided.clone(),
        row("src/b.ts", Some("t-b2"), cmd("git stash"), false),
        row("src/c.ts", Some("t-c3"), cmd("git stash"), false),
        row("src/d.ts", Some("t-b2"), cmd("git reset --hard"), false),
        row("src/e.ts", Some("t-b2"), edit(), false),
    ];
    let mut grants = GrantsFile::default();
    grants.add(
        outcome(Decision::OnTopAll, &decided, "C", "Login", now())
            .grant
            .unwrap(),
    );
    let covered: Vec<&str> = covered_pending(&rows, &grants, &decided)
        .iter()
        .map(|r| r.path.as_str())
        .collect();
    assert_eq!(covered, vec!["src/b.ts"]);
}

#[test]
fn a_command_proceeds_with_a_command_grant_and_an_unattributed_other_is_told_nothing() {
    let cmd = Some(PendingDecision::Command {
        command: "git add -A".into(),
        kind: CommandKind::Commits,
        paths: vec!["src/a.ts".into()],
    });
    for d in [Decision::OnTopFile, Decision::OnTopAll, Decision::Replace] {
        let out = outcome(
            d,
            &row("src/a.ts", Some("t-b2"), cmd.clone(), false),
            "Checkout",
            "Login",
            now(),
        );
        assert_eq!(
            out.grant.unwrap().scope,
            GrantScope::Command {
                repo: "/r".into(),
                command: "git add -A".into(),
                paths: vec!["src/a.ts".into()],
            }
        );
        assert_eq!(out.my_prompt, Some(go_ahead_command("Login", "git add -A")));
        assert!(!out.taken_over);
    }
    assert!(
        outcome(
            Decision::Hold,
            &row("src/a.ts", None, cmd, false),
            "C",
            "another session",
            now()
        )
        .hold
    );
    let anon = outcome(
        Decision::OnTopFile,
        &row("src/a.ts", None, edit(), false),
        "C",
        "another session",
        now(),
    );
    assert_eq!((anon.other_prompt, anon.other_timeline), (None, None));
    assert_eq!(anon.grant.unwrap().other_task, None);
}

#[test]
fn an_all_files_grant_answers_the_other_open_decisions_on_that_task() {
    let decided = row("src/a.ts", Some("t-b2"), edit(), false);
    let rows = [
        decided.clone(),
        row("src/b.ts", Some("t-b2"), edit(), true),
        row("src/c.ts", Some("t-c3"), edit(), false),
        row("src/d.ts", Some("t-b2"), None, false),
    ];
    let mut grants = GrantsFile::default();
    grants.add(
        outcome(Decision::OnTopAll, &decided, "C", "Login", now())
            .grant
            .unwrap(),
    );
    let covered: Vec<&str> = covered_pending(&rows, &grants, &decided)
        .iter()
        .map(|r| r.path.as_str())
        .collect();
    assert_eq!(covered, vec!["src/b.ts"]);
}

fn dirty(paths: &[&str]) -> Vec<StatusEntry> {
    paths
        .iter()
        .map(|p| StatusEntry {
            path: (*p).into(),
            status: FileStatus::Modified,
            orig_path: None,
            staged: false,
        })
        .collect()
}

fn released<'a>(rows: &'a [OverlapRow], repo: &str, status: &[StatusEntry]) -> Vec<(&'a str, Release)> {
    // t-b2 attributed src/a.ts; nothing else is any task's
    released_holds(rows, Path::new(repo), status, |r| {
        r.other_task.as_ref().map(TaskId::as_str) == Some("t-b2") && r.path == "src/a.ts"
    })
    .into_iter()
    .map(|(r, why)| (r.path.as_str(), why))
    .collect()
}

#[test]
fn a_hold_is_released_once_the_path_is_clean() {
    let rows = [
        row("src/a.ts", Some("t-b2"), edit(), true),
        row("src/b.ts", Some("t-b2"), edit(), true),
        row("src/c.ts", Some("t-b2"), edit(), false),
        row("src/d.ts", None, edit(), true),
    ];
    // src/b.ts is still dirty; src/c.ts is not held
    assert_eq!(
        released(&rows, "/r", &dirty(&["src/b.ts", "src/c.ts"])),
        vec![("src/a.ts", Release::Committed), ("src/d.ts", Release::Clean)]
    );
    assert!(released(&rows, "/other", &[]).is_empty());
}

#[test]
fn hold_is_not_released_while_the_path_is_dirty() {
    // the claim left the claims file (the claimant vanished from the list: its session was lost on a
    // relaunch, or the claims lag), but the changes are still on disk
    let rows = [
        row("src/a.ts", Some("t-b2"), edit(), true),
        row("src/d.ts", None, edit(), true),
    ];
    assert!(released(&rows, "/r", &dirty(&["src/a.ts", "src/d.ts"])).is_empty());
}

#[test]
fn archived_task_files_still_hold_a_hold() {
    // t-b2 was archived with src/a.ts dirty: it claims nothing any more, yet its changes are on disk
    let rows = [row("src/a.ts", Some("t-b2"), edit(), true)];
    assert!(released(&rows, "/r", &dirty(&["src/a.ts"])).is_empty());
    // once the file is clean the hold is over
    assert_eq!(released(&rows, "/r", &[]), vec![("src/a.ts", Release::Committed)]);
}

#[test]
fn release_lines_say_what_happened() {
    assert_eq!(
        release_line(Release::Committed, "src/a.ts", "Login"),
        "src/a.ts is free: Login committed it"
    );
    assert_eq!(
        release_line(Release::Clean, "src/a.ts", "another session"),
        "src/a.ts is free: it has no uncommitted changes now"
    );
}

#[test]
fn the_decided_line_parses_back_as_overlap_decided() {
    let line = decided_line(Path::new("/r"), "src/a.ts", Some(&id("t-b2")), Decision::Replace, now());
    assert!(line.ends_with('\n'));
    let ev = parse_line(&id("t-a1"), 1, line.trim_end()).unwrap();
    assert_eq!(
        ev.event,
        HookEvent::OverlapDecided {
            repo: "/r".into(),
            path: "src/a.ts".into(),
            other_task: Some(id("t-b2")),
            decision: "Replace".into(),
        }
    );
}

fn cmd(c: &str) -> Option<PendingDecision> {
    Some(PendingDecision::Command {
        command: c.into(),
        kind: CommandKind::Discards,
        paths: vec![],
    })
}

#[test]
fn a_command_go_ahead_waits_for_the_commands_last_open_row() {
    // B2's stop was just answered (its row closed); C3's stop of the same command is still open
    let rows = [
        row("src/a.ts", Some("t-b2"), None, false),
        row("src/b.ts", Some("t-c3"), cmd("git stash"), false),
        row("src/c.ts", Some("t-b2"), cmd("git reset --hard"), false),
    ];
    assert!(!command_go_ahead_due(&rows, "git stash"));
    assert!(
        !command_go_ahead_due(&rows, " git stash "),
        "the command matches trimmed"
    );
    // a held stop of it still waits
    let held = [row("src/b.ts", Some("t-c3"), cmd("git stash"), true)];
    assert!(!command_go_ahead_due(&held, "git stash"));
    // edits and other commands do not hold it back
    let rest = [
        rows[0].clone(),
        rows[2].clone(),
        row("src/d.ts", Some("t-c3"), edit(), false),
    ];
    assert!(command_go_ahead_due(&rest, "git stash"));
}

#[test]
fn a_hold_on_a_command_row_holds_the_commands_other_rows_on_that_task() {
    let decided = row("src/a.ts", Some("t-b2"), cmd("git stash"), false);
    let rows = [
        decided.clone(),
        row("src/b.ts", Some("t-b2"), cmd("git stash"), false),
        row("src/c.ts", Some("t-c3"), cmd("git stash"), false),
        row("src/d.ts", Some("t-b2"), cmd("git clean -fd"), false),
        row("src/e.ts", Some("t-b2"), edit(), false),
        row("src/f.ts", Some("t-b2"), None, false),
    ];
    let same: Vec<&str> = command_rows(&rows, &decided).iter().map(|r| r.path.as_str()).collect();
    assert_eq!(same, vec!["src/b.ts"]);
    assert!(command_rows(&rows, &rows[4]).is_empty(), "an edit has no command rows");
}

#[test]
fn a_released_command_hold_sends_one_message_naming_the_command() {
    let freed = vec![
        (
            row("src/a.ts", Some("t-b2"), cmd("git stash"), true),
            "Login".to_owned(),
            Release::Committed,
        ),
        (
            row("src/b.ts", Some("t-b2"), cmd("git stash"), true),
            "Login".to_owned(),
            Release::Committed,
        ),
        (
            row("src/e.ts", Some("t-b2"), edit(), true),
            "Login".to_owned(),
            Release::Committed,
        ),
    ];
    assert_eq!(
        release_prompts(&freed, &[]),
        vec![
            command_free(&[("Login".into(), Release::Committed)], "git stash"),
            go_ahead(Decision::Hold, "Login", "src/e.ts"),
        ]
    );
    // another task's stop of the same command is still open: the command waits, the edit does not
    let open = [row("src/c.ts", Some("t-c3"), cmd("git stash"), true)];
    assert_eq!(
        release_prompts(&freed, &open),
        vec![go_ahead(Decision::Hold, "Login", "src/e.ts")]
    );
    // released later, the last one names its task
    let last = vec![(open[0].clone(), "Billing".to_owned(), Release::Clean)];
    assert_eq!(
        release_prompts(&last, &[]),
        vec![command_free(&[("Billing".into(), Release::Clean)], "git stash")]
    );
}
