use super::*;
use crate::domain::HandbackReason;
use crate::overlap::CommandKind;

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
fn go_ahead_command_names_the_command_and_the_task() {
    assert_eq!(
        go_ahead_command("Login", "git add -A"),
        "Fanout Code: the developer says go ahead with `git add -A`, although it touches files Login has \
uncommitted changes in (the developer approved it). Run it now."
    );
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

#[test]
fn told_command_says_what_happened_to_the_other_sessions_files() {
    let p = s(&["src/a.ts", "src/b.ts"]);
    assert_eq!(
        told_command("Checkout", "git commit -am wip", CommandKind::Commits, &p),
        "Fanout Code: task Checkout ran `git commit -am wip` with the developer's approval; your uncommitted changes \
in src/a.ts, src/b.ts were committed into its commit. Re-read those files before your next edit."
    );
    assert_eq!(
        told_command("Checkout", "git reset --hard", CommandKind::Discards, &p[..1]),
        "Fanout Code: task Checkout ran `git reset --hard` with the developer's approval; your uncommitted changes \
in src/a.ts were discarded. Re-read those files before your next edit."
    );
    assert_eq!(
        told_command("Checkout", "git stash -u", CommandKind::Discards, &p[..1]),
        "Fanout Code: task Checkout ran `git stash -u` with the developer's approval; your uncommitted changes in \
src/a.ts were stashed. Re-read those files before your next edit."
    );
}

#[test]
fn command_free_names_the_command_and_the_tasks() {
    assert_eq!(
        command_free(&s(&["Login"]), "git stash"),
        "Fanout Code: Login's files are now committed; you may run `git stash` again."
    );
    assert_eq!(
        command_free(&s(&["Login", "Billing"]), "git add -A"),
        "Fanout Code: Login's and Billing's files are now committed; you may run `git add -A` again."
    );
}
