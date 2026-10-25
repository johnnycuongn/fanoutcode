//! The matching rules behind `evaluate`: segment by segment with a running cwd, first deny wins.

use std::path::{Component, Path, PathBuf};

use super::shell_words::{segments, Segment};
use super::{deny_reason, GuardDecision, GuardRule};

const WRITERS: &[&str] = &[
    "rm", "rmdir", "mv", "cp", "ln", "mkdir", "touch", "tee", "truncate", "dd", "shred", "unlink", "chmod", "chown",
    "chgrp", "install", "rsync", "patch",
];
/// For these only the destination (the last positional) decides.
const DEST_ONLY: &[&str] = &["cp", "mv", "rsync", "install", "ln"];
const WRAPPERS: &[&str] = &["sudo", "env", "command", "nohup", "time", "nice", "xargs"];
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash"];
const GIT_WRITES: &[&str] = &[
    "add",
    "commit",
    "checkout",
    "switch",
    "restore",
    "reset",
    "rebase",
    "merge",
    "cherry-pick",
    "revert",
    "stash",
    "clean",
    "rm",
    "mv",
    "am",
    "apply",
    "pull",
    "push",
    "init",
    "clone",
    "worktree",
    "submodule",
    "notes",
    "replace",
    "update-ref",
    "symbolic-ref",
    "filter-branch",
    "gc",
    "prune",
];
const PACKAGE_WRITES: &[(&str, &[&str])] = &[
    (
        "npm",
        &[
            "install",
            "i",
            "ci",
            "add",
            "uninstall",
            "un",
            "remove",
            "rm",
            "update",
            "up",
            "link",
            "unlink",
            "dedupe",
            "prune",
        ],
    ),
    (
        "pnpm",
        &[
            "add", "install", "i", "remove", "rm", "update", "up", "link", "unlink", "dedupe", "prune",
        ],
    ),
    ("yarn", &["add", "install", "remove", "upgrade", "link", "unlink"]),
    ("bun", &["add", "install", "i", "remove", "rm", "update", "link"]),
    ("pip", &["install", "uninstall"]),
    ("pip3", &["install", "uninstall"]),
    ("uv", &["add", "remove", "sync", "pip"]),
    ("poetry", &["add", "remove", "install", "update", "lock"]),
    ("cargo", &["add", "remove", "update"]),
    ("go", &["get", "mod"]),
    ("composer", &["require", "remove", "install", "update"]),
    ("bundle", &["add", "install", "update"]),
];
const MAX_DEPTH: u8 = 8;

/// Lexical clean-up: `.` dropped, `..` pops, trailing slash gone.
pub(crate) fn clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Case-insensitive on macOS, whose default file system ignores case.
fn fold(p: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from(p.to_string_lossy().to_lowercase())
    } else {
        p.to_owned()
    }
}

pub(super) struct Ctx<'a> {
    pub(super) roots: Vec<PathBuf>,
    pub(super) home: &'a Path,
}

impl Ctx<'_> {
    /// `word` as a path: `~` expanded, relative ones joined to `cwd`, cleaned.
    fn resolve(&self, word: &str, cwd: &Path) -> PathBuf {
        let expanded = if word == "~" {
            self.home.to_owned()
        } else if let Some(rest) = word.strip_prefix("~/") {
            self.home.join(rest)
        } else {
            PathBuf::from(word)
        };
        clean(&if expanded.is_absolute() {
            expanded
        } else {
            cwd.join(expanded)
        })
    }

    /// The protected root `path` lies in, if any.
    fn root_of(&self, path: &Path) -> Option<&PathBuf> {
        let p = fold(path);
        self.roots.iter().find(|r| p.starts_with(fold(r)))
    }

    /// The root a word names (as a path, or as the value of `--opt=value`), resolved against `cwd`.
    fn word_root(&self, word: &str, cwd: &Path) -> Option<&PathBuf> {
        let value = word.split_once('=').map(|(_, v)| v);
        [Some(word), value]
            .into_iter()
            .flatten()
            .filter(|w| !w.is_empty())
            .find_map(|w| self.root_of(&self.resolve(w, cwd)))
    }

    /// The segment names a root in any word, or runs inside one.
    fn references(&self, words: &[String], cwd: &Path) -> Option<PathBuf> {
        self.root_of(cwd)
            .or_else(|| words.iter().find_map(|w| self.word_root(w, cwd)))
            .cloned()
    }
}

fn deny(root: PathBuf, rule: GuardRule) -> GuardDecision {
    let reason = deny_reason(&root, &rule);
    GuardDecision::Deny { root, rule, reason }
}

