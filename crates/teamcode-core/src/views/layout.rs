//! Pure layout tree operations. Each returns a new tree; `None` means the pane or path was not found.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::{LayoutNode, PaneContent, PaneId, ProjectId, SessionId, SplitDirection, TaskId};
use crate::{Error, Result};

/// One step of a path from the root to a `Split`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Branch {
    First,
    Second,
}

pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

/// Rebuilds the tree, replacing the leaf for `target` with `f(leaf)`; `None` when there is no such leaf.
fn map_leaf(root: &LayoutNode, target: &PaneId, f: &dyn Fn(&LayoutNode) -> LayoutNode) -> Option<LayoutNode> {
    match root {
        LayoutNode::Leaf { pane, .. } => (pane == target).then(|| f(root)),
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } => {
            let (first, second) = match map_leaf(first, target, f) {
                Some(new_first) => (new_first, (**second).clone()),
                None => ((**first).clone(), map_leaf(second, target, f)?),
            };
            Some(LayoutNode::Split {
                direction: *direction,
                ratio: *ratio,
                first: Box::new(first),
                second: Box::new(second),
            })
        }
    }
}

/// The target leaf becomes `Split { direction, 0.5, <the leaf>, <new empty pane> }`. `Horizontal` is side by side
/// (⌘D, "split right"), `Vertical` is stacked (⌘⇧D, "split down").
pub fn split(root: &LayoutNode, target: &PaneId, direction: SplitDirection, new_pane: PaneId) -> Option<LayoutNode> {
    map_leaf(root, target, &|leaf| LayoutNode::Split {
        direction,
        ratio: 0.5,
        first: Box::new(leaf.clone()),
        second: Box::new(LayoutNode::Leaf {
            pane: new_pane.clone(),
            content: PaneContent::Empty,
        }),
    })
}

fn contains(node: &LayoutNode, target: &PaneId) -> bool {
    match node {
        LayoutNode::Leaf { pane, .. } => pane == target,
        LayoutNode::Split { first, second, .. } => contains(first, target) || contains(second, target),
    }
}

/// The sibling replaces the parent split; closing the only leaf leaves it in place, empty.
pub fn close(root: &LayoutNode, target: &PaneId) -> Option<LayoutNode> {
    match root {
        LayoutNode::Leaf { pane, .. } if pane == target => Some(LayoutNode::Leaf {
            pane: pane.clone(),
            content: PaneContent::Empty,
        }),
        LayoutNode::Leaf { .. } => None,
        LayoutNode::Split { .. } => close_inner(root, target),
    }
}

fn close_inner(node: &LayoutNode, target: &PaneId) -> Option<LayoutNode> {
    let LayoutNode::Split {
        direction,
        ratio,
        first,
        second,
    } = node
    else {
        return None;
    };
    let is_target = |n: &LayoutNode| matches!(n, LayoutNode::Leaf { pane, .. } if pane == target);
    if is_target(first) {
        return Some((**second).clone());
    }
    if is_target(second) {
        return Some((**first).clone());
    }
    let (first, second) = if contains(first, target) {
        (close_inner(first, target)?, (**second).clone())
    } else {
        ((**first).clone(), close_inner(second, target)?)
    };
    Some(LayoutNode::Split {
        direction: *direction,
        ratio: *ratio,
        first: Box::new(first),
        second: Box::new(second),
    })
}

fn content_of<'a>(root: &'a LayoutNode, target: &PaneId) -> Option<&'a PaneContent> {
    leaves(root).into_iter().find(|(p, _)| *p == target).map(|(_, c)| c)
}

/// Swaps the two leaves' contents; pane ids stay where they are (dragging a pane header to another slot).
pub fn move_pane(root: &LayoutNode, from: &PaneId, to: &PaneId) -> Option<LayoutNode> {
    let a = content_of(root, from)?.clone();
    let b = content_of(root, to)?.clone();
    let swapped = set_content(root, from, b)?;
    set_content(&swapped, to, a)
}

