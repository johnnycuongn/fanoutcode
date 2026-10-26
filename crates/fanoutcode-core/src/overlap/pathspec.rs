//! The git guard's pathspecs (§5.6): which claimed paths a pathspec names.
//!
//! This is a deliberately conservative over-approximation of git's matcher, not a copy of it. Git reads pathspecs
//! through magic, case folding, `:(glob)` rules and settings the guard cannot all see (`GIT_ICASE_PATHSPECS` may be
//! exported earlier or inherited, `core.ignorecase`, Unicode folding), and every place where the two disagree is a
//! way past the guard. So the answer is allowed to name too much, never too little:
//!
//! - an include always matches ASCII case-insensitively;
//! - `*` and `?` are matched (crossing `/`, git's default); a bracket expression, a `\` (git's escape) or `:(glob)`
//!   magic names every claim under the spec's literal directory prefix;
//! - an exclude is honoured only when it is plain ASCII without wildcards or escapes, already normalised (no trailing
//!   or doubled `/`, no `.` or `..` component), matched exactly; any other excludes nothing;
//! - unknown long-form magic, or a glob over `MAX_GLOB` chars, names every claim.

use std::path::Path;

use super::claims::Claim;
use super::git_rules::{rel_in, resolve};

/// The characters that make an include wild: git's wildcards and its escape.
const WILD: [char; 4] = ['*', '?', '[', '\\'];

/// A glob longer than this is not matched; it counts as naming every claim.
const MAX_GLOB: usize = 256;

#[derive(Debug, PartialEq)]
enum Tok {
    Lit(char),
    One,
    Star,
}

/// The `*` / `?` glob as tokens, consecutive `*` collapsed.
fn tokens(pat: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    for c in pat.chars() {
        match c {
            '*' if out.last() == Some(&Tok::Star) => {}
            '*' => out.push(Tok::Star),
            '?' => out.push(Tok::One),
            c => out.push(Tok::Lit(c)),
        }
    }
    out
}

/// Wildcard match, `*` and `?` crossing `/`, iterative: O(pattern × text), backtracking only to the last `*`.
fn glob_match(pat: &[Tok], text: &[char]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        match pat.get(p) {
            Some(Tok::Star) => {
                star = Some((p, t));
                p += 1;
            }
            Some(Tok::One) => {
                p += 1;
                t += 1;
            }
            Some(Tok::Lit(c)) if *c == text[t] => {
                p += 1;
                t += 1;
            }
            _ => match star {
                Some((sp, st)) => {
                    p = sp + 1;
                    t = st + 1;
                    star = Some((sp, st + 1));
                }
                None => return false,
            },
        }
    }
    pat[p..].iter().all(|tok| *tok == Tok::Star)
}

/// `path` with `.`, `..` and repeated `/` resolved lexically; `None` when it climbs above its start.
fn normalise(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// One pathspec's magic, and the path after it.
#[derive(Debug, Default)]
struct Magic<'a> {
    /// `:/x`, `:(top)x`: relative to the repo root.
    top: bool,
    /// `:!x`, `:^x`, `:(exclude)x`: takes paths away.
    exclude: bool,
    /// `:(literal)x`: no wildcards.
    literal: bool,
    /// `:(glob)x`: not modelled, so everything under the prefix.
    glob: bool,
    /// A long-form word not understood here: the spec counts as naming every claim.
    unknown: bool,
    rest: &'a str,
}

/// Parses `:<short magic>[:]rest` (`/`, `!`, `^`; git drops one `:` after them) and `:(<word>,…)rest`. `icase`
/// needs no flag: includes always fold case.
fn magic(spec: &str) -> Magic<'_> {
    let Some(after) = spec.strip_prefix(':') else {
        return Magic {
            rest: spec,
            ..Magic::default()
        };
    };
    if let Some(long) = after.strip_prefix('(') {
        let Some((words, rest)) = long.split_once(')') else {
            return Magic {
                unknown: true,
                rest: spec,
                ..Magic::default()
            };
        };
        let mut m = Magic {
            rest,
            ..Magic::default()
        };
        for w in words.split(',').map(str::trim).filter(|w| !w.is_empty()) {
            match w {
                "top" => m.top = true,
                "exclude" => m.exclude = true,
                "literal" => m.literal = true,
                "glob" => m.glob = true,
                "icase" => {}
                _ => m.unknown = true,
            }
        }
        return m;
    }
    let shorts = after.find(|c| !matches!(c, '/' | '!' | '^')).unwrap_or(after.len());
    let (flags, rest) = after.split_at(shorts);
    Magic {
        top: flags.contains('/'),
        exclude: flags.contains(['!', '^']),
        rest: rest.strip_prefix(':').unwrap_or(rest),
        ..Magic::default()
    }
}

