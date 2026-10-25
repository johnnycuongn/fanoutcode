use std::path::Path;

use crate::overlap::claims::Claim;
use crate::overlap::git_rules::{evaluate_git, GitRule};

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
fn top_and_glob_pathspecs_name_claimed_paths() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    // `:/` and `:(top)` are the repo root wherever the command runs
    assert_eq!(
        eval("git add :/", "/r/src/deep", &claims()).map(|h| h.1),
        Some(all.clone())
    );
    assert_eq!(
        eval("git checkout -- ':(top)'", "/r/src", &claims()).map(|h| h.1),
        Some(all)
    );
    assert_eq!(
        eval("git restore :/README.md", "/r/src", &claims()),
        Some((GitRule::RestorePath, paths(&["README.md"])))
    );
    assert_eq!(
        eval("git add ':(top)src/a.ts'", "/r/src/deep", &claims()),
        Some((GitRule::AddPath, paths(&["src/a.ts"])))
    );
    // a glob matches across directories, relative to where it runs
    assert_eq!(
        eval("git add '*.ts'", "/r", &claims()),
        Some((GitRule::AddPath, paths(&["src/a.ts", "src/deep/b.ts"])))
    );
    assert_eq!(
        eval("git checkout -- 'deep/*.ts'", "/r/src", &claims()),
        Some((GitRule::CheckoutPath, paths(&["src/deep/b.ts"])))
    );
    assert_eq!(
        eval("git restore 'src/?.ts'", "/r", &claims()),
        Some((GitRule::RestorePath, paths(&["src/a.ts"])))
    );
    assert_eq!(eval("git add '*.rs'", "/r", &claims()), None);
}

