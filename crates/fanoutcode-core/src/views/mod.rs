//! Views (§5): named, ordered layouts of panes per container. The tree operations are pure (`layout`); the rows are
//! `db::views`.

use crate::domain::{Container, LayoutNode, PaneContent, PaneId, View, ViewId};

/// A fresh view holding one empty pane.
pub fn new_view(container: Container, name: &str, order: u32) -> View {
    View {
        id: ViewId::new(),
        container,
        name: name.to_owned(),
        order,
        layout: LayoutNode::Leaf {
            pane: PaneId::new(),
            content: PaneContent::Empty,
        },
    }
}

pub mod layout;
