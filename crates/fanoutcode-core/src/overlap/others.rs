//! Planning around the other tasks (§6.1, §6.2): the "Others working here" summaries, the footprint read from a
//! plan's `## Files`, the soft notes between tasks in separate trees, and the note to the project's other sessions
//! when a task starts. The shell delivers the notes; every decision is here.

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::{json, Value};

use super::compute::{shares_tree, OtherTask};
use super::footprint::{footprints_of, parse_files_section, FootprintEntry, FootprintRow, FootprintSource};
use super::prompts::{export_names, new_task_note, others_working_here, soft_note, OtherSummary};
use super::{store, OverlapRow, Relation};
use crate::changes::{self, repo_for, AttributedFile, RepoChanges};
use crate::domain::{Task, TaskId};
use crate::git::{valid_branch, FileStatus};
use crate::hooks::HookEvent;
use crate::{db, Error, Result};

const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

fn push_once(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|x| x == item) {
        list.push(item.to_owned());
    }
}

/// The first non-blank line of `intent.md`, its heading marks dropped; `None` when there is none.
fn first_line(md: &str) -> Option<String> {
    md.lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .map(str::to_owned)
}

/// One `OtherSummary` per other task that is not archived and is not `me`, in the order given: its intent (the
/// first line of `intent.md`, else its title), its `## Files` paths, the files of its change set (`files_of`), the
/// added ones, and the exported names of every file it added or changed (read through `read_file`).
pub fn summaries(
    me: &Task,
    others: &[Task],
    files_of: impl Fn(&TaskId) -> Vec<RepoChanges>,
    footprints: &[FootprintRow],
    branches: &HashMap<TaskId, String>,
    read_file: impl Fn(&Path) -> Option<String>,
) -> Vec<OtherSummary> {
    let mut out = Vec::new();
    for o in others.iter().filter(|o| o.id != me.id && !o.archived) {
        let mut planned = Vec::new();
        for f in footprints
            .iter()
            .filter(|f| f.task_id == o.id && f.source == FootprintSource::Plan)
        {
            push_once(&mut planned, &f.path);
        }
        let (mut touched, mut added_files, mut exported) = (Vec::new(), Vec::new(), Vec::new());
        for repo in files_of(&o.id) {
            for f in &repo.files {
                push_once(&mut touched, &f.path);
                if matches!(f.status, FileStatus::Added | FileStatus::Untracked) {
                    push_once(&mut added_files, &f.path);
                }
                if f.status != FileStatus::Deleted {
                    for name in read_file(&repo.repo_root.join(&f.path))
                        .map(|s| export_names(&s))
                        .unwrap_or_default()
                    {
                        push_once(&mut exported, &name);
                    }
                }
            }
        }
        let intent = read_file(&o.memory_dir.join("intent.md"))
            .and_then(|md| first_line(&md))
            .unwrap_or_else(|| o.title.clone());
        out.push(OtherSummary {
            title: o.title.clone(),
            stage: o.stage,
            state: o.state.clone(),
            branch: branches.get(&o.id).cloned(),
            intent,
            planned,
            touched,
            added_files,
            exported,
        });
    }
    out
}

/// The branch checked out in `tree`, read from its `.git` (a directory, or a worktree's `gitdir:` file) without
/// running git; `None` when detached, unreadable or not a `valid_branch`.
pub fn head_branch(tree: &Path, read_file: &impl Fn(&Path) -> Option<String>) -> Option<String> {
    let dot_git = tree.join(".git");
    let head = match read_file(&dot_git).and_then(|s| s.trim().strip_prefix("gitdir:").map(|d| d.trim().to_owned())) {
        Some(dir) => read_file(&tree.join(dir).join("HEAD")),
        None => read_file(&dot_git.join("HEAD")),
    }?;
    let name = head.trim().strip_prefix("ref: refs/heads/")?;
    valid_branch(name).then(|| name.to_owned())
}

