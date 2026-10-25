//! The git guard's second rule set (§5.6): which shell commands would stage, commit, restore, stash, reset or clean
//! paths another active task claims. Best effort, like the Bash guard: segment by segment with a running cwd
//! (`cd`, `git -C`), no interpreter is inspected.

use std::path::{Path, PathBuf};

use super::claims::Claim;
use crate::workspace::guard::rules::{clean, strip_prefixes};
use crate::workspace::guard::shell_words::segments;

/// Why a command was caught.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRule {
    CommitAll,
    AddAll,
    CommitPath,
    AddPath,
    CheckoutPath,
    RestorePath,
    Stash,
    ResetHard,
    Clean,
}

/// `path` with the deepest existing ancestor canonicalised (so `/tmp/x/new.ts` becomes `/private/tmp/x/new.ts`).
fn canonical_lenient(path: &Path) -> Option<PathBuf> {
    let mut rest = Vec::new();
    let mut base = path;
    loop {
        if let Ok(c) = std::fs::canonicalize(base) {
            return Some(rest.iter().rev().fold(c, |acc: PathBuf, part| acc.join(part)));
        }
        rest.push(base.file_name()?.to_owned());
        base = base.parent()?;
    }
}

fn rel_string(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// `path` (absolute) relative to `repo` with `/` separators (`""` for the root itself), when it lies inside it.
/// Lexical first, then through canonical paths (symlinked temp dirs, `..` across links).
pub(crate) fn rel_in(repo: &Path, path: &Path) -> Option<String> {
    let (r, p) = (clean(repo), clean(path));
    if let Ok(rel) = p.strip_prefix(&r) {
        return Some(rel_string(rel));
    }
    let (r, p) = (canonical_lenient(&r)?, canonical_lenient(&p)?);
    p.strip_prefix(&r).ok().map(rel_string)
}

fn resolve(word: &str, cwd: &Path) -> PathBuf {
    clean(&cwd.join(word))
}

/// The claimed paths a pathspec names: the path itself, everything under it as a directory, or all at the root.
fn matching<'a>(spec: &str, claimed: &[&'a Claim]) -> Vec<&'a str> {
    claimed
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| spec.is_empty() || *p == spec || p.strip_prefix(spec).is_some_and(|r| r.starts_with('/')))
        .collect()
}

/// Splits `args` into flags and positionals, skipping the value of each flag in `valued` (short clusters like `-am`
/// end at a valued letter). `--` makes the rest positional.
struct Args<'a> {
    flags: Vec<&'a str>,
    shorts: String,
    positionals: Vec<&'a str>,
    after_dashes: Vec<&'a str>,
}

fn split_args<'a>(args: &'a [String], valued_short: &str, valued_long: &[&str]) -> Args<'a> {
    let mut out = Args {
        flags: Vec::new(),
        shorts: String::new(),
        positionals: Vec::new(),
        after_dashes: Vec::new(),
    };
    let mut i = 0;
    let mut dashes = false;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if dashes {
            out.after_dashes.push(a);
        } else if a == "--" {
            dashes = true;
        } else if let Some(long) = a.strip_prefix("--") {
            out.flags.push(a);
            if valued_long.contains(&long) {
                i += 1;
            }
        } else if let Some(cluster) = a.strip_prefix('-').filter(|c| !c.is_empty()) {
            out.flags.push(a);
            for (n, ch) in cluster.char_indices() {
                out.shorts.push(ch);
                if valued_short.contains(ch) {
                    if n + ch.len_utf8() == cluster.len() {
                        i += 1;
                    }
                    break;
                }
            }
        } else {
            out.positionals.push(a);
        }
    }
    out
}

fn has(a: &Args<'_>, short: char, long: &str) -> bool {
    a.shorts.contains(short) || a.flags.contains(&long)
}

fn all_paths(claimed: &[&Claim]) -> Vec<String> {
    claimed.iter().map(|c| c.path.clone()).collect()
}

/// The claimed paths the given pathspecs (resolved against `dir`) name in `repo`.
fn named(specs: &[&str], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    specs
        .iter()
        .filter_map(|s| rel_in(repo, &resolve(s, dir)))
        .flat_map(|rel| matching(&rel, claimed))
        .map(str::to_owned)
        .collect()
}

