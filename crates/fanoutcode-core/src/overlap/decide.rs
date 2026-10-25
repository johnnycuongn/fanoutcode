//! The developer's answer to a pending decision (§5.4, §5.5, §5.6), as pure functions: what `overlap_decide` writes
//! (the grant, the relation, the timeline lines, the prompts), which other open decisions the new grant answers too,
//! which held decisions a rebuilt claims file releases, and the `OverlapDecided` log line.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::claims::ClaimsFile;
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
/// `OnTopFile`, `OnTopAll` and `Replace` all proceed, granting one run of exactly that command over the other
/// task's claims (`GrantScope::Command`; an edit grant would switch the git guard off). The other task is told
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
                grant: grant(GrantScope::Command {
                    command: command.trim().to_owned(),
                }),
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

/// The other open decisions of `rows` that `grants` now covers, `except` the one just decided: an edit grant answers
/// the edits it covers (an `AllFiles` grant every file of its other task at once), a `Command` grant the stops of
/// that same command on its other task's files.
pub fn covered_pending<'a>(rows: &'a [OverlapRow], grants: &GrantsFile, except: &OverlapRow) -> Vec<&'a OverlapRow> {
    rows.iter()
        .filter(|r| !(r.repo_root == except.repo_root && r.path == except.path && r.other_task == except.other_task))
        .filter(|r| match &r.pending {
            None => false,
            Some(PendingDecision::Edit { .. }) => grants.covers(&r.repo_root, &r.path, r.other_task.as_ref()),
            Some(PendingDecision::Command { command, .. }) => grants.allows_command(command, r.other_task.as_ref()),
        })
        .collect()
}

/// The held decisions of `rows` in `file`'s repo whose claim is gone from it: the other task committed (or let go
/// of) the file, so the developer's Hold is over (§5.4, answer 4).
pub fn released_holds<'a>(rows: &'a [OverlapRow], file: &ClaimsFile) -> Vec<&'a OverlapRow> {
    rows.iter()
        .filter(|r| r.pending.is_some() && r.held && r.repo_root == file.repo_root)
        .filter(|r| {
            !file
                .claims
                .iter()
                .any(|c| c.path == r.path && c.task_id == r.other_task)
        })
        .collect()
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
    use crate::hooks::parse::parse_line;
    use crate::hooks::HookEvent;
    use crate::overlap::claims::Claim;

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
        let cmd = |c: &str| Some(PendingDecision::Command { command: c.into() });
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
                    command: "git add -A".into()
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

    #[test]
    fn a_hold_is_released_once_its_claim_leaves_the_claims_file() {
        let rows = [
            row("src/a.ts", Some("t-b2"), edit(), true),
            row("src/b.ts", Some("t-b2"), edit(), true),
            row("src/c.ts", Some("t-b2"), edit(), false),
            row("src/d.ts", None, edit(), true),
        ];
        let claim = |path: &str, task: Option<&str>| Claim {
            path: path.into(),
            task_id: task.map(id),
            title: "Login".into(),
            stage: None,
            state: None,
            last_edit: now(),
        };
        let file = ClaimsFile {
            repo_root: "/r".into(),
            written_at: now(),
            claims: vec![claim("src/b.ts", Some("t-b2")), claim("src/a.ts", Some("t-c3"))],
        };
        let freed: Vec<&str> = released_holds(&rows, &file).iter().map(|r| r.path.as_str()).collect();
        assert_eq!(freed, vec!["src/a.ts", "src/d.ts"]);
        let elsewhere = ClaimsFile {
            repo_root: "/other".into(),
            ..file
        };
        assert!(released_holds(&rows, &elsewhere).is_empty());
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
