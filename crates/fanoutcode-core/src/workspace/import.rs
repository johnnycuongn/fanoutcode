//! Importing a workspace file that may come from someone else (a shared repo, a download): nothing is written until
//! the developer has seen what the file brings. `preview_open` reads the file once, canonicalises its roots and keeps
//! that exact buffer under a random token for a few minutes; `confirm_open` imports from that buffer, never from
//! the disk again, and refuses when a root now resolves elsewhere (a symlink retargeted). Fields that fail
//! validation are dropped and reported; roots that are not plain project folders block the open. A file this app
//! wrote for the record already at that path, adding nothing since, opens without a preview (`open_file`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::context_docs::ContextDocKind;
use super::file::WorkspaceFile;
pub use super::import_checks::sanitize;
use super::import_checks::{adds_nothing, canonical, record_at, root_problem};
use super::store::WorkspaceStore;
use crate::db;
use crate::domain::{LinkPolicy, Workspace, WorkspaceId};
use crate::{Error, Result};

/// Which imported references become trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocTrust {
    /// Shown in a preview the developer confirmed: one flag per reference, true only for those approved by id.
    Confirmed(Vec<bool>),
    /// Unchanged from the record: each keeps the trust of the matching stored doc.
    Existing,
}

/// How long a preview may wait for its confirm.
pub const PREVIEW_TTL: Duration = Duration::from_secs(10 * 60);

/// The folders a root may never be or contain: Fanout Code's own data and config, the home folder, `/`.
pub struct OpenEnv<'a> {
    pub home_dir: &'a Path,
    pub app_config: &'a Path,
}

/// One project root the file names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportRoot {
    /// Shown, not blocking: a dot-folder or a credential location that agents would be able to read and edit.
    pub warning: Option<String>,
    pub path: PathBuf,
    pub exists: bool,
    pub is_dir: bool,
    /// No project with this root is open in Fanout Code yet.
    pub new_to_app: bool,
    pub protected: bool,
}

/// One reference the file lists, as the prompts would name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportDoc {
    pub kind: ContextDocKind,
    pub target: String,
    pub label: String,
    pub page_id: Option<String>,
    /// The host a URL or Confluence reference would be fetched from.
    pub host: Option<String>,
    /// The doc's id in the file: what `confirm_open` approves file and folder references by.
    pub id: String,
    /// Always true: every imported reference (file, folder, link, Confluence page) is trusted only when approved one
    /// by one in the preview.
    pub needs_approval: bool,
    /// A file or folder outside every project the file names (a warning shown beside it).
    pub outside_projects: bool,
}

/// The warning shown beside such a reference.
pub const OUTSIDE_PROJECTS: &str = "Outside the workspace's projects";

/// What opening the file would bring in, shown before anything is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceImportPreview {
    pub name: String,
    pub roots: Vec<ImportRoot>,
    pub policy: LinkPolicy,
    pub agent_names: Vec<String>,
    pub fragment_stages: Vec<String>,
    pub settings_keys: Vec<String>,
    pub docs: Vec<ImportDoc>,
    /// Every dropped field and every refused root, as sentences.
    pub problems: Vec<String>,
    /// A refused root: `confirm_open` will fail.
    pub blocked: bool,
    /// False when the file is this app's own for the record at that path and adds nothing since its last save.
    pub needs_confirmation: bool,
    /// Pass to `confirm_open`; random, single use, valid for `PREVIEW_TTL`.
    pub token: String,
}

/// A preview waiting for its confirm: the file exactly as read and its roots as they resolved then.
pub(crate) struct PendingImport {
    path: PathBuf,
    file: WorkspaceFile,
    roots: Vec<PathBuf>,
    blocked: Option<String>,
    at: Instant,
}

/// 128 random bits, hex.
fn random_token() -> Result<String> {
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).map_err(|e| Error::Io(format!("No randomness available: {e}")))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn read_file(path: &Path) -> Result<(WorkspaceFile, Vec<String>)> {
    let bytes = std::fs::read(path)
        .map_err(|_| Error::Precondition("That workspace file no longer exists. Relocate or remove it.".into()))?;
    let stem = path
        .file_name()
        .map(|n| {
            n.to_string_lossy()
                .trim_end_matches(super::file::FILE_SUFFIX)
                .to_owned()
        })
        .unwrap_or_default();
    Ok(sanitize(WorkspaceFile::parse(&bytes)?, &stem))
}

/// Every root of `file` at `path`, canonicalised now.
fn resolve_roots(file: &WorkspaceFile, path: &Path) -> Vec<PathBuf> {
    file.absolute_project_paths(path).iter().map(|r| canonical(r)).collect()
}

