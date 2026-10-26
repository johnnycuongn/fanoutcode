//! Builders for what `transition` and `prompts::assemble` read: settings at the task's scope, the stage prompts
//! with their overrides, and the linked projects. The shell calls these; Plan 10 fills the workspace parts.

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::{StagePrompts, TestRun, TransitionCtx};
use crate::db;
use crate::domain::{LinkPolicy, Project, Stage, Task};
use crate::memory::root_for;
use crate::prompts::{self, LinkedProject, PromptCtx};
use crate::settings::{keys, resolve, Scope, ScopeValues};
use crate::Result;

/// The task's settings scope: its workspace, its project and itself.
pub fn task_scope(task: &Task) -> Scope {
    Scope {
        workspace: task.workspace_id.clone(),
        project: Some(task.project_id.clone()),
        task: Some(task.id.clone()),
    }
}

fn values(conn: &Connection, task: &Task) -> Result<(Scope, ScopeValues)> {
    let scope = task_scope(task);
    let values = db::settings::load(conn, &scope)?;
    Ok((scope, values))
}

/// The prompt context of a task. Overrides are left `None` (`build_stage_prompts` sets them per stage); linked
/// projects come from `linked_roots`, named from the projects table or else the folder; the workspace parts are
/// filled by `workspace::launch::fill_prompt_ctx`.
pub fn build_prompt_ctx(
    conn: &Connection,
    task: &Task,
    project: &Project,
    _app_config: &Path,
    app_data: &Path,
) -> Result<PromptCtx> {
    let (scope, values) = values(conn, task)?;
    // the lazily built LiveSpec briefs (Plan 05); a project without one yet gets none
    let briefs = livespec_core::project::BriefStore::new(app_data.join("briefs"));
    let mut linked = Vec::new();
    for root in &task.linked_roots {
        let name = match db::projects::find_by_root(conn, root)? {
            Some(p) => p.name,
            None => root
                .file_name()
                .map_or_else(|| root.display().to_string(), |n| n.to_string_lossy().into_owned()),
        };
        linked.push(LinkedProject {
            name,
            root: root.clone(),
            brief: crate::helpers::briefs::cached(&briefs, root),
            protected: false,
        });
    }
    let mut ctx = PromptCtx {
        task_folder: task.memory_dir.clone(),
        project_root: project.root.clone(),
        worktree: task.worktree_path.clone(),
        project_context: Vec::new(),
        workspace_context: Vec::new(),
        linked,
        link_policy: LinkPolicy::ReadAndModifyOnRequest,
        test_command: resolve::<keys::TestCommand>(&values, &scope),
        last_test: None,
        app_override: None,
        project_override: None,
        workspace_fragment: None,
        workspace_fragments: Default::default(),
        task_additions: Vec::new(),
        others_working_here: String::new(),
    };
    // the workspace's policy, protection, references and stage instructions (Plan 10)
    crate::workspace::launch::fill_prompt_ctx(conn, task, &mut ctx, app_data)?;
    Ok(ctx)
}

/// Assembles the Spec, Plan, Build, Review and fix-tests prompts, each with its app and project overrides
/// (`<app_config>/prompts/<stage>.md`, `<memory_root>/prompts/<stage>.md`); `fix_tests` only with a last run.
pub fn build_stage_prompts(ctx: &mut PromptCtx, app_config: &Path, memory_root: &Path) -> Result<StagePrompts> {
    let app_dir = prompts::prompts_dir_app(app_config);
    let project_dir = prompts::prompts_dir_project(memory_root);
    let has_run = ctx.last_test.is_some();
    let mut one = |stage: Stage| -> Result<String> {
        ctx.workspace_fragment = ctx.workspace_fragments.get(stage.as_str()).cloned();
        ctx.app_override = prompts::load_override(&app_dir, stage)?;
        ctx.project_override = prompts::load_override(&project_dir, stage)?;
        // "Others working here" belongs to planning and building (§6.1), not to Spec or Review
        let kept = ctx.task_additions.len();
        if matches!(stage, Stage::Plan | Stage::Build) && !ctx.others_working_here.trim().is_empty() {
            ctx.task_additions.push(ctx.others_working_here.clone());
        }
        let prompt = prompts::assemble(stage, ctx);
        ctx.task_additions.truncate(kept);
        Ok(prompt)
    };
    let spec = one(Stage::Spec)?;
    let plan = one(Stage::Plan)?;
    let build = one(Stage::Build)?;
    let review = one(Stage::Review)?;
    let fix_tests = if has_run { Some(one(Stage::Test)?) } else { None };
    Ok(StagePrompts {
        spec,
        plan,
        build,
        review,
        fix_tests,
    })
}

