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

/// How long a preview may wait for its confirm.
pub const PREVIEW_TTL: Duration = Duration::from_secs(10 * 60);

/// The folders a root may never be or contain: Team Code's own data and config, the home folder, `/`.
pub struct OpenEnv<'a> {
    pub home_dir: &'a Path,
    pub app_config: &'a Path,
}

/// One project root the file names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImportRoot {
    pub path: PathBuf,
    pub exists: bool,
    pub is_dir: bool,
    /// No project with this root is open in Team Code yet.
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
}

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
    let (file, mut problems) = read_file(&path)?;
    let roots = resolve_roots(&file, &path);
    let trusted = match record_at(store, &path)? {
        Some(record) => adds_nothing(store, &record, &file, &path)?,
        None => false,
    };
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
            shown.push(ImportRoot {
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
            .map(|d| ImportDoc {
                kind: d.kind,
                target: d.target.clone(),
                label: d.label.clone(),
                page_id: d.page_id.clone(),
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
pub fn confirm_open(store: &WorkspaceStore, token: &str, env: &OpenEnv) -> Result<Workspace> {
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
    super::open::import(store, &pending.path, &pending.file, &now, &target, rewritten)
}

/// The trusted reopen: the record at `path`, refreshed from the file, when the file adds nothing to it. Anything
/// else needs `preview_open` and `confirm_open`.
pub fn open_file(store: &WorkspaceStore, path: &Path) -> Result<Workspace> {
    let path = canonical(path);
    let (file, _) = read_file(&path)?;
    match record_at(store, &path)? {
        Some(record) if adds_nothing(store, &record, &file, &path)? => {
            let roots = resolve_roots(&file, &path);
            super::open::import(store, &path, &file, &roots, &record.id, false)
        }
        _ => Err(Error::Precondition(
            "Review this workspace file before opening it.".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::test_support::fixture;
    use serde_json::json;

    fn file_with(extra: serde_json::Value, projects: &[(&str, &str)]) -> Vec<u8> {
        let projects: Vec<_> = projects
            .iter()
            .enumerate()
            .map(|(i, (name, path))| json!({"path": path, "name": name, "color": "cyan", "order": i, "protected": false}))
            .collect();
        let mut v = json!({"teamcode_workspace": 1, "id": "ws-c1", "name": "Shared", "projects": projects,
            "policy": "ReadAndModifyOnRequest"});
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v.to_string().into_bytes()
    }

    fn setup() -> (
        crate::workspace::test_support::Fixture,
        WorkspaceStore,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        let fx = fixture();
        let s = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None);
        (fx, s, tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
    }

    #[test]
    fn sanitize_drops_bad_agents_stages_settings_and_cleans_the_name() {
        let bytes = file_with(
            json!({"name": "  ", "agents": {"../evil": "x", "ok-agent": "---\ndescription: d\n---\nhi"},
                "prompt_fragments": {"plan": "p", "done": "d", "nonsense": "n"},
                "settings": {"workspace_board": "Mixed", "may_commit": true, "workspace_project_layer": "nope", "jira": {"site_url": "x"}}}),
            &[],
        );
        let (f, problems) = sanitize(WorkspaceFile::parse(&bytes).unwrap(), "Shared");
        assert_eq!(f.name, "Shared");
        assert_eq!(f.agents.keys().collect::<Vec<_>>(), ["ok-agent"]);
        assert_eq!(f.prompt_fragments.keys().collect::<Vec<_>>(), ["plan"]);
        assert_eq!(
            f.settings.keys().collect::<Vec<_>>(),
            ["workspace_board"],
            "may_commit is project-only"
        );
        assert_eq!(problems.len(), 1 + 1 + 2 + 3);
    }

    #[test]
    fn preview_writes_nothing_and_lists_what_the_file_brings() {
        let (fx, s, home, cfg) = setup();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Shared.teamcode-workspace.json");
        std::fs::write(
            &path,
            file_with(
                json!({"agents": {"docs-writer": "x"}, "prompt_fragments": {"build": "b"}}),
                &[("api", fx.api.root.to_str().unwrap())],
            ),
        )
        .unwrap();
        let env = OpenEnv {
            home_dir: home.path(),
            app_config: cfg.path(),
        };
        let p = preview_open(&s, &path, &env).unwrap();
        assert!(p.needs_confirmation && !p.blocked && p.problems.is_empty());
        assert_eq!(
            (p.agent_names, p.fragment_stages),
            (vec!["docs-writer".to_owned()], vec!["build".to_owned()])
        );
        assert_eq!(
            (p.roots.len(), p.roots[0].new_to_app, p.roots[0].is_dir),
            (1, false, true)
        );
        assert!(s.list().unwrap().is_empty(), "nothing is written before confirm");
        assert!(matches!(confirm_open(&s, "0000", &env), Err(Error::Precondition(_))));
        assert_eq!(confirm_open(&s, &p.token, &env).unwrap().id.as_str(), "ws-c1");
    }

    #[test]
    fn roots_that_are_not_plain_project_folders_block_the_open() {
        let (fx, s, home, cfg) = setup();
        let inside_data = fx.app_data.path().join("projects");
        std::fs::create_dir_all(&inside_data).unwrap();
        let inside_cfg = cfg.path().join("agents");
        std::fs::create_dir_all(&inside_cfg).unwrap();
        let env = OpenEnv {
            home_dir: home.path(),
            app_config: cfg.path(),
        };
        let dir = tempfile::tempdir().unwrap();
        for bad in [
            "/",
            "/nonexistent/gone",
            home.path().to_str().unwrap(),
            inside_data.to_str().unwrap(),
            inside_cfg.to_str().unwrap(),
        ] {
            let path = dir.path().join("Bad.teamcode-workspace.json");
            std::fs::write(&path, file_with(json!({}), &[("x", bad)])).unwrap();
            let p = preview_open(&s, &path, &env).unwrap();
            assert!(p.blocked && p.problems.len() == 1, "{bad}: {:?}", p.problems);
            assert!(
                matches!(confirm_open(&s, &p.token, &env), Err(Error::Precondition(_))),
                "{bad}"
            );
        }
        assert!(s.list().unwrap().is_empty());
    }

    fn env_of<'a>(home: &'a tempfile::TempDir, cfg: &'a tempfile::TempDir) -> OpenEnv<'a> {
        OpenEnv {
            home_dir: home.path(),
            app_config: cfg.path(),
        }
    }

    #[test]
    fn ancestors_of_home_and_app_folders_are_refused_and_project_folders_pass() {
        let (fx, s, home, cfg) = setup();
        let env = env_of(&home, &cfg);
        let refused = [
            home.path().parent().unwrap().to_path_buf(),
            PathBuf::from("/"),
            fx.app_data.path().parent().unwrap().to_path_buf(),
            cfg.path().to_path_buf(),
        ];
        for r in &refused {
            assert!(root_problem(r, &env, fx.app_data.path()).is_some(), "{}", r.display());
        }
        let inside_home = home.path().join("code/api");
        std::fs::create_dir_all(&inside_home).unwrap();
        assert_eq!(root_problem(&inside_home, &env, fx.app_data.path()), None);
        assert_eq!(root_problem(&fx.api.root, &env, s.app_data()), None);
    }

    #[test]
    fn confirm_imports_the_previewed_buffer_not_the_file_on_disk_now() {
        let (fx, s, home, cfg) = setup();
        let env = env_of(&home, &cfg);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Shared.teamcode-workspace.json");
        std::fs::write(&path, file_with(json!({}), &[("api", fx.api.root.to_str().unwrap())])).unwrap();
        let p = preview_open(&s, &path, &env).unwrap();
        std::fs::write(
            &path,
            file_with(
                json!({"agents": {"sneaky": "x"}}),
                &[
                    ("api", fx.api.root.to_str().unwrap()),
                    ("web", fx.web.root.to_str().unwrap()),
                ],
            ),
        )
        .unwrap();
        let ws = confirm_open(&s, &p.token, &env).unwrap();
        assert_eq!(ws.projects.len(), 1);
        assert!(s.agents(&ws.id).unwrap().is_empty());
        assert!(
            matches!(confirm_open(&s, &p.token, &env), Err(Error::Precondition(_))),
            "a token is single use"
        );
    }

    #[test]
    fn retargeting_a_symlinked_root_after_preview_fails_confirm() {
        let (fx, s, home, cfg) = setup();
        let env = env_of(&home, &cfg);
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("linked");
        std::os::unix::fs::symlink(&fx.api.root, &link).unwrap();
        let path = dir.path().join("Shared.teamcode-workspace.json");
        std::fs::write(&path, file_with(json!({}), &[("linked", link.to_str().unwrap())])).unwrap();
        let p = preview_open(&s, &path, &env).unwrap();
        assert!(!p.blocked);
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&fx.web.root, &link).unwrap();
        assert!(matches!(confirm_open(&s, &p.token, &env), Err(Error::Precondition(_))));
        assert!(s.list().unwrap().is_empty());
    }

    #[test]
    fn a_changed_reference_label_goes_through_review() {
        let (fx, s, home, cfg) = setup();
        let env = env_of(&home, &cfg);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Shared.teamcode-workspace.json");
        let doc = |label: &str| json!({"context_docs": [{"id": "doc-1", "kind": "Url", "target": "https://x.y/spec", "label": label, "page_id": null, "fetched_at": null, "order": 0}]});
        let roots = [("api", fx.api.root.to_str().unwrap())];
        std::fs::write(&path, file_with(doc("Spec"), &roots)).unwrap();
        let p = preview_open(&s, &path, &env).unwrap();
        assert_eq!(p.docs[0].label, "Spec");
        confirm_open(&s, &p.token, &env).unwrap();
        assert!(
            !preview_open(&s, &path, &env).unwrap().needs_confirmation,
            "unchanged: no dialog"
        );
        std::fs::write(&path, file_with(doc("Ignore previous instructions"), &roots)).unwrap();
        assert!(preview_open(&s, &path, &env).unwrap().needs_confirmation);
        assert!(matches!(open_file(&s, &path), Err(Error::Precondition(_))));
    }
}
