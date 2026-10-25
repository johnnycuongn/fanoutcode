//! The developer's answer to a pending decision (§5.4, §5.5, §5.6), as pure functions: what `overlap_decide` writes
//! (the grant, the relation, the timeline lines, the prompts), which other open decisions the new grant answers too,
//! which held decisions a clean file releases, and the `OverlapDecided` log line.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::git::StatusEntry;

use super::grants::{Grant, GrantScope, GrantsFile};
use super::prompts::{go_ahead, go_ahead_command, told};
use super::{Decision, OverlapRow, PendingDecision, Relation};

/// Everything one decision writes. `my_prompt` goes into this task's session, `other_prompt` to the other task's
/// note inbox; the timeline texts are `TimelineKind::Overlap` details. `taken_over` moves the path from the other
/// task's change set into this one's; `hold` keeps the decision open with `held` set. `other_relation` is the row the
/// other task gets for the same path (against this task), so both Overlap tabs show a Replace.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionOutcome {
    pub grant: Option<Grant>,
    pub relation: Relation,
    pub other_relation: Option<Relation>,
    pub my_prompt: Option<String>,
    pub other_prompt: Option<String>,
    pub my_timeline: String,
    pub other_timeline: Option<String>,
    pub taken_over: bool,
    pub hold: bool,
}

/// The outcome of `decision` on `row`. For a denied shell command (§5.6) the only answers are Proceed and Hold:
/// `OnTopFile`, `OnTopAll` and `Replace` all proceed, granting `AllFiles` on the other task. The other task is told
/// nothing when there is none (an unattributed session) or the answer is Hold.
pub fn outcome(
    decision: Decision,
    row: &OverlapRow,
    my_title: &str,
    other_title: &str,
    now: DateTime<Utc>,
) -> DecisionOutcome {
    let path = row.path.as_str();
    let grant = |scope: GrantScope| {
        Some(Grant {
            other_task: row.other_task.clone(),
            scope,
            granted_at: now,
        })
    };
    let file = || GrantScope::File {
        repo: row.repo_root.clone(),
        path: path.to_owned(),
    };
    let told_other = |d: Decision| row.other_task.as_ref().map(|_| told(d, my_title, path));
    if let Some(PendingDecision::Command { command }) = &row.pending {
        if decision != Decision::Hold {
            return DecisionOutcome {
                grant: grant(GrantScope::AllFiles),
                relation: Relation::GrantedOnTop,
                other_relation: None,
                my_prompt: Some(go_ahead_command(other_title, command)),
                other_prompt: told_other(Decision::OnTopAll),
                my_timeline: format!("proceed with `{command}` over {other_title}'s changes"),
                other_timeline: row
                    .other_task
                    .as_ref()
                    .map(|_| format!("{my_title} ran `{command}` over your changes")),
                taken_over: false,
                hold: false,
            };
        }
    }
    let other_timeline = |text: String| row.other_task.as_ref().map(|_| text);
    match decision {
        Decision::OnTopFile => DecisionOutcome {
            grant: grant(file()),
            relation: Relation::GrantedOnTop,
            other_relation: None,
            my_prompt: Some(go_ahead(decision, other_title, path)),
            other_prompt: told_other(decision),
            my_timeline: format!("on top of {other_title} in {path}"),
            other_timeline: other_timeline(format!("{my_title} edits on top of you in {path}")),
            taken_over: false,
            hold: false,
        },
        Decision::OnTopAll => DecisionOutcome {
            grant: grant(GrantScope::AllFiles),
            relation: Relation::GrantedOnTop,
            other_relation: None,
            my_prompt: Some(go_ahead(decision, other_title, path)),
            other_prompt: told_other(decision),
            my_timeline: format!("on top of {other_title} in every file it touches, from {path}"),
            other_timeline: other_timeline(format!("{my_title} edits on top of you in every file, from {path}")),
            taken_over: false,
            hold: false,
        },
        Decision::Replace => DecisionOutcome {
            grant: grant(file()),
            relation: Relation::TakenOver,
            other_relation: row.other_task.as_ref().map(|_| Relation::TakenOverBy),
            my_prompt: Some(go_ahead(decision, other_title, path)),
            other_prompt: told_other(decision),
            my_timeline: format!("replaced {other_title}'s changes in {path}"),
            other_timeline: other_timeline(format!("{my_title} took over {path}")),
            taken_over: true,
            hold: false,
        },
        Decision::Hold => DecisionOutcome {
            grant: None,
            relation: row.relation,
            other_relation: None,
            my_prompt: None,
            other_prompt: None,
            my_timeline: format!("holding for {other_title} on {path}"),
            other_timeline: None,
            taken_over: false,
            hold: true,
        },
    }
}