fn home_of(conn: &Connection, task: &Task) -> Result<PathBuf> {
    db::projects::get(conn, &task.project_id)?
        .map(|p| p.root)
        .ok_or_else(|| Error::Invalid(format!("Unknown project {}", task.project_id)))
}

/// The repos a task works in, as logical repos: its home root and its linked roots.
fn logical_repos(task: &Task, home: &Path) -> BTreeSet<PathBuf> {
    std::iter::once(home.to_owned())
        .chain(task.linked_roots.iter().cloned())
        .collect()
}

/// The tree a task works in for its home repo: its worktree, else the home root.
fn home_tree(task: &Task, home: &Path) -> PathBuf {
    task.worktree_path.clone().unwrap_or_else(|| home.to_owned())
}

/// Every other task that is not archived and works in a repo `me` works in (its home root or a linked root), with
/// its home root, by id.
pub fn candidates(conn: &Connection, me: &Task) -> Result<Vec<(Task, PathBuf)>> {
    let mine = logical_repos(me, &home_of(conn, me)?);
    let mut out = Vec::new();
    for t in db::tasks::list_all(conn)? {
        if t.id == me.id || t.archived {
            continue;
        }
        let home = home_of(conn, &t)?;
        if !logical_repos(&t, &home).is_disjoint(&mine) {
            out.push((t, home));
        }
    }
    out.sort_by(|a, b| a.0.id.as_str().cmp(b.0.id.as_str()));
    Ok(out)
}

fn branch_of(task: &Task, home: &Path, read_file: &impl Fn(&Path) -> Option<String>) -> Option<String> {
    head_branch(&home_tree(task, home), read_file).or_else(|| task.branch.clone().filter(|b| valid_branch(b)))
}

/// The Plan and Build prompts' "Others working here" section for `me` (`""` with no other task), from the store,
/// the change sets `files_of` gives and the files `read_file` reads.
pub fn others_section(
    conn: &Connection,
    me: &Task,
    files_of: impl Fn(&TaskId) -> Vec<RepoChanges>,
    read_file: impl Fn(&Path) -> Option<String>,
) -> Result<String> {
    let found = candidates(conn, me)?;
    if found.is_empty() {
        return Ok(String::new());
    }
    let ids: Vec<TaskId> = found.iter().map(|(t, _)| t.id.clone()).collect();
    let footprints = footprints_of(conn, &ids)?;
    let branches: HashMap<TaskId, String> = found
        .iter()
        .filter_map(|(t, home)| branch_of(t, home, &read_file).map(|b| (t.id.clone(), b)))
        .collect();
    let tasks: Vec<Task> = found.into_iter().map(|(t, _)| t).collect();
    Ok(others_working_here(&summaries(
        me,
        &tasks,
        files_of,
        &footprints,
        &branches,
        read_file,
    )))
}

/// A soft note (§6.2) for task `to`'s session about `about`'s work on `path`; the trees are each task's own.
#[derive(Debug, Clone, PartialEq)]
pub struct SoftNote {
    pub to: TaskId,
    pub about: TaskId,
    pub text: String,
    pub path: String,
    pub to_tree: PathBuf,
    pub about_tree: PathBuf,
}

fn noted_line(repo: &Path, path: &str, other: &TaskId, direction: &str, at: DateTime<Utc>) -> String {
    let payload: Value = json!({ "repo": repo, "path": path, "other_task": other, "direction": direction });
    format!("OverlapNoted\t{}\t{payload}\n", at.format("%Y-%m-%dT%H:%M:%SZ"))
}

impl SoftNote {
    /// The display-only `OverlapNoted` lines for both event logs: `to`'s (heard, `"in"`) and `about`'s (told,
    /// `"out"`). The app never acts on them.
    pub fn log_lines(&self, at: DateTime<Utc>) -> [(TaskId, String); 2] {
        [
            (
                self.to.clone(),
                noted_line(&self.to_tree, &self.path, &self.about, "in", at),
            ),
            (
                self.about.clone(),
                noted_line(&self.about_tree, &self.path, &self.to, "out", at),
            ),
        ]
    }
}

