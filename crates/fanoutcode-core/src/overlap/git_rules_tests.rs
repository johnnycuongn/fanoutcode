use super::*;

fn claim(path: &str) -> Claim {
    Claim {
        path: path.into(),
        task_id: Some(crate::domain::TaskId::parse("t-b2").unwrap()),
        title: "Checkout".into(),
        stage: None,
        state: None,
        last_edit: "2026-10-25T10:00:00Z".parse().unwrap(),
        staged: false,
    }
}

fn staged(path: &str) -> Claim {
    Claim {
        staged: true,
        ..claim(path)
    }
}

fn eval(command: &str, cwd: &str, claimed: &[Claim]) -> Option<(GitRule, Vec<String>)> {
    let refs: Vec<&Claim> = claimed.iter().collect();
    evaluate_git(command, Path::new(cwd), Path::new("/r"), &refs)
}

fn claims() -> Vec<Claim> {
    vec![claim("src/a.ts"), claim("src/deep/b.ts"), claim("README.md")]
}

fn paths(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn commit_a_with_a_claimed_dirty_path_is_caught() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "git commit -a -m wip",
        "git commit -am 'wip'",
        "git commit --all",
        "git -c x=y commit -qa",
    ] {
        assert_eq!(
            eval(cmd, "/r", &claims()),
            Some((GitRule::CommitAll, all.clone())),
            "{cmd}"
        );
    }
    assert_eq!(
        eval("git commit -m 'a' src/a.ts mine.ts", "/r", &claims()),
        Some((GitRule::CommitPath, paths(&["src/a.ts"])))
    );
    assert_eq!(
        eval("git commit -m x -- deep", "/r/src", &claims()),
        Some((GitRule::CommitPath, paths(&["src/deep/b.ts"])))
    );
    // -m's value is not a path, even when it spells one
    assert_eq!(eval("git commit -m src/a.ts", "/r", &claims()), None);
    // nothing claimed: never caught
    assert_eq!(eval("git commit -a", "/r", &[]), None);
}

#[test]
fn commit_naming_only_own_paths_is_not_caught() {
    assert_eq!(
        eval("git commit -m 'mine' src/mine.ts lib/other.ts", "/r", &claims()),
        None
    );
    assert_eq!(eval("git add src/mine.ts && git commit -m done", "/r", &claims()), None);
    // the a.ts prefix is not a directory of src/a.tsx
    assert_eq!(eval("git add src/a.tsx", "/r", &[claim("src/a.ts")]), None);
}

#[test]
fn add_dot_and_add_all_are_caught() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "git add -A",
        "git add --all",
        "git add .",
        "git add -u",
        "cd src && git add ..",
    ] {
        assert_eq!(
            eval(cmd, "/r", &claims()),
            Some((GitRule::AddAll, all.clone())),
            "{cmd}"
        );
    }
    assert_eq!(
        eval("git add .", "/r/src", &claims()),
        Some((GitRule::AddPath, paths(&["src/a.ts", "src/deep/b.ts"])))
    );
    assert_eq!(
        eval("git -C /r add README.md", "/elsewhere", &claims()),
        Some((GitRule::AddPath, paths(&["README.md"])))
    );
    // a git command in another repo is not this repo's business
    assert_eq!(eval("git add -A", "/other", &claims()), None);
}

#[test]
fn checkout_and_restore_of_a_claimed_path_are_caught() {
    assert_eq!(
        eval("git checkout -- src/a.ts", "/r", &claims()),
        Some((GitRule::CheckoutPath, paths(&["src/a.ts"])))
    );
    assert_eq!(
        eval("git checkout src/a.ts", "/r", &claims()),
        Some((GitRule::CheckoutPath, paths(&["src/a.ts"])))
    );
    assert_eq!(eval("git checkout .", "/r", &claims()).map(|h| h.1.len()), Some(3));
    assert_eq!(
        eval("git restore deep/b.ts", "/r/src", &claims()),
        Some((GitRule::RestorePath, paths(&["src/deep/b.ts"])))
    );
    assert_eq!(
        eval("git restore .", "/r", &claims()).map(|h| h.0),
        Some(GitRule::RestorePath)
    );
    // a branch switch, a new branch, an unstage only, or an unclaimed path are left alone
    assert_eq!(eval("git checkout main", "/r", &claims()), None);
    assert_eq!(eval("git checkout -b src/a.ts", "/r", &claims()), None);
    assert_eq!(eval("git restore --staged src/a.ts", "/r", &claims()), None);
    assert_eq!(eval("git checkout -- src/mine.ts", "/r", &claims()), None);
}

