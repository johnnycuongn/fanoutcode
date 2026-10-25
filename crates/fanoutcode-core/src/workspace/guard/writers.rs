//! The programs that write files (`cp`, `mv`, `rsync`, `install`, `ln`, `tar`, `unzip` and the other writers): denied
//! when any argument reaches a protected root (is one, lies in one, or holds one), whatever its option spelling. Each
//! word is checked whole, by the value after `=` in `--opt=value`, by every rest of a short cluster after a flag
//! letter (`-Cdir`, `-xfC…`, `-tdir`), and by every absolute path embedded in it (`--transform=s,^,/dir/,`). Only
//! the plain sources of a copy are left out: reading a protected root is allowed.

use std::path::{Path, PathBuf};

use super::opaque::{deny_unresolved, unresolved};
use super::rules::{deny, Ctx};
use super::{GuardDecision, GuardRule};

pub(super) const WRITERS: &[&str] = &[
    "rm", "rmdir", "mv", "cp", "ln", "mkdir", "touch", "tee", "truncate", "dd", "shred", "unlink", "chmod", "chown",
    "chgrp", "install", "rsync", "patch",
];

/// `cp`'s short option letters that take a value: a positional before the destination is a source (a read) unless
/// it may be an option's value.
const CP_VALUE_LETTERS: &str = "tS";

/// The copiers that read nothing as exempt: every argument may be written (`ln -s`'s target, `install`'s and
/// `rsync`'s options and sources included).
const CHECK_EVERY: &[&str] = &["ln", "install", "rsync"];

/// Where an embedded path may start or end inside a word (sed-style delimiters, quoting and option punctuation).
const DELIMS: &[char] = &[
    ',', '|', '#', ':', ';', '=', '\'', '"', '!', '@', '%', '^', '&', '*', '(', ')', '{', '}', '[', ']', '<', '>', '?',
    '\\', '+',
];

/// The path spellings one argument word may carry (`opt` once it is past `--`, a plain positional).
fn candidates(word: &str, opt: bool) -> Vec<&str> {
    let mut out = Vec::new();
    match (opt, word.strip_prefix("--"), word.strip_prefix('-')) {
        (true, Some(long), _) => out.extend(long.split_once('=').map(|(_, v)| v)),
        (true, None, Some(short)) => out.extend(short.char_indices().map(|(i, _)| &short[i..])),
        _ => {
            out.push(word);
            out.extend(word.split_once('=').map(|(_, v)| v));
        }
    }
    out.extend(
        word.split(DELIMS)
            .filter(|p| p.starts_with('/') || *p == "~" || p.starts_with("~/")),
    );
    out.retain(|w| !w.is_empty());
    out
}

/// The first protected root an argument reaches, skipping those `skip` says are reads.
fn args_root(ctx: &Ctx, args: &[String], cwd: &Path, skip: impl Fn(usize) -> bool) -> Option<PathBuf> {
    let mut opts = true;
    for (i, a) in args.iter().enumerate() {
        if opts && a == "--" {
            opts = false;
            continue;
        }
        if skip(i) {
            continue;
        }
        let opt = opts && a.starts_with('-');
        if let Some(r) = candidates(a, opt)
            .into_iter()
            .find_map(|c| ctx.target_root(&ctx.resolve(c, cwd)))
        {
            return Some(r.clone());
        }
    }
    None
}

/// Whether `word` may be an option that takes the next word as its value.
fn takes_next(word: &str, value_letters: &str) -> bool {
    match (word.strip_prefix("--"), word.strip_prefix('-')) {
        (Some(long), _) => !long.is_empty() && !long.contains('='),
        (None, Some(short)) => short.contains(|c| value_letters.contains(c)),
        _ => false,
    }
}

/// The positional indexes of `args` (plain words, and every word after `--`).
fn positional_indexes(args: &[String]) -> Vec<usize> {
    let mut opts = true;
    args.iter()
        .enumerate()
        .filter_map(|(i, a)| {
            if opts && a == "--" {
                opts = false;
                return None;
            }
            (!opts || !a.starts_with('-')).then_some(i)
        })
        .collect()
}

/// Whether a copier names its destination folder with `-t` / `--target-directory` (not `rsync`, whose `-t` keeps
/// times).
fn has_target_dir(name: &str, args: &[String]) -> bool {
    name != "rsync"
        && args.iter().take_while(|a| *a != "--").any(|a| {
            a.starts_with("--target")
                || a.strip_prefix('-')
                    .is_some_and(|s| !s.starts_with('-') && s.contains('t'))
        })
}

