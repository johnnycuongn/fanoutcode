//! Importing a workspace file that may come from someone else (a shared repo, a download): nothing is written until
//! the developer has seen what the file brings (`preview_open`, then `confirm_open` with the preview's token).
//! Fields that fail validation are dropped and reported; roots that are not plain project folders block the open.
//! A file this app wrote for the record already at that path, adding nothing since, opens without a preview.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::context_docs::ContextDocKind;
use super::file::WorkspaceFile;
use super::store::{clean_name, valid_agent_name, WorkspaceStore};
use crate::db;
use crate::domain::{LinkPolicy, Stage, Workspace, WorkspaceId};
use crate::settings::{validate, ScopeKind};
use crate::{Error, Result};

/// The folders a root may never be: Team Code's own data and config, the home folder itself, `/`.
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

/// What opening the file would bring in, shown before anything is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceImportPreview {
    pub name: String,
    pub roots: Vec<ImportRoot>,
    pub policy: LinkPolicy,
    pub agent_names: Vec<String>,
    pub fragment_stages: Vec<String>,
    pub settings_keys: Vec<String>,
    /// The targets of the context docs the file lists (files, folders, URLs read into prompts).
    pub doc_targets: Vec<String>,
    /// Every dropped field and every refused root, as sentences.
    pub problems: Vec<String>,
    /// A refused root: `confirm_open` will fail.
    pub blocked: bool,
    /// False when the file is this app's own for the record at that path and adds nothing since its last save.
    pub needs_confirmation: bool,
    /// Pass back to `confirm_open`; it changes when the file's bytes change.
    pub token: String,
}

/// FNV-1a over the file's bytes, so a file edited between preview and confirm is refused.
pub fn token_of(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
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
        let ok = key != "jira" && validate(key, ScopeKind::Workspace, value).is_ok();
        if !ok {
            problems.push(format!("Ignored setting {key:?}."));
        }
        ok
    });
    file.context_docs.retain(|d| {
        let ok = !matches!(d.kind, ContextDocKind::File | ContextDocKind::Folder) || Path::new(&d.target).is_absolute();
        if !ok {
            problems.push(format!("Ignored reference {:?}: not an absolute path.", d.target));
        }
        ok
    });
    (file, problems)
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_owned())
}

/// Why a root may not be imported, if it may not.
fn root_problem(root: &Path, env: &OpenEnv, app_data: &Path) -> Option<String> {
    if !root.is_dir() {
        return Some(format!("{} is not an existing folder.", root.display()));
    }
    let root = canonical(root);
    if root.parent().is_none() {
        return Some("The file names the whole disk (/) as a project.".to_owned());
    }
    if root == canonical(env.home_dir) {
        return Some("The file names your home folder itself as a project.".to_owned());
    }
    if [app_data, env.app_config]
        .iter()
        .any(|d| root.starts_with(canonical(d)))
    {
        return Some(format!("{} is inside Team Code's own data.", root.display()));
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
    let subset = |theirs: &BTreeMap<String, String>, mine: &BTreeMap<String, String>| {
        theirs.iter().all(|(k, v)| mine.get(k) == Some(v))
    };
    let docs_ok = file.context_docs.iter().all(|d| {
        mine.context_docs
            .iter()
            .any(|m| m.kind == d.kind && m.target == d.target)
    });
    let policy_ok = !(mine.policy == LinkPolicy::ReadOnly && file.policy != LinkPolicy::ReadOnly);
    Ok(roots_ok
        && subset(&file.agents, &mine.agents)
        && subset(&file.prompt_fragments, &mine.prompt_fragments)
        && docs_ok
        && policy_ok)
}

fn read_file(path: &Path) -> Result<(Vec<u8>, WorkspaceFile, Vec<String>)> {
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
    let (file, problems) = sanitize(WorkspaceFile::parse(&bytes)?, &stem);
    Ok((bytes, file, problems))
}

/// What opening `path` would bring in. Writes nothing.
pub fn preview_open(store: &WorkspaceStore, path: &Path, env: &OpenEnv) -> Result<WorkspaceImportPreview> {
    let (bytes, file, mut problems) = read_file(path)?;
    let trusted = match record_at(store, path)? {
        Some(record) => adds_nothing(store, &record, &file, path)?,
        None => false,
    };
    let mut blocked = false;
    let mut roots = Vec::new();
    {
        let conn = store.db().lock().expect("db lock");
        for (fp, root) in file.projects.iter().zip(file.absolute_project_paths(path)) {
            let root = canonical(&root);
            if let Some(why) = root_problem(&root, env, store.app_data()) {
                // the app's own record may still list a folder that went missing; it shows as Missing, not refused
                if !trusted {
                    blocked = true;
                    problems.push(why);
                }
            }
            roots.push(ImportRoot {
                exists: root.exists(),
                is_dir: root.is_dir(),
                new_to_app: db::projects::find_by_root(&conn, &root)?.is_none(),
                protected: fp.protected,
                path: root,
            });
        }
    }
    Ok(WorkspaceImportPreview {
        name: file.name.clone(),
        roots,
        policy: file.policy,
        agent_names: file.agents.keys().cloned().collect(),
        fragment_stages: file.prompt_fragments.keys().cloned().collect(),
        settings_keys: file.settings.keys().cloned().collect(),
        doc_targets: file.context_docs.iter().map(|d| d.target.clone()).collect(),
        problems,
        blocked,
        needs_confirmation: !trusted,
        token: token_of(&bytes),
    })
}

/// Imports `path` after the developer saw `preview_open`'s result with this `token`. The record at that path is
/// reused; a file whose id belongs to another record elsewhere is imported under a fresh id (written into the file on
/// its next save) and inherits nothing from that record.
pub fn confirm_open(store: &WorkspaceStore, path: &Path, token: &str, env: &OpenEnv) -> Result<Workspace> {
    let preview = preview_open(store, path, env)?;
    if preview.token != token {
        return Err(Error::Precondition(
            "The workspace file changed since you reviewed it; review it again.".into(),
        ));
    }
    if preview.blocked {
        return Err(Error::Precondition(preview.problems.join(" ")));
    }
    let (_, file, _) = read_file(path)?;
    let (target, rewritten) = match record_at(store, path)? {
        Some(record) => (record.id, false),
        None if store.exists(&file.id)? => (WorkspaceId::new(), true),
        None => (file.id.clone(), false),
    };
    super::open::import(store, path, &file, &target, rewritten)
}

/// The trusted reopen: the record at `path`, refreshed from the file, when the file adds nothing to it. Anything
/// else needs `preview_open` and `confirm_open`.
pub fn open_file(store: &WorkspaceStore, path: &Path) -> Result<Workspace> {
    let (_, file, _) = read_file(path)?;
    match record_at(store, path)? {
        Some(record) if adds_nothing(store, &record, &file, path)? => {
            super::open::import(store, path, &file, &record.id, false)
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
        assert!(matches!(
            confirm_open(&s, &path, "0000", &env),
            Err(Error::Precondition(_))
        ));
        assert_eq!(confirm_open(&s, &path, &p.token, &env).unwrap().id.as_str(), "ws-c1");
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
                matches!(confirm_open(&s, &path, &p.token, &env), Err(Error::Precondition(_))),
                "{bad}"
            );
        }
        assert!(s.list().unwrap().is_empty());
    }
}
