//! IPC commands, one file per domain. Every command is `#[tauri::command] #[specta::specta]` and listed in
//! `specta::builder()`.

use fanoutcode_core::domain::{Container, ProjectId, WorkspaceId};
use fanoutcode_core::settings::Scope;
use fanoutcode_core::Result;

pub mod activity;
pub mod canvas;
pub mod context_docs;
pub mod cost;
pub mod git;
pub mod hooks;
pub mod jira;
pub mod livespec;
pub mod memory;
pub mod overlap;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod tasks;
pub mod tests;
pub mod views;
pub mod windows;
pub mod workspaces;

/// Re-parses a container's id from the webview (serde's transparent `Deserialize` does not validate).
pub(crate) fn parse_container(c: &Container) -> Result<Container> {
    Ok(match c {
        Container::Project { id } => Container::Project {
            id: ProjectId::parse(id.as_str())?,
        },
        Container::Workspace { id } => Container::Workspace {
            id: WorkspaceId::parse(id.as_str())?,
        },
    })
}

/// Re-parses every id in a settings scope.
pub(crate) fn parse_scope(s: &Scope) -> Result<Scope> {
    Ok(Scope {
        workspace: s
            .workspace
            .as_ref()
            .map(|w| WorkspaceId::parse(w.as_str()))
            .transpose()?,
        project: s.project.as_ref().map(|p| ProjectId::parse(p.as_str())).transpose()?,
        task: s
            .task
            .as_ref()
            .map(|t| fanoutcode_core::domain::TaskId::parse(t.as_str()))
            .transpose()?,
    })
}
