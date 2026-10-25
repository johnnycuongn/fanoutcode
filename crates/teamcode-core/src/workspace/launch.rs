//! What a task's session launch and its prompts take from its workspace: the first-start binding, the protected
//! roots with their deny rules and guard hook, extra `--add-dir` folders, workspace-level agents, and the prompt
//! context (policy, protection, briefs, references, stage instructions). The shell calls these with the database
//! lock held; nothing here locks.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::briefs::cached_brief;
use super::context_docs::{context_dir, list_rows, ContextDocKind};
use super::fragments::{reference_line, LinkContext, LinkedProject};
use super::guard::prepare_guard;
use super::link::{bind_links, link_status, links_for, load_links, same_root, save_links, LinkStatus};
use super::policy::{deny_rules, protected_roots};
use super::rows;
use super::store::member_projects;
use crate::db;
use crate::domain::{AgentKind, Container, Task, TaskId, Workspace};
use crate::hooks::HookSpec;
use crate::memory::store::Loaded;
use crate::memory::TaskFolder;
use crate::prompts::PromptCtx;
use crate::roster::{parse_agent, AgentDef, RosterLevel};
use crate::Result;

/// A linked project's brief: Plan 05's LiveSpec brief store (what the brief helper writes) first, then this
/// module's per-project cache.
fn brief_of(app_data: &Path, project: Option<&crate::domain::ProjectId>, root: &Path) -> Option<String> {
    let store = livespec_core::project::BriefStore::new(app_data.join("briefs"));
    crate::helpers::briefs::cached(&store, root).or_else(|| project.and_then(|p| cached_brief(app_data, p)))
}

/// The task's workspace record, when it has one that still exists.
fn workspace_of(conn: &Connection, task: &Task) -> Result<Option<Workspace>> {
    match &task.workspace_id {
        Some(id) if rows::exists(conn, id)? => rows::get(conn, id).map(Some),
        _ => Ok(None),
    }
}

/// Binds the task's links at its first session start. `issuing` is the window's container when the caller knows
/// it; otherwise the container the task was created in (its `workspace_id`, else its project). Returns the task
/// and whether a linked root still lacks a cached brief (the shell then runs the brief helper).
pub fn bind_if_first_start(
    conn: &Connection,
    app_data: &Path,
    task_id: &TaskId,
    issuing: Option<Container>,
) -> Result<(Task, bool)> {
    let mut task = db::tasks::require(conn, task_id)?;
    let issuing = issuing.unwrap_or_else(|| match &task.workspace_id {
        Some(id) => Container::Workspace { id: id.clone() },
        None => Container::Project {
            id: task.project_id.clone(),
        },
    });
    let ws = match &issuing {
        Container::Workspace { id } => Some(rows::get(conn, id)?),
        Container::Project { .. } => None,
    };
    let projects = match &ws {
        Some(w) => member_projects(conn, w)?,
        None => Vec::new(),
    };
    let Some(binding) = bind_links(&task, &issuing, ws.as_ref(), &projects)? else {
        return Ok((task, false));
    };
    let workspace_changed = task.workspace_id != binding.workspace_id;
    task.workspace_id = binding.workspace_id;
    task.linked_roots = binding.linked_roots;
    task.links_bound_at = Some(chrono::Utc::now());
    task.updated_at = chrono::Utc::now();
    db::tasks::update(conn, &task)?;
    save_links(conn, &task.id, &binding.links)?;
    if workspace_changed {
        let folder = TaskFolder::open(task.memory_dir.clone());
        if let Ok(Loaded::Parsed(mut doc)) = folder.read_index() {
            doc.front.workspace = task.workspace_id.clone();
            folder.write_index(&doc)?;
        }
    }
    let needs_briefs = binding
        .links
        .iter()
        .any(|l| brief_of(app_data, l.project_id.as_ref(), &l.root).is_none());
    Ok((task, needs_briefs))
}

/// The task's protected roots now. A task whose workspace record is gone keeps every linked root protected
/// (fail closed).
pub fn protected_for(conn: &Connection, task: &Task) -> Result<Vec<PathBuf>> {
    if task.linked_roots.is_empty() {
        return Ok(Vec::new());
    }
    let links = links_for(&task.linked_roots, &load_links(conn, &task.id)?);
    match workspace_of(conn, task)? {
        Some(ws) => Ok(protected_roots(&task.project_id, &ws, &links)),
        None => Ok(task.linked_roots.clone()),
    }
}

