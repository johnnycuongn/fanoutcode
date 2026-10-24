//! IPC commands, one file per domain. Every command is `#[tauri::command] #[specta::specta]` and listed in
//! `specta::builder()`.

use teamcode_core::domain::{Container, ProjectId, WorkspaceId};
use teamcode_core::settings::Scope;
use teamcode_core::Result;

pub mod cost;
pub mod git;
pub mod hooks;
pub mod livespec;
pub mod memory;
pub mod projects;
pub mod settings;
pub mod tasks;
pub mod tests;
pub mod views;
pub mod windows;

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
            .map(|t| teamcode_core::domain::TaskId::parse(t.as_str()))
            .transpose()?,
    })
}
