//! Split-based view layout management.

use slotmap::{SlotMap, new_key_type};

use crate::View;

new_key_type! {
    /// Unique key for a node in the view tree.
    pub struct NodeKey;
}

/// A node in the view tree: either a leaf (single view) or a split.
#[derive(Debug)]
pub enum Node {
    /// A single editor view.
    Leaf(View),
    /// A horizontal or vertical split containing child nodes.
    Split { direction: SplitDirection, children: Vec<NodeKey> },
}

/// Direction of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Tree of views supporting splits and tabs.
#[derive(Debug)]
pub struct ViewTree {
    nodes: SlotMap<NodeKey, Node>,
    root: Option<NodeKey>,
    focus: Option<NodeKey>,
}

impl ViewTree {
    /// Create an empty view tree.
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: SlotMap::with_key(), root: None, focus: None }
    }

    /// Insert a single view as the root (or replace the root).
    pub fn set_root(&mut self, view: View) -> NodeKey {
        let key = self.nodes.insert(Node::Leaf(view));
        self.root = Some(key);
        self.focus = Some(key);
        key
    }

    /// Get a reference to a node.
    #[must_use]
    pub fn get(&self, key: NodeKey) -> Option<&Node> {
        self.nodes.get(key)
    }

    /// Get a mutable reference to a node.
    pub fn get_mut(&mut self, key: NodeKey) -> Option<&mut Node> {
        self.nodes.get_mut(key)
    }

    /// The root node key.
    #[must_use]
    pub const fn root(&self) -> Option<NodeKey> {
        self.root
    }

    /// The currently focused node key.
    #[must_use]
    pub const fn focus(&self) -> Option<NodeKey> {
        self.focus
    }

    /// Set focus to a node.
    pub fn set_focus(&mut self, key: NodeKey) {
        if self.nodes.contains_key(key) {
            self.focus = Some(key);
        }
    }
}

impl Default for ViewTree {
    fn default() -> Self {
        Self::new()
    }
}
