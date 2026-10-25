//! Which working-tree files belong to the task (§9): files its agents edited (`PostToolUse` of `Edit`, `Write`,
//! `MultiEdit`, `NotebookEdit`), files that appear in `git status` while one of its `Bash` calls runs, and files the
//! developer pulled in by hand.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{repo_for, AttributedFile, ChangeSource, MAIN_AGENT};
use crate::git::{FileStatus, StatusEntry};
use crate::hooks::{HookEvent, TaskEvent};

/// A `git status` the attributor needs: before a `Bash` call (`Snapshot`) or after it (`Delta`), in every repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    Snapshot { window: u64 },
    Delta { window: u64 },
}

/// One `Bash` call of one agent: the status per repo when it started.
#[derive(Debug, Clone, Default)]
struct Window {
    agent: String,
    before: HashMap<PathBuf, HashMap<String, FileStatus>>,
    /// The call finished; the repos still owed a delta.
    closing: Option<HashSet<PathBuf>>,
}

/// The edit tools whose `file_path` (`notebook_path`) is the file they wrote.
const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

#[derive(Debug, Clone, Default)]
pub struct Attributor {
    repos: Vec<PathBuf>,
    files: Vec<AttributedFile>,
    windows: BTreeMap<u64, Window>,
    next_window: u64,
}

impl Attributor {
    pub fn new(repos: Vec<PathBuf>) -> Self {
        Self {
            repos,
            ..Self::default()
        }
    }

    /// An attributor that already holds `files` (loaded from `task_files`).
    pub fn with_files(repos: Vec<PathBuf>, files: Vec<AttributedFile>) -> Self {
        Self {
            files,
            ..Self::new(repos)
        }
    }

    pub fn repos(&self) -> &[PathBuf] {
        &self.repos
    }

    fn add(&mut self, repo: &Path, path: &str, agent: &str, source: ChangeSource) -> Option<AttributedFile> {
        if self
            .files
            .iter()
            .any(|f| f.repo_root == repo && f.path == path && f.agent_id == agent)
        {
            return None;
        }
        let f = AttributedFile {
            repo_root: repo.to_owned(),
            path: path.to_owned(),
            agent_id: agent.to_owned(),
            source,
            added_at: Utc::now(),
        };
        self.files.push(f.clone());
        Some(f)
    }

