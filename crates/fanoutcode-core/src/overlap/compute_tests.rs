use chrono::{DateTime, Utc};
use serde_json::json;

use super::*;
use crate::changes::{ChangeSource, MAIN_AGENT};
use crate::domain::Stage;
use crate::git::FileStatus;
use crate::overlap::claims::{build_claims, TaskFiles};
use crate::overlap::footprint::FootprintSource;
use crate::overlap::{CommandKind, PendingDecision};
use crate::test_support::task_fixture;

const HOME: &str = "/app";

fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

fn now() -> DateTime<Utc> {
    at("2026-10-25T10:00:00Z")
}

fn task(id: &str, worktree: Option<&str>) -> Task {
    let mut t = task_fixture(id, "p-1a-1");
    t.worktree_path = worktree.map(PathBuf::from);
    t
}

fn file(repo: &str, path: &str) -> AttributedFile {
    AttributedFile {
        repo_root: repo.into(),
        path: path.into(),
        agent_id: MAIN_AGENT.into(),
        source: ChangeSource::Hook,
        added_at: at("2026-10-25T09:00:00Z"),
    }
}

fn planned(task: &Task, repo: &str, path: &str) -> FootprintRow {
    FootprintRow {
        task_id: task.id.clone(),
        repo_root: repo.into(),
        path: path.into(),
        source: FootprintSource::Plan,
        note: None,
        added_at: at("2026-10-25T09:00:00Z"),
    }
}

fn view<'a>(task: &'a Task, files: &'a [AttributedFile], footprint: &'a [FootprintRow]) -> OtherTask<'a> {
    OtherTask {
        task,
        files,
        footprint,
        branch: None,
        home_root: Path::new(HOME),
    }
}

fn dirty(paths: &[&str]) -> Vec<StatusEntry> {
    paths
        .iter()
        .map(|p| StatusEntry {
            path: (*p).into(),
            status: FileStatus::Modified,
            orig_path: None,
        })
        .collect()
}

fn summary(rows: &[OverlapRow]) -> Vec<(String, String, Option<String>, Relation)> {
    rows.iter()
        .map(|r| {
            (
                r.repo_root.to_string_lossy().into_owned(),
                r.path.clone(),
                r.other_task.as_ref().map(|t| t.as_str().to_owned()),
                r.relation,
            )
        })
        .collect()
}

fn row(me: &Task, other: Option<&Task>, path: &str, relation: Relation) -> OverlapRow {
    OverlapRow {
        task_id: me.id.clone(),
        other_task: other.map(|t| t.id.clone()),
        repo_root: HOME.into(),
        path: path.into(),
        relation,
        pending: None,
        held: false,
        noted_out: false,
        noted_in: false,
        decided_at: None,
        updated_at: at("2026-10-25T08:00:00Z"),
    }
}

#[test]
fn claimed_when_sharing_a_tree_and_the_other_has_a_dirty_attributed_file() {
    let (me, login) = (task("t-a1", None), task("t-b2", None));
    let my_files = [file(HOME, "src/a.ts"), file(HOME, "src/mine.ts")];
    let login_files = [file(HOME, "src/a.ts"), file(HOME, "src/b.ts")];
    let status = dirty(&["src/a.ts", "src/b.ts", "src/mine.ts", "src/gen.ts"]);
    let mut claims = build_claims(
        Path::new(HOME),
        &[
            TaskFiles {
                task: &me,
                files: &my_files,
            },
            TaskFiles {
                task: &login,
                files: &login_files,
            },
        ],
        &status,
        now(),
    );
    // another session's dirty file, which my plan names
    claims.claims.push(crate::overlap::claims::Claim {
        path: "src/gen.ts".into(),
        task_id: None,
        title: crate::overlap::claims::ANOTHER_SESSION.into(),
        stage: None,
        state: None,
        last_edit: now(),
    });
    let my_plan = [planned(&me, HOME, "src/gen.ts")];
    let rows = overlaps_for(
        &view(&me, &my_files, &my_plan),
        &[view(&login, &login_files, &[])],
        &[claims],
        &[(HOME.into(), status)],
        &[],
        now(),
    );
    assert_eq!(
        summary(&rows),
        vec![
            (HOME.into(), "src/a.ts".into(), Some("t-b2".into()), Relation::Claimed),
            (HOME.into(), "src/gen.ts".into(), None, Relation::Claimed),
        ]
    );
    assert_eq!(rows[0].task_id, me.id);
    assert_eq!(rows[0].updated_at, now());
    assert!(rows[0].pending.is_none() && !rows[0].held && rows[0].decided_at.is_none());
}

