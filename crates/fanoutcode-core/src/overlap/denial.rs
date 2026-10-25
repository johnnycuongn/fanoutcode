//! What a `ClaimDenied` line of a task's own event log may do (§5.4, §5.6): open pending decisions on that task and
//! nothing else. The shell pins the task from them; grants, relations and the other task are never touched here.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::claims::{Claim, ClaimsFile};
use super::git_rules::{evaluate_git, rel_in, GitRule};
use super::grants::GrantsFile;
use super::{CommandKind, OverlapRow, PendingDecision};
use crate::domain::TaskId;
use crate::workspace::guard::rules::{clean, strip_prefixes};
use crate::workspace::guard::shell_words::segments;

/// One decision a denial opens: `path` of the denial's repo, held by `other_task`.
#[derive(Debug, Clone, PartialEq)]
pub struct Denied {
    pub path: String,
    pub other_task: Option<TaskId>,
    pub pending: PendingDecision,
}

/// An edit tool call seen in a `PreToolUse` line: its tool, input and the session's `cwd`.
#[derive(Debug, Clone, PartialEq)]
pub struct EditCall {
    pub tool: String,
    pub input: Value,
    pub cwd: PathBuf,
}

/// The file an edit tool's input names (`file_path`, or `notebook_path`), resolved against `cwd` when relative.
pub fn edit_target(input: &Value, cwd: &Path) -> Option<PathBuf> {
    let raw = ["file_path", "notebook_path"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .filter(|s| !s.is_empty())?;
    let p = Path::new(raw);
    Some(clean(&if p.is_absolute() { p.to_owned() } else { cwd.join(p) }))
}

/// Whether `call` writes `path` of `repo`.
pub fn writes(call: &EditCall, repo: &Path, path: &str) -> bool {
    edit_target(&call.input, &call.cwd)
        .and_then(|t| rel_in(repo, &t))
        .as_deref()
        == Some(path)
}

/// An `Edit` decision still waiting for its tool input (the denial's line came before the `PreToolUse` one).
pub fn awaits_snapshot(row: &OverlapRow) -> bool {
    matches!(&row.pending, Some(PendingDecision::Edit { input, .. }) if input.is_null())
}

/// What a caught rule does to the other task's work.
pub fn rule_kind(rule: GitRule) -> CommandKind {
    match rule {
        GitRule::CommitAll | GitRule::CommitStaged | GitRule::AddAll | GitRule::CommitPath | GitRule::AddPath => {
            CommandKind::Commits
        }
        // a forced checkout or switch, `rm`, or a command the parser cannot read counts as the worst case
        GitRule::Stash
        | GitRule::ResetHard
        | GitRule::Clean
        | GitRule::CheckoutPath
        | GitRule::CheckoutForce
        | GitRule::SwitchForce
        | GitRule::RestorePath
        | GitRule::Remove
        | GitRule::Unparsed => CommandKind::Discards,
    }
}

/// The `git` subcommands of `command` with their arguments, segment by segment (global options skipped).
fn git_subcommands(command: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for seg in segments(command) {
        let words = strip_prefixes(&seg.words);
        let Some((first, rest)) = words.split_first() else {
            continue;
        };
        if first.rsplit('/').next() != Some("git") {
            continue;
        }
        let mut i = 0;
        while let Some(w) = rest.get(i).filter(|w| w.starts_with('-')) {
            i += if matches!(w.as_str(), "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace") {
                2
            } else {
                1
            };
        }
        if let Some(sub) = rest.get(i) {
            out.push((sub.clone(), rest[i + 1..].to_vec()));
        }
    }
    out
}

/// Whether `command` stashes (any `git stash` but `list`, `show`, `pop` and `apply`): its changes leave the working
/// tree but are kept, so the copy says "stashed" rather than "discarded".
pub fn is_stash(command: &str) -> bool {
    git_subcommands(command).iter().any(|(sub, args)| {
        sub == "stash"
            && !matches!(
                args.first().map(String::as_str),
                Some("list" | "show" | "pop" | "apply")
            )
    })
}

/// Whether `command` stages without committing (a `git add`, no `git commit`): the copy then says "staged into its
/// index" rather than "committed".
pub fn stages_only(command: &str) -> bool {
    let subs = git_subcommands(command);
    subs.iter().any(|(s, _)| s == "add") && !subs.iter().any(|(s, _)| s == "commit")
}

/// What a stopped `command` would do to the claims of `claimed` in `repo` (§5.6): the kind of the rule
/// `evaluate_git` catches from the repo root, else (a command the root cannot place, such as relative paths from a
/// subdirectory) the first git subcommand it names; a command it cannot read at all counts as discarding.
pub fn command_kind(command: &str, repo: &Path, claimed: &[&Claim]) -> CommandKind {
    if let Some((rule, _)) = evaluate_git(command, repo, repo, claimed) {
        return rule_kind(rule);
    }
    git_subcommands(command)
        .iter()
        .find_map(|(sub, _)| match sub.as_str() {
            "commit" | "add" => Some(CommandKind::Commits),
            "stash" | "reset" | "clean" | "checkout" | "restore" => Some(CommandKind::Discards),
            _ => None,
        })
        .unwrap_or(CommandKind::Discards)
}

/// The pending decisions one `ClaimDenied` of task `own` opens in `repo`:
/// - a denied write (`path`): one, with the newest of `recent` that writes that file as its snapshot, else an `Edit`
///   with a null input that a later `PreToolUse` fills in;
/// - a denied command: one per claimed path of another task it touches (`evaluate_git` from the repo root), else one
///   per claim of `other` in the repo, else (the claims moved before the line was read) one per claim of anyone else
///   left in the repo, each with the command. Empty without claims (nothing to key them on).
#[allow(clippy::too_many_arguments)]
pub fn denial_rows(
    own: &TaskId,
    repo: &Path,
    path: Option<&str>,
    command: Option<&str>,
    other: Option<&TaskId>,
    claims: Option<&ClaimsFile>,
    grants: &GrantsFile,
    recent: &[EditCall],
) -> Vec<Denied> {
    if let Some(path) = path {
        let pending = match recent.iter().rev().find(|c| writes(c, repo, path)) {
            Some(c) => PendingDecision::Edit {
                tool: c.tool.clone(),
                input: c.input.clone(),
            },
            None => PendingDecision::Edit {
                tool: "Edit".into(),
                input: Value::Null,
            },
        };
        return vec![Denied {
            path: path.to_owned(),
            other_task: other.cloned(),
            pending,
        }];
    }
    let (Some(command), Some(file)) = (command, claims) else {
        return Vec::new();
    };
    let others: Vec<&Claim> = file
        .claims
        .iter()
        .filter(|c| c.task_id.as_ref() != Some(own))
        .filter(|c| !grants.allows_command(command, &file.repo_root, c.task_id.as_ref()))
        .collect();
    let touched: Vec<&Claim> = match evaluate_git(command, repo, repo, &others) {
        Some((_, paths)) => others.iter().copied().filter(|c| paths.contains(&c.path)).collect(),
        None => Vec::new(),
    };
    let of_other = || -> Vec<&Claim> { others.iter().copied().filter(|c| c.task_id.as_ref() == other).collect() };
    // the claims may have moved since the hook's check: the session waits on what is left rather than on nothing
    let named = [touched, of_other()]
        .into_iter()
        .find(|v| !v.is_empty())
        .unwrap_or_else(|| others.clone());
    let kind = command_kind(command, repo, &others);
    named
        .iter()
        .map(|c| {
            let mut paths: Vec<String> = named
                .iter()
                .filter(|o| o.task_id == c.task_id)
                .map(|o| o.path.clone())
                .collect();
            paths.sort();
            paths.dedup();
            Denied {
                path: c.path.clone(),
                other_task: c.task_id.clone(),
                pending: PendingDecision::Command {
                    command: command.to_owned(),
                    kind,
                    paths,
                },
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "denial_tests.rs"]
mod tests;