#[test]
fn stash_reset_hard_and_clean_are_caught_when_any_claim_exists() {
    for (cmd, rule) in [
        ("git stash", GitRule::Stash),
        ("git stash push -m x", GitRule::Stash),
        ("git stash -u", GitRule::Stash),
        ("git reset --hard", GitRule::ResetHard),
        ("git reset --hard HEAD~1", GitRule::ResetHard),
        ("git clean -fd", GitRule::Clean),
    ] {
        assert_eq!(eval(cmd, "/r", &claims()).map(|h| h.0), Some(rule), "{cmd}");
        assert_eq!(eval(cmd, "/r", &[]), None, "{cmd} with no claim");
    }
    assert_eq!(eval("git stash list", "/r", &claims()), None);
    assert_eq!(eval("git stash show -p", "/r", &claims()), None);
    assert_eq!(eval("git reset src/a.ts", "/r", &claims()), None);
    assert_eq!(eval("git clean -n", "/r", &claims()), None);
}

#[test]
fn git_log_and_status_are_never_caught() {
    for cmd in [
        "git log --oneline -- src/a.ts",
        "git status",
        "git diff src/a.ts",
        "git show HEAD:src/a.ts",
        "echo 'git add -A'",
        "grep -r 'git stash' .",
    ] {
        assert_eq!(eval(cmd, "/r", &claims()), None, "{cmd}");
    }
}

#[test]
fn rel_in_resolves_through_canonical_paths() {
    let dir = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    std::fs::create_dir(real.join("sub")).unwrap();
    assert_eq!(
        rel_in(&real, &real.join("sub/x/../new.ts")).as_deref(),
        Some("sub/new.ts")
    );
    assert_eq!(rel_in(&real, &real).as_deref(), Some(""));
    assert_eq!(rel_in(&real.join("sub"), &real.join("other.ts")), None);
    // a link to the repo names the same files
    let link = real.join("link");
    std::os::unix::fs::symlink(real.join("sub"), &link).unwrap();
    assert_eq!(
        rel_in(&real.join("sub"), &link.join("new.ts")).as_deref(),
        Some("new.ts")
    );
}

#[test]
fn a_bare_commit_is_caught_only_when_a_claimed_path_is_staged() {
    let with_staged = vec![claim("src/a.ts"), staged("README.md")];
    for cmd in [
        "git commit -m x",
        "git commit",
        "git commit --amend --no-edit",
        "git commit -qm 'wip'",
    ] {
        assert_eq!(
            eval(cmd, "/r", &with_staged),
            Some((GitRule::CommitStaged, paths(&["README.md"]))),
            "{cmd}"
        );
    }
    // nothing claimed is staged: the commit takes only this task's own staged work
    assert_eq!(eval("git commit -m x", "/r", &claims()), None);
    // naming paths commits just those, staged or not
    assert_eq!(eval("git commit -m x src/mine.ts", "/r", &with_staged), None);
}

#[test]
fn a_forced_checkout_or_switch_is_caught() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for (cmd, rule) in [
        ("git checkout -f main", GitRule::CheckoutForce),
        ("git checkout --force main", GitRule::CheckoutForce),
        ("git checkout -f -b next", GitRule::CheckoutForce),
        ("git switch --discard-changes main", GitRule::SwitchForce),
        ("git switch -f main", GitRule::SwitchForce),
        ("git switch --force main", GitRule::SwitchForce),
    ] {
        assert_eq!(eval(cmd, "/r", &claims()), Some((rule, all.clone())), "{cmd}");
        assert_eq!(eval(cmd, "/r", &[]), None, "{cmd} with no claim");
    }
    // a plain switch carries the changes along
    assert_eq!(eval("git switch main", "/r", &claims()), None);
    assert_eq!(eval("git switch -c next", "/r", &claims()), None);
}

#[test]
fn git_rm_of_a_claimed_path_is_caught() {
    for cmd in [
        "git rm src/a.ts",
        "git rm -f src/a.ts",
        "git rm --cached src/a.ts",
        "git rm -- src/a.ts",
    ] {
        assert_eq!(
            eval(cmd, "/r", &claims()),
            Some((GitRule::Remove, paths(&["src/a.ts"]))),
            "{cmd}"
        );
    }
    assert_eq!(
        eval("git rm -r src", "/r", &claims()),
        Some((GitRule::Remove, paths(&["src/a.ts", "src/deep/b.ts"])))
    );
    assert_eq!(
        eval("git rm -rf deep", "/r/src", &claims()),
        Some((GitRule::Remove, paths(&["src/deep/b.ts"])))
    );
    assert_eq!(eval("git rm src/mine.ts", "/r", &claims()), None);
}

