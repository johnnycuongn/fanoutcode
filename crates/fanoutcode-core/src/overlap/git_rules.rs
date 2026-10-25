//! The git guard's second rule set (§5.6): which shell commands would stage, commit, restore, stash, reset or clean
//! paths another active task claims. Best effort, like the Bash guard: segment by segment with a running cwd
//! (`cd`, `git -C`), into the string of `sh -c` / `bash -c` (and zsh, dash), but no script file or other interpreter
//! is inspected. Pathspecs are read in `pathspec`. Where the parser cannot be sure (paths from a file, shells nested
//! past the cap, an over-long glob) it stops the command rather than let it pass.

use std::path::{Path, PathBuf};

use super::claims::Claim;
use super::pathspec::named;
use crate::workspace::guard::rules::{clean, strip_prefixes};
use crate::workspace::guard::shell_words::segments;

/// Why a command was caught.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRule {
    CommitAll,
    /// A commit naming no path: whatever is staged, including another task's staged claims.
    CommitStaged,
    AddAll,
    CommitPath,
    AddPath,
    CheckoutPath,
    /// `checkout -f` / `--force`: throws away every uncommitted change.
    CheckoutForce,
    /// `switch -f` / `--force` / `--discard-changes`.
    SwitchForce,
    RestorePath,
    /// `git rm` of a claimed path, or a directory holding one.
    Remove,
    Stash,
    ResetHard,
    Clean,
    /// The parser cannot tell what the command touches (paths read from a file, shells nested past the cap, an
    /// over-long glob): every claim counts. As bad as a discard for whoever owns them.
    Unparsed,
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

pub(super) fn resolve(word: &str, cwd: &Path) -> PathBuf {
    clean(&cwd.join(word))
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

fn staged_paths(claimed: &[&Claim]) -> Vec<String> {
    claimed.iter().filter(|c| c.staged).map(|c| c.path.clone()).collect()
}

/// The subcommands whose pathspecs `--pathspec-from-file` can carry.
const TAKES_PATHSPECS: &[&str] = &["commit", "add", "checkout", "restore", "rm", "reset", "stash"];

/// One `git <sub> <args>` run in `dir`, inside `repo`.
fn git_rule(sub: &str, args: &[String], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Option<(GitRule, Vec<String>)> {
    // paths read from a file (or stdin) cannot be seen: every claim counts
    let from_file = args
        .iter()
        .any(|a| a.starts_with("--pathspec-from-file") || a == "--pathspec-file-nul");
    if from_file && TAKES_PATHSPECS.contains(&sub) {
        return Some((GitRule::Unparsed, sorted(all_paths(claimed))));
    }
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
            let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
            // `-i` adds the named paths to what is staged; `-p` / `--interactive` pick hunks on top of the index
            let with_index = has(&a, 'i', "--include") || has(&a, 'p', "--patch") || a.flags.contains(&"--interactive");
            if has(&a, 'a', "--all") {
                (GitRule::CommitAll, all_paths(claimed))
            } else if specs.is_empty() {
                (GitRule::CommitStaged, staged_paths(claimed))
            } else if with_index {
                let mut paths = named(&specs, dir, repo, claimed);
                paths.extend(staged_paths(claimed));
                (GitRule::CommitStaged, paths)
            } else {
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
            if has(&a, 'f', "--force") {
                (GitRule::CheckoutForce, all_paths(claimed))
            } else {
                (GitRule::CheckoutPath, named(specs, dir, repo, claimed))
            }
        }
        "switch" => {
            let a = split_args(args, "cC", &["create", "force-create", "orphan"]);
            if !(has(&a, 'f', "--force") || a.flags.contains(&"--discard-changes")) {
                return None;
            }
            (GitRule::SwitchForce, all_paths(claimed))
        }
        "rm" => {
            let a = split_args(args, "", &[]);
            let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
            (GitRule::Remove, named(&specs, dir, repo, claimed))
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
        // `pop` and `apply` refuse to overwrite dirty files, so they discard nothing
        "stash" => match args.first().map(String::as_str) {
            Some("list" | "show" | "pop" | "apply") => return None,
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
    let paths = sorted(paths);
    (!paths.is_empty()).then_some((rule, paths))
}

fn sorted(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.dedup();
    paths
}

/// Shells whose `-c` string is read as a command of its own.
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash"];
/// How deep `sh -c 'bash -c …'` is followed; a `-c` script deeper than this stops the command.
const MAX_DEPTH: usize = 3;

/// The first segment of `command` (run in `cwd`) that is a git command inside `repo` touching one of `claimed` (the
/// other tasks' dirty claims there), with the claimed paths it touches. `None` when nothing is claimed.
pub fn evaluate_git(command: &str, cwd: &Path, repo: &Path, claimed: &[&Claim]) -> Option<(GitRule, Vec<String>)> {
    if claimed.is_empty() {
        return None;
    }
    evaluate_in(command, &clean(cwd), repo, claimed, 0)
}

fn evaluate_in(
    command: &str,
    cwd: &Path,
    repo: &Path,
    claimed: &[&Claim],
    depth: usize,
) -> Option<(GitRule, Vec<String>)> {
    let mut cwd = cwd.to_owned();
    for seg in segments(command) {
        let words = strip_prefixes(&seg.words);
        // no `--icase-pathspecs` / `GIT_ICASE_PATHSPECS` tracking: `pathspec` always folds case, which covers every
        // way of turning it on, including an earlier `export` the guard cannot see
        let Some((first, rest)) = words.split_first() else {
            continue;
        };
        if first == "cd" {
            if let Some(dir) = rest.iter().find(|w| !w.starts_with('-')) {
                cwd = resolve(dir, &cwd);
            }
            continue;
        }
        let program = first.rsplit('/').next().unwrap_or(first);
        if SHELLS.contains(&program) {
            // `-c`, or a cluster holding it (`-lc`, `-ec`): the next word is the script
            let script = rest
                .iter()
                .position(|a| a.starts_with('-') && !a.starts_with("--") && a.contains('c'))
                .and_then(|i| rest.get(i + 1));
            match script {
                // nested past the cap: not read, so stopped
                Some(_) if depth >= MAX_DEPTH => return Some((GitRule::Unparsed, sorted(all_paths(claimed)))),
                Some(script) => {
                    if let Some(hit) = evaluate_in(script, &cwd, repo, claimed, depth + 1) {
                        return Some(hit);
                    }
                }
                None => {}
            }
            continue;
        }
        if program != "git" {
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
#[path = "git_rules_tests.rs"]
mod tests;
