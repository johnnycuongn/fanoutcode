//! The Bash guard behind the read-only policy (§4.3): a `PreToolUse` hook that refuses shell commands which look like
//! they write under a protected root. Best effort by design (an interpreter running a script is not inspected); the
//! `Edit` deny rules remain the hard guarantee for the file tools.

use std::path::{Path, PathBuf};

mod opaque;
mod protocol;
pub(crate) mod rules;
pub(crate) mod shell_words;
mod writers;

pub use protocol::{guard_hook_spec, prepare_guard, run_guard, write_roots_file};
pub use rules::DB_PREFIX;

/// What the hook is asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInput {
    pub command: String,
    pub cwd: PathBuf,
}

/// Why a command looks like a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardRule {
    Redirect,
    Program(&'static str),
    SedInPlace,
    PerlInPlace,
    Git(&'static str),
    PackageInstall(&'static str),
    FindDelete,
    /// A writer, redirection or unpacker given a path the guard cannot read (`$VAR`, `$(…)`, backticks, `xargs`).
    Unresolved(&'static str),
    /// A command naming the app data folder that does more than read it.
    NamesAppData,
}

impl std::fmt::Display for GuardRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardRule::Redirect => f.write_str("output redirection"),
            GuardRule::Program(p) => f.write_str(p),
            GuardRule::SedInPlace => f.write_str("sed -i"),
            GuardRule::PerlInPlace => f.write_str("perl -i"),
            GuardRule::Git(sub) => write!(f, "git {sub}"),
            GuardRule::PackageInstall(tool) => write!(f, "{tool} package change"),
            GuardRule::FindDelete => f.write_str("find -delete/-exec"),
            GuardRule::Unresolved(p) => write!(f, "{p} with a path it cannot read: $VAR, $(…), backticks or xargs"),
            GuardRule::NamesAppData => f.write_str("a command naming the app data folder that does more than read it"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
    Allow,
    Deny {
        root: PathBuf,
        rule: GuardRule,
        reason: String,
    },
}

pub fn deny_reason(root: &Path, rule: &GuardRule) -> String {
    format!(
        "Fanout Code: {} is read-only in this workspace; this shell command looks like it writes there ({rule}). Read \
         it freely; to change it, ask the developer to change the linking policy.",
        root.display()
    )
}

pub fn evaluate(input: &GuardInput, protected: &[PathBuf], home_dir: &Path) -> GuardDecision {
    evaluate_with_app_data(input, protected, home_dir, None)
}

/// `evaluate`, also denying any command that names `app_data` (its written or canonical form) and does more than
/// read it.
pub fn evaluate_with_app_data(
    input: &GuardInput,
    protected: &[PathBuf],
    home_dir: &Path,
    app_data: Option<&Path>,
) -> GuardDecision {
    let mut forms: Vec<PathBuf> = Vec::new();
    if let Some(a) = app_data.map(rules::clean).filter(|a| a.parent().is_some()) {
        if let Ok(c) = std::fs::canonicalize(&a) {
            if c != a {
                forms.push(c);
            }
        }
        forms.insert(0, a);
    }
    let ctx = rules::Ctx {
        roots: protected
            .iter()
            .map(|r| rules::clean(r))
            .filter(|r| r.parent().is_some())
            .collect(),
        home: home_dir,
        app_data: forms,
    };
    rules::evaluate_in(&ctx, &input.command, &mut rules::clean(&input.cwd), 0)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fail_closed_tests;