/// What opening `path` would bring in. Writes nothing to the database; keeps the buffer for `confirm_open`.
pub fn preview_open(store: &WorkspaceStore, path: &Path, env: &OpenEnv) -> Result<WorkspaceImportPreview> {
    let path = canonical(path);
    let (mut file, mut problems) = read_file(&path)?;
    let roots = resolve_roots(&file, &path);
    // file and folder references are canonicalised now; the canonical path is what is shown, stored and allowed
    let mut outside = Vec::new();
    for d in &mut file.context_docs {
        let is_path = matches!(d.kind, ContextDocKind::File | ContextDocKind::Folder);
        if is_path {
            d.target = canonical(Path::new(&d.target)).to_string_lossy().into_owned();
        }
        outside.push(is_path && !roots.iter().any(|r| Path::new(&d.target).starts_with(r)));
    }
    let trusted = match record_at(store, &path)? {
        Some(record) => adds_nothing(store, &record, &file, &path)?,
        None => false,
    };
    for (d, out) in file.context_docs.iter().zip(&outside) {
        if *out {
            problems.push(format!("{}: {OUTSIDE_PROJECTS}.", d.target));
        }
    }
    let mut blocked = false;
    let mut shown = Vec::new();
    {
        let conn = store.db().lock().expect("db lock");
        for (fp, root) in file.projects.iter().zip(&roots) {
            // the app's own record may still list a folder that went missing; it shows as Missing, not refused
            if let Some(why) = root_problem(root, env, store.app_data()).filter(|_| !trusted) {
                blocked = true;
                problems.push(why);
            }
            let warning = super::import_checks::sensitive_root(root, env.home_dir);
            if let Some(w) = &warning {
                problems.push(w.clone());
            }
            shown.push(ImportRoot {
                warning,
                path: root.clone(),
                exists: root.exists(),
                is_dir: root.is_dir(),
                new_to_app: db::projects::find_by_root(&conn, root)?.is_none(),
                protected: fp.protected,
            });
        }
    }
    let token = random_token()?;
    let preview = WorkspaceImportPreview {
        name: file.name.clone(),
        roots: shown,
        policy: file.policy,
        agent_names: file.agents.keys().cloned().collect(),
        fragment_stages: file.prompt_fragments.keys().cloned().collect(),
        settings_keys: file.settings.keys().cloned().collect(),
        docs: file
            .context_docs
            .iter()
            .zip(&outside)
            .map(|(d, out)| ImportDoc {
                kind: d.kind,
                target: d.target.clone(),
                label: d.label.clone(),
                page_id: d.page_id.clone(),
                id: d.id.clone(),
                // every imported reference is approved one by one, whatever its kind
                needs_approval: true,
                outside_projects: *out,
                host: url::Url::parse(&d.target)
                    .ok()
                    .filter(|_| matches!(d.kind, ContextDocKind::Url | ContextDocKind::Confluence))
                    .and_then(|u| u.host_str().map(super::url_guard::normalise_host)),
            })
            .collect(),
        problems: problems.clone(),
        blocked,
        needs_confirmation: !trusted,
        token: token.clone(),
    };
    let mut pending = store.pending_imports();
    pending.retain(|_, p| p.at.elapsed() < PREVIEW_TTL);
    pending.insert(
        token,
        PendingImport {
            path,
            file,
            roots,
            blocked: blocked.then(|| problems.join(" ")),
            at: Instant::now(),
        },
    );
    Ok(preview)
}

/// Imports the file exactly as `preview_open` read it. The record at that path is reused; a file whose id belongs to
/// another record elsewhere is imported under a fresh id (written into the file on its next save) and inherits
/// nothing from that record. Refused when the token is unknown or expired, the preview was blocked, or a root now
/// resolves to a different folder.
/// `approved_doc_ids` names the references (by their id in the preview) the developer approved one by one; every
/// other reference, of any kind, is imported untrusted and never read, fetched nor named in prompts. A root the file
/// declares never implies approval.
pub fn confirm_open(
    store: &WorkspaceStore,
    token: &str,
    approved_doc_ids: &[String],
    env: &OpenEnv,
) -> Result<Workspace> {
    let review_again = || Error::Precondition("This review expired; open the workspace file again.".into());
    let pending = store.pending_imports().remove(token).ok_or_else(review_again)?;
    if pending.at.elapsed() >= PREVIEW_TTL {
        return Err(review_again());
    }
    if let Some(why) = pending.blocked {
        return Err(Error::Precondition(why));
    }
    let now = resolve_roots(&pending.file, &pending.path);
    if now != pending.roots || now.iter().any(|r| root_problem(r, env, store.app_data()).is_some()) {
        return Err(Error::Precondition(
            "A folder in this workspace changed since you reviewed it; open the file again.".into(),
        ));
    }
    let (target, rewritten) = match record_at(store, &pending.path)? {
        Some(record) => (record.id, false),
        None if store.exists(&pending.file.id)? => (WorkspaceId::new(), true),
        None => (pending.file.id.clone(), false),
    };
    // a reference is trusted only when the developer approved it by id in the preview
    let trust = pending
        .file
        .context_docs
        .iter()
        .map(|d| approved_doc_ids.contains(&d.id))
        .collect();
    super::open::import(
        store,
        &pending.path,
        &pending.file,
        &now,
        &target,
        rewritten,
        DocTrust::Confirmed(trust),
    )
}

/// The trusted reopen: the record at `path`, refreshed from the file, when the file adds nothing to it. A dirty
/// record is the truth and its file the stale mirror, so it opens as it is (nothing is read from the file). Anything
/// else needs `preview_open` and `confirm_open`.
pub fn open_file(store: &WorkspaceStore, path: &Path) -> Result<Workspace> {
    reopen(store, path, true)
}

/// `open_file`; `keep_unsaved: false` (Reload, discarding) refreshes a dirty record from its file too.
pub(crate) fn reopen(store: &WorkspaceStore, path: &Path, keep_unsaved: bool) -> Result<Workspace> {
    let path = canonical(path);
    let (file, _) = read_file(&path)?;
    match record_at(store, &path)? {
        Some(record) if record.dirty && keep_unsaved => Ok(record),
        Some(record) if adds_nothing(store, &record, &file, &path)? => {
            let roots = resolve_roots(&file, &path);
            super::open::import(store, &path, &file, &roots, &record.id, false, DocTrust::Existing)
        }
        _ => Err(Error::Precondition(
            "Review this workspace file before opening it.".into(),
        )),
    }
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
