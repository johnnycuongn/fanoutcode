//! What a task's session launch and its prompts take from its workspace: the first-start binding, the protected
//! roots with their deny rules and guard hook, extra `--add-dir` folders, workspace-level agents, and the prompt
//! context (policy, protection, briefs, references, stage instructions). The shell calls these with the database
//! lock held; nothing here locks.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::briefs::cached_brief;
use super::context_docs::{context_dir, list_rows};
use super::fragments::{reference_lines, LinkContext, LinkedProject};
use super::guard::prepare_guard;
use super::link::{bind_links, link_status, links_for, load_links, same_root, save_links, LinkStatus};
use super::policy::{deny_rules, protected_roots};
use super::rows;
use super::store::member_projects;
use crate::changes::repos::repos_of;
use crate::db;
use crate::domain::{AgentKind, Container, Task, TaskId, Workspace};
use crate::hooks::{event_log_path, HookSpec};
use crate::memory::store::Loaded;
use crate::memory::TaskFolder;
use crate::overlap::check::{check_hook_spec, CheckArgs};
use crate::overlap::claims::claims_path;
use crate::overlap::grants::grants_path;
use crate::overlap::notes::notes_path;
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
            Err(e) => eprintln!("Fanout Code: skipping workspace agent {name}: {e}"),
        }
    }
    Ok(out)
}

/// What a Claude Code launch adds for the task's workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchExtras {
    /// `permissions.deny` rules for the protected roots and the overlap folders (`overlap_dirs`).
    pub deny: Vec<String>,
    /// The Bash guard hook: always for Claude Code, since the overlap folders are always guarded.
    pub guard: Option<HookSpec>,
    /// Beyond the linked roots (which the launch adds itself): the context-doc cache when a reference is cached.
    pub add_dirs: Vec<PathBuf>,
    /// The claim-check hook (Plan 11 §5.3, §5.6), unless both `ask_before_other_tasks_files` and
    /// `guard_git_for_other_tasks` are off; Bash only when just the git guard is on.
    pub claim_check: Option<HookSpec>,
}

/// The app's folders the claim-check hook trusts (Plan 11): every task's grants, the claims, the queued notes and the
/// event logs. Every Claude Code task gets `Edit` deny rules for them and the Bash guard refusing shell writes there,
/// whatever the overlap settings, so the agent cannot grant itself, clear a claim or forge a log line.
pub fn overlap_dirs(app_data: &Path) -> Vec<PathBuf> {
    ["grants", "claims", "notes", "events"]
        .iter()
        .map(|d| app_data.join(d))
        .collect()
}

/// The claim-check hook over the claims files of the task's repos (its worktree or home root, then its links).
fn claim_check_hook(
    conn: &Connection,
    task: &Task,
    app_data: &Path,
    exe: &Path,
    ask_before: bool,
    guard_git: bool,
) -> Result<Option<HookSpec>> {
    if !ask_before && !guard_git {
        return Ok(None);
    }
    let home_root = db::projects::get(conn, &task.project_id)?.map(|p| p.root);
    let repos = match &home_root {
        Some(root) => repos_of(task, root),
        // no home project on record: only what the task names itself
        None => task.worktree_path.iter().chain(&task.linked_roots).cloned().collect(),
    };
    let args = CheckArgs {
        task_id: task.id.clone(),
        claims_paths: repos.iter().map(|r| claims_path(app_data, r)).collect(),
        grants_path: grants_path(app_data, &task.id),
        notes_path: notes_path(app_data, &task.id),
        log_path: event_log_path(app_data, &task.id),
        guard_git,
    };
    let mut spec = check_hook_spec(exe, &args);
    if !ask_before {
        spec.matcher = Some("Bash".into());
    }
    Ok(Some(spec))
}