fn find<'a>(rows: &'a [OverlapRow], tree: &Path, path: &str, other: &TaskId) -> Option<&'a OverlapRow> {
    rows.iter()
        .find(|r| r.repo_root == tree && r.path == path && r.other_task.as_ref() == Some(other))
}

/// Makes sure the `PlannedBy` row the overlap sync would derive exists, so its note flags have a home.
fn ensure_row(
    conn: &Connection,
    task: &TaskId,
    tree: &Path,
    path: &str,
    other: &TaskId,
    now: DateTime<Utc>,
) -> Result<()> {
    if find(&store::list(conn, task)?, tree, path, other).is_some() {
        return Ok(());
    }
    store::upsert(
        conn,
        &OverlapRow {
            task_id: task.clone(),
            other_task: Some(other.clone()),
            repo_root: tree.to_owned(),
            path: path.to_owned(),
            relation: Relation::PlannedBy,
            pending: None,
            held: false,
            noted_out: false,
            noted_in: false,
            decided_at: None,
            updated_at: now,
        },
    )
}

/// The soft notes an edit by `editor` of `path` in its tree `tree` sends (§6.2): for every other active task whose
/// footprint or change set names the file while the two work in different trees, once per direction (the note to
/// the editor and the mirror to the other task), recorded in both tasks' `noted_out`/`noted_in`.
pub fn soft_notes(
    conn: &Connection,
    editor: &TaskId,
    tree: &Path,
    path: &str,
    read_file: &impl Fn(&Path) -> Option<String>,
    now: DateTime<Utc>,
) -> Result<Vec<SoftNote>> {
    let me = db::tasks::require(conn, editor)?;
    let mut all = vec![(me.clone(), home_of(conn, &me)?)];
    all.extend(candidates(conn, &me)?);
    let ids: Vec<TaskId> = all.iter().map(|(t, _)| t.id.clone()).collect();
    let footprints = footprints_of(conn, &ids)?;
    let mut files: HashMap<TaskId, Vec<AttributedFile>> = HashMap::new();
    for id in &ids {
        files.insert(id.clone(), changes::store::load(conn, id)?);
    }
    let prints: HashMap<&TaskId, Vec<FootprintRow>> = ids
        .iter()
        .map(|id| (id, footprints.iter().filter(|f| &f.task_id == id).cloned().collect()))
        .collect();
    let views: Vec<OtherTask<'_>> = all
        .iter()
        .map(|(t, home)| OtherTask {
            task: t,
            files: &files[&t.id],
            footprint: &prints[&t.id],
            branch: branch_of(t, home, read_file),
            home_root: home,
        })
        .collect();
    let (mine, others) = views.split_first().expect("the editor is first");
    let logical = mine.logical(tree);
    let key = (logical.clone(), path.to_owned());
    let mut out = Vec::new();
    for o in others {
        if !o.paths().contains(&key) || shares_tree(mine.task, o.task, &logical) {
            continue;
        }
        let (my_tree, their_tree) = (mine.physical(&logical), o.physical(&logical));
        let (me_id, o_id) = (&mine.task.id, &o.task.id);
        let (my_rows, their_rows) = (store::list(conn, me_id)?, store::list(conn, o_id)?);
        let flags = |rows: &[OverlapRow], t: &Path, other: &TaskId| {
            find(rows, t, path, other).map_or((false, false), |r| (r.noted_out, r.noted_in))
        };
        let (my_out, my_in) = flags(&my_rows, &my_tree, o_id);
        let (their_out, their_in) = flags(&their_rows, &their_tree, me_id);
        let (to_me, to_them) = (!(my_out || their_in), !(my_in || their_out));
        if !to_me && !to_them {
            continue;
        }
        ensure_row(conn, me_id, &my_tree, path, o_id, now)?;
        ensure_row(conn, o_id, &their_tree, path, me_id, now)?;
        let branch = |v: &OtherTask<'_>| v.branch.clone().unwrap_or_else(|| "another branch".to_owned());
        if to_me {
            store::mark_noted(conn, me_id, &my_tree, path, Some(o_id), true)?;
            store::mark_noted(conn, o_id, &their_tree, path, Some(me_id), false)?;
            out.push(SoftNote {
                to: me_id.clone(),
                about: o_id.clone(),
                text: soft_note(&o.task.title, path, &branch(o)),
                path: path.to_owned(),
                to_tree: my_tree.clone(),
                about_tree: their_tree.clone(),
            });
        }
        if to_them {
            store::mark_noted(conn, me_id, &my_tree, path, Some(o_id), false)?;
            store::mark_noted(conn, o_id, &their_tree, path, Some(me_id), true)?;
            out.push(SoftNote {
                to: o_id.clone(),
                about: me_id.clone(),
                text: soft_note(&mine.task.title, path, &branch(mine)),
                path: path.to_owned(),
                to_tree: their_tree,
                about_tree: my_tree,
            });
        }
    }
    Ok(out)
}

