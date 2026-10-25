//! The guard's fail-closed rules, kept simple rather than parsing every option: a writer given a path it cannot read
//! (`$VAR`, `${…}`, `$(…)`, backticks, `xargs` input) is denied, and so is any command naming the app data folder
//! that does more than read it (the text rule, which also catches interpreters and database shells the program
//! rules never see). Tasks' memory folders under `<app_data>/projects/` stay writable.

use std::path::{Path, PathBuf};

use super::rules::{clean, deny, program_name, strip_prefixes, Ctx};
use super::shell_words::Segment;
use super::writers::{long_is, options};
use super::{GuardDecision, GuardRule};

/// The folder of `<app_data>` that holds tasks' memory (`MemoryLocation::AppStorage`): agents write it.
const MEMORY_DIR: &str = "projects";

/// Whether `word` (with `~` / `$HOME` expanded) still holds a variable or a substitution.
pub(super) fn unresolved(ctx: &Ctx, word: &str) -> bool {
    let w = ctx.expand_home(word);
    let value = w.split_once('=').map(|(_, v)| ctx.expand_home(v));
    [Some(w), value]
        .into_iter()
        .flatten()
        .any(|w| w.contains('$') || w.contains('`'))
}

pub(super) fn any_unresolved(ctx: &Ctx, args: &[String]) -> bool {
    args.iter().any(|a| unresolved(ctx, a))
}

/// `sed -i` / `perl -i`: whether a file operand (a positional after the script, or every positional when the script
/// comes from `-e`) cannot be read. The script itself may hold `$` freely.
pub(super) fn files_unresolved(ctx: &Ctx, args: &[String]) -> bool {
    let script_flag = args
        .iter()
        .any(|a| a == "-e" || a.starts_with("--expression") || a == "-f" || a.starts_with("--file"));
    let mut skip_next = false;
    let mut seen_script = script_flag;
    for a in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a.starts_with('-') {
            skip_next = matches!(a.as_str(), "-e" | "-f" | "--expression" | "--file");
            continue;
        }
        if !seen_script {
            seen_script = true;
            continue;
        }
        if unresolved(ctx, a) {
            return true;
        }
    }
    false
}

/// What a blind write is denied against: the app data folder when known, else the first protected root.
fn blind_root(ctx: &Ctx) -> Option<PathBuf> {
    ctx.app_data.first().or(ctx.roots.first()).cloned()
}

/// A deny for `what` given a path the guard cannot read, whenever anything is guarded at all.
pub(super) fn deny_unresolved(ctx: &Ctx, what: &'static str) -> GuardDecision {
    blind_root(ctx).map_or(GuardDecision::Allow, |r| deny(r, GuardRule::Unresolved(what)))
}

/// Case-insensitive on macOS, as `rules` compares.
fn fold(s: &str) -> String {
    if cfg!(target_os = "macos") {
        s.to_lowercase()
    } else {
        s.to_owned()
    }
}

/// Whether `rest` (what follows an app-data form in a word) stays out of the memory folders.
fn outside_memory(rest: &str) -> bool {
    let rest = rest.trim_start_matches('/');
    !(rest == MEMORY_DIR || rest.starts_with(&format!("{MEMORY_DIR}/")))
}

/// Whether `text` names the app data folder outside its memory folders, at any position.
fn text_names(ctx: &Ctx, text: &str) -> bool {
    let t = fold(text);
    ctx.app_data.iter().any(|a| {
        let a = fold(&a.display().to_string());
        t.match_indices(&a).any(|(i, _)| {
            let rest = &t[i + a.len()..];
            // a longer name beside it (`app-old`) is not the folder
            (rest.is_empty()
                || rest.starts_with(|c: char| c == '/' || !c.is_alphanumeric() && c != '-' && c != '_' && c != '.'))
                && outside_memory(rest)
        })
    })
}

