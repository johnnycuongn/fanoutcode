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
        staged: false,
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
        None,
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
        None,
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
            paths: vec!["src/b.ts".into()],
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
    let kind = |c: &str| command_kind(c, Path::new("/r"), Path::new("/r"), &refs);
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
        "ls && git stash save wip",
    ] {
        assert!(is_stash(c), "{c}");
    }
    // the stash stack commands are worded on their own (`stash_stack_op`)
    for c in [
        "git stash pop",
        "git stash apply stash@{1}",
        "git stash drop",
        "git stash clear",
        "git stash list",
        "git stash show",
        "git commit -m stash",
        "git reset --hard",
        "echo git stash",
    ] {
        assert!(!is_stash(c), "{c}");
    }
}

#[test]
fn a_relative_command_from_a_subfolder_lists_only_the_files_it_touches() {
    let file = claims(vec![
        claim("src/a.ts", Some("t-b2")),
        claim("src/b.ts", Some("t-b2")),
        claim("a.ts", Some("t-b2")),
    ]);
    let rows = |cwd: Option<&Path>| {
        denial_rows(
            &id("t-a1"),
            Path::new("/r"),
            cwd,
            None,
            Some("git add a.ts"),
            Some(&id("t-b2")),
            Some(&file),
            &GrantsFile::default(),
            &[],
        )
    };
    let from_src = rows(Some(Path::new("/r/src")));
    assert_eq!(from_src.len(), 1, "{from_src:?}");
    assert_eq!(from_src[0].path, "src/a.ts");
    assert!(
        matches!(&from_src[0].pending, PendingDecision::Command { paths, kind, .. } if paths == &vec!["src/a.ts".to_owned()] && *kind == CommandKind::Commits)
    );
    // without a cwd (an older line) it is read from the repo root
    let from_root = rows(None);
    assert_eq!(from_root.len(), 1);
    assert_eq!(from_root[0].path, "a.ts");
}

#[test]
fn stash_stack_op_reads_pop_apply_drop_and_clear() {
    assert_eq!(stash_stack_op("git stash pop"), Some(StashOp::Apply));
    assert_eq!(stash_stack_op("ls && git stash apply stash@{1}"), Some(StashOp::Apply));
    assert_eq!(stash_stack_op("git stash drop"), Some(StashOp::Drop));
    assert_eq!(stash_stack_op("git stash clear"), Some(StashOp::Drop));
    for c in ["git stash", "git stash push", "git stash list", "git reset --hard"] {
        assert_eq!(stash_stack_op(c), None, "{c}");
    }
}
