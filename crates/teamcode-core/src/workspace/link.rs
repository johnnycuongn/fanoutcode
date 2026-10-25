//! Which projects a task links: fixed at its first session start from the issuing window (§4.1), then compared
//! with the workspace as it is now.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{AgentKind, Container, Project, ProjectId, Task, Workspace, WorkspaceId};
use crate::{Error, Result};

/// What the caller stores on the task at its first session start (with `links_bound_at = now`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkBinding {
    pub workspace_id: Option<WorkspaceId>,
    pub linked_roots: Vec<PathBuf>,
}

/// A bound task's links against the workspace as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum LinkStatus {
    Unbound,
    Bound {
        workspace_id: Option<WorkspaceId>,
        linked_roots: Vec<PathBuf>,
    },
    Changed {
        added: Vec<PathBuf>,
        removed: Vec<PathBuf>,
    },
}

/// The roots of every member but `home`, in workspace order.
fn other_roots(home: &ProjectId, workspace: &Workspace, projects: &[Project]) -> Vec<PathBuf> {
    let mut members: Vec<_> = workspace.projects.iter().filter(|m| &m.project_id != home).collect();
    members.sort_by_key(|m| m.order);
    members
        .into_iter()
        .filter_map(|m| projects.iter().find(|p| p.id == m.project_id).map(|p| p.root.clone()))
        .collect()
}

/// `None` for a task already bound. From a project window nothing is linked, even when the project is in a
/// workspace; from a workspace window every other member is, except for Codex and shell tasks, which see only their
/// home project.
pub fn bind_links(
    task: &Task,
    issuing: &Container,
    workspace: Option<&Workspace>,
    projects: &[Project],
) -> Result<Option<LinkBinding>> {
    if task.links_bound_at.is_some() {
        return Ok(None);
    }
    let Container::Workspace { id } = issuing else {
        return Ok(Some(LinkBinding {
            workspace_id: None,
            linked_roots: Vec::new(),
        }));
    };
    let ws = workspace
        .filter(|w| &w.id == id)
        .ok_or_else(|| Error::Precondition("This workspace is not loaded.".into()))?;
    if !ws.projects.iter().any(|m| m.project_id == task.project_id) {
        return Err(Error::Precondition(
            "The task's home project is not in this workspace".into(),
        ));
    }
    let linked_roots = if task.agent_kind == AgentKind::ClaudeCode {
        other_roots(&task.project_id, ws, projects)
    } else {
        Vec::new()
    };
    Ok(Some(LinkBinding {
        workspace_id: Some(id.clone()),
        linked_roots,
    }))
}

pub fn link_status(task: &Task, workspace: Option<&Workspace>, projects: &[Project]) -> LinkStatus {
    if task.links_bound_at.is_none() {
        return LinkStatus::Unbound;
    }
    let bound = || LinkStatus::Bound {
        workspace_id: task.workspace_id.clone(),
        linked_roots: task.linked_roots.clone(),
    };
    let (Some(ws_id), Some(ws)) = (&task.workspace_id, workspace) else {
        return bound();
    };
    if &ws.id != ws_id || task.agent_kind != AgentKind::ClaudeCode {
        return bound();
    }
    let mut removed: Vec<PathBuf>;
    let mut added: Vec<PathBuf>;
    if ws.projects.iter().any(|m| m.project_id == task.project_id) {
        let now = other_roots(&task.project_id, ws, projects);
        added = now.iter().filter(|r| !task.linked_roots.contains(r)).cloned().collect();
        removed = task.linked_roots.iter().filter(|r| !now.contains(r)).cloned().collect();
    } else {
        // the home project left: the task no longer belongs to this workspace's link set at all
        added = Vec::new();
        removed = task.linked_roots.clone();
    }
    added.sort();
    removed.sort();
    if added.is_empty() && removed.is_empty() && ws.projects.iter().any(|m| m.project_id == task.project_id) {
        bound()
    } else {
        LinkStatus::Changed { added, removed }
    }
}

