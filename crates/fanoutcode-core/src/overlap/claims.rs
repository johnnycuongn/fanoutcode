//! The claims file per shared repo (§5.1): `<app_data>/claims/<repo-key>.json`, every dirty, attributed path of an
//! active task, rewritten atomically whenever attribution or `git status` changes. The claim-check hook reads it.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::changes::{AttributedFile, ChangeSource};
use crate::domain::{AgentState, Stage, Task, TaskId};
use crate::git::StatusEntry;
use crate::memory::store::write_atomic;
use crate::Result;

/// The title of a claim no task owns: a dirty file observed while several hookless sessions were active (§4).
pub const ANOTHER_SESSION: &str = "another session";

/// One path with uncommitted changes of one task (`task_id: None`: an unattributed hookless session).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Claim {
    pub path: String,
    pub task_id: Option<TaskId>,
    pub title: String,
    pub stage: Option<Stage>,
    pub state: Option<AgentState>,
    pub last_edit: DateTime<Utc>,
}

/// One repo's claims, sorted by path then task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ClaimsFile {
    pub repo_root: PathBuf,
    pub written_at: DateTime<Utc>,
    pub claims: Vec<Claim>,
}

/// A task and its attributed change set (every repo; `build_claims` keeps those of the repo it builds).
pub struct TaskFiles<'a> {
    pub task: &'a Task,
    pub files: &'a [AttributedFile],
}

/// FNV-1a, 64 bit: stable across Rust releases, unlike `DefaultHasher`.
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// 16 lowercase hex chars of a stable hash of the canonical repo root (the root as given when it cannot be
/// canonicalised).
pub fn repo_key(root: &Path) -> String {
    let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    format!("{:016x}", fnv1a64(canonical.as_os_str().as_encoded_bytes()))
}

/// `<app_data>/claims/<repo_key>.json`.
pub fn claims_path(app_data: &Path, root: &Path) -> PathBuf {
    app_data.join("claims").join(format!("{}.json", repo_key(root)))
}

fn dirty_set(status: &[StatusEntry]) -> HashSet<&str> {
    status.iter().map(|e| e.path.as_str()).collect()
}

