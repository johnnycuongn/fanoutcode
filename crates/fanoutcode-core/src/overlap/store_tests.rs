use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::json;

use super::*;
use crate::overlap::{OverlapRow, PendingDecision, Relation};
use crate::test_support::test_conn;

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

fn row(task: &str, other: Option<&str>, path: &str, relation: Relation) -> OverlapRow {
    OverlapRow {
        task_id: id(task),
        other_task: other.map(id),
        repo_root: "/r".into(),
        path: path.into(),
        relation,
        pending: None,
        held: false,
        noted_out: false,
        noted_in: false,
        decided_at: None,
        updated_at: at("2026-10-25T10:00:00Z"),
    }
}

fn edit() -> PendingDecision {
    PendingDecision::Edit {
        tool: "Edit".into(),
        input: json!({"file_path": "/r/src/a.ts", "old_string": "a", "new_string": "b"}),
    }
}

#[test]
fn upsert_then_list_round_trips_every_field() {
    let conn = test_conn();
    let full = OverlapRow {
        pending: Some(edit()),
        held: true,
        noted_out: true,
        noted_in: true,
        decided_at: Some(at("2026-10-25T09:59:00.123456789Z")),
        ..row("t-a1", Some("t-b2"), "src/a.ts", Relation::GrantedOnTop)
    };
    let anon = row("t-a1", None, "src/b.ts", Relation::Claimed);
    upsert(&conn, &anon).unwrap();
    upsert(&conn, &full).unwrap();
    assert_eq!(list(&conn, &id("t-a1")).unwrap(), vec![full.clone(), anon.clone()]);

    // the same key, including a NULL other task, updates in place
    let changed = OverlapRow {
        relation: Relation::Committed,
        ..anon
    };
    upsert(&conn, &changed).unwrap();
    upsert(&conn, &changed).unwrap();
    assert_eq!(list(&conn, &id("t-a1")).unwrap(), vec![full, changed]);
    assert!(list(&conn, &id("t-b2")).unwrap().is_empty());
}

#[test]
fn pending_lists_only_rows_with_a_decision() {
    let conn = test_conn();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/a.ts", Relation::Claimed)).unwrap();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/b.ts", Relation::Claimed)).unwrap();
    set_pending(
        &conn,
        &id("t-a1"),
        Path::new("/r"),
        "src/b.ts",
        Some(&id("t-b2")),
        Some(edit()),
    )
    .unwrap();
    let open = pending(&conn, &id("t-a1")).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(
        (open[0].path.as_str(), open[0].pending.clone()),
        ("src/b.ts", Some(edit()))
    );
}

#[test]
fn set_pending_creates_a_claimed_row_and_closing_it_clears_the_hold() {
    let conn = test_conn();
    let (t, repo) = (id("t-a1"), Path::new("/r"));
    let cmd = PendingDecision::Command {
        command: "git stash".into(),
    };
    set_pending(&conn, &t, repo, "src/a.ts", None, Some(cmd.clone())).unwrap();
    set_held(&conn, &t, repo, "src/a.ts", None, true).unwrap();
    let rows = list(&conn, &t).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].relation,
            rows[0].other_task.clone(),
            rows[0].pending.clone(),
            rows[0].held
        ),
        (Relation::Claimed, None, Some(cmd), true)
    );
    set_pending(&conn, &t, repo, "src/a.ts", None, None).unwrap();
    let rows = list(&conn, &t).unwrap();
    assert_eq!((rows[0].pending.clone(), rows[0].held), (None, false));
    // a hold on a missing row stays missing
    set_held(&conn, &t, repo, "src/zz.ts", None, true).unwrap();
    assert_eq!(list(&conn, &t).unwrap().len(), 1);
}

#[test]
fn set_relation_records_the_answer() {
    let conn = test_conn();
    let (t, other) = (id("t-a1"), id("t-b2"));
    set_pending(&conn, &t, Path::new("/r"), "src/a.ts", Some(&other), Some(edit())).unwrap();
    let when = at("2026-10-25T11:00:00Z");
    set_relation(
        &conn,
        &t,
        Path::new("/r"),
        "src/a.ts",
        Some(&other),
        Relation::TakenOver,
        Some(when),
    )
    .unwrap();
    let r = &list(&conn, &t).unwrap()[0];
    assert_eq!(
        (r.relation, r.decided_at, r.pending.clone()),
        (Relation::TakenOver, Some(when), Some(edit()))
    );
}