/// The workspace's own agent definitions, for the roster's workspace level. One that does not parse is skipped.
pub fn workspace_agents(conn: &Connection, task: &Task) -> Result<Vec<AgentDef>> {
    let Some(ws) = workspace_of(conn, task)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, text) in rows::map_column(conn, &ws.id, "agents_json")? {
        match parse_agent(&name, &text, RosterLevel::Workspace) {
            Ok(def) => out.push(def),
            Err(e) => eprintln!("Team Code: skipping workspace agent {name}: {e}"),
        }
    }
    Ok(out)
}

/// What a Claude Code launch adds for the task's workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchExtras {
    /// `permissions.deny` rules for the protected roots.
    pub deny: Vec<String>,
    /// The Bash guard hook, when anything is protected.
    pub guard: Option<HookSpec>,
    /// Beyond the linked roots (which the launch adds itself): the context-doc cache when a reference is cached.
    pub add_dirs: Vec<PathBuf>,
}

/// Refuses the launch (`Precondition`) when a protected root cannot be expressed as a safe deny rule.
pub fn launch_extras(conn: &Connection, task: &Task, app_data: &Path, exe: &Path, home: &Path) -> Result<LaunchExtras> {
    if task.agent_kind != AgentKind::ClaudeCode {
        return Ok(LaunchExtras::default());
    }
    let protected = protected_for(conn, task)?;
    let deny = deny_rules(&protected)?;
    let guard = prepare_guard(app_data, exe, &task.id, &protected, home)?;
    let mut containers = vec![Container::Project {
        id: task.project_id.clone(),
    }];
    if let Some(id) = &task.workspace_id {
        containers.push(Container::Workspace { id: id.clone() });
    }
    let mut cached = false;
    for c in &containers {
        cached |= list_rows(conn, c)?
            .iter()
            .any(|d| matches!(d.kind, ContextDocKind::Url | ContextDocKind::Confluence) && d.fetched_at.is_some());
    }
    let add_dirs = if cached {
        vec![context_dir(app_data)]
    } else {
        Vec::new()
    };
    Ok(LaunchExtras { deny, guard, add_dirs })
}

/// The workspace's part of a task's prompts, for `{{linked.projects}}` and the references.
pub fn link_context(conn: &Connection, task: &Task, app_data: &Path) -> Result<Option<LinkContext>> {
    let Some(ws) = workspace_of(conn, task)? else {
        return Ok(None);
    };
    let protected = protected_for(conn, task)?;
    let mut linked = Vec::new();
    for root in &task.linked_roots {
        let project = db::projects::find_by_root(conn, root)?;
        linked.push(LinkedProject {
            name: project.as_ref().map_or_else(
                || {
                    root.file_name()
                        .map_or_else(|| root.display().to_string(), |n| n.to_string_lossy().into_owned())
                },
                |p| p.name.clone(),
            ),
            root: root.clone(),
            brief: brief_of(app_data, project.as_ref().map(|p| &p.id), root),
            protected: protected.iter().any(|p| same_root(p, root)),
        });
    }
    Ok(Some(LinkContext {
        policy: ws.policy,
        linked,
        workspace_docs: list_rows(conn, &Container::Workspace { id: ws.id.clone() })?,
        workspace_instructions: None,
    }))
}

/// Fills what Plan 03's prompt context leaves to the workspace: the policy, each linked project's protection and
/// (when Plan 05's brief store has none) the cached brief, the Project and Workspace references, and the stage
/// instructions from the workspace file.
pub fn fill_prompt_ctx(conn: &Connection, task: &Task, ctx: &mut PromptCtx, app_data: &Path) -> Result<()> {
    ctx.project_context = list_rows(
        conn,
        &Container::Project {
            id: task.project_id.clone(),
        },
    )?
    .iter()
    .map(|d| reference_line(d, app_data))
    .collect();
    let Some(link) = link_context(conn, task, app_data)? else {
        return Ok(());
    };
    ctx.link_policy = link.policy;
    for l in &mut ctx.linked {
        if let Some(mine) = link.linked.iter().find(|m| same_root(&m.root, &l.root)) {
            l.protected = mine.protected;
            if l.brief.is_none() {
                l.brief = mine.brief.clone();
            }
        }
    }
    ctx.workspace_context = link
        .workspace_docs
        .iter()
        .map(|d| reference_line(d, app_data))
        .collect();
    if let Some(id) = &task.workspace_id {
        ctx.workspace_fragments = rows::map_column(conn, id, "fragments_json")?;
    }
    Ok(())
}