#[test]
fn a_shell_dash_c_string_is_checked_too() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "sh -c 'git add -A'",
        "bash -c \"git add -A\"",
        "bash -lc 'cd src && git add ..'",
        "env FOO=1 zsh -c 'git stash'",
    ] {
        assert_eq!(eval(cmd, "/r", &claims()).map(|h| h.1), Some(all.clone()), "{cmd}");
    }
    assert_eq!(eval("sh -c 'git status'", "/r", &claims()), None);
    assert_eq!(eval("bash script.sh", "/r", &claims()), None);
}

#[test]
fn restore_source_and_clean_exclude_values_are_not_paths() {
    // the -s / --source value is a revision, even when it spells a claimed path
    assert_eq!(eval("git restore -s src/a.ts src/mine.ts", "/r", &claims()), None);
    assert_eq!(eval("git restore --source src/a.ts src/mine.ts", "/r", &claims()), None);
    assert_eq!(
        eval("git restore -s HEAD~1 src/a.ts", "/r", &claims()),
        Some((GitRule::RestorePath, paths(&["src/a.ts"])))
    );
    // -e takes a pattern; the clean is still caught, a dry run with one is not
    assert_eq!(
        eval("git clean -fd -e '*.log'", "/r", &claims()).map(|h| h.0),
        Some(GitRule::Clean)
    );
    assert_eq!(
        eval("git clean -e -n -f", "/r", &claims()).map(|h| h.0),
        Some(GitRule::Clean)
    );
    assert_eq!(eval("git clean -n -e x", "/r", &claims()), None);
}

#[test]
fn commit_include_takes_staged_claims() {
    let with_staged = vec![claim("src/a.ts"), staged("README.md")];
    for cmd in [
        "git commit -i -m x src/mine.ts",
        "git commit --include -m x src/mine.ts",
        "git commit -p -m x",
        "git commit --patch -m x src/mine.ts",
        "git commit --interactive",
    ] {
        assert_eq!(
            eval(cmd, "/r", &with_staged).map(|h| h.1),
            Some(paths(&["README.md"])),
            "{cmd}"
        );
    }
    assert_eq!(
        eval("git commit -i -m x src/a.ts", "/r", &with_staged).map(|h| h.1),
        Some(paths(&["README.md", "src/a.ts"]))
    );
}

#[test]
fn pathspec_from_file_is_conservative() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "git add --pathspec-from-file=list.txt",
        "git add --pathspec-from-file list.txt",
        "git commit -m x --pathspec-from-file=- --pathspec-file-nul",
        "git restore --pathspec-from-file=list.txt",
        "git rm --pathspec-from-file list.txt",
        "git checkout --pathspec-from-file=list.txt",
    ] {
        assert_eq!(
            eval(cmd, "/r", &claims()),
            Some((GitRule::Unparsed, all.clone())),
            "{cmd}"
        );
    }
}

#[test]
fn pathological_glob_returns_fast() {
    let pattern = "*a".repeat(30);
    let text = format!("{}b", "a".repeat(60));
    let start = std::time::Instant::now();
    let hit = eval(&format!("git add '{pattern}'"), "/r", &[claim(&text)]);
    assert!(
        start.elapsed() < std::time::Duration::from_millis(50),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(hit, None);
    // over the length cap: counted as a match
    let long = format!("git add '{}*'", "x".repeat(300));
    assert_eq!(eval(&long, "/r", &claims()).map(|h| h.1.len()), Some(3));
}

#[test]
fn nesting_past_the_cap_is_conservative() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    // three levels are followed; the fourth is not read, so the command is stopped
    let deep = "sh -c \"bash -c 'zsh -c \\\"dash -c x\\\"'\"";
    assert_eq!(eval(deep, "/r", &claims()), Some((GitRule::Unparsed, all)));
    assert_eq!(eval("sh -c \"bash -c 'git status'\"", "/r", &claims()), None);
}

fn all() -> Vec<String> {
    let mut v: Vec<String> = claims().iter().map(|c| c.path.clone()).collect();
    v.sort();
    v
}

#[test]
fn stash_pop_and_apply_are_still_stopped() {
    // the stash stack is shared by every task in the repo: a bare pop or apply may take another task's stash
    for c in [
        "git stash pop",
        "git stash apply",
        "git stash apply stash@{1}",
        "git stash pop --index",
    ] {
        assert_eq!(eval(c, "/r", &claims()), Some((GitRule::Stash, all())), "{c}");
    }
}