/// Whether `path` (absolute) lies in the app data folder outside its memory folders.
fn path_in(ctx: &Ctx, path: &Path) -> bool {
    let p = clean(path);
    ctx.app_data.iter().any(|a| {
        p.strip_prefix(a)
            .is_ok_and(|rest| outside_memory(&rest.to_string_lossy()))
    })
}

/// Whether one word names app data: in its text (`~` / `$HOME` expanded, the value after `=` too), or, relative,
/// once resolved against `cwd`.
fn word_names(ctx: &Ctx, word: &str, cwd: &Path) -> bool {
    let value = word.split_once('=').map(|(_, v)| v);
    [Some(word), value].into_iter().flatten().any(|w| {
        text_names(ctx, w)
            || text_names(ctx, &ctx.expand_home(w))
            || (!w.is_empty() && path_in(ctx, &ctx.resolve(w, cwd)))
    })
}

/// The programs that only read, as long as none of their options writes or runs a command.
const READERS: &[&str] = &[
    "cat", "less", "head", "tail", "grep", "egrep", "fgrep", "rg", "wc", "stat", "file", "ls", "diff", "cd", "pushd",
    "tar", "bsdtar", "gtar", "unzip",
];

/// A short-option cluster (`-So`), not a long option.
fn short_has(a: &str, letters: &str) -> bool {
    a.strip_prefix('-')
        .filter(|s| !s.starts_with('-'))
        .is_some_and(|s| s.contains(|c| letters.contains(c)))
}

/// Whether `program` with `args` only reads.
fn only_reads(program: &str, args: &[String]) -> bool {
    match program {
        "less" => !args.iter().any(|a| {
            short_has(a, "oO") || a.starts_with("--log-file") || a.starts_with("--LOG-FILE") || a.starts_with('+')
        }),
        "rg" => !args.iter().any(|a| a.starts_with("--pre")),
        "file" => !args.iter().any(|a| short_has(a, "C") || a.starts_with("--compile")),
        "tar" | "bsdtar" | "gtar" => {
            let (short, long) = options(args, true);
            let has = |letters: &str| short.iter().any(|s| s.contains(|c| letters.contains(c)));
            let lists = has("t") || long_is(&long, &["list"]);
            let writes = has("xcruAIF")
                || long_is(
                    &long,
                    &[
                        "extract",
                        "get",
                        "create",
                        "append",
                        "update",
                        "catenate",
                        "concatenate",
                        "delete",
                        "to-command",
                        "checkpoint-action",
                        "use-compress-program",
                        "rsh-command",
                        "index-file",
                        "info-script",
                        "new-volume-script",
                    ],
                );
            // a `host:path` archive is opened through a remote shell
            lists && !writes && !args.iter().any(|a| a.contains(':'))
        }
        "unzip" => args.iter().any(|a| a == "-l"),
        _ => READERS.contains(&program),
    }
}

/// The text rule: a segment naming the app data folder (outside its memory folders) in any word, assignment or
/// redirection, or run inside it, is denied unless it is a reader with no option that writes or runs a command.
/// `None` when the rule does not apply.
pub(super) fn app_data_rule(ctx: &Ctx, seg: &Segment, cwd: &Path) -> Option<GuardDecision> {
    let root = ctx.app_data.first()?;
    let rest = strip_prefixes(&seg.words);
    let prefix = &seg.words[..seg.words.len() - rest.len()];
    let in_prefix = prefix.iter().any(|w| word_names(ctx, w, Path::new("/")));
    let in_target = seg.write_targets.iter().any(|t| word_names(ctx, t, cwd));
    let in_args = rest.iter().skip(1).any(|w| word_names(ctx, w, cwd));
    let program = rest.first().map(|p| program_name(p));
    let in_program = rest.first().is_some_and(|p| p.contains('/') && word_names(ctx, p, cwd));
    if !(in_prefix || in_target || in_args || in_program) {
        return None;
    }
    let reads = !in_prefix && !in_target && !in_program && program.is_some_and(|p| only_reads(p, &rest[1..]));
    (!reads).then(|| deny(root.clone(), GuardRule::NamesAppData))
}