/// One `git <sub> <args>` run in `dir`, inside `repo`.
fn git_rule(sub: &str, args: &[String], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Option<(GitRule, Vec<String>)> {
    let (rule, paths) = match sub {
        "commit" => {
            let a = split_args(
                args,
                "mFCct",
                &[
                    "message",
                    "file",
                    "author",
                    "date",
                    "template",
                    "reuse-message",
                    "reedit-message",
                    "fixup",
                    "squash",
                    "cleanup",
                    "trailer",
                ],
            );
            if has(&a, 'a', "--all") {
                (GitRule::CommitAll, all_paths(claimed))
            } else {
                let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
                (GitRule::CommitPath, named(&specs, dir, repo, claimed))
            }
        }
        "add" => {
            let a = split_args(args, "", &[]);
            let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
            let names_root = specs
                .iter()
                .any(|s| rel_in(repo, &resolve(s, dir)).as_deref() == Some(""));
            if has(&a, 'A', "--all") || has(&a, 'u', "--update") || names_root {
                (GitRule::AddAll, all_paths(claimed))
            } else {
                (GitRule::AddPath, named(&specs, dir, repo, claimed))
            }
        }
        "checkout" => {
            let a = split_args(args, "bB", &["orphan"]);
            // with `--`, what precedes it is a tree-ish; without, any word may be a path
            let specs = if a.after_dashes.is_empty() {
                &a.positionals
            } else {
                &a.after_dashes
            };
            (GitRule::CheckoutPath, named(specs, dir, repo, claimed))
        }
        "restore" => {
            let a = split_args(args, "s", &["source"]);
            let staged_only = has(&a, 'S', "--staged") && !has(&a, 'W', "--worktree");
            if staged_only {
                return None;
            }
            let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
            (GitRule::RestorePath, named(&specs, dir, repo, claimed))
        }
        "stash" => match args.first().map(String::as_str) {
            Some("list" | "show") => return None,
            _ => (GitRule::Stash, all_paths(claimed)),
        },
        "reset" if args.iter().any(|a| a == "--hard") => (GitRule::ResetHard, all_paths(claimed)),
        "clean" => {
            let a = split_args(args, "e", &["exclude"]);
            if has(&a, 'n', "--dry-run") {
                return None;
            }
            (GitRule::Clean, all_paths(claimed))
        }
        _ => return None,
    };
    let mut paths = paths;
    paths.sort();
    paths.dedup();
    (!paths.is_empty()).then_some((rule, paths))
}

/// The first segment of `command` (run in `cwd`) that is a git command inside `repo` touching one of `claimed` (the
/// other tasks' dirty claims there), with the claimed paths it touches. `None` when nothing is claimed.
pub fn evaluate_git(command: &str, cwd: &Path, repo: &Path, claimed: &[&Claim]) -> Option<(GitRule, Vec<String>)> {
    if claimed.is_empty() {
        return None;
    }
    let mut cwd = clean(cwd);
    for seg in segments(command) {
        let words = strip_prefixes(&seg.words);
        let Some((first, rest)) = words.split_first() else {
            continue;
        };
        if first == "cd" {
            if let Some(dir) = rest.iter().find(|w| !w.starts_with('-')) {
                cwd = resolve(dir, &cwd);
            }
            continue;
        }
        if first.rsplit('/').next() != Some("git") {
            continue;
        }
        let mut dir = cwd.clone();
        let mut i = 0;
        while let Some(w) = rest.get(i).filter(|w| w.starts_with('-')) {
            i += 1;
            match w.as_str() {
                "-C" => {
                    if let Some(d) = rest.get(i) {
                        dir = resolve(d, &dir);
                    }
                    i += 1;
                }
                "-c" | "--git-dir" | "--work-tree" | "--namespace" => i += 1,
                _ => {}
            }
        }
        let Some(sub) = rest.get(i) else { continue };
        if rel_in(repo, &dir).is_none() {
            continue;
        }
        if let Some(hit) = git_rule(sub, &rest[i + 1..], &dir, repo, claimed) {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(path: &str) -> Claim {
        Claim {
            path: path.into(),
            task_id: Some(crate::domain::TaskId::parse("t-b2").unwrap()),
            title: "Checkout".into(),
            stage: None,
            state: None,
            last_edit: "2026-10-25T10:00:00Z".parse().unwrap(),
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
}