/// A claim for every path of `root` that an active (not archived) task has attributed and `status` shows dirty; the
/// claim's `last_edit` is the task's latest attribution of the path.
pub fn build_claims(root: &Path, tasks: &[TaskFiles<'_>], status: &[StatusEntry], now: DateTime<Utc>) -> ClaimsFile {
    let dirty = dirty_set(status);
    let mut claims = Vec::new();
    for tf in tasks.iter().filter(|tf| !tf.task.archived) {
        let mut latest: BTreeMap<&str, DateTime<Utc>> = BTreeMap::new();
        for f in tf.files {
            if f.repo_root == root && dirty.contains(f.path.as_str()) {
                let at = latest.entry(f.path.as_str()).or_insert(f.added_at);
                *at = (*at).max(f.added_at);
            }
        }
        claims.extend(latest.into_iter().map(|(path, last_edit)| Claim {
            path: path.to_owned(),
            task_id: Some(tf.task.id.clone()),
            title: tf.task.title.clone(),
            stage: Some(tf.task.stage),
            state: Some(tf.task.state.clone()),
            last_edit,
        }));
    }
    sort(&mut claims);
    ClaimsFile {
        repo_root: root.to_path_buf(),
        written_at: now,
        claims,
    }
}

/// Adds a taskless *another session* claim for every dirty `Observed` file of `file.repo_root` in `observed` that no
/// claim already covers (several hookless sessions were active, so the edit could not be given to one, §4).
pub fn add_unattributed(file: &mut ClaimsFile, observed: &[AttributedFile], status: &[StatusEntry]) {
    let dirty = dirty_set(status);
    let mut seen: HashSet<String> = file.claims.iter().map(|c| c.path.clone()).collect();
    for f in observed {
        let fresh = f.source == ChangeSource::Observed
            && f.repo_root == file.repo_root
            && dirty.contains(f.path.as_str())
            && seen.insert(f.path.clone());
        if fresh {
            file.claims.push(Claim {
                path: f.path.clone(),
                task_id: None,
                title: ANOTHER_SESSION.to_owned(),
                stage: None,
                state: None,
                last_edit: f.added_at,
            });
        }
    }
    sort(&mut file.claims);
}

/// The entries of `now` that became dirty (or changed status) since `prev` and that no attribution in `owned` names
/// for `repo` (by path, or by a rename's old path): what a hookless session may have written (§4). `prev: None` is
/// the first status seen, the baseline, so nothing is new.
pub fn new_unowned_dirt(
    repo: &Path,
    prev: Option<&[StatusEntry]>,
    now: &[StatusEntry],
    owned: &[AttributedFile],
) -> Vec<StatusEntry> {
    let Some(prev) = prev else {
        return Vec::new();
    };
    let owned: HashSet<&str> = owned
        .iter()
        .filter(|f| f.repo_root == repo)
        .map(|f| f.path.as_str())
        .collect();
    now.iter()
        .filter(|e| !prev.iter().any(|p| p.path == e.path && p.status == e.status))
        .filter(|e| !owned.contains(e.path.as_str()) && !e.orig_path.as_deref().is_some_and(|o| owned.contains(o)))
        .cloned()
        .collect()
}

fn sort(claims: &mut [Claim]) {
    claims.sort_by(|a, b| (&a.path, &a.task_id).cmp(&(&b.path, &b.task_id)));
}

/// Writes `file` atomically to its `claims_path`, which it returns.
pub fn write_claims(app_data: &Path, file: &ClaimsFile) -> Result<PathBuf> {
    let path = claims_path(app_data, &file.repo_root);
    write_atomic(&path, &serde_json::to_vec(file)?)?;
    Ok(path)
}

/// The claims at `path`; `None` when the file is missing or does not parse (the hook then fails open).
pub fn read_claims(path: &Path) -> Option<ClaimsFile> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::changes::{AttributedFile, ChangeSource, MAIN_AGENT};
    use crate::domain::{AgentState, Stage, Task};
    use crate::git::{FileStatus, StatusEntry};
    use crate::test_support::task_fixture;

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn file(repo: &str, path: &str, source: ChangeSource, when: &str) -> AttributedFile {
        AttributedFile {
            repo_root: repo.into(),
            path: path.into(),
            agent_id: MAIN_AGENT.into(),
            source,
            added_at: at(when),
        }
    }

    fn dirty(paths: &[&str]) -> Vec<StatusEntry> {
        paths
            .iter()
            .map(|p| StatusEntry {
                path: (*p).into(),
                status: FileStatus::Modified,
                orig_path: None,
            })
            .collect()
    }

    fn task(id: &str, title: &str) -> Task {
        let mut t = task_fixture(id, "p-1a-1");
        t.title = title.into();
        t.stage = Stage::Build;
        t.state = AgentState::Working;
        t
    }

    const NOW: &str = "2026-10-25T10:00:00Z";

    #[test]
    fn claims_only_dirty_attributed_files_of_active_tasks() {
        let login = task("t-a1", "Login");
        let checkout = task("t-b2", "Checkout");
        let login_files = [
            file("/r", "src/a.ts", ChangeSource::Hook, "2026-10-25T09:00:00Z"),
            file("/r", "src/a.ts", ChangeSource::BashWindow, "2026-10-25T09:30:00Z"),
            file("/r", "src/clean.ts", ChangeSource::Hook, "2026-10-25T09:00:00Z"),
            file("/elsewhere", "src/a.ts", ChangeSource::Hook, "2026-10-25T09:00:00Z"),
        ];
        let checkout_files = [file("/r", "src/b.ts", ChangeSource::Hook, "2026-10-25T09:10:00Z")];
        let tasks = [
            TaskFiles {
                task: &login,
                files: &login_files,
            },
            TaskFiles {
                task: &checkout,
                files: &checkout_files,
            },
        ];
        let out = build_claims(
            Path::new("/r"),
            &tasks,
            &dirty(&["src/a.ts", "src/b.ts", "src/c.ts"]),
            at(NOW),
        );
        assert_eq!(out.repo_root, PathBuf::from("/r"));
        assert_eq!(out.written_at, at(NOW));
        assert_eq!(
            out.claims,
            vec![
                Claim {
                    path: "src/a.ts".into(),
                    task_id: Some(login.id.clone()),
                    title: "Login".into(),
                    stage: Some(Stage::Build),
                    state: Some(AgentState::Working),
                    last_edit: at("2026-10-25T09:30:00Z"),
                },
                Claim {
                    path: "src/b.ts".into(),
                    task_id: Some(checkout.id.clone()),
                    title: "Checkout".into(),
                    stage: Some(Stage::Build),
                    state: Some(AgentState::Working),
                    last_edit: at("2026-10-25T09:10:00Z"),
                },
            ]
        );
    }

    #[test]
    fn archived_tasks_never_claim() {
        let mut gone = task("t-a1", "Login");
        gone.archived = true;
        let files = [file("/r", "src/a.ts", ChangeSource::Hook, NOW)];
        let out = build_claims(
            Path::new("/r"),
            &[TaskFiles {
                task: &gone,
                files: &files,
            }],
            &dirty(&["src/a.ts"]),
            at(NOW),
        );
        assert!(out.claims.is_empty());
    }

    #[test]
    fn a_committed_file_drops_its_claim() {
        let login = task("t-a1", "Login");
        let files = [file("/r", "src/a.ts", ChangeSource::Hook, NOW)];
        let tasks = [TaskFiles {
            task: &login,
            files: &files,
        }];
        assert_eq!(
            build_claims(Path::new("/r"), &tasks, &dirty(&["src/a.ts"]), at(NOW))
                .claims
                .len(),
            1
        );
        assert!(build_claims(Path::new("/r"), &tasks, &[], at(NOW)).claims.is_empty());
    }

    #[test]
    fn observed_files_without_a_task_claim_as_another_session() {
        let login = task("t-a1", "Login");
        let files = [file("/r", "src/a.ts", ChangeSource::Hook, NOW)];
        let mut out = build_claims(
            Path::new("/r"),
            &[TaskFiles {
                task: &login,
                files: &files,
            }],
            &dirty(&["src/a.ts", "src/gen.ts", "src/old.ts"]),
            at(NOW),
        );
        let observed = [
            file("/r", "src/gen.ts", ChangeSource::Observed, "2026-10-25T09:50:00Z"),
            // already claimed by a task: not doubled
            file("/r", "src/a.ts", ChangeSource::Observed, NOW),
            // not Observed: ignored
            file("/r", "src/old.ts", ChangeSource::Hook, NOW),
            // not dirty: ignored
            file("/r", "src/clean.ts", ChangeSource::Observed, NOW),
        ];
        add_unattributed(&mut out, &observed, &dirty(&["src/a.ts", "src/gen.ts", "src/old.ts"]));
        assert_eq!(out.claims.len(), 2);
        assert_eq!(
            out.claims[1],
            Claim {
                path: "src/gen.ts".into(),
                task_id: None,
                title: ANOTHER_SESSION.into(),
                stage: None,
                state: None,
                last_edit: at("2026-10-25T09:50:00Z"),
            }
        );
    }

    #[test]
    fn repo_key_is_stable_and_path_safe() {
        let dir = tempfile::tempdir().unwrap();
        let key = repo_key(dir.path());
        assert_eq!(key.len(), 16);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(key, repo_key(dir.path()));
        // the canonical path is hashed, so a spelling through `..` names the same file
        let roundabout = dir.path().join("sub/..");
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        assert_eq!(repo_key(&roundabout), key);
        assert_ne!(
            repo_key(Path::new("/no/such/repo-a")),
            repo_key(Path::new("/no/such/repo-b"))
        );
        // FNV-1a 64 of the bytes, so the key never changes between Rust releases
        assert_eq!(
            repo_key(Path::new("/no/such/repo-a")),
            repo_key(Path::new("/no/such/repo-a"))
        );
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(
            claims_path(Path::new("/data"), dir.path()),
            PathBuf::from(format!("/data/claims/{key}.json"))
        );
    }

    #[test]
    fn write_then_read_round_trips() {
        let app_data = tempfile::tempdir().unwrap();
        let login = task("t-a1", "Login");
        let files = [file("/r", "src/a.ts", ChangeSource::Hook, NOW)];
        let built = build_claims(
            Path::new("/r"),
            &[TaskFiles {
                task: &login,
                files: &files,
            }],
            &dirty(&["src/a.ts"]),
            at(NOW),
        );
        let path = write_claims(app_data.path(), &built).unwrap();
        assert_eq!(path, claims_path(app_data.path(), Path::new("/r")));
        assert_eq!(read_claims(&path), Some(built));
    }

    #[test]
    fn read_returns_none_for_malformed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.json");
        assert_eq!(read_claims(&path), None);
        std::fs::write(&path, "{\"repo_root\": 3").unwrap();
        assert_eq!(read_claims(&path), None);
    }

    #[test]
    fn new_unowned_dirt_is_what_became_dirty_and_no_task_owns() {
        let root = Path::new("/r");
        let prev = dirty(&["src/old.ts", "src/both.ts"]);
        let mut now = dirty(&[
            "src/old.ts",
            "src/both.ts",
            "src/new.ts",
            "src/owned.ts",
            "src/moved.ts",
        ]);
        // a status change on an already dirty path counts as new dirt
        now[1].status = FileStatus::Deleted;
        now[4].orig_path = Some("src/was.ts".into());
        let owned = [
            file("/r", "src/owned.ts", ChangeSource::Hook, NOW),
            file("/r", "src/was.ts", ChangeSource::Hook, NOW),
            file("/elsewhere", "src/new.ts", ChangeSource::Hook, NOW),
        ];
        let fresh: Vec<String> = new_unowned_dirt(root, Some(&prev), &now, &owned)
            .into_iter()
            .map(|e| e.path)
            .collect();
        assert_eq!(fresh, vec!["src/both.ts".to_owned(), "src/new.ts".into()]);
        // without an earlier status there is nothing to compare: the first one is the baseline
        assert!(new_unowned_dirt(root, None, &now, &owned).is_empty());
    }
}