#[test]
fn clear_pending_returns_the_count() {
    let conn = test_conn();
    let t = id("t-a1");
    for p in ["src/a.ts", "src/b.ts"] {
        set_pending(&conn, &t, Path::new("/r"), p, Some(&id("t-b2")), Some(edit())).unwrap();
        set_held(&conn, &t, Path::new("/r"), p, Some(&id("t-b2")), true).unwrap();
    }
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/c.ts", Relation::Claimed)).unwrap();
    assert_eq!(clear_pending(&conn, &t).unwrap(), 2);
    assert!(pending(&conn, &t).unwrap().is_empty());
    assert!(list(&conn, &t).unwrap().iter().all(|r| !r.held));
    assert_eq!(clear_pending(&conn, &t).unwrap(), 0);
}

#[test]
fn taken_over_paths_reads_the_other_side() {
    let conn = test_conn();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/a.ts", Relation::TakenOver)).unwrap();
    upsert(&conn, &row("t-c3", Some("t-b2"), "src/a.ts", Relation::TakenOver)).unwrap();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/b.ts", Relation::GrantedOnTop)).unwrap();
    upsert(&conn, &row("t-a1", Some("t-c3"), "src/c.ts", Relation::TakenOver)).unwrap();
    upsert(
        &conn,
        &OverlapRow {
            repo_root: "/other".into(),
            ..row("t-a1", Some("t-b2"), "src/d.ts", Relation::TakenOver)
        },
    )
    .unwrap();
    assert_eq!(
        taken_over_paths(&conn, &id("t-b2"), Path::new("/r")).unwrap(),
        vec!["src/a.ts".to_owned()]
    );
    assert!(taken_over_paths(&conn, &id("t-a1"), Path::new("/r"))
        .unwrap()
        .is_empty());
}

#[test]
fn taken_over_by_rows_do_not_affect_done_paths() {
    let conn = test_conn();
    // t-a1 replaced t-b2's changes in src/a.ts: the replacer's row and the mirror on the side that lost the file
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/a.ts", Relation::TakenOver)).unwrap();
    upsert(&conn, &row("t-b2", Some("t-a1"), "src/a.ts", Relation::TakenOverBy)).unwrap();
    assert_eq!(
        taken_over_paths(&conn, &id("t-b2"), Path::new("/r")).unwrap(),
        vec!["src/a.ts".to_owned()]
    );
    // the mirror never drops the path from the replacer's own Done check
    assert!(taken_over_paths(&conn, &id("t-a1"), Path::new("/r"))
        .unwrap()
        .is_empty());
    assert_eq!(list(&conn, &id("t-b2")).unwrap()[0].relation, Relation::TakenOverBy);
}

#[test]
fn mark_noted_sets_one_direction() {
    let conn = test_conn();
    let t = id("t-a1");
    upsert(&conn, &row("t-a1", None, "src/a.ts", Relation::PlannedBy)).unwrap();
    mark_noted(&conn, &t, Path::new("/r"), "src/a.ts", None, true).unwrap();
    let r = &list(&conn, &t).unwrap()[0];
    assert_eq!((r.noted_out, r.noted_in), (true, false));
    mark_noted(&conn, &t, Path::new("/r"), "src/a.ts", None, false).unwrap();
    let r = &list(&conn, &t).unwrap()[0];
    assert_eq!((r.noted_out, r.noted_in), (true, true));
}

#[test]
fn remove_absent_keeps_only_the_listed_keys() {
    let conn = test_conn();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/a.ts", Relation::Claimed)).unwrap();
    upsert(&conn, &row("t-a1", None, "src/a.ts", Relation::Claimed)).unwrap();
    upsert(&conn, &row("t-a1", Some("t-b2"), "src/b.ts", Relation::Claimed)).unwrap();
    upsert(&conn, &row("t-c3", Some("t-b2"), "src/b.ts", Relation::Claimed)).unwrap();
    let keep = [(PathBuf::from("/r"), "src/a.ts".to_owned(), None)];
    assert_eq!(remove_absent(&conn, &id("t-a1"), &keep).unwrap(), 2);
    let left = list(&conn, &id("t-a1")).unwrap();
    assert_eq!((left.len(), left[0].other_task.clone()), (1, None));
    assert_eq!(list(&conn, &id("t-c3")).unwrap().len(), 1);
}

