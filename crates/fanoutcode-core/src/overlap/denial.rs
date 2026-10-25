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
        GitRule::CommitAll | GitRule::AddAll | GitRule::CommitPath | GitRule::AddPath => CommandKind::Commits,
        GitRule::Stash | GitRule::ResetHard | GitRule::Clean | GitRule::CheckoutPath | GitRule::RestorePath => {
            CommandKind::Discards
        }
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

/// Whether `command` stashes (any `git stash` but `list` and `show`): its changes leave the working tree but are
/// kept, so the copy says "stashed" rather than "discarded".
pub fn is_stash(command: &str) -> bool {
    git_subcommands(command)
        .iter()
        .any(|(sub, args)| sub == "stash" && !matches!(args.first().map(String::as_str), Some("list" | "show")))
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
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::overlap::grants::{Grant, GrantScope};
    use crate::overlap::Relation;

    fn id(s: &str) -> TaskId {
        TaskId::parse(s).unwrap()
    }

    fn claim(path: &str, task: Option<&str>) -> Claim {
        Claim {
            path: path.into(),
            task_id: task.map(id),
            title: "Login".into(),
            stage: None,
            state: None,
            last_edit: Utc::now(),
        }
    }

    fn claims(list: Vec<Claim>) -> ClaimsFile {
        ClaimsFile {
            repo_root: "/r".into(),
            written_at: Utc::now(),
            claims: list,
        }
    }

    fn call(tool: &str, file: &str, cwd: &str, marker: &str) -> EditCall {
        EditCall {
            tool: tool.into(),
            input: json!({"file_path": file, "new_string": marker}),
            cwd: cwd.into(),
        }
    }

    #[test]
    fn edit_target_resolves_relative_paths_against_cwd() {
        let cwd = Path::new("/r/src");
        assert_eq!(
            edit_target(&json!({"file_path": "a.ts"}), cwd),
            Some("/r/src/a.ts".into())
        );
        assert_eq!(
            edit_target(&json!({"file_path": "../b.ts"}), cwd),
            Some("/r/b.ts".into())
        );
        assert_eq!(
            edit_target(&json!({"notebook_path": "/n.ipynb"}), cwd),
            Some("/n.ipynb".into())
        );
        assert_eq!(edit_target(&json!({"file_path": ""}), cwd), None);
        assert_eq!(edit_target(&Value::Null, cwd), None);
    }

    #[test]
    fn a_denied_write_takes_the_newest_matching_call_as_its_snapshot() {
        let recent = [
            call("Edit", "/r/src/a.ts", "/r", "old"),
            call("Write", "src/a.ts", "/r", "new"),
            call("Edit", "/r/src/b.ts", "/r", "other file"),
        ];
        let rows = denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            Some("src/a.ts"),
            None,
            Some(&id("t-b2")),
            None,
            &GrantsFile::default(),
            &recent,
        );
        assert_eq!(
            rows,
            vec![Denied {
                path: "src/a.ts".into(),
                other_task: Some(id("t-b2")),
                pending: PendingDecision::Edit {
                    tool: "Write".into(),
                    input: json!({"file_path": "src/a.ts", "new_string": "new"}),
                },
            }]
        );
    }

    #[test]
    fn a_denied_write_without_its_call_waits_for_the_snapshot() {
        let rows = denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            Some("src/a.ts"),
            None,
            None,
            None,
            &GrantsFile::default(),
            &[],
        );
        let row = OverlapRow {
            task_id: id("t-a1"),
            other_task: None,
            repo_root: "/r".into(),
            path: "src/a.ts".into(),
            relation: Relation::Claimed,
            pending: Some(rows[0].pending.clone()),
            held: false,
            noted_out: false,
            noted_in: false,
            decided_at: None,
            updated_at: Utc::now(),
        };
        assert!(awaits_snapshot(&row));
        assert!(writes(
            &call("Edit", "/r/src/a.ts", "/", "x"),
            Path::new("/r"),
            "src/a.ts"
        ));
        assert!(!writes(
            &call("Edit", "/r/src/b.ts", "/", "x"),
            Path::new("/r"),
            "src/a.ts"
        ));
        assert!(!awaits_snapshot(&OverlapRow {
            pending: Some(PendingDecision::Command {
                command: "git stash".into(),
                kind: CommandKind::Discards,
                paths: vec![],
            }),
            ..row
        }));
    }

    #[test]
    fn a_denied_command_opens_one_decision_per_claimed_path_it_touches() {
        let file = claims(vec![
            claim("src/a.ts", Some("t-b2")),
            claim("src/b.ts", Some("t-c3")),
            claim("src/mine.ts", Some("t-a1")),
            claim("src/c.ts", Some("t-b2")),
        ]);
        // C3's claims are already answered for this command; an edit grant on B2's c.ts answers nothing
        let mut grants = GrantsFile::default();
        grants.add(Grant {
            other_task: Some(id("t-c3")),
            scope: GrantScope::Command {
                repo: "/r".into(),
                command: "git add -A".into(),
            },
            granted_at: Utc::now(),
        });
        grants.add(Grant {
            other_task: Some(id("t-b2")),
            scope: GrantScope::File {
                repo: "/r".into(),
                path: "src/c.ts".into(),
            },
            granted_at: Utc::now(),
        });
        let rows = denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            None,
            Some("git add -A"),
            Some(&id("t-b2")),
            Some(&file),
            &grants,
            &[],
        );
        let keys: Vec<(&str, Option<&str>)> = rows
            .iter()
            .map(|d| (d.path.as_str(), d.other_task.as_ref().map(TaskId::as_str)))
            .collect();
        assert_eq!(keys, vec![("src/a.ts", Some("t-b2")), ("src/c.ts", Some("t-b2"))]);
        assert!(rows.iter().all(|d| d.pending
            == PendingDecision::Command {
                command: "git add -A".into(),
                kind: CommandKind::Commits,
                paths: vec!["src/a.ts".into(), "src/c.ts".into()],
            }));
    }

    #[test]
    fn a_command_whose_claims_moved_still_opens_a_decision() {
        // t-b2's claim left the file before the denial was processed (it committed): the command still waits on the
        // claims that are left, rather than on nothing
        let file = claims(vec![claim("src/b.ts", Some("t-c3")), claim("src/g.ts", None)]);
        for command in ["git add a.ts", "git add src/a.ts"] {
            let rows = denial_rows(
                &id("t-a1"),
                Path::new("/r"),
                None,
                Some(command),
                Some(&id("t-b2")),
                Some(&file),
                &GrantsFile::default(),
                &[],
            );
            let keys: Vec<(&str, Option<&str>)> = rows
                .iter()
                .map(|d| (d.path.as_str(), d.other_task.as_ref().map(TaskId::as_str)))
                .collect();
            assert_eq!(keys, vec![("src/b.ts", Some("t-c3")), ("src/g.ts", None)], "{command}");
        }
    }

    #[test]
    fn a_denied_command_the_root_cannot_place_falls_back_to_the_named_task() {
        let file = claims(vec![claim("src/a.ts", Some("t-b2")), claim("src/b.ts", Some("t-c3"))]);
        // run from a subdirectory the log does not record: evaluated from the root it touches nothing
        let rows = denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            None,
            Some("git add a.ts"),
            Some(&id("t-b2")),
            Some(&file),
            &GrantsFile::default(),
            &[],
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "src/a.ts");
        assert_eq!(
            rows[0].pending,
            PendingDecision::Command {
                command: "git add a.ts".into(),
                kind: CommandKind::Commits,
                paths: vec!["src/a.ts".into()],
            }
        );
        assert!(denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            None,
            Some("git add -A"),
            None,
            None,
            &GrantsFile::default(),
            &[]
        )
        .is_empty());
    }

    #[test]
    fn command_kind_names_what_the_command_does_to_the_other_tasks_work() {
        let held = [claim("src/a.ts", Some("t-b2"))];
        let refs: Vec<&Claim> = held.iter().collect();
        let kind = |c: &str| command_kind(c, Path::new("/r"), &refs);
        for c in [
            "git commit -am wip",
            "git add -A",
            "git commit src/a.ts -m x",
            "git add src/a.ts",
        ] {
            assert_eq!(kind(c), CommandKind::Commits, "{c}");
        }
        for c in [
            "git stash",
            "git reset --hard",
            "git clean -fd",
            "git checkout -- src/a.ts",
            "git restore src/a.ts",
        ] {
            assert_eq!(kind(c), CommandKind::Discards, "{c}");
        }
        // what the root cannot place is read from its words; what has no git command at all counts as discarding
        assert_eq!(kind("cd src && git add a.ts"), CommandKind::Commits);
        assert_eq!(kind("git -C sub restore a.ts"), CommandKind::Discards);
        assert_eq!(kind("make tidy"), CommandKind::Discards);
        for (rule, k) in [
            (GitRule::CommitAll, CommandKind::Commits),
            (GitRule::AddAll, CommandKind::Commits),
            (GitRule::CommitPath, CommandKind::Commits),
            (GitRule::AddPath, CommandKind::Commits),
            (GitRule::Stash, CommandKind::Discards),
            (GitRule::ResetHard, CommandKind::Discards),
            (GitRule::Clean, CommandKind::Discards),
            (GitRule::CheckoutPath, CommandKind::Discards),
            (GitRule::RestorePath, CommandKind::Discards),
        ] {
            assert_eq!(rule_kind(rule), k, "{rule:?}");
        }
    }

    #[test]
    fn is_stash_reads_the_git_subcommand() {
        for c in [
            "git stash",
            "git stash push -u",
            "git -C /r stash",
            "ls && git stash pop",
        ] {
            assert!(is_stash(c), "{c}");
        }
        for c in [
            "git stash list",
            "git stash show",
            "git commit -m stash",
            "git reset --hard",
            "echo git stash",
        ] {
            assert!(!is_stash(c), "{c}");
        }
    }
}