/// Sets the ratio of the split at the end of `path`, clamped to 0.1..=0.9; `None` when the path does not end on a
/// split.
pub fn set_ratio(root: &LayoutNode, path: &[Branch], ratio: f32) -> Option<LayoutNode> {
    let LayoutNode::Split {
        direction,
        ratio: old,
        first,
        second,
    } = root
    else {
        return None;
    };
    let Some((step, rest)) = path.split_first() else {
        let ratio = if ratio.is_nan() {
            *old
        } else {
            ratio.clamp(MIN_RATIO, MAX_RATIO)
        };
        return Some(LayoutNode::Split {
            direction: *direction,
            ratio,
            first: first.clone(),
            second: second.clone(),
        });
    };
    let (first, second) = match step {
        Branch::First => (set_ratio(first, rest, ratio)?, (**second).clone()),
        Branch::Second => ((**first).clone(), set_ratio(second, rest, ratio)?),
    };
    Some(LayoutNode::Split {
        direction: *direction,
        ratio: *old,
        first: Box::new(first),
        second: Box::new(second),
    })
}

pub fn set_content(root: &LayoutNode, pane: &PaneId, content: PaneContent) -> Option<LayoutNode> {
    map_leaf(root, pane, &|leaf| match leaf {
        LayoutNode::Leaf { pane, .. } => LayoutNode::Leaf {
            pane: pane.clone(),
            content: content.clone(),
        },
        split => split.clone(),
    })
}

/// Every leaf, depth first, first before second.
pub fn leaves(root: &LayoutNode) -> Vec<(&PaneId, &PaneContent)> {
    match root {
        LayoutNode::Leaf { pane, content } => vec![(pane, content)],
        LayoutNode::Split { first, second, .. } => {
            let mut out = leaves(first);
            out.extend(leaves(second));
            out
        }
    }
}

/// Every leaf showing one of `tasks` (its page or its team canvas) becomes empty: the project left the workspace
/// (spec §4.1).
pub fn close_task_panes(root: &LayoutNode, tasks: &[TaskId]) -> LayoutNode {
    match root {
        LayoutNode::Leaf { pane, content } => {
            let shows = match content {
                PaneContent::Task { task_id } | PaneContent::TeamCanvas { task_id } => tasks.contains(task_id),
                _ => false,
            };
            let content = if shows { PaneContent::Empty } else { content.clone() };
            LayoutNode::Leaf {
                pane: pane.clone(),
                content,
            }
        }
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } => LayoutNode::Split {
            direction: *direction,
            ratio: *ratio,
            first: Box::new(close_task_panes(first, tasks)),
            second: Box::new(close_task_panes(second, tasks)),
        },
    }
}

/// The ids inside a pane's content are valid (serde's transparent `Deserialize` does not check them).
pub fn check_content(content: &PaneContent) -> Result<()> {
    match content {
        PaneContent::Empty => {}
        PaneContent::Task { task_id } | PaneContent::TeamCanvas { task_id } => {
            TaskId::parse(task_id.as_str())?;
        }
        PaneContent::Terminal { project_id, session_id } => {
            ProjectId::parse(project_id.as_str())?;
            SessionId::parse(session_id.as_str())?;
        }
    }
    Ok(())
}