#[test]
fn a_claim_in_a_tree_i_do_not_work_in_is_not_mine() {
    let me = task("t-a1", Some("/app/.worktrees/a"));
    let login = task("t-b2", None);
    let login_files = [file(HOME, "src/a.ts")];
    let my_files = [file("/app/.worktrees/a", "src/a.ts")];
    let status = dirty(&["src/a.ts"]);
    let claims = build_claims(
        Path::new(HOME),
        &[TaskFiles {
            task: &login,
            files: &login_files,
        }],
        &status,
        now(),
    );
    let rows = overlaps_for(
        &view(&me, &my_files, &[]),
        &[view(&login, &login_files, &[])],
        &[claims],
        &[],
        &[],
        now(),
    );
    // not a claim: our trees differ, so it is the other task's plan territory instead
    assert_eq!(
        summary(&rows),
        vec![(
            "/app/.worktrees/a".into(),
            "src/a.ts".into(),
            Some("t-b2".into()),
            Relation::PlannedBy
        )]
    );
}

#[test]
fn planned_by_when_trees_differ_and_the_footprint_names_my_file() {
    let me = task("t-a1", Some("/app/.worktrees/a"));
    let login = task("t-b2", Some("/app/.worktrees/b"));
    let mut gone = task("t-c3", Some("/app/.worktrees/c"));
    gone.archived = true;
    let my_files = [file("/app/.worktrees/a", "src/a.ts")];
    let my_plan = [planned(&me, HOME, "src/b.ts")];
    let login_plan = [
        planned(&login, HOME, "src/a.ts"),
        planned(&login, "/app/.worktrees/b", "src/b.ts"),
        planned(&login, HOME, "src/unrelated.ts"),
    ];
    let gone_plan = [planned(&gone, HOME, "src/a.ts")];
    let rows = overlaps_for(
        &view(&me, &my_files, &my_plan),
        &[view(&login, &[], &login_plan), view(&gone, &[], &gone_plan)],
        &[],
        &[],
        &[],
        now(),
    );
    assert_eq!(
        summary(&rows),
        vec![
            (
                "/app/.worktrees/a".into(),
                "src/a.ts".into(),
                Some("t-b2".into()),
                Relation::PlannedBy
            ),
            (
                "/app/.worktrees/a".into(),
                "src/b.ts".into(),
                Some("t-b2".into()),
                Relation::PlannedBy
            ),
        ]
    );
    // a shared tree is the claims' business, not the footprint's
    let neighbour = task("t-d4", None);
    let home_me = task("t-a1", None);
    let n_plan = [planned(&neighbour, HOME, "src/a.ts")];
    let home_files = [file(HOME, "src/a.ts")];
    assert!(overlaps_for(
        &view(&home_me, &home_files, &[]),
        &[view(&neighbour, &[], &n_plan)],
        &[],
        &[],
        &[],
        now()
    )
    .is_empty());
}

#[test]
fn committed_once_the_other_file_is_clean() {
    let (me, login) = (task("t-a1", None), task("t-b2", None));
    let my_files = [file(HOME, "src/a.ts"), file(HOME, "src/b.ts")];
    let login_files = [file(HOME, "src/a.ts"), file(HOME, "src/b.ts")];
    let held = OverlapRow {
        pending: Some(PendingDecision::Edit {
            tool: "Edit".into(),
            input: json!({"file_path": "/app/src/a.ts"}),
        }),
        held: true,
        ..row(&me, Some(&login), "src/a.ts", Relation::Claimed)
    };
    let stale = row(&me, None, "src/c.ts", Relation::Claimed);
    let existing = [
        held.clone(),
        row(&me, Some(&login), "src/b.ts", Relation::Claimed),
        stale,
    ];
    // a.ts is committed (clean); b.ts is still dirty though the claims file lags
    let status = dirty(&["src/b.ts"]);
    let me_v = view(&me, &my_files, &[]);
    let others = [view(&login, &login_files, &[])];
    let empty_claims = ClaimsFile {
        repo_root: HOME.into(),
        written_at: now(),
        claims: vec![],
    };
    let rows = overlaps_for(
        &me_v,
        &others,
        std::slice::from_ref(&empty_claims),
        &[(HOME.into(), status)],
        &existing,
        now(),
    );
    assert_eq!(
        summary(&rows),
        vec![
            (HOME.into(), "src/a.ts".into(), Some("t-b2".into()), Relation::Committed),
            (HOME.into(), "src/b.ts".into(), Some("t-b2".into()), Relation::Claimed),
        ]
    );
    // the decision and hold stay for the shell to answer; the relation change moves updated_at
    assert_eq!((rows[0].pending.clone(), rows[0].held), (held.pending, true));
    assert_eq!(rows[0].updated_at, now());
    assert_eq!(rows[1].updated_at, at("2026-10-25T08:00:00Z"));
    // without a status for the tree nothing is known yet: the stored rows stay
    let unknown = overlaps_for(&me_v, &others, &[empty_claims], &[], &existing, now());
    assert_eq!(unknown.len(), 3);
}

