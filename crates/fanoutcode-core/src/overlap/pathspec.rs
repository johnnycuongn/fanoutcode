//! The git guard's pathspecs (§5.6): which claimed paths a pathspec names. Understands `:/`, `:(top)`, excludes
//! (`:!`, `:^`, `:(exclude)`) and `*` / `?` / `[…]` globs; other magic is read as a plain path. When unsure it names
//! more, never less.

use std::path::Path;

use super::claims::Claim;
use super::git_rules::{rel_in, resolve};

/// A glob longer than this is not matched; it counts as naming every claim.
const MAX_GLOB: usize = 256;

/// The claimed paths a repo-relative path names: the path itself, everything under it as a directory, or all at the
/// root.
fn matching<'a>(spec: &str, claimed: &[&'a Claim]) -> Vec<&'a str> {
    claimed
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| spec.is_empty() || *p == spec || p.strip_prefix(spec).is_some_and(|r| r.starts_with('/')))
        .collect()
}

#[derive(Debug, PartialEq)]
enum Tok {
    Lit(char),
    One,
    Set { negate: bool, ranges: Vec<(char, char)> },
    Star,
}

/// The glob as tokens, consecutive `*` collapsed. A `[` without its `]` is a literal.
fn tokens(pat: &str) -> Vec<Tok> {
    let chars: Vec<char> = pat.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if out.last() != Some(&Tok::Star) {
                    out.push(Tok::Star);
                }
                i += 1;
            }
            '?' => {
                out.push(Tok::One);
                i += 1;
            }
            '[' => match chars[i + 1..].iter().skip(1).position(|c| *c == ']').map(|n| i + 2 + n) {
                Some(close) => {
                    let mut set = &chars[i + 1..close];
                    let negate = matches!(set.first(), Some('!' | '^'));
                    if negate {
                        set = &set[1..];
                    }
                    let mut ranges = Vec::new();
                    let mut j = 0;
                    while j < set.len() {
                        if j + 2 < set.len() && set[j + 1] == '-' {
                            ranges.push((set[j], set[j + 2]));
                            j += 3;
                        } else {
                            ranges.push((set[j], set[j]));
                            j += 1;
                        }
                    }
                    out.push(Tok::Set { negate, ranges });
                    i = close + 1;
                }
                None => {
                    out.push(Tok::Lit('['));
                    i += 1;
                }
            },
            c => {
                out.push(Tok::Lit(c));
                i += 1;
            }
        }
    }
    out
}

fn one(tok: &Tok, c: char) -> bool {
    match tok {
        Tok::Lit(l) => *l == c,
        Tok::One => true,
        Tok::Set { negate, ranges } => ranges.iter().any(|(a, b)| (*a..=*b).contains(&c)) != *negate,
        Tok::Star => false,
    }
}

/// Git's pathspec wildcard match without `:(glob)` magic (`*` and `?` cross `/`), iterative: O(pattern × text),
/// backtracking only to the last `*`.
fn glob_match(pat: &[Tok], text: &[char]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pat.len() && pat[p] == Tok::Star {
            star = Some((p, t));
            p += 1;
        } else if p < pat.len() && one(&pat[p], text[t]) {
            p += 1;
            t += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
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

/// One pathspec's magic: `(top, exclude, rest)`. `:/x` and `:(top)x` are relative to the repo root; `:!x`, `:^x`
/// and `:(exclude)x` take paths away.
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

/// The claimed paths one pathspec (its magic stripped) names, relative to `dir` or, `top`, to the repo root.
fn name_one(rest: &str, top: bool, dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let base = if top { repo } else { dir };
    if !rest.contains(['*', '?', '[']) {
        return rel_in(repo, &resolve(rest, base))
            .map(|rel| matching(&rel, claimed).into_iter().map(str::to_owned).collect())
            .unwrap_or_default();
    }
    if rest.chars().count() > MAX_GLOB {
        return claimed.iter().map(|c| c.path.clone()).collect();
    }
    let Some(prefix) = rel_in(repo, base) else {
        return Vec::new();
    };
    // a glob that climbs out of the repo names nothing in it
    let Some(full) = normalise(&format!("{prefix}/{rest}")) else {
        return Vec::new();
    };
    let pat = tokens(&full);
    claimed
        .iter()
        .filter(|c| glob_match(&pat, &c.path.chars().collect::<Vec<_>>()))
        .map(|c| c.path.clone())
        .collect()
}

/// The claimed paths the given pathspecs (resolved against `dir`) name in `repo`: what the includes name, minus what
/// the excludes name. Excludes alone stand for `dir` minus them, as git reads them.
pub(super) fn named(specs: &[&str], dir: &Path, repo: &Path, claimed: &[&Claim]) -> Vec<String> {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let (mut any_include, mut any_exclude) = (false, false);
    for spec in specs {
        let (top, exclude, rest) = magic(spec);
        if exclude {
            any_exclude = true;
            excluded.extend(name_one(rest, top, dir, repo, claimed));
        } else {
            any_include = true;
            included.extend(name_one(rest, top, dir, repo, claimed));
        }
    }
    if !any_include && any_exclude {
        included = name_one(".", false, dir, repo, claimed);
    }
    included.retain(|p| !excluded.contains(p));
    included
}