/// Every pane id is valid and unique, every id inside a pane's content is valid, and every ratio is in 0.1..=0.9;
/// else `Invalid`.
pub fn validate(root: &LayoutNode) -> Result<()> {
    fn walk<'a>(node: &'a LayoutNode, seen: &mut HashSet<&'a str>) -> Result<()> {
        match node {
            LayoutNode::Leaf { pane, content } => {
                PaneId::parse(pane.as_str())?;
                check_content(content)?;
                if !seen.insert(pane.as_str()) {
                    return Err(Error::Invalid(format!("Pane {pane} appears twice in the layout")));
                }
                Ok(())
            }
            LayoutNode::Split {
                ratio, first, second, ..
            } => {
                if !(MIN_RATIO..=MAX_RATIO).contains(ratio) {
                    return Err(Error::Invalid(format!("Split ratio {ratio} is outside 0.1 to 0.9")));
                }
                walk(first, seen)?;
                walk(second, seen)
            }
        }
    }
    walk(root, &mut HashSet::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TaskId;

    fn pid(p: &str) -> PaneId {
        PaneId::parse(p).unwrap()
    }

    fn tid(t: &str) -> TaskId {
        TaskId::parse(t).unwrap()
    }

    fn leaf(p: &str) -> LayoutNode {
        LayoutNode::Leaf {
            pane: pid(p),
            content: PaneContent::Empty,
        }
    }

    #[test]
    fn split_right_puts_the_new_empty_pane_second_at_half() {
        let t = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-2"),
        )
        .unwrap();
        let LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } = t
        else {
            panic!()
        };
        assert_eq!((direction, ratio), (SplitDirection::Horizontal, 0.5));
        assert_eq!(*first, leaf("pane-1"));
        assert_eq!(*second, leaf("pane-2"));
        assert!(split(&leaf("pane-1"), &pid("pane-9"), SplitDirection::Vertical, pid("pane-2")).is_none());
    }

    #[test]
    fn close_collapses_to_the_sibling_and_the_last_pane_becomes_empty() {
        let t = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-2"),
        )
        .unwrap();
        let t = set_content(&t, &pid("pane-2"), PaneContent::Task { task_id: tid("t-1a-1") }).unwrap();
        let t = split(&t, &pid("pane-2"), SplitDirection::Vertical, pid("pane-3")).unwrap();
        let after = close(&t, &pid("pane-3")).unwrap();
        assert_eq!(leaves(&after).len(), 2);
        assert_eq!(leaves(&after)[1].1, &PaneContent::Task { task_id: tid("t-1a-1") });
        let only = set_content(
            &leaf("pane-1"),
            &pid("pane-1"),
            PaneContent::Task { task_id: tid("t-1a-1") },
        )
        .unwrap();
        assert_eq!(close(&only, &pid("pane-1")).unwrap(), leaf("pane-1"));
    }

    #[test]
    fn move_pane_swaps_contents_and_keeps_pane_ids_in_place() {
        let t = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-2"),
        )
        .unwrap();
        let t = set_content(&t, &pid("pane-1"), PaneContent::Task { task_id: tid("t-1a-1") }).unwrap();
        let t = move_pane(&t, &pid("pane-1"), &pid("pane-2")).unwrap();
        let l = leaves(&t);
        assert_eq!((l[0].0.as_str(), l[0].1), ("pane-1", &PaneContent::Empty));
        assert_eq!(
            (l[1].0.as_str(), l[1].1),
            ("pane-2", &PaneContent::Task { task_id: tid("t-1a-1") })
        );
    }

    #[test]
    fn set_ratio_follows_a_path_and_clamps() {
        let t = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-2"),
        )
        .unwrap();
        let t = split(&t, &pid("pane-2"), SplitDirection::Vertical, pid("pane-3")).unwrap();
        let t = set_ratio(&t, &[Branch::Second], 0.95).unwrap();
        let LayoutNode::Split { second, .. } = &t else { panic!() };
        let LayoutNode::Split { ratio, .. } = **second else {
            panic!()
        };
        assert_eq!(ratio, 0.9);
        assert!(set_ratio(&t, &[Branch::First], 0.3).is_none(), "First is a leaf");
        assert_eq!(
            set_ratio(&t, &[], 0.0).map(|n| match n {
                LayoutNode::Split { ratio, .. } => ratio,
                _ => -1.0,
            }),
            Some(0.1)
        );
    }

    #[test]
    fn close_task_panes_empties_only_those_tasks_and_validate_catches_duplicates() {
        let t = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-2"),
        )
        .unwrap();
        let t = set_content(&t, &pid("pane-1"), PaneContent::Task { task_id: tid("t-1a-1") }).unwrap();
        let t = set_content(&t, &pid("pane-2"), PaneContent::Task { task_id: tid("t-1a-2") }).unwrap();
        let t2 = close_task_panes(&t, &[tid("t-1a-1")]);
        assert_eq!(leaves(&t2)[0].1, &PaneContent::Empty);
        assert_eq!(leaves(&t2)[1].1, &PaneContent::Task { task_id: tid("t-1a-2") });
        assert!(validate(&t).is_ok());
        let dup = split(
            &leaf("pane-1"),
            &pid("pane-1"),
            SplitDirection::Horizontal,
            pid("pane-1"),
        )
        .unwrap();
        assert!(matches!(validate(&dup), Err(crate::Error::Invalid(_))));
        let forged: TaskId = serde_json::from_str("\"t-../x\"").unwrap();
        let bad = set_content(&t, &pid("pane-1"), PaneContent::Task { task_id: forged }).unwrap();
        assert!(
            matches!(validate(&bad), Err(crate::Error::Invalid(_))),
            "ids inside contents are checked too"
        );
    }
}
