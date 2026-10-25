//! Which projects a task links: fixed at its first session start from the issuing window (§4.1), then compared
//! with the workspace as it is now.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::domain::{AgentKind, Container, Project, ProjectId, Task, TaskId, Workspace, WorkspaceId};
use crate::{Error, Result};

/// One bound root and the project it was bound for. `project_id` is `None` only for a root with no recorded
/// project (a task bound before links were recorded); such a root is always treated as protected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundLink {
    pub project_id: Option<ProjectId>,
    pub root: PathBuf,
}

/// What the caller stores at the task's first session start: `workspace_id` and `linked_roots` on the task (with
/// `links_bound_at = now`), `links` through `save_links`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkBinding {
    pub workspace_id: Option<WorkspaceId>,
    pub linked_roots: Vec<PathBuf>,
    pub links: Vec<BoundLink>,
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

/// The canonical form of a root: symlinks resolved, no trailing slash. A root that cannot be resolved (gone, or
/// never existed) keeps its spelling without the trailing slash.
pub fn canonical_root(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| {
        let s = p.to_string_lossy();
        let t = s.trim_end_matches('/');
        PathBuf::from(if t.is_empty() { "/" } else { t })
    })
}

/// Two roots name the same folder: compared canonicalised, and case-insensitively on macOS (whose default file
/// system ignores case).
pub fn same_root(a: &Path, b: &Path) -> bool {
    let (a, b) = (canonical_root(a), canonical_root(b));
    if cfg!(target_os = "macos") {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Every member but `home`, in workspace order, with its canonical root.
fn other_links(home: &ProjectId, workspace: &Workspace, projects: &[Project]) -> Vec<BoundLink> {
    let mut members: Vec<_> = workspace.projects.iter().filter(|m| &m.project_id != home).collect();
    members.sort_by_key(|m| m.order);
    members
        .into_iter()
        .filter_map(|m| projects.iter().find(|p| p.id == m.project_id))
        .map(|p| BoundLink {
            project_id: Some(p.id.clone()),
            root: canonical_root(&p.root),
        })
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
            links: Vec::new(),
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
    let links = if task.agent_kind == AgentKind::ClaudeCode {
        other_links(&task.project_id, ws, projects)
    } else {
        Vec::new()
    };
    Ok(Some(LinkBinding {
        workspace_id: Some(id.clone()),
        linked_roots: links.iter().map(|l| l.root.clone()).collect(),
        links,
    }))
}

/// Each of the task's linked roots with the project it was recorded for, or `None` when no stored link matches it.
pub fn links_for(linked_roots: &[PathBuf], stored: &[BoundLink]) -> Vec<BoundLink> {
    linked_roots
        .iter()
        .map(|root| BoundLink {
            project_id: stored
                .iter()
                .find(|l| same_root(&l.root, root))
                .and_then(|l| l.project_id.clone()),
            root: root.clone(),
        })
        .collect()
}

pub fn save_links(conn: &Connection, task_id: &TaskId, links: &[BoundLink]) -> Result<()> {
    conn.execute("DELETE FROM task_links WHERE task_id = ?1", [task_id.as_str()])?;
    for (i, l) in links.iter().enumerate() {
        conn.execute(
            "INSERT INTO task_links (task_id, ord, project_id, root) VALUES (?1, ?2, ?3, ?4)",
            params![
                task_id.as_str(),
                i as u32,
                l.project_id.as_ref().map(|p| p.as_str().to_owned()),
                l.root.to_string_lossy()
            ],
        )?;
    }
    Ok(())
}

pub fn load_links(conn: &Connection, task_id: &TaskId) -> Result<Vec<BoundLink>> {
    let mut stmt = conn.prepare("SELECT project_id, root FROM task_links WHERE task_id = ?1 ORDER BY ord")?;
    let rows = stmt.query_map([task_id.as_str()], |r| {
        Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (project_id, root) = row?;
        out.push(BoundLink {
            // an unparseable id fails closed: the root counts as unmatched
            project_id: project_id.and_then(|p| ProjectId::parse(&p).ok()),
            root: root.into(),
        });
    }
    Ok(out)
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
    let home_in = ws.projects.iter().any(|m| m.project_id == task.project_id);
    let (mut added, mut removed): (Vec<PathBuf>, Vec<PathBuf>) = if home_in {
        let now: Vec<PathBuf> = other_links(&task.project_id, ws, projects)
            .into_iter()
            .map(|l| l.root)
            .collect();
        (
            now.iter()
                .filter(|r| !task.linked_roots.iter().any(|b| same_root(b, r)))
                .cloned()
                .collect(),
            task.linked_roots
                .iter()
                .filter(|b| !now.iter().any(|r| same_root(b, r)))
                .cloned()
                .collect(),
        )
    } else {
        // the home project left: none of the task's links belong to this workspace any more
        (Vec::new(), task.linked_roots.clone())
    };
    added.sort();
    removed.sort();
    if home_in && added.is_empty() && removed.is_empty() {
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

    #[test]
    fn binding_records_project_ids_with_canonical_roots() {
        let base = tempfile::tempdir().unwrap();
        let real = std::fs::canonicalize(base.path()).unwrap().join("api");
        std::fs::create_dir(&real).unwrap();
        let link = base.path().join("api-link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let (mut ws, mut projects, task) = three_project_workspace("web");
        projects.iter_mut().find(|p| p.id == pid("api")).unwrap().root = PathBuf::from(format!("{}/", link.display()));
        ws.projects.retain(|m| m.project_id != pid("docs"));
        let b = bind_links(&task, &Container::Workspace { id: ws.id.clone() }, Some(&ws), &projects)
            .unwrap()
            .unwrap();
        assert_eq!(
            b.links,
            vec![BoundLink {
                project_id: Some(pid("api")),
                root: real.clone()
            }]
        );
        assert_eq!(b.linked_roots, vec![real]);
    }

    #[test]
    fn links_for_matches_case_variants_and_leaves_unknown_roots_unmatched() {
        let stored = vec![BoundLink {
            project_id: Some(pid("api")),
            root: root("api"),
        }];
        let out = links_for(&[root("api"), PathBuf::from("/w/other")], &stored);
        assert_eq!(out[0].project_id, Some(pid("api")));
        assert_eq!(out[1].project_id, None);
        if cfg!(target_os = "macos") {
            assert_eq!(
                links_for(&[PathBuf::from("/W/API/")], &stored)[0].project_id,
                Some(pid("api"))
            );
        }
    }

    #[test]
    fn links_persist_per_task() {
        let conn = crate::test_support::test_conn();
        crate::test_support::insert_project(&conn, "p-1a-1");
        let t = crate::test_support::insert_task(&conn, "t-1a-1", "p-1a-1");
        let links = vec![
            BoundLink {
                project_id: Some(pid("api")),
                root: root("api"),
            },
            BoundLink {
                project_id: None,
                root: root("docs"),
            },
        ];
        save_links(&conn, &t.id, &links).unwrap();
        assert_eq!(load_links(&conn, &t.id).unwrap(), links);
    }
}
