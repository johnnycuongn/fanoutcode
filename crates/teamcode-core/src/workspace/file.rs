//! The workspace file `<name>.teamcode-workspace.json`: the serde model, relative project paths, default location.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::context_docs::ContextDoc;
use crate::domain::{LinkPolicy, Project, ProjectColor, Workspace, WorkspaceId};
use crate::{Error, Result};

/// The workspace file's suffix.
pub const FILE_SUFFIX: &str = ".teamcode-workspace.json";

/// The format version this build reads and writes.
pub const FORMAT_VERSION: u32 = 1;

/// The file's contents. Keys are the Rust field names (A16); `teamcode_workspace` is the format version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub teamcode_workspace: u32,
    pub id: WorkspaceId,
    pub name: String,
    pub projects: Vec<WorkspaceFileProject>,
    pub policy: LinkPolicy,
    #[serde(default)]
    pub settings: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub prompt_fragments: BTreeMap<String, String>,
    #[serde(default)]
    pub agents: BTreeMap<String, String>,
    #[serde(default)]
    pub context_docs: Vec<ContextDoc>,
}

/// One project: its path relative to the file's folder when it lies under it, else absolute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileProject {
    pub path: PathBuf,
    pub name: String,
    pub color: ProjectColor,
    pub order: u32,
    pub protected: bool,
}

/// What a file is built from: the record and the rows the file mirrors.
pub struct WorkspaceParts<'a> {
    pub workspace: &'a Workspace,
    pub projects: &'a [Project],
    pub settings: &'a BTreeMap<String, serde_json::Value>,
    pub prompt_fragments: &'a BTreeMap<String, String>,
    pub agents: &'a BTreeMap<String, String>,
    pub context_docs: &'a [ContextDoc],
}

impl WorkspaceFile {
    /// The file for `parts` saved at `file_path`: projects in workspace order, each root relativised to the file's
    /// folder where possible. A member whose `Project` is not in `parts.projects` is left out.
    pub fn from_parts(parts: &WorkspaceParts, file_path: &Path) -> WorkspaceFile {
        let dir = file_path.parent();
        let mut members: Vec<_> = parts.workspace.projects.iter().collect();
        members.sort_by_key(|m| m.order);
        let projects = members
            .into_iter()
            .filter_map(|m| {
                let p = parts.projects.iter().find(|p| p.id == m.project_id)?;
                let path = dir
                    .and_then(|d| p.root.strip_prefix(d).ok())
                    .filter(|rel| !rel.as_os_str().is_empty())
                    .map_or_else(|| p.root.clone(), Path::to_path_buf);
                Some(WorkspaceFileProject {
                    path,
                    name: p.name.clone(),
                    color: p.color.clone(),
                    order: m.order,
                    protected: m.protected,
                })
            })
            .collect();
        WorkspaceFile {
            teamcode_workspace: FORMAT_VERSION,
            id: parts.workspace.id.clone(),
            name: parts.workspace.name.clone(),
            projects,
            policy: parts.workspace.policy,
            settings: parts.settings.clone(),
            prompt_fragments: parts.prompt_fragments.clone(),
            agents: parts.agents.clone(),
            context_docs: parts.context_docs.to_vec(),
        }
    }

    /// Pretty JSON with a trailing newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a workspace file always serialises");
        s.push('\n');
        s
    }

    /// `Error::Invalid` for malformed JSON or a format version other than 1.
    pub fn parse(bytes: &[u8]) -> Result<WorkspaceFile> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| Error::Invalid(format!("workspace file: {e}")))?;
        match value.get("teamcode_workspace").and_then(serde_json::Value::as_u64) {
            Some(v) if v == u64::from(FORMAT_VERSION) => {}
            Some(v) => return Err(Error::Invalid(format!("unsupported teamcode_workspace version {v}"))),
            None => return Err(Error::Invalid("workspace file: missing teamcode_workspace".into())),
        }
        serde_json::from_value(value).map_err(|e| Error::Invalid(format!("workspace file: {e}")))
    }

    /// Every project's absolute root, in file order; relative paths resolve against the file's folder.
    pub fn absolute_project_paths(&self, file_path: &Path) -> Vec<PathBuf> {
        let dir = file_path.parent().unwrap_or(Path::new("/"));
        self.projects
            .iter()
            .map(|p| {
                if p.path.is_absolute() {
                    p.path.clone()
                } else {
                    dir.join(&p.path)
                }
            })
            .collect()
    }
}

/// `<app_data>/workspaces/<id>/<name>.teamcode-workspace.json`; the id folder keeps two workspaces of one name
/// apart. Path separators in the name become dashes.
pub fn default_file_path(app_data: &Path, id: &WorkspaceId, name: &str) -> PathBuf {
    let mut safe: String = name
        .trim()
        .chars()
        .map(|c| {
            if c == '/' || c == ':' || c == '\\' || c == '\0' {
                '-'
            } else {
                c
            }
        })
        .collect();
    if safe.is_empty() || safe.starts_with('.') {
        safe = format!("workspace{safe}");
    }
    app_data
        .join("workspaces")
        .join(id.as_str())
        .join(format!("{safe}{FILE_SUFFIX}"))
}

