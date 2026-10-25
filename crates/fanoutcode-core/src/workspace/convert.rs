//! Container conversions: closing a removed project's panes, and moving project windows' views into a workspace.

use std::collections::HashSet;

use rusqlite::params;

use crate::db::{self, Db};
use crate::domain::{Container, LayoutNode, PaneContent, PaneId, ProjectId, TaskId, WorkspaceId};
use crate::Result;

fn closes(content: &PaneContent, project_id: &ProjectId, tasks: &HashSet<TaskId>) -> bool {
    match content {
        PaneContent::Task { task_id } | PaneContent::TeamCanvas { task_id } => tasks.contains(task_id),
        PaneContent::Terminal { project_id: p, .. } => p == project_id,
        PaneContent::Empty => false,
    }
}

/// `None` when the whole subtree closes.
fn prune(
    node: &LayoutNode,
    project_id: &ProjectId,
    tasks: &HashSet<TaskId>,
    closed: &mut Vec<PaneId>,
) -> Option<LayoutNode> {
    match node {
        LayoutNode::Leaf { pane, content } => {
            if closes(content, project_id, tasks) {
                closed.push(pane.clone());
                None
            } else {
                Some(node.clone())
            }
        }
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            match (
                prune(first, project_id, tasks, closed),
                prune(second, project_id, tasks, closed),
            ) {
                (Some(a), Some(b)) => Some(LayoutNode::Split {
                    direction: *direction,
                    ratio: *ratio,
                    first: Box::new(a),
                    second: Box::new(b),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            }
        }
    }
}

/// Removes every pane showing one of the project's tasks (task page or team canvas) or one of its terminals. A
/// split left with one child collapses to it; a layout with nothing left becomes an empty leaf keeping the first
/// closed pane's id. Returns the closed pane ids so the shell can detach their sessions.
pub fn close_panes_for_project(
    layout: &LayoutNode,
    project_id: &ProjectId,
    project_tasks: &HashSet<TaskId>,
) -> (LayoutNode, Vec<PaneId>) {
    let mut closed = Vec::new();
    let out = prune(layout, project_id, project_tasks, &mut closed).unwrap_or_else(|| LayoutNode::Leaf {
        pane: closed.first().cloned().unwrap_or_else(PaneId::new),
        content: PaneContent::Empty,
    });
    (out, closed)
}

/// Moves every view of each source container to the workspace, keeping view ids, pane ids and layouts; the moved
/// views are ordered after the workspace's own. Returns how many moved.
pub fn merge_containers(db: &Db, sources: &[Container], target: &WorkspaceId) -> Result<usize> {
    let conn = db.lock().expect("db lock");
    let to = Container::Workspace { id: target.clone() };
    let tx = conn.unchecked_transaction()?;
    let mut order = db::views::next_order(&tx, &to)?;
    let mut moved = 0;
    for source in sources.iter().filter(|s| **s != to) {
        for view in db::views::list(&tx, source)? {
            tx.execute(
                "UPDATE views SET container_kind = 'Workspace', container_id = ?1, \"order\" = ?2 WHERE id = ?3",
                params![target.as_str(), order, view.id.as_str()],
            )?;
            order += 1;
            moved += 1;
        }
    }
    tx.commit()?;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PaneContent::*, SessionId, SplitDirection, View, ViewId};
    use crate::workspace::store::WorkspaceStore;
    use crate::workspace::test_support::fixture;

    fn tid(s: &str) -> TaskId {
        TaskId::parse(s).unwrap()
    }
    fn pid(s: &str) -> ProjectId {
        ProjectId::parse(s).unwrap()
    }
    fn leaf(pane: &str, content: PaneContent) -> LayoutNode {
        LayoutNode::Leaf {
            pane: PaneId::parse(pane).unwrap(),
            content,
        }
    }
    fn split(a: LayoutNode, b: LayoutNode) -> LayoutNode {
        LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(a),
            second: Box::new(b),
        }
    }
    fn set(ids: &[&str]) -> HashSet<TaskId> {
        ids.iter().map(|i| tid(i)).collect()
    }
    fn seed_view(db: &Db, c: Container, id: &str, order: u32) {
        let v = View {
            id: ViewId::parse(id).unwrap(),
            container: c,
            name: id.into(),
            order,
            layout: leaf("pane-9", Empty),
        };
        db::views::insert(&db.lock().unwrap(), &v).unwrap();
    }
    fn views_for(db: &Db, c: &Container) -> Vec<View> {
        db::views::list(&db.lock().unwrap(), c).unwrap()
    }

    #[test]
    fn closing_leaf_collapses_split_to_sibling_and_reports_pane() {
        let layout = split(
            leaf("pane-1", Task { task_id: tid("t-a1") }),
            leaf("pane-2", Task { task_id: tid("t-b2") }),
        );
        let (out, closed) = close_panes_for_project(&layout, &pid("p-a1"), &set(&["t-a1"]));
        assert_eq!(out, leaf("pane-2", Task { task_id: tid("t-b2") }));
        assert_eq!(closed, vec![PaneId::parse("pane-1").unwrap()]);
    }

    #[test]
    fn root_leaf_becomes_empty_and_terminals_close_too() {
        let t = leaf(
            "pane-1",
            Terminal {
                project_id: pid("p-a1"),
                session_id: SessionId::parse("s-1").unwrap(),
            },
        );
        let (out, _) = close_panes_for_project(&t, &pid("p-a1"), &set(&[]));
        assert_eq!(out, leaf("pane-1", Empty));
    }

    #[test]
    fn unrelated_panes_untouched() {
        let layout = split(leaf("pane-1", Task { task_id: tid("t-b2") }), leaf("pane-2", Empty));
        assert_eq!(
            close_panes_for_project(&layout, &pid("p-a1"), &set(&["t-a1"])).0,
            layout
        );
    }

    #[test]
    fn merge_moves_views_from_each_source_and_keeps_ids() {
        let fx = fixture();
        seed_view(&fx.db, Container::Project { id: fx.api.id.clone() }, "v-1", 0);
        seed_view(&fx.db, Container::Project { id: fx.web.id.clone() }, "v-2", 0);
        let ws = WorkspaceStore::new(fx.db.clone(), fx.app_data.path().into(), None)
            .create("Shop", &[fx.api.id.clone(), fx.web.id.clone()])
            .unwrap();
        let sources = [
            Container::Project { id: fx.api.id.clone() },
            Container::Project { id: fx.web.id.clone() },
        ];
        assert_eq!(merge_containers(&fx.db, &sources, &ws.id).unwrap(), 2);
        let views = views_for(&fx.db, &Container::Workspace { id: ws.id.clone() });
        assert_eq!(
            views.iter().map(|v| (v.id.as_str(), v.order)).collect::<Vec<_>>(),
            vec![("v-1", 0), ("v-2", 1)]
        );
        assert!(views_for(&fx.db, &Container::Project { id: fx.api.id.clone() }).is_empty());
    }
}