#[test]
fn fresh_session_clears_pending_and_unpins() {
    use crate::domain::{AgentState, HandbackReason};
    use crate::overlap::{pin_for, pinned};
    use std::collections::HashMap;

    let conn = test_conn();
    let t = id("t-a1");
    set_pending(&conn, &t, Path::new("/r"), "src/a.ts", Some(&id("t-b2")), Some(edit())).unwrap();
    set_pending(&conn, &t, Path::new("/r"), "src/b.ts", None, Some(edit())).unwrap();
    set_held(&conn, &t, Path::new("/r"), "src/b.ts", None, true).unwrap();
    let titles = HashMap::new();
    assert_eq!(
        pin_for(&list(&conn, &t).unwrap(), &titles),
        Some(HandbackReason::FileInUse { pending: 2 })
    );
    // Start fresh session (or Archive): every open decision goes, so nothing pins the new session
    assert_eq!(clear_pending(&conn, &t).unwrap(), 2);
    let left = pending(&conn, &t).unwrap();
    assert!(left.is_empty());
    assert_eq!(pin_for(&list(&conn, &t).unwrap(), &titles), None);
    assert_eq!(
        pinned(left.len() as u32, Some(AgentState::Working)),
        Some(AgentState::Working)
    );
}

#[test]
fn a_taken_over_path_counts_again_once_the_loser_edits_it() {
    use crate::changes::{done_paths, AttributedFile, ChangeSource, MAIN_AGENT};
    let conn = test_conn();
    // t-a1 replaced t-b2's changes in src/a.ts at 10:00
    upsert(
        &conn,
        &OverlapRow {
            decided_at: Some(at("2026-10-25T10:00:00Z")),
            ..row("t-a1", Some("t-b2"), "src/a.ts", Relation::TakenOver)
        },
    )
    .unwrap();
    let file = |path: &str, when: &str| AttributedFile {
        repo_root: "/r".into(),
        path: path.into(),
        agent_id: MAIN_AGENT.into(),
        source: ChangeSource::Hook,
        added_at: at(when),
    };
    let repo = Path::new("/r");
    let repos = [PathBuf::from("/r")];
    // t-b2 still has only its other file: src/a.ts stays out of its Done check
    let before = [file("src/b.ts", "2026-10-25T09:00:00Z")];
    let taken = vec![(
        PathBuf::from("/r"),
        taken_over_paths_for(&conn, &id("t-b2"), repo, &before).unwrap(),
    )];
    assert_eq!(taken[0].1, vec!["src/a.ts".to_owned()]);
    assert_eq!(
        done_paths(&repos, &before, None, &taken),
        vec![(PathBuf::from("/r"), vec!["src/b.ts".to_owned()])]
    );
    // an attribution older than the decision does not bring it back either
    let stale = [
        file("src/a.ts", "2026-10-25T09:30:00Z"),
        file("src/b.ts", "2026-10-25T09:00:00Z"),
    ];
    assert_eq!(
        taken_over_paths_for(&conn, &id("t-b2"), repo, &stale).unwrap(),
        vec!["src/a.ts".to_owned()]
    );
    // t-b2 edits src/a.ts again after the Replace: it is t-b2's to commit once more
    let after = [
        file("src/a.ts", "2026-10-25T10:05:00Z"),
        file("src/b.ts", "2026-10-25T09:00:00Z"),
    ];
    let taken = vec![(
        PathBuf::from("/r"),
        taken_over_paths_for(&conn, &id("t-b2"), repo, &after).unwrap(),
    )];
    assert!(taken[0].1.is_empty());
    assert_eq!(
        done_paths(&repos, &after, None, &taken),
        vec![(PathBuf::from("/r"), vec!["src/a.ts".to_owned(), "src/b.ts".to_owned()])]
    );
}
