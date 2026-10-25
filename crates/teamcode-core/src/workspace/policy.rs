//! The linking policy as enforcement: which bound roots are protected, and the `Edit` deny rules that block every
//! built-in file tool there (Claude Code's `Edit(path)` rules govern Write, MultiEdit and NotebookEdit too).

use std::path::{Path, PathBuf};

use crate::domain::{LinkPolicy, Project, ProjectId, Workspace};

pub const READ_ONLY_LABEL: &str = "Read only: file tools blocked, shell commands checked";
pub const READ_AND_MODIFY_LABEL: &str = "Read and modify on request";
pub const POLICY_APPLIES_NOTE: &str = "Applies to sessions started from now on.";

fn trim_slash(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    let t = s.trim_end_matches('/');
    PathBuf::from(if t.is_empty() { "/" } else { t })
}

/// A bound root is protected when its project is not `home` and the workspace is Read only or the project is a
/// protected target. A bound root that matches no member (the project left) is protected only under Read only.
/// Sorted, deduplicated.
pub fn protected_roots(
    home: &ProjectId,
    workspace: &Workspace,
    projects: &[Project],
    bound_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let home_root = projects.iter().find(|p| &p.id == home).map(|p| trim_slash(&p.root));
    let mut out: Vec<PathBuf> = bound_roots
        .iter()
        .map(|r| trim_slash(r))
        .filter(|r| Some(r) != home_root.as_ref())
        .filter(|r| {
            let member = projects
                .iter()
                .find(|p| &trim_slash(&p.root) == r)
                .and_then(|p| workspace.projects.iter().find(|m| m.project_id == p.id));
            if member.is_some_and(|m| &m.project_id == home) {
                return false;
            }
            workspace.policy == LinkPolicy::ReadOnly || member.is_some_and(|m| m.protected)
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One `Edit(//<absolute path without its leading slash>/**)` per root, for `permissions.deny`.
pub fn deny_rules(protected: &[PathBuf]) -> Vec<String> {
    protected
        .iter()
        .map(|r| {
            let s = trim_slash(r).to_string_lossy().into_owned();
            format!("Edit(//{}/**)", s.trim_start_matches('/'))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MemoryLocation, ProjectColor, WorkspaceId, WorkspaceProject};

    fn pid(name: &str) -> ProjectId {
        ProjectId::parse(match name {
            "api" => "p-a1",
            "web" => "p-b2",
            _ => "p-d3",
        })
        .unwrap()
    }
    fn root(name: &str) -> PathBuf {
        PathBuf::from(format!("/w/{name}"))
    }
    fn ws_with(names: &[&str]) -> (Workspace, Vec<Project>) {
        let projects: Vec<Project> = names
            .iter()
            .map(|n| Project {
                id: pid(n),
                name: (*n).into(),
                root: root(n),
                color: ProjectColor("cyan".into()),
                memory_location: MemoryLocation::AppStorage,
                created_at: chrono::Utc::now(),
            })
            .collect();
        let ws = Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
            name: "Shop".into(),
            file_path: None,
            dirty: false,
            projects: names
                .iter()
                .enumerate()
                .map(|(i, n)| WorkspaceProject {
                    project_id: pid(n),
                    order: i as u32,
                    protected: false,
                })
                .collect(),
            policy: LinkPolicy::ReadAndModifyOnRequest,
        };
        (ws, projects)
    }
    fn set_protected(ws: &mut Workspace, name: &str) {
        ws.projects
            .iter_mut()
            .find(|m| m.project_id == pid(name))
            .unwrap()
            .protected = true;
    }

    #[test]
    fn read_only_protects_every_bound_root_but_home() {
        let (mut ws, projects) = ws_with(&["api", "web", "docs"]);
        ws.policy = LinkPolicy::ReadOnly;
        assert_eq!(
            protected_roots(&pid("web"), &ws, &projects, &[root("api"), root("docs")]),
            vec![root("api"), root("docs")]
        );
    }

    #[test]
    fn protected_flag_protects_one_root_under_default_policy() {
        let (mut ws, projects) = ws_with(&["api", "web", "docs"]);
        set_protected(&mut ws, "docs");
        assert_eq!(
            protected_roots(&pid("web"), &ws, &projects, &[root("api"), root("docs")]),
            vec![root("docs")]
        );
    }

    #[test]
    fn home_project_is_never_protected_even_when_flagged() {
        let (mut ws, projects) = ws_with(&["api", "web"]);
        ws.policy = LinkPolicy::ReadOnly;
        set_protected(&mut ws, "web");
        assert!(protected_roots(&pid("web"), &ws, &projects, &[root("api")])
            .iter()
            .all(|r| r != &root("web")));
    }

    #[test]
    fn only_bound_roots_count() {
        let (mut ws, projects) = ws_with(&["api", "web", "docs"]);
        ws.policy = LinkPolicy::ReadOnly;
        assert_eq!(
            protected_roots(&pid("web"), &ws, &projects, &[root("api")]),
            vec![root("api")]
        );
    }

    #[test]
    fn deny_rules_use_double_slash_absolute_edit_rules() {
        assert_eq!(
            deny_rules(&[PathBuf::from("/Users/j/w/api"), PathBuf::from("/Users/j/my api")]),
            vec![
                "Edit(//Users/j/w/api/**)".to_string(),
                "Edit(//Users/j/my api/**)".to_string()
            ]
        );
        assert!(deny_rules(&[]).is_empty());
    }
}
