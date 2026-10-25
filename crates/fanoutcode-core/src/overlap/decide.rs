//! The developer's answer to a pending decision (§5.4, §5.5, §5.6), as pure functions: what `overlap_decide` writes
//! (the grant, the relation, the timeline lines, the prompts), which other open decisions the new grant answers too,
//! which held decisions a clean file releases, and the `OverlapDecided` log line.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::git::StatusEntry;

use super::grants::{Grant, GrantScope, GrantsFile};
use super::prompts::{command_free, go_ahead, go_ahead_command, told, told_command};
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
    if let Some(PendingDecision::Command { command, kind, paths }) = &row.pending {
        if decision != Decision::Hold {
            return DecisionOutcome {
                grant: grant(GrantScope::Command {
                    repo: row.repo_root.clone(),
                    command: command.trim().to_owned(),
                }),
                relation: Relation::GrantedOnTop,
                other_relation: None,
                my_prompt: Some(go_ahead_command(other_title, command)),
                other_prompt: row
                    .other_task
                    .as_ref()
                    .map(|_| told_command(my_title, command, *kind, paths)),
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
            Some(PendingDecision::Command { command, .. }) => {
                grants.allows_command(command, &r.repo_root, r.other_task.as_ref())
            }
        })
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

/// The other open stops of the same command as `row` on the same other task: a Hold on one holds them all (§5.6).
pub fn command_rows<'a>(rows: &'a [OverlapRow], row: &OverlapRow) -> Vec<&'a OverlapRow> {
    let Some(command) = command_of(row) else {
        return Vec::new();
    };
    rows.iter()
        .filter(|r| r.other_task == row.other_task && !(r.repo_root == row.repo_root && r.path == row.path))
        .filter(|r| command_of(r) == Some(command))
        .collect()
}

/// The trimmed command of an open command stop.
fn command_of(row: &OverlapRow) -> Option<&str> {
    match &row.pending {
        Some(PendingDecision::Command { command, .. }) => Some(command.trim()),
        _ => None,
    }
}

/// Whether the go-ahead for `command` may be typed in: no open stop of that command is left in `rows` (the task's
/// rows once the decision is written). A command that touches two tasks' files runs only once both are answered.
pub fn command_go_ahead_due(rows: &[OverlapRow], command: &str) -> bool {
    !rows.iter().any(|r| command_of(r) == Some(command.trim()))
}

/// What the session is told when held decisions are released (`freed`, each with its other task's title): the
/// "file is now free" go-ahead per released edit, and for a stopped command one message naming it, once no open
/// stop of that command remains in `remaining` (the task's rows after the release).
pub fn release_prompts(freed: &[(OverlapRow, String)], remaining: &[OverlapRow]) -> Vec<String> {
    let mut out = Vec::new();
    let mut done: Vec<&str> = Vec::new();
    for (r, title) in freed {
        let Some(command) = command_of(r) else {
            out.push(go_ahead(Decision::Hold, title, &r.path));
            continue;
        };
        if done.contains(&command) {
            continue;
        }
        done.push(command);
        if !command_go_ahead_due(remaining, command) {
            continue;
        }
        let mut titles: Vec<String> = Vec::new();
        for (_, t) in freed.iter().filter(|(o, _)| command_of(o) == Some(command)) {
            if !titles.contains(t) {
                titles.push(t.clone());
            }
        }
        out.push(command_free(&titles, command));
    }
    out
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
#[path = "decide_tests.rs"]
mod tests;