/// The file lives under app data, so it is written through on every change.
pub fn is_in_app_data(path: &Path, app_data: &Path) -> bool {
    path.starts_with(app_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LinkPolicy, MemoryLocation, Project, ProjectColor, Workspace, WorkspaceProject};
    use crate::ids::{ProjectId, WorkspaceId};
    use crate::Error;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn project(n: usize, root: &str) -> Project {
        Project {
            id: ProjectId::parse(&format!("p-{n}")).unwrap(),
            name: Path::new(root).file_name().unwrap().to_string_lossy().into_owned(),
            root: root.into(),
            color: ProjectColor("indigo".into()),
            memory_location: MemoryLocation::AppStorage,
            created_at: chrono::Utc::now(),
        }
    }

    struct Owned {
        ws: Workspace,
        projects: Vec<Project>,
        settings: BTreeMap<String, serde_json::Value>,
        fragments: BTreeMap<String, String>,
        agents: BTreeMap<String, String>,
    }

    impl Owned {
        fn parts(&self) -> WorkspaceParts<'_> {
            WorkspaceParts {
                workspace: &self.ws,
                projects: &self.projects,
                settings: &self.settings,
                prompt_fragments: &self.fragments,
                agents: &self.agents,
                context_docs: &[],
            }
        }
    }

    fn owned(roots: &[&str]) -> Owned {
        let projects: Vec<Project> = roots.iter().enumerate().map(|(i, r)| project(i + 1, r)).collect();
        // stored in reverse so `from_parts` must sort by `order`
        let ws = Workspace {
            id: WorkspaceId::parse("ws-1").unwrap(),
            name: "Shop".into(),
            file_path: None,
            dirty: false,
            projects: projects
                .iter()
                .enumerate()
                .rev()
                .map(|(i, p)| WorkspaceProject {
                    project_id: p.id.clone(),
                    order: i as u32,
                    protected: i == 1,
                })
                .collect(),
            policy: LinkPolicy::ReadAndModifyOnRequest,
        };
        Owned {
            ws,
            projects,
            settings: BTreeMap::new(),
            fragments: BTreeMap::new(),
            agents: BTreeMap::new(),
        }
    }

    fn parts_with_roots(roots: &[&str]) -> Owned {
        owned(roots)
    }

    fn sample_file() -> WorkspaceFile {
        let mut o = owned(&["/w/api", "/w/web"]);
        o.fragments
            .insert("plan".into(), "Prefer the api crate's naming.".into());
        o.settings.insert("workspace_board".into(), serde_json::json!("Mixed"));
        WorkspaceFile::from_parts(&o.parts(), Path::new("/w/shop.teamcode-workspace.json"))
    }

    #[test]
    fn round_trips_through_json() {
        let f = sample_file(); // two projects, one fragment, one setting
        assert_eq!(f.projects.len(), 2);
        assert_eq!(WorkspaceFile::parse(f.to_json().as_bytes()).unwrap(), f);
        assert!(f.to_json().ends_with("}\n"));
    }

    #[test]
    fn relativises_project_paths_under_file_dir() {
        let file = Path::new("/w/shop.teamcode-workspace.json");
        let f = WorkspaceFile::from_parts(&parts_with_roots(&["/w/api", "/elsewhere/web"]).parts(), file);
        assert_eq!(f.projects[0].path, PathBuf::from("api"));
        assert_eq!(f.projects[1].path, PathBuf::from("/elsewhere/web"));
        assert!(f.projects[1].protected && !f.projects[0].protected);
        assert_eq!(
            f.absolute_project_paths(file),
            vec![PathBuf::from("/w/api"), PathBuf::from("/elsewhere/web")]
        );
    }

    #[test]
    fn parse_rejects_unknown_version() {
        let err = WorkspaceFile::parse(
            br#"{"teamcode_workspace":2,"id":"ws-1","name":"x","projects":[],"policy":"ReadOnly"}"#,
        )
        .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m.contains("teamcode_workspace")));
    }

    #[test]
    fn parse_defaults_optional_sections() {
        let f = WorkspaceFile::parse(
            br#"{"teamcode_workspace":1,"id":"ws-1","name":"x","projects":[],"policy":"ReadAndModifyOnRequest"}"#,
        )
        .unwrap();
        assert!(
            f.settings.is_empty() && f.prompt_fragments.is_empty() && f.agents.is_empty() && f.context_docs.is_empty()
        );
        assert_eq!(f.policy, LinkPolicy::ReadAndModifyOnRequest);
        assert!(WorkspaceFile::parse(
            br#"{"teamcode_workspace":1,"id":"ws-1","name":"x","projects":[],"policy":"read_only"}"#
        )
        .is_err());
    }

    #[test]
    fn default_file_path_differs_for_same_name_different_ids() {
        let a = default_file_path(Path::new("/app"), &WorkspaceId::parse("ws-a").unwrap(), "Shop");
        let b = default_file_path(Path::new("/app"), &WorkspaceId::parse("ws-b").unwrap(), "Shop");
        assert_eq!(a, PathBuf::from("/app/workspaces/ws-a/Shop.teamcode-workspace.json"));
        assert_ne!(a, b);
        assert!(
            is_in_app_data(&a, Path::new("/app"))
                && !is_in_app_data(Path::new("/w/x.teamcode-workspace.json"), Path::new("/app"))
        );
    }
}