#[test]
fn keeps_granted_and_taken_over_rows() {
    let (me, login) = (task("t-a1", None), task("t-b2", None));
    let my_files = [file(HOME, "src/a.ts"), file(HOME, "src/b.ts")];
    let login_files = [file(HOME, "src/a.ts"), file(HOME, "src/b.ts")];
    let status = dirty(&["src/a.ts"]);
    let claims = build_claims(
        Path::new(HOME),
        &[TaskFiles {
            task: &login,
            files: &login_files,
        }],
        &status,
        now(),
    );
    let granted = OverlapRow {
        noted_out: true,
        decided_at: Some(at("2026-10-25T09:30:00Z")),
        ..row(&me, Some(&login), "src/a.ts", Relation::GrantedOnTop)
    };
    let taken = row(&me, Some(&login), "src/b.ts", Relation::TakenOver);
    let planned_gone = row(&me, Some(&login), "src/z.ts", Relation::PlannedBy);
    let rows = overlaps_for(
        &view(&me, &my_files, &[]),
        &[view(&login, &login_files, &[])],
        &[claims],
        &[(HOME.into(), status)],
        &[granted.clone(), taken.clone(), planned_gone],
        now(),
    );
    assert_eq!(rows, vec![granted, taken]);
}

#[test]
fn overlaps_for_keeps_taken_over_by_rows() {
    let (me, login) = (task("t-a1", None), task("t-b2", None));
    // the path left my change set when Login replaced my changes in it, and Login's work there is still dirty
    let my_files = [file(HOME, "src/b.ts")];
    let login_files = [file(HOME, "src/a.ts")];
    let status = dirty(&["src/a.ts"]);
    let claims = build_claims(
        Path::new(HOME),
        &[TaskFiles {
            task: &login,
            files: &login_files,
        }],
        &status,
        now(),
    );
    let lost = OverlapRow {
        decided_at: Some(at("2026-10-25T09:30:00Z")),
        ..row(&me, Some(&login), "src/a.ts", Relation::TakenOverBy)
    };
    let rows = overlaps_for(
        &view(&me, &my_files, &[]),
        &[view(&login, &login_files, &[])],
        &[claims],
        &[(HOME.into(), status)],
        std::slice::from_ref(&lost),
        now(),
    );
    assert_eq!(rows, vec![lost]);
}

#[test]
fn shares_tree_rules() {
    let home = Path::new(HOME);
    let (a, b) = (task("t-a1", None), task("t-b2", None));
    let wt = task("t-c3", Some("/app/.worktrees/c"));
    assert!(shares_tree(&a, &b, home));
    assert!(!shares_tree(&a, &wt, home));
    assert!(!shares_tree(&wt, &a, home));
    let mut elsewhere = task("t-d4", None);
    elsewhere.project_id = crate::domain::ProjectId::parse("p-2b-2").unwrap();
    assert!(!shares_tree(&a, &elsewhere, home));
    // linked roots are always worked in place
    let api = Path::new("/api");
    let mut wt_linked = task("t-e5", Some("/app/.worktrees/e"));
    wt_linked.linked_roots = vec![api.into()];
    let mut other_linked = task("t-f6", Some("/app/.worktrees/f"));
    other_linked.linked_roots = vec![api.into()];
    assert!(shares_tree(&wt_linked, &other_linked, api));
    // a task of the linked project in its home root, no worktree
    let mut api_task = task("t-e7", None);
    api_task.project_id = crate::domain::ProjectId::parse("p-2b-2").unwrap();
    assert!(shares_tree(&wt_linked, &api_task, api));
    let api_wt = task("t-f8", Some("/api/.worktrees/h"));
    assert!(!shares_tree(&wt_linked, &api_wt, api));
}

