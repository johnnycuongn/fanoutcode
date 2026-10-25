//! Stage prompts (§6.8): the shipped templates and their layered assembly.

pub mod templates;

pub use templates::default_template;

use std::path::{Path, PathBuf};

use crate::domain::{LinkPolicy, Stage};
use crate::memory::store::{self, Loaded};
use crate::pipeline::TestRun;
use crate::Result;

/// Another project in the task's bound set (Plan 10 fills briefs and protection).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkedProject {
    pub name: String,
    pub root: PathBuf,
    pub brief: Option<String>,
    pub protected: bool,
}

/// Everything a stage prompt is assembled from.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptCtx {
    pub task_folder: PathBuf,
    pub project_root: PathBuf,
    pub worktree: Option<PathBuf>,
    /// Paths and URLs listed under "Project references" (Plan 10 fills them); empty renders `(none)`.
    pub project_context: Vec<String>,
    /// "Workspace references".
    pub workspace_context: Vec<String>,
    /// Every other project in the task's bound set (Plan 10); empty for a single-project task.
    pub linked: Vec<LinkedProject>,
    /// From `Workspace.policy`, its sole source (A12); `ReadAndModifyOnRequest` outside a workspace.
    pub link_policy: LinkPolicy,
    pub test_command: Option<String>,
    pub last_test: Option<TestRun>,
    /// `<app_config>/prompts/<stage>.md`, if any.
    pub app_override: Option<String>,
    /// `<project memory root>/prompts/<stage>.md`, if any.
    pub project_override: Option<String>,
    /// Instructions the workspace adds for the stage being assembled (workspace file, Plan 10).
    pub workspace_fragment: Option<String>,
    /// The workspace's instructions per stage word (`"plan"` …); `build_stage_prompts` picks the stage's one.
    pub workspace_fragments: std::collections::BTreeMap<String, String>,
    /// For example a reviewer-note reply prefixed with `file:line` (Plan 08).
    pub task_additions: Vec<String>,
}

const LINKED_INTRO: &str =
    "This task runs in a workspace. You may read these projects freely for reference. Modify one \
only when the task's intent or plan calls for it, and list every such edit in `plan.md` under that project's heading.";

const NO_LINKED: &str = "(This task has no linked projects.)";

/// The sentinel a stage's prompt must end with; the fix-tests prompt returns the card to Build.
fn prompt_sentinel(stage: Stage) -> String {
    match stage {
        Stage::Test => Stage::Build.sentinel(),
        s => s.sentinel(),
    }
}

/// The `## Linked projects` block; under `ReadOnly` every project is marked protected.
pub fn linked_projects_block(linked: &[LinkedProject], policy: LinkPolicy) -> String {
    let mut out = format!("## Linked projects\n{LINKED_INTRO}\n");
    for p in linked {
        out.push_str(&format!("- {} — `{}`", p.name, p.root.display()));
        if p.protected || policy == LinkPolicy::ReadOnly {
            out.push_str(" (protected: read only)");
        }
        out.push('\n');
        if let Some(brief) = &p.brief {
            for line in brief.lines() {
                out.push_str(&format!("  {line}\n"));
            }
        }
    }
    out.trim_end().to_owned()
}

/// What `{{linked.projects}}` becomes: the block (or the no-links line), then the workspace references.
fn linked_section(ctx: &PromptCtx) -> String {
    let mut out = if ctx.linked.is_empty() {
        NO_LINKED.to_owned()
    } else {
        linked_projects_block(&ctx.linked, ctx.link_policy)
    };
    if !ctx.workspace_context.is_empty() {
        out.push_str("\nWorkspace references:\n");
        out.push_str(&bullets(&ctx.workspace_context));
    }
    out
}

fn bullets(items: &[String]) -> String {
    items.iter().map(|i| format!("- {i}")).collect::<Vec<_>>().join("\n")
}

