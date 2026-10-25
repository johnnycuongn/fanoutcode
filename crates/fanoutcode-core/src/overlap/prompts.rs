//! Every overlap prompt text (spec §5.4, §5.5, §6.1–§6.3).

use serde::{Deserialize, Serialize};

use super::Decision;
use crate::domain::{AgentState, Stage};
use crate::pipeline::handback::chip_text;
use crate::settings::Integration;

/// The line the integrate prompt asks the session to end with (§6.3).
pub const INTEGRATE_SENTINEL: &str = "FANOUTCODE: integrate done";

/// The §6.1 instruction that closes "Others working here", verbatim.
const PLAN_AROUND: &str = "Prefer files none of them touch. Where you must touch one, say so in the plan and keep \
the change narrow. Reuse what they added rather than duplicating it. List every file you intend to change under \
`## Files`.";

/// The §6.3 conflict rule, verbatim.
const KEEP_BOTH: &str = "Keep both behaviours. Never resolve a conflict by taking one side wholesale. Where both \
sides changed the same lines, write the version that does what each task's spec asks. Run the tests. If you cannot \
keep both, stop, leave the conflict markers in place, and say which files and why.";

/// One other active task as the Plan prompt's "Others working here" section shows it (§6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct OtherSummary {
    pub title: String,
    pub stage: Stage,
    pub state: AgentState,
    /// The current branch of its worktree or of the shared tree; `None` when git could not say.
    pub branch: Option<String>,
    /// The first line of its `intent.md`, else its title.
    pub intent: String,
    /// Its footprint: the `## Files` paths of its plan.
    pub planned: Vec<String>,
    /// The files its change set holds so far.
    pub touched: Vec<String>,
    pub added_files: Vec<String>,
    /// `export`-level names of its added or changed files (`export_names`).
    pub exported: Vec<String>,
}

/// One file the integrate prompt lists, with the other task that touched it on the base side and that task's
/// spec summary and plan excerpt (§6.3). Also what the merge hand-back pane shows (`overlap_integration_context`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ConflictCtx {
    pub path: String,
    pub other_title: String,
    pub spec_excerpt: String,
    pub plan_excerpt: String,
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn state_text(state: &AgentState, stage: Stage) -> String {
    match state {
        AgentState::Working => "working".to_owned(),
        AgentState::Idle => "idle".to_owned(),
        AgentState::NeedsYou { reason } => format!("needs you ({})", chip_text(reason, stage)),
    }
}

/// The Plan prompt's "Others working here" section (§6.1); `""` when no other task is active.
pub fn others_working_here(others: &[OtherSummary]) -> String {
    if others.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## Others working here\n\nOther active tasks in this project and the linked projects you can write:\n",
    );
    for o in others {
        out.push_str(&format!(
            "\n### {}\n- stage: {:?}, state: {}\n- branch: {}\n- intent: {}\n- planned files: {}\n\
- files changed so far: {}\n- files added: {}\n- exported names: {}\n",
            o.title,
            o.stage,
            state_text(&o.state, o.stage),
            o.branch.as_deref().unwrap_or("unknown"),
            o.intent,
            if o.planned.is_empty() {
                "none listed".to_owned()
            } else {
                o.planned.join(", ")
            },
            list(&o.touched),
            list(&o.added_files),
            list(&o.exported),
        ));
    }
    out.push('\n');
    out.push_str(PLAN_AROUND);
    out.push('\n');
    out
}

/// The go-ahead injected into the session the developer just answered (§5.4). `Hold` gives the message sent when
/// the held file is released.
pub fn go_ahead(decision: Decision, other_title: &str, path: &str) -> String {
    let on_top = format!(
        "Fanout Code: the developer says go ahead with {path}, editing on top of {other_title}'s uncommitted \
changes (keep them; add yours around them)."
    );
    match decision {
        Decision::OnTopFile => on_top,
        Decision::OnTopAll => format!(
            "{on_top} This holds for every file {other_title} touches; you will not be asked about any further \
file of {other_title}'s."
        ),
        Decision::Replace => format!(
            "Fanout Code: the developer says go ahead with {path}, replacing {other_title}'s uncommitted changes \
in {path}; the developer has seen them. The developer wants your version of this file; {other_title}'s \
uncommitted changes in it may go; rewrite it as your task needs. If an edit no longer matches, re-read the file \
and write it."
        ),
        Decision::Hold => format!(
            "Fanout Code: the file is now free: {other_title} no longer has uncommitted changes in {path}. Go \
ahead with {path}; re-read it before your next edit to it."
        ),
    }
}