/// The `--add-dir` roots this plan adds for a Claude Code task: its linked roots, then the context-doc cache.
pub fn add_dirs(task: &Task, context_dir: Option<&Path>) -> Vec<PathBuf> {
    if task.agent_kind != AgentKind::ClaudeCode {
        return Vec::new();
    }
    task.linked_roots
        .iter()
        .cloned()
        .chain(context_dir.map(Path::to_path_buf))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, LinkPolicy, MemoryLocation, ProjectColor, WorkspaceProject};
    use crate::test_support::task_fixture;
    use chrono::Utc;

    const NAMES: [(&str, &str); 4] = [("api", "p-a1"), ("web", "p-b2"), ("docs", "p-d3"), ("infra", "p-e4")];

    fn pid(name: &str) -> ProjectId {
        ProjectId::parse(NAMES.iter().find(|(n, _)| *n == name).unwrap().1).unwrap()
    }
    fn root(name: &str) -> PathBuf {
        PathBuf::from(format!("/w/{name}"))
    }
    fn project(name: &str) -> Project {
        Project {
            id: pid(name),
            name: name.into(),
            root: root(name),
            color: ProjectColor("cyan".into()),
            memory_location: MemoryLocation::AppStorage,
            created_at: Utc::now(),
        }
    }
    fn add_project(ws: &mut Workspace, projects: &mut Vec<Project>, name: &str) {
        projects.push(project(name));
        ws.projects.push(WorkspaceProject {
            project_id: pid(name),
            order: ws.projects.len() as u32,
            protected: false,
        });
    }
    fn three_project_workspace(home: &str) -> (Workspace, Vec<Project>, Task) {
        let mut ws = Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
            name: "Shop".into(),
            file_path: None,
            dirty: false,
            projects: vec![],
            policy: LinkPolicy::ReadAndModifyOnRequest,
        };
        let mut projects = vec![];
        for n in ["api", "web", "docs"] {
            add_project(&mut ws, &mut projects, n);
        }
        let task = task_fixture("t-1", pid(home).as_str());
        (ws, projects, task)
    }
    fn bound_workspace_task(home: &str) -> (Workspace, Vec<Project>, Task) {
        let (ws, projects, mut task) = three_project_workspace(home);
        let b = bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects)
            .unwrap()
            .unwrap();
        task.workspace_id = b.workspace_id;
        task.linked_roots = b.linked_roots;
        task.links_bound_at = Some(Utc::now());
        (ws, projects, task)
    }

    #[test]
    fn binds_to_workspace_set_excluding_home_in_order() {
        let (ws, projects, task) = three_project_workspace("web"); // api, web, docs; task home = web
        let b = bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects)
            .unwrap()
            .unwrap();
        assert_eq!(
            (b.workspace_id, b.linked_roots),
            (Some(ws.id), vec![root("api"), root("docs")])
        );
    }

    #[test]
    fn binds_nothing_from_project_window_even_when_project_is_in_a_workspace() {
        let (ws, projects, task) = three_project_workspace("web");
        let b = bind_links(
            &task,
            &Container::Project {
                id: task.project_id.clone(),
            },
            Some(&ws),
            &projects,
        )
        .unwrap()
        .unwrap();
        assert_eq!((b.workspace_id, b.linked_roots.len()), (None, 0));
    }

    #[test]
    fn does_not_rebind_a_bound_task() {
        let (ws, projects, mut task) = three_project_workspace("web");
        task.links_bound_at = Some(Utc::now());
        assert!(
            bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_workspace_without_home_project() {
        let (mut ws, projects, task) = three_project_workspace("web");
        ws.projects.retain(|p| p.project_id != task.project_id);
        assert!(matches!(
            bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects),
            Err(Error::Precondition(_))
        ));
    }

    #[test]
    fn codex_task_binds_with_no_roots() {
        let (ws, projects, mut task) = three_project_workspace("web");
        task.agent_kind = AgentKind::Codex;
        let b = bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects)
            .unwrap()
            .unwrap();
        assert_eq!((b.workspace_id.is_some(), b.linked_roots.len()), (true, 0));
    }

    #[test]
    fn status_changed_lists_added_and_removed() {
        let (mut ws, mut projects, task) = bound_workspace_task("web"); // bound to [api, docs]
        ws.projects.retain(|p| p.project_id != pid("docs"));
        add_project(&mut ws, &mut projects, "infra");
        assert_eq!(
            link_status(&task, Some(&ws), &projects),
            LinkStatus::Changed {
                added: vec![root("infra")],
                removed: vec![root("docs")]
            }
        );
    }

    #[test]
    fn status_when_home_project_left_workspace() {
        let (mut ws, projects, task) = bound_workspace_task("web");
        ws.projects.retain(|p| p.project_id != task.project_id);
        assert_eq!(
            link_status(&task, Some(&ws), &projects),
            LinkStatus::Changed {
                added: vec![],
                removed: vec![root("api"), root("docs")]
            }
        );
    }

    #[test]
    fn status_bound_for_project_window_task_and_unbound_before_start() {
        let (ws, projects, mut task) = three_project_workspace("web");
        assert_eq!(link_status(&task, Some(&ws), &projects), LinkStatus::Unbound);
        task.links_bound_at = Some(Utc::now());
        task.workspace_id = None;
        assert!(matches!(
            link_status(&task, Some(&ws), &projects),
            LinkStatus::Bound { workspace_id: None, .. }
        ));
    }

    #[test]
    fn add_dirs_only_for_claude_code() {
        let (_ws, _p, mut task) = bound_workspace_task("web");
        assert_eq!(
            add_dirs(&task, Some(Path::new("/app/context"))),
            vec![root("api"), root("docs"), PathBuf::from("/app/context")]
        );
        task.agent_kind = AgentKind::Shell;
        assert!(add_dirs(&task, Some(Path::new("/app/context"))).is_empty());
    }

    #[test]
    fn link_status_round_trips_with_type_tags() {
        for s in [
            LinkStatus::Unbound,
            LinkStatus::Bound {
                workspace_id: None,
                linked_roots: vec![root("api")],
            },
            LinkStatus::Changed {
                added: vec![root("a")],
                removed: vec![],
            },
        ] {
            let json = serde_json::to_value(&s).unwrap();
            assert!(json["type"].is_string());
            assert_eq!(serde_json::from_value::<LinkStatus>(json).unwrap(), s);
        }
    }
}