fn placeholder(key: &str, ctx: &PromptCtx) -> Option<String> {
    Some(match key {
        "task.folder" => ctx.task_folder.display().to_string(),
        "project.root" => ctx.project_root.display().to_string(),
        "worktree" => match &ctx.worktree {
            Some(w) => format!("`{}` (the task's worktree)", w.display()),
            None => format!(
                "`{}` (no worktree; work in the main checkout)",
                ctx.project_root.display()
            ),
        },
        "project.context" if ctx.project_context.is_empty() => "(none)".to_owned(),
        "project.context" => bullets(&ctx.project_context),
        "linked.projects" => linked_section(ctx),
        "test.command" => ctx
            .test_command
            .clone()
            .unwrap_or_else(|| "(none configured)".to_owned()),
        "test.exit_code" => ctx
            .last_test
            .as_ref()
            .map_or("(unknown)".to_owned(), |r| r.exit_code.to_string()),
        "test.output" => ctx
            .last_test
            .as_ref()
            .map_or("(no output)".to_owned(), |r| r.tail.clone()),
        _ => return None,
    })
}

/// Placeholder substitution only, in one pass, so a substituted value (test output, a brief) is never re-read for
/// placeholders. Unknown `{{…}}` tokens are left untouched.
pub fn render(template: &str, ctx: &PromptCtx) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after
            .find("}}")
            .and_then(|end| placeholder(&after[..end], ctx).map(|v| (end, v)))
        {
            Some((end, value)) => {
                out.push_str(&value);
                rest = &after[end + 2..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The stage prompt: base (project override, else app override, else the shipped template), rendered; the linked
/// projects block when the base lacks its placeholder; the workspace fragment; the task additions; and the sentinel
/// instruction when the text does not already end with it. Done has no prompt. Ends with exactly one newline.
pub fn assemble(stage: Stage, ctx: &PromptCtx) -> String {
    if !stage.has_agent_prompt() {
        return String::new();
    }
    let base = match (
        &ctx.project_override,
        &ctx.app_override,
        templates::default_template(stage),
    ) {
        (Some(p), _, _) => p.as_str(),
        (None, Some(a), _) => a.as_str(),
        (None, None, Some(d)) => d,
        (None, None, None) => return String::new(),
    };
    let mut out = render(base, ctx).trim_end().to_owned();
    if !base.contains("{{linked.projects}}") && !ctx.linked.is_empty() {
        out.push_str("\n\n");
        out.push_str(&linked_section(ctx));
    }
    let extras = ctx.workspace_fragment.iter().chain(&ctx.task_additions);
    for extra in extras.map(|e| e.trim()).filter(|e| !e.is_empty()) {
        out.push_str("\n\n");
        out.push_str(extra);
    }
    let sentinel = prompt_sentinel(stage);
    if out.lines().rev().map(str::trim).find(|l| !l.is_empty()) != Some(sentinel.as_str()) {
        out.push_str("\n\nWhen you are finished, end your final message with this single line and nothing after it:\n");
        out.push_str(&sentinel);
    }
    out.push('\n');
    out
}

/// `<dir>/<stage>.md`: `None` when absent; a file that is not UTF-8 is set aside and reads as `None`.
pub fn load_override(dir: &Path, stage: Stage) -> Result<Option<String>> {
    Ok(match store::load_text(&dir.join(format!("{}.md", stage.as_str())))? {
        Loaded::Parsed(text) => Some(text),
        Loaded::Missing | Loaded::Corrupt { .. } => None,
    })
}

/// `<app_config>/prompts`.
pub fn prompts_dir_app(app_config: &Path) -> PathBuf {
    app_config.join("prompts")
}

/// `<memory_root>/prompts`.
pub fn prompts_dir_project(memory_root: &Path) -> PathBuf {
    memory_root.join("prompts")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> PromptCtx {
        PromptCtx {
            task_folder: "/m/2026-10-24-t".into(),
            project_root: "/r".into(),
            worktree: None,
            project_context: vec![],
            workspace_context: vec![],
            linked: vec![],
            link_policy: LinkPolicy::ReadAndModifyOnRequest,
            test_command: Some("npm test".into()),
            last_test: None,
            app_override: None,
            project_override: None,
            workspace_fragment: None,
            workspace_fragments: Default::default(),
            task_additions: vec![],
        }
    }

    #[test]
    fn render_substitutes_every_placeholder_and_leaves_unknown_ones() {
        let out = render(
            "{{task.folder}}|{{project.root}}|{{worktree}}|{{project.context}}|{{test.command}}|{{nope}}",
            &ctx(),
        );
        assert_eq!(
            out,
            "/m/2026-10-24-t|/r|`/r` (no worktree; work in the main checkout)|(none)|npm test|{{nope}}"
        );
        let mut c = ctx();
        c.worktree = Some("/r/.worktrees/t".into());
        c.test_command = None;
        c.project_context = vec!["docs/ARCHITECTURE.md".into()];
        assert_eq!(
            render("{{worktree}}|{{test.command}}|{{project.context}}", &c),
            "`/r/.worktrees/t` (the task's worktree)|(none configured)|- docs/ARCHITECTURE.md"
        );
    }

    #[test]
    fn default_base_is_used_when_no_override_exists_and_ends_with_the_sentinel() {
        let out = assemble(Stage::Plan, &ctx());
        assert!(out.contains("/m/2026-10-24-t/intent.md"));
        assert_eq!(out.trim_end().lines().last().unwrap(), "TEAMCODE: plan done");
        assert!(out.ends_with("done\n") && !out.ends_with("\n\n"));
        assert_eq!(assemble(Stage::Done, &ctx()), "");
    }

    #[test]
    fn project_override_wins_over_app_override_and_still_gets_linked_rules_and_sentinel() {
        let mut c = ctx();
        c.app_override = Some("APP BASE".into());
        c.project_override = Some("PROJECT BASE for {{project.root}}".into());
        c.linked = vec![LinkedProject {
            name: "api".into(),
            root: "/api".into(),
            brief: Some("The API.".into()),
            protected: false,
        }];
        let out = assemble(Stage::Build, &c);
        assert!(out.starts_with("PROJECT BASE for /r\n\n## Linked projects\n"));
        assert!(!out.contains("APP BASE"));
        assert!(out.contains("- api — `/api`\n  The API.\n"));
        assert!(out
            .trim_end()
            .ends_with("end your final message with this single line and nothing after it:\nTEAMCODE: build done"));
    }

    #[test]
    fn linked_block_is_not_duplicated_when_the_base_already_has_the_placeholder() {
        let mut c = ctx();
        c.linked = vec![LinkedProject {
            name: "api".into(),
            root: "/api".into(),
            brief: None,
            protected: true,
        }];
        let out = assemble(Stage::Plan, &c);
        assert_eq!(out.matches("## Linked projects").count(), 1);
        assert!(out.contains("- api — `/api` (protected: read only)"));
        assert!(!out.contains("(This task has no linked projects.)"));
    }

    #[test]
    fn read_only_policy_marks_every_linked_project() {
        let block = linked_projects_block(
            &[LinkedProject {
                name: "a".into(),
                root: "/a".into(),
                brief: None,
                protected: false,
            }],
            LinkPolicy::ReadOnly,
        );
        assert!(block.contains("- a — `/a` (protected: read only)"));
    }

    #[test]
    fn workspace_fragment_and_task_additions_come_last_in_order() {
        let mut c = ctx();
        c.workspace_fragment = Some("WS RULES".into());
        c.task_additions = vec!["src/a.ts:12 — reviewer note: rename this".into(), "SECOND".into()];
        let out = assemble(Stage::Review, &c);
        let i = |s: &str| out.find(s).unwrap_or_else(|| panic!("{s} missing"));
        assert!(
            i("TEAMCODE: review done") < i("WS RULES")
                && i("WS RULES") < i("src/a.ts:12")
                && i("src/a.ts:12") < i("SECOND")
        );
        assert_eq!(out.trim_end().lines().last().unwrap(), "TEAMCODE: review done");
    }

    #[test]
    fn fix_tests_prompt_carries_the_run() {
        let mut c = ctx();
        c.last_test = Some(TestRun {
            command: "npm test".into(),
            exit_code: 1,
            tail: "FAIL src/a.test.ts".into(),
        });
        let out = assemble(Stage::Test, &c);
        assert!(out.contains("Exit code: 1") && out.contains("FAIL src/a.test.ts"));
        assert_eq!(out.trim_end().lines().last().unwrap(), "TEAMCODE: build done");
    }

    #[test]
    fn load_override_reads_stage_files_and_tolerates_absence() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_override(dir.path(), Stage::Plan).unwrap(), None);
        std::fs::write(dir.path().join("plan.md"), "MY PLAN").unwrap();
        assert_eq!(
            load_override(dir.path(), Stage::Plan).unwrap().as_deref(),
            Some("MY PLAN")
        );
        assert_eq!(prompts_dir_app(Path::new("/cfg")), PathBuf::from("/cfg/prompts"));
        assert_eq!(prompts_dir_project(Path::new("/mem")), PathBuf::from("/mem/prompts"));
    }
}
