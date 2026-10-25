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
use super::link::{bind_links, canonical_root, link_status, links_for, load_links, same_root, save_links, LinkStatus};
use super::policy::{deny_file_rules, deny_rules, protected_roots};
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
    /// The claim-check hook (Plan 11 §5.3, §5.6, §8), on the file tools and Bash for every Claude Code task. With
    /// `ask_before_other_tasks_files` off it logs an overlapping edit (`ClaimNoted`) and allows it; with
    /// `guard_git_for_other_tasks` off it leaves Bash alone. With both off it never denies but still runs, so the
    /// overlap is logged and queued notes reach a working session through `additionalContext`.
    pub claim_check: Option<HookSpec>,
}

/// The app's folders its hooks trust (Plan 11): every task's grants, the claims, the queued notes, the event logs, the
/// Bash guard's roots files and the generated Claude settings. Every Claude Code task gets `Edit` deny rules for them
/// and the Bash guard refusing shell writes there, whatever the overlap settings, so the agent cannot grant itself,
/// clear a claim, forge a log line or switch a guard off.
pub fn overlap_dirs(app_data: &Path) -> Vec<PathBuf> {
    ["grants", "claims", "notes", "events", "guards", "claude-settings"]
        .iter()
        .map(|d| app_data.join(d))
        .collect()
}

/// The app's database and its SQLite sidecars: no agent may edit them or write them from the shell either.
pub fn app_db_files(app_data: &Path) -> Vec<PathBuf> {
    ["fanoutcode.db", "fanoutcode.db-wal", "fanoutcode.db-shm"]
        .iter()
        .map(|f| app_data.join(f))
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
        ask_before,
        guard_git,
    };
    Ok(Some(check_hook_spec(exe, &args)))
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
    let db_files = app_db_files(app_data);
    for rule in deny_rules(&app_files)?.into_iter().chain(deny_file_rules(&db_files)?) {
        if !deny.contains(&rule) {
            deny.push(rule);
        }
    }
    // the guard compares paths as written, so it gets both forms of each folder, as the deny rules do
    let mut guarded: Vec<PathBuf> = Vec::new();
    let db_forms = db_files.iter().map(|f| match (f.parent(), f.file_name()) {
        (Some(dir), Some(name)) => (f.clone(), canonical_root(dir).join(name)),
        _ => (f.clone(), f.clone()),
    });
    let pairs: Vec<(PathBuf, PathBuf)> = app_files
        .iter()
        .map(|d| (d.clone(), canonical_root(d)))
        .chain(db_forms)
        .collect();
    for (written, canonical) in pairs {
        for form in [written, canonical] {
            if !guarded.contains(&form) {
                guarded.push(form);
            }
        }
    }
    let guard = prepare_guard(app_data, exe, &task.id, &protected, &guarded, home)?;
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
#[path = "launch_tests.rs"]
mod tests;