/// The repo and repo-relative path a `PostToolUse` edit (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`) changed, when
/// its absolute path is inside one of `repos`.
pub fn edited_file(event: &HookEvent, repos: &[PathBuf]) -> Option<(PathBuf, String)> {
    let HookEvent::PostToolUse { tool, input, .. } = event else {
        return None;
    };
    if !EDIT_TOOLS.contains(&tool.as_str()) {
        return None;
    }
    let key = if tool == "NotebookEdit" {
        "notebook_path"
    } else {
        "file_path"
    };
    let abs = Path::new(input.get(key)?.as_str()?);
    let repo = repo_for(abs, repos)?;
    let rel = abs.strip_prefix(repo).ok()?.to_string_lossy().into_owned();
    (!rel.is_empty()).then(|| (repo.to_owned(), rel))
}

/// `path` as a clean path relative to one of `roots`: `./` dropped, an absolute path inside a root made relative;
/// `None` for a path outside them or one with `..`.
fn relative(path: &str, roots: &[PathBuf]) -> Option<String> {
    let p = Path::new(path);
    let rel = if p.is_absolute() {
        roots.iter().find_map(|r| p.strip_prefix(r).ok())?
    } else {
        p
    };
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::CurDir => {}
            Component::Normal(s) => parts.push(s.to_str()?),
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The footprint a plan names (§6.1): its `## Files` entries with every path relative to the home repo (`roots`
/// are the task's tree and its home root), each once; entries outside them are dropped.
pub fn plan_entries(plan_md: &str, roots: &[PathBuf]) -> Vec<FootprintEntry> {
    let mut out: Vec<FootprintEntry> = Vec::new();
    for e in parse_files_section(plan_md) {
        if let Some(path) = relative(&e.path, roots) {
            if !out.iter().any(|o| o.path == path) {
                out.push(FootprintEntry { path, note: e.note });
            }
        }
    }
    out
}

/// The note to every other active task of `me`'s project when `me` starts planning (§6.1): its title, its branch
/// and the files its plan names so far.
pub fn new_task_notes(
    conn: &Connection,
    me: &Task,
    read_file: &impl Fn(&Path) -> Option<String>,
) -> Result<Vec<(TaskId, String)>> {
    let home = home_of(conn, me)?;
    let branch = branch_of(me, &home, read_file).unwrap_or_else(|| "a new branch".to_owned());
    let mut planned = Vec::new();
    for f in footprints_of(conn, std::slice::from_ref(&me.id))? {
        if f.source == FootprintSource::Plan {
            push_once(&mut planned, &f.path);
        }
    }
    let note = new_task_note(&me.title, &branch, &planned);
    let mut out: Vec<(TaskId, String)> = db::tasks::list_all(conn)?
        .into_iter()
        .filter(|t| t.id != me.id && !t.archived && t.project_id == me.project_id)
        .map(|t| (t.id, note.clone()))
        .collect();
    out.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    Ok(out)
}

#[cfg(test)]
#[path = "others_tests.rs"]
mod tests;
