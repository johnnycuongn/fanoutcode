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
mod tests {
    use super::*;
    use crate::domain::HandbackReason;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn login() -> OtherSummary {
        OtherSummary {
            title: "Login".into(),
            stage: Stage::Build,
            state: AgentState::Working,
            branch: Some("feat/login".into()),
            intent: "Let users sign in with email".into(),
            planned: s(&["src/auth/session.ts", "src/auth/login.tsx"]),
            touched: s(&["src/auth/session.ts"]),
            added_files: s(&["src/auth/login.tsx"]),
            exported: s(&["LoginForm", "startSession"]),
        }
    }

    const SIX_ONE: &str = "Prefer files none of them touch. Where you must touch one, say so in the plan and keep \
the change narrow. Reuse what they added rather than duplicating it. List every file you intend to change under \
`## Files`.";

    #[test]
    fn others_working_here_lists_each_task_and_ends_with_the_instruction() {
        let checkout = OtherSummary {
            title: "Checkout".into(),
            stage: Stage::Plan,
            state: AgentState::Idle,
            branch: None,
            intent: "Checkout".into(),
            planned: vec![],
            touched: vec![],
            added_files: vec![],
            exported: vec![],
        };
        let text = others_working_here(&[login(), checkout]);
        for needle in [
            "Login",
            "Build",
            "working",
            "feat/login",
            "Let users sign in with email",
            "src/auth/session.ts",
            "src/auth/login.tsx",
            "LoginForm",
            "startSession",
            "Checkout",
            "Plan",
            "idle",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
        assert!(text.trim_end().ends_with(SIX_ONE), "{text}");
        assert!(!text.contains("{{"), "no placeholders: {text}");
    }

    #[test]
    fn others_working_here_names_a_needs_you_reason() {
        let mut o = login();
        o.state = AgentState::NeedsYou {
            reason: HandbackReason::StageDone,
        };
        assert!(others_working_here(&[o]).contains("needs you"));
    }

    #[test]
    fn others_working_here_is_empty_with_no_others() {
        assert_eq!(others_working_here(&[]), "");
    }

    #[test]
    fn go_ahead_and_told_name_the_file_and_the_task() {
        let p = "src/auth/session.ts";
        assert_eq!(
            go_ahead(Decision::OnTopFile, "Login", p),
            "Fanout Code: the developer says go ahead with src/auth/session.ts, editing on top of Login's \
uncommitted changes (keep them; add yours around them)."
        );
        assert!(go_ahead(Decision::Replace, "Login", p).starts_with(
            "Fanout Code: the developer says go ahead with src/auth/session.ts, replacing Login's uncommitted \
changes in src/auth/session.ts; the developer has seen them."
        ));
        let all = go_ahead(Decision::OnTopAll, "Login", p);
        assert!(all.contains("every file Login touches") && all.contains(p), "{all}");
        let free = go_ahead(Decision::Hold, "Login", p);
        assert!(free.contains("the file is now free") && free.contains(p), "{free}");

        assert_eq!(
            told(Decision::OnTopFile, "Checkout", p),
            "Fanout Code: task Checkout is now editing src/auth/session.ts on top of your uncommitted changes \
(the developer approved it). Re-read the file before your next edit to it."
        );
        assert_eq!(
            told(Decision::Replace, "Checkout", p),
            "Fanout Code: task Checkout has replaced your uncommitted changes in src/auth/session.ts with the \
developer's approval; your change set marks it as taken over. Re-read it before touching it again."
        );
        let all = told(Decision::OnTopAll, "Checkout", p);
        assert!(all.contains("Checkout") && all.contains(p), "{all}");
        assert_eq!(told(Decision::Hold, "Checkout", p), "");
    }

    #[test]
    fn soft_and_new_task_notes_use_the_spec_sentences() {
        assert_eq!(
            soft_note("Login", "src/a.ts", "feat/login"),
            "Fanout Code: task Login also changes src/a.ts on feat/login. Keep your change narrow and expect a \
merge."
        );
        assert_eq!(
            new_task_note("Login", "feat/login", &s(&["src/a.ts", "src/b.ts"])),
            "Fanout Code: task Login has started on feat/login; its planned files: src/a.ts, src/b.ts"
        );
        assert!(new_task_note("Login", "feat/login", &[]).contains("none listed yet"));
    }

    #[test]
    fn integrate_carries_the_rule_and_the_sentinel() {
        let conflicts = vec![
            ConflictCtx {
                path: "src/auth/session.ts".into(),
                other_title: "Login".into(),
                spec_excerpt: "Users sign in with email.".into(),
                plan_excerpt: "## Files\n- src/auth/session.ts — session cookie".into(),
            },
            ConflictCtx {
                path: "src/auth/login.tsx".into(),
                other_title: "Login".into(),
                spec_excerpt: "Users sign in with email.".into(),
                plan_excerpt: "## Files\n- src/auth/session.ts — session cookie".into(),
            },
        ];
        let merge = integrate("main", Integration::Merge, &conflicts);
        assert!(merge.contains(
            "Keep both behaviours. Never resolve a conflict by taking one side wholesale. Where both sides \
changed the same lines, write the version that does what each task's spec asks. Run the tests. If you cannot keep \
both, stop, leave the conflict markers in place, and say which files and why."
        ));
        assert!(merge.contains("`git merge -- 'main'`"), "{merge}");
        assert!(!merge.contains("git rebase"), "{merge}");
        for needle in [
            "src/auth/session.ts",
            "src/auth/login.tsx",
            "Users sign in with email.",
            "session cookie",
        ] {
            assert!(merge.contains(needle), "missing {needle:?}");
        }
        assert_eq!(
            merge.matches("Users sign in with email.").count(),
            1,
            "one excerpt per task"
        );
        assert_eq!(merge.trim_end().lines().last().unwrap(), INTEGRATE_SENTINEL);
        assert!(!merge.contains("{{"));

        let rebase = integrate("develop", Integration::Rebase, &[]);
        assert!(rebase.contains("`git rebase -- 'develop'`"), "{rebase}");
        assert_eq!(rebase.trim_end().lines().last().unwrap(), INTEGRATE_SENTINEL);
        assert_eq!(INTEGRATE_SENTINEL, "FANOUTCODE: integrate done");
    }

    #[test]
    fn export_names_reads_ts_and_rust_exports() {
        let ts = "import x from 'y';\n\
export function startSession() {}\n\
export default function LoginForm() {}\n\
export async function refresh() {}\n\
export const TOKEN_KEY = 'k';\n\
export let counter = 0;\n\
export var legacy = 1;\n\
export class Session {}\n\
export type Role = 'a';\n\
export interface User { id: string }\n\
export enum Kind { A }\n\
export default class Store<T> {}\n\
export { a, b };\n\
export default function () {}\n\
const hidden = 1;\n\
export function startSession() {}\n";
        assert_eq!(
            export_names(ts),
            s(&[
                "startSession",
                "LoginForm",
                "refresh",
                "TOKEN_KEY",
                "counter",
                "legacy",
                "Session",
                "Role",
                "User",
                "Kind",
                "Store",
            ])
        );
        let rs = "use std::fs;\n\
pub fn build_claims(a: u8) {}\n\
pub struct Claim { x: u8 }\n\
pub enum Grain<T> { A(T) }\n\
pub trait Store {}\n\
fn private() {}\n\
    pub fn method(&self) {}\n\
pub(crate) fn internal() {}\n";
        assert_eq!(
            export_names(rs),
            s(&["build_claims", "Claim", "Grain", "Store", "method"])
        );
        assert!(export_names("").is_empty());
    }
}
