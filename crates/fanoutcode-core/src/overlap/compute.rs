//! `overlaps_for` (§4, §7): the `task_overlaps` rows of one task, from the claims files (shared trees), the other
//! tasks' footprints and change sets (separate trees) and the rows already stored (decisions and notes survive).
//!
//! Paths are compared per *logical* repo: a task's worktree stands for its home root, so a file of the home repo is
//! the same file in every worktree of it. A row's `repo_root` is the physical tree of the task it belongs to.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::claims::ClaimsFile;
use super::footprint::FootprintRow;
use super::{OverlapRow, Relation};
use crate::changes::AttributedFile;
use crate::domain::{Task, TaskId};
use crate::git::StatusEntry;

/// A task as `overlaps_for` sees it: its attributed change set, its footprint, the branch its tree is on, and the
/// root of its home project (which its worktree, if any, is a tree of).
pub struct OtherTask<'a> {
    pub task: &'a Task,
    pub files: &'a [AttributedFile],
    pub footprint: &'a [FootprintRow],
    pub branch: Option<String>,
    pub home_root: &'a Path,
}

impl OtherTask<'_> {
    /// The repo a physical root of this task is a tree of: its worktree stands for its home root.
    fn logical(&self, root: &Path) -> PathBuf {
        if self.task.worktree_path.as_deref() == Some(root) {
            self.home_root.to_owned()
        } else {
            root.to_owned()
        }
    }

    /// The tree this task works in for the logical repo `repo`: its worktree for its home root, else `repo` itself.
    fn physical(&self, repo: &Path) -> PathBuf {
        match &self.task.worktree_path {
            Some(w) if repo == self.home_root && !self.task.linked_roots.iter().any(|r| r == repo) => w.clone(),
            _ => repo.to_owned(),
        }
    }

    /// Every (logical repo, path) of the change set and the footprint.
    fn paths(&self) -> BTreeSet<(PathBuf, String)> {
        let files = self.files.iter().map(|f| (self.logical(&f.repo_root), f.path.clone()));
        let planned = self
            .footprint
            .iter()
            .map(|f| (self.logical(&f.repo_root), f.path.clone()));
        files.chain(planned).collect()
    }
}

/// Whether `a` and `b` work in the same tree for `repo`: a linked root is always worked in place, so it is shared
/// whenever the other task works there too (linked, or in its home root without a worktree); two tasks in their home
/// root without a worktree share it when they are of the same project. A worktree is never shared.
pub fn shares_tree(a: &Task, b: &Task, repo: &Path) -> bool {
    let linked = |t: &Task| t.linked_roots.iter().any(|r| r == repo);
    let in_home = |t: &Task| !linked(t) && t.worktree_path.is_none();
    match (linked(a), linked(b)) {
        (true, true) => true,
        (true, false) => in_home(b),
        (false, true) => in_home(a),
        (false, false) => in_home(a) && in_home(b) && a.project_id == b.project_id,
    }
}

type Key = (PathBuf, String, Option<TaskId>);

fn key(row: &OverlapRow) -> Key {
    (row.repo_root.clone(), row.path.clone(), row.other_task.clone())
}

/// The rows of `me`:
/// - `Claimed` for every claim of another task (or of *another session*) on a path of my change set or footprint in a
///   tree I work in directly;
/// - `PlannedBy` for every path of mine that another active task's footprint or change set names while our trees for
///   that repo differ;
/// - `Committed` for a stored `Claimed` or `Committed` row whose claim went because the other task's file is clean
///   in `status_by_repo` (a repo without a status there keeps the stored row: it is not known yet).
///
/// Stored `GrantedOnTop` and `TakenOver` rows are kept as they are, and so is any stored row with an open decision
/// (the shell answers it once the claim is released). A recomputed row keeps the stored decision, hold, notes and
/// `decided_at`; its `updated_at` moves to `now` only when its relation changes. Sorted by repo, path, other task.
pub fn overlaps_for(
    me: &OtherTask<'_>,
    others: &[OtherTask<'_>],
    claims: &[ClaimsFile],
    status_by_repo: &[(PathBuf, Vec<StatusEntry>)],
    existing: &[OverlapRow],
    now: DateTime<Utc>,
) -> Vec<OverlapRow> {
    let mine = me.paths();
    let stored: BTreeMap<Key, &OverlapRow> = existing.iter().map(|r| (key(r), r)).collect();
    let mut out: BTreeMap<Key, Relation> = BTreeMap::new();

    for file in claims {
        let repo = me.logical(&file.repo_root);
        if me.physical(&repo) != file.repo_root {
            continue;
        }
        for c in &file.claims {
            if c.task_id.as_ref() != Some(&me.task.id) && mine.contains(&(repo.clone(), c.path.clone())) {
                out.insert(
                    (file.repo_root.clone(), c.path.clone(), c.task_id.clone()),
                    Relation::Claimed,
                );
            }
        }
    }

    for other in others.iter().filter(|o| o.task.id != me.task.id && !o.task.archived) {
        for (repo, path) in other.paths().intersection(&mine) {
            let tree = me.physical(repo);
            if tree != other.physical(repo) {
                out.entry((tree, path.clone(), Some(other.task.id.clone())))
                    .or_insert(Relation::PlannedBy);
            }
        }
    }

    for (k, row) in &stored {
        if out.contains_key(k) && !matches!(row.relation, Relation::GrantedOnTop | Relation::TakenOver) {
            continue;
        }
        let kept = match row.relation {
            Relation::GrantedOnTop | Relation::TakenOver => Some(row.relation),
            Relation::Claimed | Relation::Committed => released(row, others, status_by_repo),
            Relation::PlannedBy => None,
        };
        match kept.or(row.pending.as_ref().map(|_| row.relation)) {
            Some(relation) => {
                out.insert(k.clone(), relation);
            }
            None => {
                out.remove(k);
            }
        }
    }

    out.into_iter()
        .map(|((repo_root, path, other_task), relation)| {
            let k = (repo_root, path, other_task);
            match stored.get(&k) {
                Some(row) => OverlapRow {
                    relation,
                    updated_at: if row.relation == relation { row.updated_at } else { now },
                    ..(*row).clone()
                },
                None => OverlapRow {
                    task_id: me.task.id.clone(),
                    other_task: k.2,
                    repo_root: k.0,
                    path: k.1,
                    relation,
                    pending: None,
                    held: false,
                    noted_out: false,
                    noted_in: false,
                    decided_at: None,
                    updated_at: now,
                },
            }
        })
        .collect()
}

/// A stored `Claimed`/`Committed` row whose claim is gone: `Committed` once the other task's attributed file is
/// clean, the stored relation while the tree's status is unknown or the file is still dirty (the claims lag), else
/// nothing.
fn released(
    row: &OverlapRow,
    others: &[OtherTask<'_>],
    status_by_repo: &[(PathBuf, Vec<StatusEntry>)],
) -> Option<Relation> {
    let Some(status) = status_by_repo.iter().find(|(r, _)| *r == row.repo_root).map(|(_, s)| s) else {
        return Some(row.relation);
    };
    let dirty: HashSet<&str> = status.iter().map(|e| e.path.as_str()).collect();
    if dirty.contains(row.path.as_str()) {
        return Some(row.relation);
    }
    let other = others.iter().find(|o| Some(&o.task.id) == row.other_task.as_ref())?;
    other
        .files
        .iter()
        .any(|f| f.repo_root == row.repo_root && f.path == row.path)
        .then_some(Relation::Committed)
}

#[cfg(test)]
#[path = "compute_tests.rs"]
mod tests;