#[test]
fn exclude_only_pathspec_still_matches_other_claims() {
    // with no include, an exclude means "everything here but"
    for cmd in [
        "git add ':!src/a.ts'",
        "git add ':^src/a.ts'",
        "git add ':(exclude)src/a.ts'",
        "git checkout -- ':!src/a.ts'",
    ] {
        assert_eq!(
            eval(cmd, "/r", &claims()).map(|h| h.1),
            Some(paths(&["README.md", "src/deep/b.ts"])),
            "{cmd}"
        );
    }
    // relative to where it runs
    assert_eq!(
        eval("git add ':!a.ts'", "/r/src", &claims()).map(|h| h.1),
        Some(paths(&["src/deep/b.ts"]))
    );
    // an include minus an exclude
    assert_eq!(
        eval("git add src ':!src/deep'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["src/a.ts"]))
    );
    // an exclude that matches no claim leaves every claim here
    assert_eq!(
        eval("git add ':!nothing.ts'", "/r", &claims()).map(|h| h.1.len()),
        Some(3)
    );
    // excluding every claim leaves nothing
    assert_eq!(eval("git add ':!src' ':!README.md'", "/r", &claims()), None);
}

#[test]
fn glob_with_dot_dot_segments_is_resolved() {
    for (cmd, cwd) in [
        ("git add './src/*.ts'", "/r"),
        ("git add '../src/a*'", "/r/src"),
        ("git add 'src//a.*'", "/r"),
        ("git add 'deep/../a.*'", "/r/src"),
    ] {
        assert_eq!(
            eval(cmd, cwd, &claims())
                .map(|h| h.1)
                .as_deref()
                .map(|v| v.contains(&"src/a.ts".to_owned())),
            Some(true),
            "{cmd} in {cwd}"
        );
    }
    // climbing out of the repo matches nothing in it
    assert_eq!(eval("git add '../../*.ts'", "/r/src", &claims()), None);
}

#[test]
fn short_magic_with_a_trailing_colon_is_parsed() {
    assert_eq!(
        eval("git add ':!:src/a.ts'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["README.md", "src/deep/b.ts"]))
    );
    assert_eq!(
        eval("git add ':^:src'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["README.md"]))
    );
    assert_eq!(
        eval("git restore ':/:src/a.ts'", "/r/src/deep", &claims()),
        Some((GitRule::RestorePath, paths(&["src/a.ts"])))
    );
}

#[test]
fn unknown_long_magic_names_every_claim() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "git add ':(attr:binary)nothing'",
        "git checkout -- ':(future)x'",
        "git add ':(top,whatever)x'",
    ] {
        assert_eq!(eval(cmd, "/r", &claims()).map(|h| h.1), Some(all.clone()), "{cmd}");
    }
    // `literal` reads the path as written, no glob
    assert_eq!(eval("git add ':(literal)src/*.ts'", "/r", &claims()), None);
    assert_eq!(
        eval("git add ':(literal)src/a.ts'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["src/a.ts"]))
    );
    // `glob` is read as a (wider) glob
    assert_eq!(
        eval("git add ':(glob)src/*.ts'", "/r", &claims()).map(|h| h.1.contains(&"src/a.ts".to_owned())),
        Some(true)
    );
}

#[test]
fn matching_is_case_insensitive_always() {
    // git may be told to ignore case (`:(icase)`, `--icase-pathspecs`, an exported `GIT_ICASE_PATHSPECS`), and the
    // guard cannot see every way: it always folds ASCII case
    for cmd in [
        "git add SRC/A.TS",
        "git add ':(icase)SRC/A.TS'",
        "git --icase-pathspecs add Src/A.ts",
        "export GIT_ICASE_PATHSPECS=1; git add SRC/A.TS",
        "git add 'SRC/*.Ts'",
    ] {
        assert!(
            eval(cmd, "/r", &claims()).is_some_and(|h| h.1.contains(&"src/a.ts".to_owned())),
            "{cmd}"
        );
    }
    assert_eq!(
        eval("git checkout -- ':/readme.MD'", "/r/src", &claims()).map(|h| h.1),
        Some(paths(&["README.md"]))
    );
}

#[test]
fn glob_magic_names_every_claim_under_its_prefix() {
    // `**/` and the other `:(glob)` rules are not modelled: everything under the literal prefix counts
    assert_eq!(
        eval("git add ':(glob)src/**/nothing'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["src/a.ts", "src/deep/b.ts"]))
    );
    assert_eq!(
        eval("git add ':(glob)**/x'", "/r", &claims()).map(|h| h.1.len()),
        Some(3)
    );
    assert_eq!(
        eval("git add ':(glob)*.rs'", "/r/src/deep", &claims()).map(|h| h.1),
        Some(paths(&["src/deep/b.ts"]))
    );
}

#[test]
fn a_bracket_glob_names_every_claim() {
    assert_eq!(
        eval("git add 'src/[!a].ts'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["src/a.ts", "src/deep/b.ts"]))
    );
    assert_eq!(eval("git add '[A-Z]*'", "/r", &claims()).map(|h| h.1.len()), Some(3));
    assert_eq!(
        eval("git restore 'deep/[b].ts'", "/r/src", &claims()).map(|h| h.1),
        Some(paths(&["src/deep/b.ts"]))
    );
}

#[test]
fn a_non_ascii_or_wildcard_exclude_excludes_nothing() {
    let all = paths(&["README.md", "src/a.ts", "src/deep/b.ts"]);
    for cmd in [
        "git add src README.md ':!*.ts'",
        "git add src README.md ':!src/?.ts'",
        "git add src README.md ':(exclude)src/[a].ts'",
        "git add src README.md ':!src/\u{e9}.ts'",
        "git add ':!SRC/A.TS'",
    ] {
        assert_eq!(eval(cmd, "/r", &claims()).map(|h| h.1), Some(all.clone()), "{cmd}");
    }
    // a plain one still applies, matched exactly
    assert_eq!(
        eval("git add src README.md ':!src/a.ts'", "/r", &claims()).map(|h| h.1),
        Some(paths(&["README.md", "src/deep/b.ts"]))
    );
}