/// Refuses the launch (`Precondition`) when a protected root cannot be expressed as a safe deny rule. `ask_before`
/// and `guard_git` are the task's resolved `ask_before_other_tasks_files` and `guard_git_for_other_tasks`.
pub fn launch_extras(
    conn: &Connection,
    task: &Task,
    app_data: &Path,
    exe: &Path,
    home: &Path,
    ask_before: bool,
    guard_git: bool,
) -> Result<LaunchExtras> {
    if task.agent_kind != AgentKind::ClaudeCode {
        return Ok(LaunchExtras::default());
    }
    let claim_check = claim_check_hook(conn, task, app_data, exe, ask_before, guard_git)?;
    let protected = protected_for(conn, task)?;
    let app_files = overlap_dirs(app_data);
    // created now, so the rules also name their canonical form (a symlinked app data folder)
    for d in &app_files {
        std::fs::create_dir_all(d)?;
    }
    let mut deny = deny_rules(&protected)?;
    for rule in deny_rules(&app_files)? {
        if !deny.contains(&rule) {
            deny.push(rule);
        }
    }
    let guard = prepare_guard(app_data, exe, &task.id, &protected, &app_files, home)?;
    // only the home project's cache and, when bound to one, the workspace's: never another project's
    let mut containers = vec![Container::Project {
        id: task.project_id.clone(),
    }];
    if let Some(id) = task.workspace_id.as_ref().filter(|_| task.links_bound_at.is_some()) {
        containers.push(Container::Workspace { id: id.clone() });
    }
    let mut add_dirs = Vec::new();
    for c in &containers {
        let dir = context_dir(app_data, c)?;
        if dir.is_dir() {
            add_dirs.push(dir);
        }
    }
    Ok(LaunchExtras {
        deny,
        guard,
        add_dirs,
        claim_check,
    })
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
        workspace: Container::Workspace { id: ws.id.clone() },
        workspace_roots: member_projects(conn, &ws)?.into_iter().map(|p| p.root).collect(),
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
    let project = Container::Project {
        id: task.project_id.clone(),
    };
    let project_roots: Vec<PathBuf> = db::projects::get(conn, &task.project_id)?
        .map(|p| p.root)
        .into_iter()
        .collect();
    ctx.project_context = reference_lines(&project, &list_rows(conn, &project)?, &project_roots, app_data);
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
    ctx.workspace_context = reference_lines(&link.workspace, &link.workspace_docs, &link.workspace_roots, app_data);
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
    use crate::workspace::context_docs::ContextDocKind;
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
        let exe = Path::new("/x/fanoutcode");
        let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap();
        let none = launch_extras(
            &fx.db.lock().unwrap(),
            &task,
            fx.app_data.path(),
            exe,
            Path::new("/h"),
            true,
            true,
        )
        .unwrap();
        assert!(none.deny.iter().all(|r| !r.contains(&*fx.api.root.to_string_lossy())));
        s.set_policy(&ws.id, LinkPolicy::ReadOnly, &[]).unwrap();
        let ro = launch_extras(
            &fx.db.lock().unwrap(),
            &task,
            fx.app_data.path(),
            exe,
            Path::new("/h"),
            true,
            true,
        )
        .unwrap();
        assert!(ro.deny.contains(&format!("Edit(/{}/**)", fx.api.root.display())));
        assert!(ro.guard.is_some());
    }

    #[test]
    fn generated_settings_deny_edits_to_the_overlap_files() {
        let (fx, _s, _ws, t) = setup();
        let conn = fx.db.lock().unwrap();
        let task = db::tasks::require(&conn, &t.id).unwrap(); // nothing linked, nothing protected
        let data = fx.app_data.path();
        let canonical = std::fs::canonicalize(data).unwrap();
        // whatever the overlap settings say: the hook trusts these files, so the agent never writes them
        for (ask, git) in [(true, true), (false, false)] {
            let extras = launch_extras(
                &conn,
                &task,
                data,
                Path::new("/x/fanoutcode"),
                Path::new("/h"),
                ask,
                git,
            )
            .unwrap();
            for dir in ["grants", "claims", "notes", "events"] {
                for base in [data, canonical.as_path()] {
                    let rule = format!("Edit(/{}/**)", base.join(dir).display());
                    assert!(extras.deny.contains(&rule), "{rule} in {:?}", extras.deny);
                }
            }
            // and the Bash guard runs to refuse shell writes there, though no linked root is protected
            let guard = extras.guard.expect("the Bash guard");
            assert_eq!(guard.matcher.as_deref(), Some("Bash"));
        }
        let mut shell = task.clone();
        shell.agent_kind = AgentKind::Shell;
        let none = launch_extras(&conn, &shell, data, Path::new("/x"), Path::new("/h"), true, true).unwrap();
        assert!(none.deny.is_empty() && none.guard.is_none());
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

    #[test]
    fn launch_extras_adds_the_claim_check_hook() {
        let (fx, _s, _ws, t) = setup();
        bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
        let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap();
        let exe = Path::new("/x/fanoutcode");
        let extras = |ask: bool, git: bool| {
            launch_extras(
                &fx.db.lock().unwrap(),
                &task,
                fx.app_data.path(),
                exe,
                Path::new("/h"),
                ask,
                git,
            )
            .unwrap()
        };
        let spec = extras(true, false).claim_check.expect("a claim-check hook");
        assert!(
            spec.command.starts_with("'/x/fanoutcode' claim-check 't-1' "),
            "{}",
            spec.command
        );
        assert_eq!(spec.matcher.as_deref(), Some("Edit|Write|MultiEdit|NotebookEdit|Bash"));
        assert!(spec.command.contains(" 0 "), "git guard off: {}", spec.command);
        // the home root's claims file, then the linked root's
        let data = fx.app_data.path();
        let tail = format!(
            " '{}' '{}'",
            claims_path(data, &fx.web.root).display(),
            claims_path(data, &fx.api.root).display()
        );
        assert!(spec.command.ends_with(&tail), "{}", spec.command);
        assert!(spec
            .command
            .contains(&format!("'{}'", grants_path(data, &task.id).display())));
        assert!(spec
            .command
            .contains(&format!("'{}'", notes_path(data, &task.id).display())));
        assert!(spec
            .command
            .contains(&format!("'{}'", event_log_path(data, &task.id).display())));
        let git_only = extras(false, true).claim_check.expect("the git guard alone");
        assert_eq!(git_only.matcher.as_deref(), Some("Bash"));
        assert!(git_only.command.contains(" 1 "));
        assert_eq!(extras(false, false).claim_check, None);
        let mut shell = task.clone();
        shell.agent_kind = AgentKind::Shell;
        let none = launch_extras(&fx.db.lock().unwrap(), &shell, data, exe, Path::new("/h"), true, true).unwrap();
        assert_eq!(none.claim_check, None);
    }

    #[test]
    fn a_task_gets_only_its_own_projects_and_workspaces_cache() {
        let (fx, _s, ws, t) = setup();
        bind_if_first_start(&fx.db.lock().unwrap(), fx.app_data.path(), &t.id, None).unwrap();
        for c in [
            Container::Project { id: fx.api.id.clone() },
            Container::Project { id: fx.web.id.clone() },
            Container::Workspace { id: ws.id.clone() },
        ] {
            std::fs::create_dir_all(context_dir(fx.app_data.path(), &c).unwrap()).unwrap();
        }
        let task = db::tasks::require(&fx.db.lock().unwrap(), &t.id).unwrap(); // home: web
        let extras = launch_extras(
            &fx.db.lock().unwrap(),
            &task,
            fx.app_data.path(),
            Path::new("/x"),
            Path::new("/h"),
            false,
            false,
        )
        .unwrap();
        let api_cache = context_dir(fx.app_data.path(), &Container::Project { id: fx.api.id.clone() }).unwrap();
        assert!(extras.add_dirs.iter().all(|d| !d.starts_with(&api_cache)));
        assert_eq!(extras.add_dirs.len(), 2, "web's own cache and the workspace's");
    }
}