/// The message to the other session whose claim a grant was written against (§5.5); `""` for `Hold`, which
/// grants nothing.
pub fn told(decision: Decision, my_title: &str, path: &str) -> String {
    match decision {
        Decision::OnTopFile => format!(
            "Fanout Code: task {my_title} is now editing {path} on top of your uncommitted changes (the developer \
approved it). Re-read the file before your next edit to it."
        ),
        Decision::OnTopAll => format!(
            "Fanout Code: task {my_title} is now editing {path}, and may edit any other file you have uncommitted \
changes in, on top of your changes (the developer approved it). Re-read each such file before your next edit to it."
        ),
        Decision::Replace => format!(
            "Fanout Code: task {my_title} has replaced your uncommitted changes in {path} with the developer's \
approval; your change set marks it as taken over. Re-read it before touching it again."
        ),
        Decision::Hold => String::new(),
    }
}

/// The go-ahead for a shell command the git guard stopped (§5.6), once the developer chose Proceed.
pub fn go_ahead_command(other_title: &str, command: &str) -> String {
    format!(
        "Fanout Code: the developer says go ahead with `{command}`, although it touches files {other_title} has \
uncommitted changes in (the developer approved it). Run it now."
    )
}

/// The once-per-file note to a worktree session editing a file in another task's footprint (§6.2).
pub fn soft_note(other_title: &str, path: &str, branch: &str) -> String {
    format!(
        "Fanout Code: task {other_title} also changes {path} on {branch}. Keep your change narrow and expect a \
merge."
    )
}

/// The note sent to the project's other sessions when a new task becomes active (§6.1, last paragraph).
pub fn new_task_note(title: &str, branch: &str, planned: &[String]) -> String {
    let files = if planned.is_empty() {
        "none listed yet".to_owned()
    } else {
        planned.join(", ")
    };
    format!("Fanout Code: task {title} has started on {branch}; its planned files: {files}")
}

/// The prompt behind **Bring <base> in** (§6.3), assembled from data rather than from the stage templates. The
/// only prompt that tells the agent to merge or rebase (`.claude/rules/prompts.md`). `base` must be a
/// `git::valid_branch` (the shell checks it); it is single-quoted after `--` in the command line all the same.
pub fn integrate(base: &str, mode: Integration, conflicts: &[ConflictCtx]) -> String {
    let (verb, how) = match mode {
        Integration::Merge => (
            "merge",
            format!("Merge `{base}` into your current branch: `git merge -- '{base}'`."),
        ),
        Integration::Rebase => (
            "rebase",
            format!("Rebase your current branch onto `{base}`: `git rebase -- '{base}'`."),
        ),
    };
    let mut out = format!(
        "Fanout Code: the developer asks you to bring `{base}` into this task's branch. This task's work is \
already committed. {how}\n\n"
    );
    if conflicts.is_empty() {
        out.push_str("No file of this task is recorded as overlapping another task's work.\n");
    } else {
        out.push_str("Files that conflict or overlap with other tasks' work on the base side:\n");
        for c in conflicts {
            out.push_str(&format!("- {} — changed by task {}\n", c.path, c.other_title));
        }
        let mut seen: Vec<&str> = Vec::new();
        for c in conflicts {
            if seen.contains(&c.other_title.as_str()) {
                continue;
            }
            seen.push(&c.other_title);
            out.push_str(&format!(
                "\n### Task {}\nSpec summary:\n{}\n\nPlan excerpt:\n{}\n",
                c.other_title,
                none_if_empty(&c.spec_excerpt),
                none_if_empty(&c.plan_excerpt),
            ));
        }
    }
    out.push_str(&format!(
        "\n{KEEP_BOTH}\n\nWhen every conflict is resolved, finish the {verb} so the working tree is clean. Do not \
push, and do not delete any branch.\n\nWhen the {verb} is finished and the tests pass, end your final message with \
this single line and nothing after it:\n{INTEGRATE_SENTINEL}\n"
    ));
    out
}

fn none_if_empty(s: &str) -> &str {
    if s.trim().is_empty() {
        "(none recorded)"
    } else {
        s.trim()
    }
}

const TS_KINDS: [&str; 9] = [
    "async function ",
    "function ",
    "class ",
    "const ",
    "let ",
    "var ",
    "type ",
    "interface ",
    "enum ",
];
const RS_KINDS: [&str; 4] = ["fn ", "struct ", "enum ", "trait "];

fn ident(s: &str) -> Option<String> {
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn after_kind<'a>(rest: &'a str, kinds: &[&str]) -> Option<&'a str> {
    kinds.iter().find_map(|k| rest.strip_prefix(k))
}

/// The exported names of a source file, in order and without repeats: TypeScript or JavaScript
/// `export [default] function|async function|class|const|let|var|type|interface|enum NAME` and Rust
/// `pub fn|struct|enum|trait NAME`. Lightweight line parsing (§6.1); anything else is skipped.
pub fn export_names(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in source.lines() {
        let line = line.trim_start();
        let found = if let Some(rest) = line.strip_prefix("export ") {
            let rest = rest.strip_prefix("default ").unwrap_or(rest);
            after_kind(rest, &TS_KINDS).and_then(ident)
        } else if let Some(rest) = line.strip_prefix("pub ") {
            after_kind(rest, &RS_KINDS).and_then(ident)
        } else {
            None
        };
        if let Some(name) = found {
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
