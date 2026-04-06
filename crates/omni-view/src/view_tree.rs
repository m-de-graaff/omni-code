//! Split-based view layout management.
//!
//! The view tree organizes editor panes as a binary tree of splits.
//! Each tab can have its own split tree (or just a single leaf).

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
    /// A split containing two child nodes with a configurable ratio.
    Split {
        direction: SplitDirection,
        children: Vec<NodeKey>,
        /// Split ratio (0.0–1.0): fraction of space given to the first child.
        ratio: f32,
    },
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

    /// Insert a view without changing root or focus.
    pub fn insert_view(&mut self, view: View) -> NodeKey {
        let key = self.nodes.insert(Node::Leaf(view));
        if self.root.is_none() {
            self.root = Some(key);
            self.focus = Some(key);
        }
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

    // ── Split operations ────────────────────────────────────────────

    /// Split a leaf node into two panes showing the same document.
    ///
    /// The original leaf is moved to the left/top child. A new view
    /// for the same document becomes the right/bottom child.
    /// Returns `(left_key, right_key)` or `None` if the key isn't a leaf.
    pub fn split_leaf(
        &mut self,
        key: NodeKey,
        direction: SplitDirection,
    ) -> Option<(NodeKey, NodeKey)> {
        // Extract the view from the leaf
        let node = self.nodes.get(key)?;
        let Node::Leaf(view) = node else {
            return None;
        };

        // Create a second view for the same document
        let new_view = View::new(view.doc_id, view.width, view.height);

        // The original leaf becomes the left child (keep same key)
        let right_key = self.nodes.insert(Node::Leaf(new_view));

        // Replace the original leaf with a split node
        // We need to move the original view out, create a new leaf for it
        let Some(Node::Leaf(original_view)) = self.nodes.remove(key) else {
            return None;
        };
        let left_key = self.nodes.insert(Node::Leaf(original_view));

        // Create the split node at the original key's slot
        let split_key = self.nodes.insert(Node::Split {
            direction,
            children: vec![left_key, right_key],
            ratio: 0.5,
        });

        // If the split node replaced the root, update root
        if self.root == Some(key) {
            self.root = Some(split_key);
        } else {
            // Update parent's children to point to split_key instead of key
            self.replace_child(key, split_key);
        }

        // Focus the new pane
        self.focus = Some(right_key);

        Some((left_key, right_key))
    }

    /// Close a leaf pane. If it's inside a split, collapse the split
    /// so the sibling takes over. Returns the remaining sibling's key.
    pub fn close_leaf(&mut self, key: NodeKey) -> Option<NodeKey> {
        // Find the parent split
        let parent_key = self.find_parent(key)?;
        let parent = self.nodes.get(parent_key)?;
        let Node::Split { children, .. } = parent else {
            return None;
        };

        // Find the sibling
        let sibling_key = children.iter().find(|&&k| k != key).copied()?;

        // Remove the leaf
        self.nodes.remove(key);

        // Remove the parent split and replace with sibling
        self.nodes.remove(parent_key);

        // Update grandparent to point to sibling
        if self.root == Some(parent_key) {
            self.root = Some(sibling_key);
        } else {
            self.replace_child(parent_key, sibling_key);
        }

        // Focus the sibling
        let focus_target = self.first_leaf(sibling_key).unwrap_or(sibling_key);
        self.focus = Some(focus_target);

        Some(sibling_key)
    }

    /// Get all leaf keys in depth-first order.
    #[must_use]
    pub fn leaves(&self) -> Vec<NodeKey> {
        let mut result = Vec::new();
        if let Some(root) = self.root {
            self.collect_leaves(root, &mut result);
        }
        result
    }

    /// Get all leaf keys under a specific subtree root.
    #[must_use]
    pub fn leaves_under(&self, key: NodeKey) -> Vec<NodeKey> {
        let mut result = Vec::new();
        self.collect_leaves(key, &mut result);
        result
    }

    /// Get the next leaf after the given key (depth-first order, wraps around).
    #[must_use]
    pub fn next_leaf(&self, key: NodeKey) -> Option<NodeKey> {
        let leaves = self.leaves();
        let pos = leaves.iter().position(|&k| k == key)?;
        let next = (pos + 1) % leaves.len();
        Some(leaves[next])
    }

    /// Get the previous leaf before the given key (depth-first order, wraps around).
    #[must_use]
    pub fn prev_leaf(&self, key: NodeKey) -> Option<NodeKey> {
        let leaves = self.leaves();
        let pos = leaves.iter().position(|&k| k == key)?;
        let prev = if pos == 0 { leaves.len() - 1 } else { pos - 1 };
        Some(leaves[prev])
    }

    // ── Private helpers ─────────────────────────────────────────────

    fn collect_leaves(&self, key: NodeKey, result: &mut Vec<NodeKey>) {
        match self.nodes.get(key) {
            Some(Node::Leaf(_)) => result.push(key),
            Some(Node::Split { children, .. }) => {
                for &child in children {
                    self.collect_leaves(child, result);
                }
            }
            None => {}
        }
    }

    /// Find the first leaf in a subtree.
    fn first_leaf(&self, key: NodeKey) -> Option<NodeKey> {
        match self.nodes.get(key)? {
            Node::Leaf(_) => Some(key),
            Node::Split { children, .. } => {
                children.first().and_then(|&child| self.first_leaf(child))
            }
        }
    }

    /// Find the parent of a node by traversing the tree.
    fn find_parent(&self, target: NodeKey) -> Option<NodeKey> {
        let root = self.root?;
        self.find_parent_recursive(root, target)
    }

    fn find_parent_recursive(&self, current: NodeKey, target: NodeKey) -> Option<NodeKey> {
        let node = self.nodes.get(current)?;
        if let Node::Split { children, .. } = node {
            for &child in children {
                if child == target {
                    return Some(current);
                }
                if let Some(found) = self.find_parent_recursive(child, target) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Replace a child key in all parent nodes (used after split restructuring).
    fn replace_child(&mut self, old_key: NodeKey, new_key: NodeKey) {
        // Walk all split nodes to find and replace the old child
        let keys: Vec<NodeKey> = self.nodes.keys().collect();
        for key in keys {
            if let Some(Node::Split { children, .. }) = self.nodes.get_mut(key) {
                for child in children.iter_mut() {
                    if *child == old_key {
                        *child = new_key;
                    }
                }
            }
        }
    }
}

impl Default for ViewTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_core::DocumentId;

    fn make_view() -> View {
        View::new(DocumentId::next(), 80, 24)
    }

    #[test]
    fn split_leaf_creates_two_panes() {
        let mut tree = ViewTree::new();
        let root = tree.insert_view(make_view());

        let result = tree.split_leaf(root, SplitDirection::Vertical);
        assert!(result.is_some());

        let (left, right) = result.unwrap();
        assert!(matches!(tree.get(left), Some(Node::Leaf(_))));
        assert!(matches!(tree.get(right), Some(Node::Leaf(_))));

        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn close_leaf_collapses_split() {
        let mut tree = ViewTree::new();
        let root = tree.insert_view(make_view());

        let (left, right) = tree.split_leaf(root, SplitDirection::Vertical).unwrap();

        // Close the right pane
        let remaining = tree.close_leaf(right);
        assert!(remaining.is_some());

        // Should be back to a single leaf
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0], left);
    }

    #[test]
    fn next_prev_leaf() {
        let mut tree = ViewTree::new();
        let root = tree.insert_view(make_view());

        let (left, right) = tree.split_leaf(root, SplitDirection::Vertical).unwrap();

        assert_eq!(tree.next_leaf(left), Some(right));
        assert_eq!(tree.next_leaf(right), Some(left)); // wraps
        assert_eq!(tree.prev_leaf(right), Some(left));
    }

    #[test]
    fn leaves_under_subtree() {
        let mut tree = ViewTree::new();
        let root = tree.insert_view(make_view());
        let (left, _right) = tree.split_leaf(root, SplitDirection::Vertical).unwrap();

        // Split the left pane again
        let result = tree.split_leaf(left, SplitDirection::Horizontal);
        assert!(result.is_some());

        // Should have 3 leaves total
        let all_leaves = tree.leaves();
        assert_eq!(all_leaves.len(), 3);
    }
}