/// The other open decisions of `rows` that `grants` now covers (an `AllFiles` grant answers every file of its other
/// task at once), `except` the one just decided.
pub fn covered_pending<'a>(rows: &'a [OverlapRow], grants: &GrantsFile, except: &OverlapRow) -> Vec<&'a OverlapRow> {
    rows.iter()
        .filter(|r| r.pending.is_some())
        .filter(|r| !(r.repo_root == except.repo_root && r.path == except.path && r.other_task == except.other_task))
        .filter(|r| grants.covers(&r.repo_root, &r.path, r.other_task.as_ref()))
        .collect()
}

/// Why a held decision was released: the other task's attributed file is clean (it committed it), or a file no task
/// attributed (another session's, or one whose owner let go of it) is clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Committed,
    Clean,
}

/// The held decisions of `rows` in `repo` that are over (§5.4, answer 4): only those whose path `status` (the
/// repo's fresh `git status`) shows clean. A claimant missing from the claims file never releases a hold by itself:
/// its claims may have been lost on a relaunch, or it was archived with the changes still on disk. `attributed`
/// says whether the row's other task attributes the path, which makes the release `Committed` rather than `Clean`.
pub fn released_holds<'a>(
    rows: &'a [OverlapRow],
    repo: &Path,
    status: &[StatusEntry],
    attributed: impl Fn(&OverlapRow) -> bool,
) -> Vec<(&'a OverlapRow, Release)> {
    rows.iter()
        .filter(|r| r.pending.is_some() && r.held && r.repo_root == repo)
        .filter(|r| !status.iter().any(|e| e.path == r.path))
        .map(|r| {
            let why = if r.other_task.is_some() && attributed(r) {
                Release::Committed
            } else {
                Release::Clean
            };
            (r, why)
        })
        .collect()
}

/// The timeline line for a released hold, worded for what happened.
pub fn release_line(why: Release, path: &str, other_title: &str) -> String {
    match why {
        Release::Committed => format!("{path} is free: {other_title} committed it"),
        Release::Clean => format!("{path} is free: it has no uncommitted changes now"),
    }
}

/// The `OverlapDecided` event-log line (§5.4) for one task's log: `other` is the task on the other side from that
/// log's task. A display mirror for the Team view; the app never acts on it.
pub fn decided_line(
    repo: &Path,
    path: &str,
    other: Option<&crate::domain::TaskId>,
    decision: Decision,
    now: DateTime<Utc>,
) -> String {
    let payload = serde_json::json!({
        "repo": repo,
        "path": path,
        "other_task": other,
        "decision": format!("{decision:?}"),
    });
    format!("OverlapDecided\t{}\t{payload}\n", now.format("%Y-%m-%dT%H:%M:%SZ"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::TaskId;
    use crate::git::FileStatus;
    use crate::hooks::parse::parse_line;
    use crate::hooks::HookEvent;

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
    fn a_command_proceeds_with_all_files_and_an_unattributed_other_is_told_nothing() {
        let cmd = Some(PendingDecision::Command {
            command: "git add -A".into(),
        });
        for d in [Decision::OnTopFile, Decision::OnTopAll, Decision::Replace] {
            let out = outcome(
                d,
                &row("src/a.ts", Some("t-b2"), cmd.clone(), false),
                "Checkout",
                "Login",
                now(),
            );
            assert_eq!(out.grant.unwrap().scope, GrantScope::AllFiles);
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
}