/// `link::link_status` for a task against its workspace as stored now.
pub fn status_of(conn: &Connection, task: &Task) -> Result<LinkStatus> {
    let ws = workspace_of(conn, task)?;
    let projects = match &ws {
        Some(w) => member_projects(conn, w)?,
        None => Vec::new(),
    };
    Ok(link_status(task, ws.as_ref(), &projects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LinkPolicy, Stage};
    use crate::pipeline::ctx::{build_prompt_ctx, build_stage_prompts};
    use crate::workspace::store::WorkspaceStore;
    use crate::workspace::test_support::{fixture, Fixture};

    /// A workspace of api and web, and a Claude Code task in web created there.
    fn setup() -> (Fixture, WorkspaceStore, Workspace, Task) {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        let ws = s.create("Shop", &[fx.api.id.clone(), fx.web.id.clone()]).unwrap();
        let conn = fx.db.lock().unwrap();
        let mut t = crate::test_support::task_fixture("t-1", fx.web.id.as_str());
        t.workspace_id = Some(ws.id.clone());
        db::tasks::insert(&conn, &t).unwrap();
        drop(conn);
        (fx, s, ws, t)
    }

    #[test]
    fn first_start_binds_once_and_records_links() {
        let (fx, _s, _ws, t) = setup();
        let conn = fx.db.lock().unwrap();
        let (bound, needs_briefs) = bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
        assert_eq!(bound.linked_roots, vec![fx.api.root.clone()]);
        assert!(bound.links_bound_at.is_some() && needs_briefs);
        assert_eq!(load_links(&conn, &t.id).unwrap()[0].project_id, Some(fx.api.id.clone()));
        let (again, _) = bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
        assert_eq!(again.links_bound_at, bound.links_bound_at);
    }

    #[test]
    fn protection_deny_rules_and_guard_follow_the_policy() {
        let (fx, s, ws, t) = setup();
        bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
        let exe = Path::new("/x/teamcode");
        let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap();
        let none = launch_extras(&fx.db.lock().unwrap(), &task, fx.app_data.path(), exe, Path::new("/h")).unwrap();
        assert!(none.deny.is_empty() && none.guard.is_none());
        s.set_policy(&ws.id, LinkPolicy::ReadOnly, &[]).unwrap();
        let ro = launch_extras(&fx.db.lock().unwrap(), &task, fx.app_data.path(), exe, Path::new("/h")).unwrap();
        assert!(ro.deny.contains(&format!("Edit(/{}/**)", fx.api.root.display())));
        assert!(ro.guard.is_some());
    }

    #[test]
    fn a_task_whose_workspace_is_gone_keeps_its_links_protected() {
        let (fx, _s, ws, t) = setup();
        let conn = fx.db.lock().unwrap();
        bind_if_first_start(&conn, fx.app_data.path(), &t.id, None).unwrap();
        // a record removed out from under the task (the foreign key would normally keep it)
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", [ws.id.as_str()])
            .unwrap();
        let task = db::tasks::require(&conn, &t.id).unwrap();
        assert_eq!(protected_for(&conn, &task).unwrap(), vec![fx.api.root.clone()]);
    }

    #[test]
    fn prompts_carry_policy_protection_references_and_stage_instructions() {
        let (fx, s, ws, t) = setup();
        bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
        s.set_policy(
            &ws.id,
            LinkPolicy::ReadAndModifyOnRequest,
            std::slice::from_ref(&fx.api.id),
        )
        .unwrap();
        s.set_fragment(&ws.id, "plan", Some("Run both suites.")).unwrap();
        let docs = crate::workspace::context_docs::ContextDocs::new(fx.db.clone());
        docs.add(
            &Container::Workspace { id: ws.id.clone() },
            ContextDocKind::Url,
            "https://x.y/spec",
            None,
        )
        .unwrap();
        let conn = fx.db.lock().unwrap();
        let task = db::tasks::require(&conn, &t.id).unwrap();
        let mut ctx = build_prompt_ctx(&conn, &task, &fx.web, fx.app_data.path(), fx.app_data.path()).unwrap();
        assert!(ctx.linked[0].protected);
        let prompts = build_stage_prompts(&mut ctx, fx.app_data.path(), fx.app_data.path()).unwrap();
        assert!(prompts.plan.contains("Run both suites.") && !prompts.build.contains("Run both suites."));
        assert!(prompts.plan.contains("url: https://x.y/spec — not fetched yet"));
        assert!(prompts.plan.contains("(protected: read only)"));
        let _ = Stage::Plan;
    }
}
