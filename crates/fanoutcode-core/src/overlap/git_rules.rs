//! The git guard's second rule set (§5.6): which shell commands would stage, commit, restore, stash, reset or clean
//! paths another active task claims. Best effort, like the Bash guard: segment by segment with a running cwd
//! (`cd`, `git -C`), into the string of `sh -c` / `bash -c` (and zsh, dash), but no script file or other interpreter
//! is inspected. Pathspecs understand `:/`, `:(top)`, excludes and `*` / `?` / `[…]` globs; other magic is read as a
//! plain path.

use std::path::{Path, PathBuf};

use super::claims::Claim;
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

/// The claimed paths a repo-relative path names: the path itself, everything under it as a directory, or all at the
/// root.
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

/// Git's pathspec wildcard match, without `:(glob)` magic: `*` and `?` cross `/`, `[…]` is a set (`!` or `^`
/// negates, `a-z` ranges).
fn glob_match(pat: &[char], text: &[char]) -> bool {
    match pat.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => (0..=text.len()).any(|i| glob_match(rest, &text[i..])),
        Some(('?', rest)) => !text.is_empty() && glob_match(rest, &text[1..]),
        Some(('[', rest)) => {
            let Some(close) = rest.iter().skip(1).position(|c| *c == ']').map(|i| i + 1) else {
                return text.first() == Some(&'[') && glob_match(rest, &text[1..]);
            };
            let Some(ch) = text.first() else { return false };
            let (negate, set) = match rest[..close].split_first() {
                Some(('!' | '^', set)) => (true, set),
                _ => (false, &rest[..close]),
            };
            let mut hit = false;
            let mut i = 0;
            while i < set.len() {
                if i + 2 < set.len() && set[i + 1] == '-' {
                    hit |= (set[i]..=set[i + 2]).contains(ch);
                    i += 3;
                } else {
                    hit |= set[i] == *ch;
                    i += 1;
                }
            }
            hit != negate && glob_match(&rest[close + 1..], &text[1..])
        }
        Some((c, rest)) => text.first() == Some(c) && glob_match(rest, &text[1..]),
    }
}

/// One pathspec's magic: `(top, exclude, rest)`. `:/x` and `:(top)x` are relative to the repo root; `:!x`, `:^x`
/// and `:(exclude)x` name nothing to change.
fn magic(spec: &str) -> (bool, bool, &str) {
    let Some(after) = spec.strip_prefix(':') else {
        return (false, false, spec);
    };
    if let Some(long) = after.strip_prefix('(') {
        let Some((words, rest)) = long.split_once(')') else {
            return (false, false, spec);
        };
        let has = |w: &str| words.split(',').any(|x| x.trim() == w);
        return (has("top"), has("exclude"), rest);
    }
    let shorts = after.find(|c| !matches!(c, '/' | '!' | '^')).unwrap_or(after.len());
    let (flags, rest) = after.split_at(shorts);
    (flags.contains('/'), flags.contains(['!', '^']), rest)
}

/// The claimed paths the given pathspecs (resolved against `dir`, or the repo root for `:/`) name in `repo`.
fn named(specs: &[&str], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let mut out = Vec::new();
    for spec in specs {
        let (top, exclude, rest) = magic(spec);
        if exclude {
            continue;
        }
        let base = if top { repo } else { dir };
        if rest.contains(['*', '?', '[']) {
            let Some(prefix) = rel_in(repo, base) else { continue };
            let full = if prefix.is_empty() {
                rest.to_owned()
            } else {
                format!("{prefix}/{rest}")
            };
            let pat: Vec<char> = full.trim_start_matches("./").chars().collect();
            out.extend(
                claimed
                    .iter()
                    .filter(|c| glob_match(&pat, &c.path.chars().collect::<Vec<_>>()))
                    .map(|c| c.path.clone()),
            );
        } else if let Some(rel) = rel_in(repo, &resolve(rest, base)) {
            out.extend(matching(&rel, claimed).into_iter().map(str::to_owned));
        }
    }
    out
}

fn staged_paths(claimed: &[&Claim]) -> Vec<String> {
    claimed.iter().filter(|c| c.staged).map(|c| c.path.clone()).collect()
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
            let specs: Vec<&str> = a.positionals.iter().chain(&a.after_dashes).copied().collect();
            if has(&a, 'a', "--all") {
                (GitRule::CommitAll, all_paths(claimed))
            } else if specs.is_empty() {
                (GitRule::CommitStaged, staged_paths(claimed))
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
            let a = split_args(args, "", &["pathspec-from-file"]);
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
/// How deep `sh -c 'bash -c …'` is followed.
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
            if let (Some(script), true) = (script, depth < MAX_DEPTH) {
                if let Some(hit) = evaluate_in(script, &cwd, repo, claimed, depth + 1) {
                    return Some(hit);
                }
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