/// `rel` names `path`: the path itself, everything under it as a directory, or all at the root.
fn names(rel: &str, path: &str) -> bool {
    rel.is_empty() || path == rel || path.strip_prefix(rel).is_some_and(|r| r.starts_with('/'))
}

/// The claimed paths one include names, relative to `dir` or, `top`, to the repo root, ASCII case folded.
fn name_include(m: &Magic<'_>, dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let every = || claimed.iter().map(|c| c.path.clone()).collect();
    let fold = |s: &str| s.to_ascii_lowercase();
    if m.unknown {
        return every();
    }
    let base = if m.top { repo } else { dir };
    let wild = !m.literal && m.rest.contains(WILD);
    if !wild && !m.glob {
        let Some(rel) = rel_in(repo, &resolve(m.rest, base)) else {
            return Vec::new();
        };
        let rel = fold(&rel);
        return claimed
            .iter()
            .filter(|c| names(&rel, &fold(&c.path)))
            .map(|c| c.path.clone())
            .collect();
    }
    if m.rest.chars().count() > MAX_GLOB {
        return every();
    }
    let Some(prefix) = rel_in(repo, base) else {
        return Vec::new();
    };
    // a glob that climbs out of the repo names nothing in it
    let Some(full) = normalise(&format!("{prefix}/{}", m.rest)) else {
        return Vec::new();
    };
    let full = fold(&full);
    if m.glob || full.contains(['[', '\\']) {
        // everything under the literal directories before the first wildcard or escape
        let literal: Vec<&str> = full.split('/').take_while(|part| !part.contains(WILD)).collect();
        let dir_prefix = if literal.len() == full.split('/').count() {
            full.clone()
        } else {
            literal.join("/")
        };
        return claimed
            .iter()
            .filter(|c| names(&dir_prefix, &fold(&c.path)))
            .map(|c| c.path.clone())
            .collect();
    }
    let pat = tokens(&full);
    claimed
        .iter()
        .filter(|c| glob_match(&pat, &fold(&c.path).chars().collect::<Vec<_>>()))
        .map(|c| c.path.clone())
        .collect()
}

/// The claimed paths one exclude takes away: only a plain ASCII path without wildcards, escapes or other magic, and
/// already normalised (git's reading of `a/`, `a//b`, `./a` or `a/../b` is not modelled), matched exactly; any other
/// exclude takes nothing away.
fn name_exclude(m: &Magic<'_>, dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let normalised =
        !m.rest.ends_with('/') && !m.rest.contains("//") && !m.rest.split('/').any(|part| matches!(part, "." | ".."));
    let plain = !m.unknown && !m.glob && m.rest.is_ascii() && !m.rest.contains(WILD) && normalised;
    if !plain {
        return Vec::new();
    }
    let base = if m.top { repo } else { dir };
    let Some(rel) = rel_in(repo, &resolve(m.rest, base)) else {
        return Vec::new();
    };
    claimed
        .iter()
        .filter(|c| names(&rel, &c.path))
        .map(|c| c.path.clone())
        .collect()
}

/// Whether `specs` hold at least one include (a spec that is not a recognised exclude). Without one, git reads the
/// list as the whole tree minus the excludes, from the repo root (`PATHSPEC_PREFER_FULL`), whatever the cwd.
pub(super) fn has_include(specs: &[&str]) -> bool {
    specs.iter().any(|s| {
        let m = magic(s);
        !m.exclude || m.unknown
    })
}

/// The claimed paths the given pathspecs (resolved against `dir`) name in `repo`: what the includes name, minus what
/// the excludes name. Excludes alone stand for the whole repo minus them, from its root (never only `dir`, which would
/// name too little).
pub(super) fn named(specs: &[&str], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let (mut any_include, mut any_exclude) = (false, false);
    for spec in specs {
        let m = magic(spec);
        if m.exclude && !m.unknown {
            any_exclude = true;
            excluded.extend(name_exclude(&m, dir, repo, claimed));
        } else {
            any_include = true;
            included.extend(name_include(&m, dir, repo, claimed));
        }
    }
    if !any_include && any_exclude {
        let here = Magic {
            top: true,
            rest: ".",
            ..Magic::default()
        };
        included = name_include(&here, dir, repo, claimed);
    }
    included.retain(|p| !excluded.contains(p));
    included
}

#[cfg(test)]
#[path = "pathspec_tests.rs"]
mod tests;
