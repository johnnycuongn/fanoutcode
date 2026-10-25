//! The checks behind a workspace import: dropping invalid fields, refusing roots that are not plain project
//! folders, and deciding whether a file adds anything to the record already at its path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::context_docs::ContextDocKind;
use super::file::WorkspaceFile;
use super::import::OpenEnv;
use super::store::{clean_name, valid_agent_name, WorkspaceStore};
use crate::domain::{LinkPolicy, Stage, Workspace};
use crate::settings::{validate, ScopeKind};
use crate::Result;

pub(crate) fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_owned())
}

/// Drops what the app would refuse to store and says so: agents with bad names, fragment keys that are not a
/// prompting stage, settings that fail `settings::validate` at workspace scope (and `jira`, never mirrored), context
/// docs whose file or folder target is not absolute. The name is cleaned as a created workspace's is.
pub fn sanitize(mut file: WorkspaceFile, fallback_name: &str) -> (WorkspaceFile, Vec<String>) {
    let mut problems = Vec::new();
    file.name = clean_name(&file.name).unwrap_or_else(|_| {
        problems.push("The file has no workspace name; using the file name.".to_owned());
        clean_name(fallback_name).unwrap_or_else(|_| "Workspace".to_owned())
    });
    file.agents.retain(|name, _| {
        let ok = valid_agent_name(name);
        if !ok {
            problems.push(format!(
                "Ignored agent {name:?}: names use lower-case letters, digits and dashes."
            ));
        }
        ok
    });
    file.prompt_fragments.retain(|stage, _| {
        let ok = Stage::parse(stage).is_some_and(|s| s.has_agent_prompt());
        if !ok {
            problems.push(format!("Ignored prompt instructions for {stage:?}: not a stage."));
        }
        ok
    });
    file.settings.retain(|key, value| {
        if key == "jira" || key == "jira_transitions" {
            problems.push("Jira links aren't imported; link Jira in Fanout Code".to_owned());
            return false;
        }
        let ok = validate(key, ScopeKind::Workspace, value).is_ok();
        if !ok {
            problems.push(format!("Ignored setting {key:?}."));
        }
        ok
    });
    problems.dedup();
    file.context_docs.retain(|d| {
        let why = match d.kind {
            ContextDocKind::File | ContextDocKind::Folder if !Path::new(&d.target).is_absolute() => {
                Some("not an absolute path".to_owned())
            }
            ContextDocKind::Url | ContextDocKind::Confluence => super::url_guard::check_lexical(&d.target, true)
                .err()
                .map(|e| e.to_string()),
            _ => None,
        };
        if let Some(why) = &why {
            problems.push(format!("Ignored reference {:?}: {why}", d.target));
        }
        why.is_none()
    });
    let mut seen = std::collections::HashSet::new();
    file.context_docs.retain(|d| {
        let first = seen.insert(d.id.clone());
        if !first {
            problems.push(format!("Ignored a second reference with id {:?}.", d.id));
        }
        first
    });
    // trust is the app's to give (in the preview), never the file's to claim
    for d in &mut file.context_docs {
        d.trusted = false;
    }
    (file, problems)
}

/// Folders under the home folder that commonly hold credentials or app data.
const SENSITIVE_UNDER_HOME: &[&str] = &[
    "Library",
    ".ssh",
    ".aws",
    ".config",
    ".gnupg",
    ".kube",
    ".docker",
    ".azure",
    ".gcloud",
    ".password-store",
];

/// A warning for a root that is a dot-folder or sits in a credential location under the home folder.
pub(crate) fn sensitive_root(root: &Path, home: &Path) -> Option<String> {
    let root = canonical(root);
    let home = canonical(home);
    let dot = root.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
    let sensitive = SENSITIVE_UNDER_HOME.iter().any(|s| root.starts_with(home.join(s)));
    (dot || sensitive).then(|| {
        format!(
            "{}: agents would be able to read and edit this folder, which may hold credentials or app data.",
            root.display()
        )
    })
}

/// Why a canonical root may not be imported, if it may not: not an existing folder, `/`, or equal to or an ancestor
/// of the home folder, app data or app config, or inside app data or app config. Folders inside home are fine.
pub(crate) fn root_problem(root: &Path, env: &OpenEnv, app_data: &Path) -> Option<String> {
    if !root.is_dir() {
        return Some(format!("{} is not an existing folder.", root.display()));
    }
    let root = canonical(root);
    if root.parent().is_none() {
        return Some("The file names the whole disk (/) as a project.".to_owned());
    }
    let home = canonical(env.home_dir);
    if home.starts_with(&root) {
        return Some(format!("{} is your home folder or contains it.", root.display()));
    }
    for own in [canonical(app_data), canonical(env.app_config)] {
        if own.starts_with(&root) || root.starts_with(&own) {
            return Some(format!(
                "{} contains or is inside Fanout Code's own data.",
                root.display()
            ));
        }
    }
    None
}

/// The record whose file is `path` (compared canonicalised).
pub(crate) fn record_at(store: &WorkspaceStore, path: &Path) -> Result<Option<Workspace>> {
    let want = canonical(path);
    Ok(store
        .list()?
        .into_iter()
        .find(|w| w.file_path.as_deref().is_some_and(|p| canonical(p) == want)))
}

/// True when `file` (at `path`) adds nothing to `record`: no new root, agent, instruction or reference, no agent or
/// instruction text changed, no protection loosened.
pub(crate) fn adds_nothing(
    store: &WorkspaceStore,
    record: &Workspace,
    file: &WorkspaceFile,
    path: &Path,
) -> Result<bool> {
    let mine = store.snapshot(&record.id)?;
    let mine_path = record.file_path.clone().unwrap_or_else(|| path.to_owned());
    let my_roots: Vec<(PathBuf, bool)> = mine
        .absolute_project_paths(&mine_path)
        .iter()
        .map(|p| canonical(p))
        .zip(mine.projects.iter().map(|p| p.protected))
        .collect();
    let their_roots: Vec<(PathBuf, bool)> = file
        .absolute_project_paths(path)
        .iter()
        .map(|p| canonical(p))
        .zip(file.projects.iter().map(|p| p.protected))
        .collect();
    let roots_ok = their_roots.iter().all(|(r, _)| my_roots.iter().any(|(m, _)| m == r))
        && my_roots
            .iter()
            .filter(|(_, protected)| *protected)
            .all(|(m, _)| their_roots.iter().find(|(r, _)| r == m).is_none_or(|(_, p)| *p));
    // agents by name and body, fragments by stage and text: an edited body counts as an addition
    let subset = |theirs: &BTreeMap<String, String>, mine: &BTreeMap<String, String>| {
        theirs.iter().all(|(k, v)| mine.get(k) == Some(v))
    };
    // every field but the volatile ones (id, fetched_at, order), so a changed label or page id is reviewed
    let docs_ok = file.context_docs.iter().all(|d| {
        mine.context_docs
            .iter()
            .any(|m| m.kind == d.kind && m.target == d.target && m.label == d.label && m.page_id == d.page_id)
    });
    let policy_ok = !(mine.policy == LinkPolicy::ReadOnly && file.policy != LinkPolicy::ReadOnly);
    Ok(roots_ok
        && subset(&file.agents, &mine.agents)
        && subset(&file.prompt_fragments, &mine.prompt_fragments)
        && docs_ok
        && policy_ok)
}
