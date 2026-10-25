//! The per-task grants file (§5.2): `<app_data>/grants/<task-id>.json`, the developer's answers that let this task
//! edit on top of another task's claim. Written by the app when the developer decides, read by the claim-check hook,
//! pruned once the claims it answered are gone (§4).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::claims::ClaimsFile;
use crate::domain::TaskId;
use crate::memory::store::write_atomic;
use crate::Result;

/// What a grant covers: one file of one repo or every file the other task claims (edits only), or one run of one
/// exact shell command over the other task's claims (the git guard, §5.6; edit grants never allow a command).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum GrantScope {
    File {
        repo: PathBuf,
        path: String,
    },
    AllFiles,
    /// Single use: removed from the grants file once the hook allows the command (`consume_command`).
    Command {
        command: String,
    },
}

/// One answer: this task may edit on top of `other_task`'s claims in `scope` (`other_task: None`: an unattributed
/// *another session* claim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Grant {
    pub other_task: Option<TaskId>,
    pub scope: GrantScope,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct GrantsFile {
    pub grants: Vec<Grant>,
}

/// `<app_data>/grants/<task-id>.json`. The id was validated when it was parsed, so it is safe in a path.
pub fn grants_path(app_data: &Path, task_id: &TaskId) -> PathBuf {
    app_data.join("grants").join(format!("{}.json", task_id.as_str()))
}