/// A file writer: `cp`'s sources are reads, `ln`, `install` and `rsync` may write any argument, a move writes both
/// ends, any other also writes the cwd it runs in. A copier with one positional and no `-t` copies into the cwd.
/// `blind` (arguments from a substitution or `xargs`) or an argument the guard cannot read, other than a `cp`
/// source, denies.
pub(super) fn writer(ctx: &Ctx, name: &'static str, args: &[String], cwd: &Path, blind: bool) -> GuardDecision {
    if blind {
        return deny_unresolved(ctx, name);
    }
    let pos = positional_indexes(args);
    let copier = name == "cp" || CHECK_EVERY.contains(&name);
    let into_cwd = copier && pos.len() == 1 && !has_target_dir(name, args);
    let read = |i: usize| -> bool {
        if name != "cp" || !pos.contains(&i) {
            return false;
        }
        let dest = if into_cwd { None } else { pos.last().copied() };
        Some(i) != dest && !(i > 0 && takes_next(&args[i - 1], CP_VALUE_LETTERS))
    };
    if args.iter().enumerate().any(|(i, a)| !read(i) && unresolved(ctx, a)) {
        return deny_unresolved(ctx, name);
    }
    let root = if copier {
        args_root(ctx, args, cwd, read).or_else(|| into_cwd.then(|| ctx.target_root(cwd).cloned()).flatten())
    } else if name == "mv" {
        // a move writes both ends
        args_root(ctx, args, cwd, |_| false)
    } else {
        ctx.root_of(cwd)
            .cloned()
            .or_else(|| args_root(ctx, args, cwd, |_| false))
    };
    root.map_or(GuardDecision::Allow, |r| deny(r, GuardRule::Program(name)))
}

/// The short-option clusters of `args` (the old-style first word of `tar` too) and the long option names.
pub(super) fn options(args: &[String], old_style: bool) -> (Vec<&str>, Vec<&str>) {
    let (mut short, mut long) = (Vec::new(), Vec::new());
    for (i, a) in args.iter().enumerate() {
        if a == "--" {
            break;
        }
        if let Some(l) = a.strip_prefix("--") {
            long.push(l.split_once('=').map_or(l, |(n, _)| n));
        } else if let Some(s) = a.strip_prefix('-') {
            short.push(s);
        } else if i == 0 && old_style {
            short.push(a);
        }
    }
    (short, long)
}

/// A long option name that is, or abbreviates, one of `names`.
pub(super) fn long_is(long: &[&str], names: &[&str]) -> bool {
    long.iter()
        .any(|l| !l.is_empty() && names.iter().any(|n| n.starts_with(l)))
}

/// Whether a cluster names its directory option `dir` before any letter that takes the rest as its own value.
fn names_dir(short: &[&str], dir: char, value_letters: &str) -> bool {
    short.iter().any(|s| {
        s.chars()
            .find(|c| *c == dir || value_letters.contains(*c))
            .is_some_and(|c| c == dir)
    })
}

/// An unpacking program's answer: an argument reaching a root, the cwd it unpacks into (no directory option it can
/// read), or, `unbounded` (members may land anywhere), the first root.
fn unpack(
    ctx: &Ctx,
    args: &[String],
    cwd: &Path,
    into_cwd: bool,
    unbounded: bool,
    name: &'static str,
) -> GuardDecision {
    let root = args_root(ctx, args, cwd, |_| false)
        .or_else(|| into_cwd.then(|| ctx.target_root(cwd).cloned()).flatten())
        .or_else(|| unbounded.then(|| ctx.roots.first().cloned()).flatten());
    root.map_or(GuardDecision::Allow, |r| deny(r, GuardRule::Program(name)))
}

/// `tar`: allowed only when it plainly lists; otherwise every argument counts, and an extract also writes its cwd
/// unless it names a directory (`-C`, `--directory`) and keeps member names relative (no `-P`).
pub(super) fn tar(ctx: &Ctx, args: &[String], cwd: &Path) -> GuardDecision {
    let (short, long) = options(args, true);
    let has = |letters: &str| short.iter().any(|s| s.contains(|c| letters.contains(c)));
    const WRITE_MODES: &[&str] = &[
        "extract",
        "get",
        "create",
        "append",
        "update",
        "catenate",
        "concatenate",
        "delete",
    ];
    let writes = has("xcruA") || long_is(&long, WRITE_MODES);
    let lists = has("t") || long_is(&long, &["list"]);
    if lists && !writes {
        return GuardDecision::Allow;
    }
    // no mode it can read counts as an extract
    let extracts = has("x") || long_is(&long, &["extract", "get"]) || !(writes || lists);
    let dir = names_dir(&short, 'C', "bfFgHIKLNTVX") || long.iter().any(|l| l.len() >= 3 && "directory".starts_with(l));
    let absolute = has("P") || long_is(&long, &["absolute-names", "absolute-paths", "insecure"]);
    unpack(ctx, args, cwd, extracts && !dir, extracts && absolute, "tar")
}

/// `unzip`: allowed when it lists, tests or pipes; otherwise every argument counts, and it writes its cwd unless it
/// names `-d`. `-:` keeps `..` in member names, so they may land anywhere.
pub(super) fn unzip(ctx: &Ctx, args: &[String], cwd: &Path) -> GuardDecision {
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "-l" | "-t" | "-v" | "-Z" | "-p"))
    {
        return GuardDecision::Allow;
    }
    let (short, _) = options(args, false);
    let dir = names_dir(&short, 'd', "Px");
    let unbounded = short.iter().any(|s| s.contains(':'));
    unpack(ctx, args, cwd, !dir, unbounded, "unzip")
}

#[cfg(test)]
#[path = "writers_tests.rs"]
mod tests;