    /// Edits attribute their file; a `Bash` call opens (`PreToolUse`) or closes (`PostToolUse`) a window and asks
    /// for the matching probe.
    pub fn on_event(&mut self, ev: &TaskEvent) -> Vec<Probe> {
        let agent = ev.agent_id.clone().unwrap_or_else(|| MAIN_AGENT.to_owned());
        match &ev.event {
            HookEvent::PreToolUse { tool, .. } if tool == "Bash" => {
                self.next_window += 1;
                let window = self.next_window;
                self.windows.insert(
                    window,
                    Window {
                        agent,
                        ..Window::default()
                    },
                );
                vec![Probe::Snapshot { window }]
            }
            HookEvent::PostToolUse { tool, .. } if tool == "Bash" => {
                let repos: HashSet<PathBuf> = self.repos.iter().cloned().collect();
                let open = self
                    .windows
                    .iter_mut()
                    .find(|(_, w)| w.agent == agent && w.closing.is_none());
                match open {
                    Some((&window, w)) => {
                        w.closing = Some(repos);
                        vec![Probe::Delta { window }]
                    }
                    None => Vec::new(),
                }
            }
            HookEvent::PostToolUse { tool, input, .. } if EDIT_TOOLS.contains(&tool.as_str()) => {
                let key = if tool == "NotebookEdit" {
                    "notebook_path"
                } else {
                    "file_path"
                };
                let Some(abs) = input.get(key).and_then(Value::as_str) else {
                    return Vec::new();
                };
                let abs = Path::new(abs);
                if let Some(repo) = repo_for(abs, &self.repos).map(Path::to_owned) {
                    if let Ok(rel) = abs.strip_prefix(&repo) {
                        let rel = rel.to_string_lossy().into_owned();
                        self.add(&repo, &rel, &agent, ChangeSource::Hook);
                    }
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// A `git status` of `repo` for `window`. The first one is the window's snapshot; every later one (the delta,
    /// or the git watcher while the call runs) attributes paths that are new or changed since, to the window's
    /// agent. A closed window is dropped once every repo has had its delta.
    pub fn on_status(&mut self, repo: &Path, window: u64, status: &[StatusEntry]) -> Vec<AttributedFile> {
        let Some(w) = self.windows.get_mut(&window) else {
            return Vec::new();
        };
        let now: HashMap<String, FileStatus> = status.iter().map(|e| (e.path.clone(), e.status)).collect();
        let mut fresh: Vec<String> = match w.before.get(repo) {
            Some(before) => now
                .iter()
                .filter(|(p, s)| before.get(*p) != Some(*s))
                .map(|(p, _)| p.clone())
                .collect(),
            // the first status is the snapshot; a delta that never had one cannot attribute anything safely
            None => {
                w.before.insert(repo.to_owned(), now);
                Vec::new()
            }
        };
        fresh.sort();
        let agent = w.agent.clone();
        if let Some(pending) = w.closing.as_mut() {
            pending.remove(repo);
            if pending.is_empty() {
                self.windows.remove(&window);
            }
        }
        fresh
            .into_iter()
            .filter_map(|path| self.add(repo, &path, &agent, ChangeSource::BashWindow))
            .collect()
    }

    /// The developer pulled a file from "Other changes" into the task's set.
    pub fn include(&mut self, repo: &Path, path: &str) -> AttributedFile {
        self.add(repo, path, MAIN_AGENT, ChangeSource::Manual)
            .unwrap_or_else(|| AttributedFile {
                repo_root: repo.to_owned(),
                path: path.to_owned(),
                agent_id: MAIN_AGENT.to_owned(),
                source: ChangeSource::Manual,
                added_at: Utc::now(),
            })
    }

    /// A hookless session's edits in a shared root (Plan 11, §4): every dirty path of `repo` that no agent has
    /// attributed yet (neither it nor, for a rename, its old name) is attributed to the main agent as `Observed`.
    /// The caller decides that this task's session is the only hookless one producing output there.
    pub fn observe_dirty(&mut self, repo: &Path, status: &[StatusEntry], now: DateTime<Utc>) -> Vec<AttributedFile> {
        let mut fresh = Vec::new();
        for e in status {
            let known = self
                .files
                .iter()
                .any(|f| f.repo_root == repo && (f.path == e.path || e.orig_path.as_deref() == Some(f.path.as_str())));
            if !known {
                let f = AttributedFile {
                    repo_root: repo.to_owned(),
                    path: e.path.clone(),
                    agent_id: MAIN_AGENT.to_owned(),
                    source: ChangeSource::Observed,
                    added_at: now,
                };
                self.files.push(f.clone());
                fresh.push(f);
            }
        }
        fresh
    }

    /// Drops every agent's attribution of `path` in `repo` (another task took it over, Plan 11 §5.4); whether there
    /// was one.
    pub fn remove(&mut self, repo: &Path, path: &str) -> bool {
        let before = self.files.len();
        self.files.retain(|f| !(f.repo_root == repo && f.path == path));
        self.files.len() != before
    }

    pub fn files(&self) -> &[AttributedFile] {
        &self.files
    }

    pub fn has_open_bash_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    /// The windows still waiting on a status, oldest first (the git watcher feeds them).
    pub fn open_windows(&self) -> Vec<u64> {
        self.windows.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, orig: Option<&str>) -> StatusEntry {
        StatusEntry {
            path: path.into(),
            status: if orig.is_some() {
                FileStatus::Renamed
            } else {
                FileStatus::Modified
            },
            orig_path: orig.map(str::to_owned),
            staged: false,
        }
    }

    #[test]
    fn observe_dirty_attributes_only_new_dirty_paths_as_observed() {
        let repo = Path::new("/r");
        let earlier: DateTime<Utc> = "2026-10-25T09:00:00Z".parse().unwrap();
        let now: DateTime<Utc> = "2026-10-25T10:00:00Z".parse().unwrap();
        let known = |path: &str, agent: &str, root: &str| AttributedFile {
            repo_root: root.into(),
            path: path.into(),
            agent_id: agent.into(),
            source: ChangeSource::Hook,
            added_at: earlier,
        };
        let mut a = Attributor::with_files(
            vec![repo.into()],
            vec![
                known("src/a.ts", "sub-1", "/r"),
                known("src/old.ts", MAIN_AGENT, "/r"),
                known("src/b.ts", MAIN_AGENT, "/elsewhere"),
            ],
        );
        let status = [
            entry("src/a.ts", None),
            entry("src/b.ts", None),
            entry("src/new.ts", Some("src/old.ts")),
        ];
        let fresh = a.observe_dirty(repo, &status, now);
        assert_eq!(
            fresh,
            vec![AttributedFile {
                repo_root: repo.into(),
                path: "src/b.ts".into(),
                agent_id: MAIN_AGENT.into(),
                source: ChangeSource::Observed,
                added_at: now,
            }]
        );
        assert_eq!(a.files().len(), 4);
        // seen once, never again
        assert!(a.observe_dirty(repo, &status, now).is_empty());
    }

    #[test]
    fn remove_drops_every_agent_s_attribution_of_the_path() {
        let repo = Path::new("/r");
        let f = |path: &str, agent: &str, root: &str| AttributedFile {
            repo_root: root.into(),
            path: path.into(),
            agent_id: agent.into(),
            source: ChangeSource::Hook,
            added_at: Utc::now(),
        };
        let mut a = Attributor::with_files(
            vec![repo.into()],
            vec![
                f("src/a.ts", MAIN_AGENT, "/r"),
                f("src/a.ts", "sub-1", "/r"),
                f("src/a.ts", MAIN_AGENT, "/elsewhere"),
                f("src/b.ts", MAIN_AGENT, "/r"),
            ],
        );
        assert!(a.remove(repo, "src/a.ts"));
        let left: Vec<(&str, &Path)> = a
            .files()
            .iter()
            .map(|f| (f.path.as_str(), f.repo_root.as_path()))
            .collect();
        assert_eq!(left, vec![("src/a.ts", Path::new("/elsewhere")), ("src/b.ts", repo)]);
        assert!(!a.remove(repo, "src/a.ts"));
    }
}