#[test]
fn a_granted_row_goes_once_neither_task_claims_the_path() {
    let (me, login) = (task("t-a1", None), task("t-b2", None));
    let my_files = [file(HOME, "src/a.ts")];
    let login_files = [file(HOME, "src/a.ts")];
    let granted = OverlapRow {
        decided_at: Some(at("2026-10-25T09:30:00Z")),
        ..row(&me, Some(&login), "src/a.ts", Relation::GrantedOnTop)
    };
    let run = |claimants: &[(&Task, &[AttributedFile])], status: Vec<StatusEntry>, with_claims: bool| {
        let tasks: Vec<TaskFiles<'_>> = claimants
            .iter()
            .map(|(task, files)| TaskFiles { task, files })
            .collect();
        let claims = build_claims(Path::new(HOME), &tasks, &status, now());
        overlaps_for(
            &view(&me, &my_files, &[]),
            &[view(&login, &login_files, &[])],
            if with_claims {
                std::slice::from_ref(&claims)
            } else {
                &[]
            },
            &[(HOME.into(), status.clone())],
            std::slice::from_ref(&granted),
            now(),
        )
    };
    // only my own edits on top are still uncommitted: the grant still matters
    assert_eq!(
        run(&[(&me, &my_files)], dirty(&["src/a.ts"]), true),
        vec![granted.clone()]
    );
    // both committed: neither claims it, the row goes
    assert!(run(&[(&me, &my_files), (&login, &login_files)], dirty(&[]), true).is_empty());
    // no claims file read for the repo yet: kept
    assert_eq!(run(&[], dirty(&[]), false), vec![granted.clone()]);
    // an open decision keeps it
    let pending = OverlapRow {
        pending: Some(PendingDecision::Command {
            command: "git add -A".into(),
            kind: CommandKind::Commits,
            paths: vec!["src/a.ts".into()],
        }),
        ..granted.clone()
    };
    let claims = build_claims(Path::new(HOME), &[], &[], now());
    let kept = overlaps_for(
        &view(&me, &my_files, &[]),
        &[view(&login, &login_files, &[])],
        &[claims],
        &[(HOME.into(), vec![])],
        std::slice::from_ref(&pending),
        now(),
    );
    assert_eq!(kept, vec![pending]);
}

#[test]
fn a_taken_over_row_goes_once_the_path_is_committed_and_clean() {
    let me = task("t-a1", None);
    let mut login = task("t-b2", None);
    login.stage = Stage::Done;
    let my_files = [file(HOME, "src/a.ts")];
    let taken = OverlapRow {
        decided_at: Some(at("2026-10-25T09:30:00Z")),
        ..row(&me, Some(&login), "src/a.ts", Relation::TakenOver)
    };
    let run = |me: &Task, other: &Task, stored: &OverlapRow, status: Vec<StatusEntry>| {
        overlaps_for(
            &view(me, &my_files, &[]),
            &[view(other, &[], &[])],
            &[],
            &[(HOME.into(), status)],
            std::slice::from_ref(stored),
            now(),
        )
    };
    // still dirty: kept
    assert_eq!(run(&me, &login, &taken, dirty(&["src/a.ts"])), vec![taken.clone()]);
    // committed and clean, and the task that lost it is Done: the Replace is settled, the row goes
    assert!(run(&me, &login, &taken, dirty(&[])).is_empty());
    // the task that lost it is not Done yet: its Done check still reads the row
    let mut busy = login.clone();
    busy.stage = Stage::Commit;
    assert_eq!(run(&me, &busy, &taken, dirty(&[])), vec![taken.clone()]);
    // status unknown: kept
    let unknown = overlaps_for(
        &view(&me, &my_files, &[]),
        &[view(&login, &[], &[])],
        &[],
        &[],
        std::slice::from_ref(&taken),
        now(),
    );
    assert_eq!(unknown, vec![taken.clone()]);
    // the mirror row on the side that lost it goes the same way, once that side is Done
    let lost = OverlapRow {
        decided_at: Some(at("2026-10-25T09:30:00Z")),
        ..row(&login, Some(&me), "src/a.ts", Relation::TakenOverBy)
    };
    assert!(run(&login, &me, &lost, dirty(&[])).is_empty());
    assert_eq!(run(&busy, &me, &lost, dirty(&[])), vec![lost.clone()]);
}