/// The grants at `path`; empty when the file is missing or does not parse (the hook then asks again, never allows
/// more than it was told).
pub fn read_grants(path: &Path) -> GrantsFile {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Writes `file` atomically to the task's `grants_path` (the id re-validated, since it becomes a file name).
pub fn write_grants(app_data: &Path, task_id: &TaskId, file: &GrantsFile) -> Result<PathBuf> {
    let id = TaskId::parse(task_id.as_str())?;
    let path = grants_path(app_data, &id);
    write_atomic(&path, &serde_json::to_vec(file)?)?;
    Ok(path)
}

impl GrantsFile {
    /// Whether a grant lets this task write `path` of `repo` over the claim of `other` (`None`: unattributed). A
    /// grant only ever covers claims of the task it names.
    pub fn covers(&self, repo: &Path, path: &str, other: Option<&TaskId>) -> bool {
        self.grants.iter().any(|g| {
            g.other_task.as_ref() == other
                && match &g.scope {
                    GrantScope::AllFiles => true,
                    GrantScope::File { repo: r, path: p } => r == repo && p == path,
                    GrantScope::Command { .. } => false,
                }
        })
    }

    /// Whether a `Command` grant lets this task run exactly `command` (both trimmed) over the claims of `other`.
    pub fn allows_command(&self, command: &str, other: Option<&TaskId>) -> bool {
        self.grants.iter().any(|g| {
            g.other_task.as_ref() == other
                && matches!(&g.scope, GrantScope::Command { command: c } if c.trim() == command.trim())
        })
    }

    /// Adds `grant`, replacing an earlier one with the same other task and scope.
    pub fn add(&mut self, grant: Grant) {
        self.grants
            .retain(|g| !(g.other_task == grant.other_task && g.scope == grant.scope));
        self.grants.push(grant);
    }

    /// Drops grants whose claims are gone: a File grant whose path is no longer claimed by its other task, an
    /// AllFiles grant whose other task holds no claim in any of `claims`.
    pub fn prune(&mut self, claims: &[&ClaimsFile]) {
        self.grants.retain(|g| match &g.scope {
            GrantScope::File { repo, path } => claims
                .iter()
                .filter(|f| &f.repo_root == repo)
                .any(|f| f.claims.iter().any(|c| &c.path == path && c.task_id == g.other_task)),
            GrantScope::AllFiles | GrantScope::Command { .. } => claims
                .iter()
                .any(|f| f.claims.iter().any(|c| c.task_id == g.other_task)),
        });
    }
}

/// Spends the `Command` grants for exactly `command` in the grants file at `path`, once the hook has allowed it (a
/// command grant is single use). Leaves the file alone when none matches; a failed write is reported on stderr (the
/// hook fails open).
pub fn consume_command(path: &Path, command: &str) {
    // TODO(grants-file lock): this read-modify-write races the app's `write_grants`; take the grants-file lock being
    // added to `AppState` (wired at merge) around it.
    let mut file = read_grants(path);
    let before = file.grants.len();
    file.grants
        .retain(|g| !matches!(&g.scope, GrantScope::Command { command: c } if c.trim() == command.trim()));
    if file.grants.len() == before {
        return;
    }
    let written = serde_json::to_vec(&file)
        .map_err(crate::Error::from)
        .and_then(|bytes| write_atomic(path, &bytes));
    if let Err(e) = written {
        eprintln!("fanoutcode claim-check: cannot update {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::overlap::claims::Claim;

    fn id(s: &str) -> TaskId {
        TaskId::parse(s).unwrap()
    }

    fn at() -> DateTime<Utc> {
        "2026-10-25T10:00:00Z".parse().unwrap()
    }

    fn file_grant(other: Option<&str>, repo: &str, path: &str) -> Grant {
        Grant {
            other_task: other.map(id),
            scope: GrantScope::File {
                repo: repo.into(),
                path: path.into(),
            },
            granted_at: at(),
        }
    }

    fn all_grant(other: &str) -> Grant {
        Grant {
            other_task: Some(id(other)),
            scope: GrantScope::AllFiles,
            granted_at: at(),
        }
    }

    fn claims(repo: &str, held: &[(&str, Option<&str>)]) -> ClaimsFile {
        ClaimsFile {
            repo_root: repo.into(),
            written_at: at(),
            claims: held
                .iter()
                .map(|(path, task)| Claim {
                    path: (*path).into(),
                    task_id: task.map(id),
                    title: "x".into(),
                    stage: None,
                    state: None,
                    last_edit: at(),
                })
                .collect(),
        }
    }

    #[test]
    fn file_grant_covers_exactly_its_path() {
        let g = GrantsFile {
            grants: vec![file_grant(Some("t-b2"), "/r", "src/a.ts")],
        };
        let b = id("t-b2");
        assert!(g.covers(Path::new("/r"), "src/a.ts", Some(&b)));
        assert!(!g.covers(Path::new("/r"), "src/b.ts", Some(&b)));
        assert!(!g.covers(Path::new("/other"), "src/a.ts", Some(&b)));
        assert!(!g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-c3"))));
    }

    #[test]
    fn all_files_grant_covers_only_the_named_task() {
        let g = GrantsFile {
            grants: vec![all_grant("t-b2")],
        };
        assert!(g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-b2"))));
        assert!(g.covers(Path::new("/s"), "lib/z.rs", Some(&id("t-b2"))));
        assert!(!g.covers(Path::new("/r"), "src/a.ts", Some(&id("t-c3"))));
        assert!(!g.covers(Path::new("/r"), "src/a.ts", None));
    }

    #[test]
    fn a_grant_with_no_other_task_matches_an_unattributed_claim() {
        let g = GrantsFile {
            grants: vec![file_grant(None, "/r", "src/gen.ts")],
        };
        assert!(g.covers(Path::new("/r"), "src/gen.ts", None));
        assert!(!g.covers(Path::new("/r"), "src/gen.ts", Some(&id("t-b2"))));
    }

    #[test]
    fn add_replaces_the_same_grant() {
        let mut g = GrantsFile::default();
        g.add(all_grant("t-b2"));
        g.add(all_grant("t-b2"));
        g.add(file_grant(Some("t-b2"), "/r", "a"));
        assert_eq!(g.grants.len(), 2);
    }

    #[test]
    fn prune_drops_grants_without_claims() {
        let mut g = GrantsFile {
            grants: vec![
                file_grant(Some("t-b2"), "/r", "src/a.ts"),    // still claimed by B: kept
                file_grant(Some("t-b2"), "/r", "src/gone.ts"), // committed: dropped
                file_grant(Some("t-c3"), "/r", "src/a.ts"),    // a.ts is B's, not C's: dropped
                file_grant(None, "/s", "gen.ts"),              // unattributed claim in /s: kept
                all_grant("t-b2"),                             // B claims something: kept
                all_grant("t-d4"),                             // D claims nothing: dropped
            ],
        };
        let r = claims("/r", &[("src/a.ts", Some("t-b2"))]);
        let s = claims("/s", &[("gen.ts", None)]);
        g.prune(&[&r, &s]);
        assert_eq!(
            g.grants,
            vec![
                file_grant(Some("t-b2"), "/r", "src/a.ts"),
                file_grant(None, "/s", "gen.ts"),
                all_grant("t-b2"),
            ]
        );
    }

    #[test]
    fn a_command_grant_covers_no_edit_and_matches_its_command_trimmed() {
        let g = GrantsFile {
            grants: vec![Grant {
                other_task: Some(id("t-b2")),
                scope: GrantScope::Command {
                    command: "git stash".into(),
                },
                granted_at: at(),
            }],
        };
        let b = id("t-b2");
        assert!(!g.covers(Path::new("/r"), "src/a.ts", Some(&b)));
        assert!(g.allows_command(" git stash\n", Some(&b)));
        assert!(!g.allows_command("git stash -u", Some(&b)));
        assert!(!g.allows_command("git stash", Some(&id("t-c3"))));
        assert!(!g.allows_command("git stash", None));
        let dir = tempfile::tempdir().unwrap();
        let path = write_grants(dir.path(), &id("t-a1"), &g).unwrap();
        consume_command(&path, "git reset --hard");
        assert_eq!(read_grants(&path), g, "another command spends nothing");
        consume_command(&path, "git stash");
        assert!(read_grants(&path).grants.is_empty());
    }

    #[test]
    fn read_is_empty_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_grants(&dir.path().join("none.json")), GrantsFile::default());
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "{\"grants\": [").unwrap();
        assert_eq!(read_grants(&bad), GrantsFile::default());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let file = GrantsFile {
            grants: vec![file_grant(Some("t-b2"), "/r", "src/a.ts"), all_grant("t-c3")],
        };
        let path = write_grants(dir.path(), &id("t-a1"), &file).unwrap();
        assert_eq!(path, dir.path().join("grants/t-a1.json"));
        assert_eq!(read_grants(&path), file);
    }

    #[test]
    fn grant_scope_is_internally_tagged() {
        for scope in [
            GrantScope::AllFiles,
            GrantScope::File {
                repo: "/r".into(),
                path: "a.ts".into(),
            },
            GrantScope::Command {
                command: "git stash".into(),
            },
        ] {
            let v = serde_json::to_value(&scope).unwrap();
            assert_eq!(serde_json::from_value::<GrantScope>(v).unwrap(), scope);
        }
        assert_eq!(
            serde_json::to_value(GrantScope::AllFiles).unwrap(),
            json!({"type": "AllFiles"})
        );
        assert_eq!(
            serde_json::to_value(file_grant(None, "/r", "a.ts")).unwrap()["scope"],
            json!({"type": "File", "repo": "/r", "path": "a.ts"})
        );
    }
}