fn is_assignment(w: &str) -> bool {
    match w.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

/// Drops leading `VAR=val` words and wrappers (with their flags) before the program name.
pub(crate) fn strip_prefixes(words: &[String]) -> &[String] {
    let mut w = words;
    loop {
        match w.first().map(String::as_str) {
            Some(first) if is_assignment(first) => w = &w[1..],
            Some(first) if WRAPPERS.contains(&first) => {
                w = &w[1..];
                while let Some(flag) = w.first().filter(|f| f.starts_with('-')) {
                    let takes_value = matches!(flag.as_str(), "-u" | "-g" | "-n" | "-C" | "-I" | "-L" | "-P");
                    w = &w[1..];
                    if takes_value && !w.is_empty() {
                        w = &w[1..];
                    }
                }
            }
            _ => return w,
        }
    }
}

fn program_name(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Positionals: words not starting with `-`, and every word after `--`.
fn positionals(args: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut rest = false;
    for a in args {
        if rest || !a.starts_with('-') {
            out.push(a);
        } else if a == "--" {
            rest = true;
        }
    }
    out
}

/// A short-option cluster (`-ni`, `-pi`) or a long `--in-place` that edits in place.
fn in_place(args: &[String]) -> bool {
    args.iter().any(|a| {
        a.starts_with("--in-place")
            || (a.starts_with('-')
                && !a.starts_with("--")
                && a[1..].chars().all(|c| c.is_ascii_alphabetic())
                && a.contains('i'))
            || a.starts_with("-i")
    })
}

pub(super) fn evaluate_in(ctx: &Ctx, command: &str, cwd: &mut PathBuf, depth: u8) -> GuardDecision {
    for seg in segments(command) {
        let d = segment(ctx, &seg, cwd, depth);
        if d != GuardDecision::Allow {
            return d;
        }
    }
    GuardDecision::Allow
}

fn segment(ctx: &Ctx, seg: &Segment, cwd: &mut PathBuf, depth: u8) -> GuardDecision {
    for t in &seg.write_targets {
        if let Some(root) = ctx.root_of(&ctx.resolve(t, cwd)) {
            return deny(root.clone(), GuardRule::Redirect);
        }
    }
    let words = strip_prefixes(&seg.words);
    let Some((first, args)) = words.split_first() else {
        return GuardDecision::Allow;
    };
    let program = program_name(first);
    if SHELLS.contains(&program) {
        if let Some(i) = args
            .iter()
            .position(|a| a.starts_with('-') && !a.starts_with("--") && a.contains('c'))
        {
            if let (Some(script), true) = (args.get(i + 1), depth < MAX_DEPTH) {
                return evaluate_in(ctx, script, &mut cwd.clone(), depth + 1);
            }
        }
        return GuardDecision::Allow;
    }
    match program {
        "cd" | "pushd" => {
            if let Some(dir) = positionals(args).first().filter(|d| d.as_str() != "-") {
                *cwd = ctx.resolve(dir, cwd);
            }
            GuardDecision::Allow
        }
        "sed" | "perl" if in_place(args) => {
            let rule = if program == "sed" {
                GuardRule::SedInPlace
            } else {
                GuardRule::PerlInPlace
            };
            ctx.references(args, cwd)
                .map_or(GuardDecision::Allow, |r| deny(r, rule))
        }
        "git" => git(ctx, args, cwd),
        "find" => find(ctx, args, cwd),
        _ => {
            if let Some(&name) = WRITERS.iter().find(|w| **w == program) {
                return writer(ctx, name, args, cwd);
            }
            if let Some((tool, subs)) = PACKAGE_WRITES.iter().find(|(t, _)| *t == program) {
                let sub = positionals(args)
                    .into_iter()
                    .find(|p| !p.contains('/') && !p.contains('='));
                if sub.is_some_and(|s| subs.contains(&s.as_str())) {
                    return ctx
                        .references(args, cwd)
                        .map_or(GuardDecision::Allow, |r| deny(r, GuardRule::PackageInstall(tool)));
                }
            }
            GuardDecision::Allow
        }
    }
}

fn writer(ctx: &Ctx, name: &'static str, args: &[String], cwd: &Path) -> GuardDecision {
    let pos = positionals(args);
    let root = if DEST_ONLY.contains(&name) {
        pos.last().and_then(|d| ctx.word_root(d, cwd)).cloned()
    } else {
        ctx.root_of(cwd)
            .or_else(|| pos.iter().find_map(|p| ctx.word_root(p, cwd)))
            .cloned()
    };
    root.map_or(GuardDecision::Allow, |r| deny(r, GuardRule::Program(name)))
}

fn git(ctx: &Ctx, args: &[String], cwd: &Path) -> GuardDecision {
    let mut dir = cwd.to_owned();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "-C" {
            if let Some(p) = args.get(i + 1) {
                dir = ctx.resolve(p, &dir);
            }
            i += 2;
        } else if a == "-c" {
            i += 2;
        } else if let Some(v) = a.strip_prefix("--git-dir=").or_else(|| a.strip_prefix("--work-tree=")) {
            dir = ctx.resolve(v, &dir);
            i += 1;
        } else if a.starts_with('-') {
            i += 1;
        } else {
            break;
        }
    }
    let Some(sub) = args.get(i) else {
        return GuardDecision::Allow;
    };
    let rest = &args[i + 1..];
    let named = GIT_WRITES
        .iter()
        .find(|w| **w == sub.as_str())
        .copied()
        .or(match sub.as_str() {
            "branch" | "tag"
                if !positionals(rest).is_empty()
                    || rest.iter().any(|a| {
                        matches!(
                            a.as_str(),
                            "-d" | "-D" | "-m" | "-M" | "-f" | "--delete" | "--move" | "--force"
                        )
                    }) =>
            {
                Some(if sub == "branch" { "branch" } else { "tag" })
            }
            _ => None,
        });
    match (named, ctx.root_of(&dir)) {
        (Some(name), Some(root)) => deny(root.clone(), GuardRule::Git(name)),
        _ => GuardDecision::Allow,
    }
}

fn find(ctx: &Ctx, args: &[String], cwd: &Path) -> GuardDecision {
    if !args
        .iter()
        .any(|a| matches!(a.as_str(), "-delete" | "-exec" | "-execdir" | "-ok" | "-okdir"))
    {
        return GuardDecision::Allow;
    }
    let paths: Vec<&String> = args
        .iter()
        .take_while(|a| !a.starts_with('-') && a.as_str() != "!" && a.as_str() != "(")
        .collect();
    let root = if paths.is_empty() {
        ctx.root_of(cwd).cloned()
    } else {
        paths.iter().find_map(|p| ctx.word_root(p, cwd)).cloned()
    };
    root.map_or(GuardDecision::Allow, |r| deny(r, GuardRule::FindDelete))
}