/// Everything `transition` reads for this task now: the test and commit settings at the task's scope, the last
/// test run (Plan 05's), and the assembled prompts, the Plan and Build ones with `others_working_here` (Plan 11).
#[allow(clippy::too_many_arguments)]
pub fn build_transition_ctx(
    conn: &Connection,
    task: &Task,
    project: &Project,
    app_config: &Path,
    app_data: &Path,
    now: DateTime<Utc>,
    last_test: Option<TestRun>,
    others_working_here: &str,
) -> Result<TransitionCtx> {
    let (scope, values) = values(conn, task)?;
    let mut prompt_ctx = build_prompt_ctx(conn, task, project, app_config, app_data)?;
    prompt_ctx.last_test = last_test.clone();
    prompt_ctx.others_working_here = others_working_here.to_owned();
    let prompts = build_stage_prompts(&mut prompt_ctx, app_config, &root_for(project, app_data))?;
    Ok(TransitionCtx {
        now,
        test_command: prompt_ctx.test_command,
        test_enabled: resolve::<keys::TestEnabled>(&values, &scope),
        advance_on_green: resolve::<keys::AdvanceOnGreen>(&values, &scope),
        may_commit: resolve::<keys::MayCommit>(&values, &scope),
        last_test,
        prompts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory;
    use crate::settings::ScopeKind;
    use crate::test_support::{insert_project, insert_project_at, insert_task, test_conn};

    #[test]
    fn transition_ctx_reads_project_and_task_settings() {
        let c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let t = insert_task(&c, "t-1a-1", "p-1a-1");
        db::settings::set(
            &c,
            ScopeKind::Project,
            "p-1a-1",
            "test_command",
            &serde_json::json!("npm test"),
        )
        .unwrap();
        db::settings::set(&c, ScopeKind::Project, "p-1a-1", "may_commit", &serde_json::json!(true)).unwrap();
        db::settings::set(&c, ScopeKind::Task, "t-1a-1", "test_enabled", &serde_json::json!(false)).unwrap();
        let ctx = build_transition_ctx(&c, &t, &p, cfg.path(), data.path(), Utc::now(), None, "").unwrap();
        assert_eq!(
            (
                ctx.test_command.as_deref(),
                ctx.test_enabled,
                ctx.may_commit,
                ctx.advance_on_green
            ),
            (Some("npm test"), false, true, false)
        );
        assert!(ctx.prompts.plan.contains(&t.memory_dir.display().to_string()));
        assert!(ctx.prompts.fix_tests.is_none());
    }

    #[test]
    fn project_prompt_override_is_used_and_app_override_is_the_fallback() {
        let c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let t = insert_task(&c, "t-1a-1", "p-1a-1");
        let root = memory::root_for(&p, data.path());
        std::fs::create_dir_all(root.join("prompts")).unwrap();
        std::fs::write(root.join("prompts/plan.md"), "PROJECT PLAN {{project.root}}").unwrap();
        std::fs::create_dir_all(cfg.path().join("prompts")).unwrap();
        std::fs::write(cfg.path().join("prompts/build.md"), "APP BUILD").unwrap();
        let ctx = build_transition_ctx(&c, &t, &p, cfg.path(), data.path(), Utc::now(), None, "").unwrap();
        assert!(ctx
            .prompts
            .plan
            .starts_with(&format!("PROJECT PLAN {}", p.root.display())));
        assert!(ctx.prompts.build.starts_with("APP BUILD"));
        assert!(
            ctx.prompts.review.contains("review.md"),
            "shipped default still used where no override exists"
        );
    }

    #[test]
    fn linked_roots_become_linked_projects_named_from_the_projects_table() {
        let c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let other = insert_project_at(&c, "p-1a-2", "/srv/api-service");
        let mut t = insert_task(&c, "t-1a-1", "p-1a-1");
        t.linked_roots = vec![other.root.clone(), "/tmp/unknown-repo".into()];
        db::tasks::update(&c, &t).unwrap();
        let ctx = build_prompt_ctx(&c, &t, &p, cfg.path(), data.path()).unwrap();
        assert_eq!(
            ctx.linked.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            [other.name.as_str(), "unknown-repo"]
        );
        let mut ctx2 = ctx;
        let prompts = build_stage_prompts(&mut ctx2, cfg.path(), &memory::root_for(&p, data.path())).unwrap();
        assert!(prompts.plan.contains("## Linked projects") && prompts.plan.contains("/srv/api-service"));
    }

    #[test]
    fn others_working_here_reaches_only_the_plan_and_build_prompts() {
        let c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let t = insert_task(&c, "t-1a-1", "p-1a-1");
        let others = "## Others working here\n\n### Login";
        let ctx = build_transition_ctx(&c, &t, &p, cfg.path(), data.path(), Utc::now(), None, others).unwrap();
        for prompt in [&ctx.prompts.plan, &ctx.prompts.build] {
            assert!(prompt.contains("### Login"), "{prompt}");
            assert!(
                prompt.trim_end().lines().last().unwrap().starts_with("FANOUTCODE:"),
                "the sentinel stays last"
            );
        }
        for prompt in [&ctx.prompts.spec, &ctx.prompts.review] {
            assert!(!prompt.contains("Others working here"), "{prompt}");
        }
    }

    #[test]
    fn fix_tests_prompt_exists_only_with_a_last_run() {
        let c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let t = insert_task(&c, "t-1a-1", "p-1a-1");
        let run = TestRun {
            command: "npm test".into(),
            exit_code: 2,
            tail: "boom".into(),
        };
        let ctx = build_transition_ctx(&c, &t, &p, cfg.path(), data.path(), Utc::now(), Some(run), "").unwrap();
        assert!(ctx.prompts.fix_tests.as_deref().unwrap().contains("Exit code: 2"));
    }
}