#[test]
fn stash_drop_and_clear_discard() {
    for c in ["git stash drop", "git stash drop stash@{0}", "git stash clear"] {
        let (rule, paths) = eval(c, "/r", &claims()).expect(c);
        assert_eq!((rule, paths), (GitRule::Stash, all()), "{c}");
        assert_eq!(
            crate::overlap::denial::rule_kind(rule),
            crate::overlap::CommandKind::Discards
        );
    }
}

#[test]
fn stash_list_and_show_pass() {
    for c in ["git stash list", "git stash show", "git stash show -p stash@{1}"] {
        assert_eq!(eval(c, "/r", &claims()), None, "{c}");
    }
}

#[test]
fn add_all_or_update_with_a_pathspec_is_limited_to_those_paths() {
    for c in [
        "git add -A src/deep",
        "git add --all src/deep",
        "git add -u src/deep/b.ts",
        "git add --update -- src/deep",
    ] {
        assert_eq!(
            eval(c, "/r", &claims()),
            Some((GitRule::AddPath, vec!["src/deep/b.ts".into()])),
            "{c}"
        );
    }
    assert_eq!(eval("git add -A lib", "/r", &claims()), None, "names no claimed path");
    // with no pathspec, or naming the root, every claim counts
    for c in ["git add -A", "git add -u", "git add -A ."] {
        assert_eq!(eval(c, "/r", &claims()).map(|(r, _)| r), Some(GitRule::AddAll), "{c}");
    }
}

#[test]
fn exclude_only_add_all_from_a_subfolder_names_claims_outside_it() {
    // git reads an exclude-only pathspec list as the whole tree from the repo root (PATHSPEC_PREFER_FULL)
    let held = vec![claim("lib/c.ts"), claim("src/x/y.ts"), claim("src/x/foo")];
    for c in [
        "cd src/x && git add -A ':!foo'",
        "cd src/x && git add -u ':(exclude)foo'",
        "cd src/x && git add --all ':^foo'",
    ] {
        let (rule, paths) = eval(c, "/r", &held).expect(c);
        assert_eq!(rule, GitRule::AddAll, "{c}");
        assert!(paths.contains(&"lib/c.ts".to_owned()), "{c}: {paths:?}");
    }
    // a plain add or a commit of excludes alone reaches outside the subfolder too
    for c in ["cd src/x && git add ':!foo'", "cd src/x && git commit -m x -- ':!foo'"] {
        let (_, paths) = eval(c, "/r", &held).expect(c);
        assert!(paths.contains(&"lib/c.ts".to_owned()), "{c}: {paths:?}");
        assert!(
            !paths.contains(&"src/x/foo".to_owned()),
            "{c}: the exclude still takes its path away"
        );
    }
    // one real include limits -A to it
    assert_eq!(
        eval("cd src/x && git add -A y.ts ':!foo'", "/r", &held),
        Some((GitRule::AddPath, vec!["src/x/y.ts".into()]))
    );
}

#[test]
fn a_pathspec_the_guard_cannot_read_is_unparsed() {
    for c in [
        "git add $FILES",
        "git add \"${DIR}/x\"",
        "git checkout -- $(git ls-files -m)",
        "git restore `cat list`",
        "git rm -- $OLD",
        "git commit -m x -- $PATHS",
        "git -C $REPO add -A",
        "git ls-files -m | xargs git add",
        "find . -name '*.ts' | xargs git checkout --",
        "xargs git rm < gone.txt",
        "printf '%s\\n' a b | xargs -I{} git restore {}",
    ] {
        assert_eq!(eval(c, "/r", &claims()), Some((GitRule::Unparsed, all())), "{c}");
    }
    // a variable only in a message, or a quoted substitution in it, reads fine
    assert_eq!(
        eval("git commit -m \"$(cat msg)\"", "/r", &claims()).map(|(r, _)| r),
        None,
        "nothing staged, so nothing to stop"
    );
    assert_eq!(eval("git commit -m \"wip $X\" -- lib", "/r", &claims()), None);
    assert_eq!(eval("git log $REV", "/r", &claims()), None);
}

#[test]
fn shell_keywords_before_git_are_skipped() {
    for c in [
        "if true; then git add -A; fi",
        "while false; do :; done; { git stash; }",
        "! git reset --hard",
        "exec git clean -fd",
    ] {
        assert!(eval(c, "/r", &claims()).is_some(), "{c}");
    }
}
