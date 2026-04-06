//! Branching undo/redo history stored as a DAG (tree).
//!
//! Unlike a linear undo stack, this history preserves **all** edit branches.
//! Undoing past a fork and making a new edit creates a new branch; the old
//! branch is still reachable. This matches Vim's undo-tree behaviour.

use std::time::{Duration, Instant};

use crate::{Text, Transaction};

/// How many edits between automatic rope snapshots.
const SNAPSHOT_INTERVAL: usize = 100;

/// Maximum time gap to merge rapid edits into one undo step.
const GROUP_INTERVAL: Duration = Duration::from_millis(500);

// ── Node ────────────────────────────────────────────────────────────

/// A single node in the undo tree.
#[derive(Debug)]
struct HistoryNode {
    /// The inverse transaction: applying this to the *child's* state
    /// restores the *parent's* state.
    inverse: Transaction,
    /// Index of the parent node (root has `parent == 0`, i.e. self).
    parent: usize,
    /// Indices of child nodes (branches). The last child is the
    /// "preferred" redo target.
    children: Vec<usize>,
    /// An optional full rope snapshot for O(1) state restoration.
    /// Stored every [`SNAPSHOT_INTERVAL`] edits.
    snapshot: Option<Text>,
    /// Wall-clock time when this edit was made.
    timestamp: Instant,
}

// ── History ─────────────────────────────────────────────────────────

/// A branching undo/redo history stored as a tree of [`Transaction`]s.
///
/// # Structure
///
/// ```text
///       root(0)
///      /       \
///   edit1(1)   edit4(4)   ← branch created by undo + new edit
///     |
///   edit2(2)
///     |
///   edit3(3)
/// ```
///
/// `current` always points to the node representing the **last applied edit**.
/// For a fresh document, `current == 0` (the root sentinel).
#[derive(Debug)]
pub struct History {
    /// Arena of nodes. Index 0 is the root sentinel (no real edit).
    nodes: Vec<HistoryNode>,
    /// Index of the current position in the tree.
    current: usize,
    /// Total number of real edits recorded (for snapshot scheduling).
    edit_count: usize,
}

impl History {
    /// Create an empty history with just the root sentinel.
    #[must_use]
    pub fn new() -> Self {
        let root = HistoryNode {
            // The root's inverse is never applied; use a dummy identity.
            inverse: Transaction::from_changes(crate::ChangeSet::empty()),
            parent: 0,
            children: Vec::new(),
            snapshot: None,
            timestamp: Instant::now(),
        };
        Self { nodes: vec![root], current: 0, edit_count: 0 }
    }

    /// Record a new edit. `inverse` is the transaction that undoes this edit.
    /// `text_after` is the document text *after* the edit was applied (used
    /// for periodic snapshots).
    ///
    /// If the previous edit happened within [`GROUP_INTERVAL`], the two are
    /// composed into a single undo step instead of creating a new node.
    #[allow(clippy::missing_panics_doc)]
    pub fn push(&mut self, inverse: Transaction, text_after: &Text) {
        self.edit_count += 1;

        // ── Time-based grouping ──
        // If the current node is a real edit (not root) and was recent,
        // compose into it rather than creating a new node.
        if self.current != 0 {
            let cur = &self.nodes[self.current];
            let gap = inverse
                .timestamp()
                .checked_duration_since(cur.timestamp)
                .unwrap_or(Duration::MAX);
            if gap <= GROUP_INTERVAL && cur.children.is_empty() {
                // Compose: merge the new inverse with the existing one.
                // The composed inverse undoes *both* edits in one step.
                let composed = inverse.compose(&cur.inverse);
                let node = &mut self.nodes[self.current];
                node.inverse = composed;
                node.timestamp = inverse.timestamp();
                // Update snapshot if interval hit
                if self.edit_count % SNAPSHOT_INTERVAL == 0 {
                    node.snapshot = Some(text_after.clone());
                }
                return;
            }
        }

        // ── Create new node ──
        let snapshot = if self.edit_count % SNAPSHOT_INTERVAL == 0 {
            Some(text_after.clone())
        } else {
            None
        };

        let new_idx = self.nodes.len();
        self.nodes.push(HistoryNode {
            inverse,
            parent: self.current,
            children: Vec::new(),
            snapshot,
            timestamp: Instant::now(),
        });

        // Register as child of current node
        self.nodes[self.current].children.push(new_idx);
        self.current = new_idx;
    }

    /// Undo: move to the parent node. Returns the inverse transaction to apply,
    /// or `None` if we're already at the root.
    pub fn undo(&mut self) -> Option<&Transaction> {
        if self.current == 0 {
            return None;
        }
        let node_idx = self.current;
        self.current = self.nodes[node_idx].parent;
        Some(&self.nodes[node_idx].inverse)
    }

    /// Redo: move to the most recently visited child. Returns the *forward*
    /// transaction to apply (the inverse of the child's inverse), or `None`
    /// if there are no children.
    ///
    /// The "most recently visited child" is the last entry in the children
    /// list — when we undo past a fork, we push the undone child to the end
    /// so it becomes the preferred redo target.
    pub fn redo(&mut self) -> Option<usize> {
        let children = &self.nodes[self.current].children;
        if children.is_empty() {
            return None;
        }
        // Prefer the last child (most recently used branch)
        let child_idx = *children.last()?;
        self.current = child_idx;
        Some(child_idx)
    }

    /// Get the inverse transaction at a specific node index.
    /// Used by `Document::redo` to get the child's inverse for computing
    /// the forward transaction.
    #[must_use]
    pub fn node_inverse(&self, idx: usize) -> Option<&Transaction> {
        self.nodes.get(idx).map(|n| &n.inverse)
    }

    /// Whether undo is available (not at root).
    #[must_use]
    pub const fn can_undo(&self) -> bool {
        self.current != 0
    }

    /// Whether redo is available (current node has children).
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.nodes[self.current].children.is_empty()
    }

    /// The current node index.
    #[must_use]
    pub const fn current(&self) -> usize {
        self.current
    }

    /// Total number of nodes (including root sentinel).
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the history is empty (only the root sentinel).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }

    /// Number of undo steps from current to root.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        let mut depth = 0;
        let mut idx = self.current;
        while idx != 0 {
            idx = self.nodes[idx].parent;
            depth += 1;
        }
        depth
    }

    /// The rope snapshot at a given node, if one exists.
    #[must_use]
    pub fn snapshot_at(&self, idx: usize) -> Option<&Text> {
        self.nodes.get(idx).and_then(|n| n.snapshot.as_ref())
    }

    // ── Tree visualization helpers ──────────────────────────────────

    /// Get information about a node for visualization.
    #[must_use]
    pub fn node_info(&self, idx: usize) -> Option<NodeInfo> {
        let node = self.nodes.get(idx)?;
        Some(NodeInfo {
            index: idx,
            parent: node.parent,
            children: node.children.clone(),
            is_current: idx == self.current,
            timestamp: node.timestamp,
            has_snapshot: node.snapshot.is_some(),
        })
    }

    /// Iterate over all node indices (for tree visualization).
    pub fn node_indices(&self) -> impl Iterator<Item = usize> {
        0..self.nodes.len()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

/// Public information about a history node (for the undo-tree visualizer).
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node index in the arena.
    pub index: usize,
    /// Parent node index.
    pub parent: usize,
    /// Child node indices.
    pub children: Vec<usize>,
    /// Whether this is the current position.
    pub is_current: bool,
    /// When this edit was made.
    pub timestamp: Instant,
    /// Whether this node has a rope snapshot.
    pub has_snapshot: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeSet;

    /// Create an identity-like inverse transaction for a document of length `len`.
    /// Each test inverse is distinct (different retain length) to avoid
    /// compose issues. In real usage, inverses are produced by `Transaction::invert`.
    fn make_inverse(len: usize) -> Transaction {
        let cs = ChangeSet::identity(len);
        let mut txn = Transaction::from_changes(cs);
        // Give each transaction a distinct timestamp to avoid grouping
        txn.set_timestamp_for_test(Instant::now() + Duration::from_secs(len as u64));
        txn
    }

    #[test]
    fn fresh_history_is_at_root() {
        let h = History::new();
        assert_eq!(h.current(), 0);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert!(h.is_empty());
    }

    #[test]
    fn push_and_undo() {
        let mut h = History::new();
        let text = Text::from("hello");
        h.push(make_inverse(5), &text);
        h.push(make_inverse(5), &text);
        h.push(make_inverse(5), &text);

        assert_eq!(h.undo_depth(), 3);
        assert!(h.can_undo());

        let inv = h.undo();
        assert!(inv.is_some());
        assert_eq!(h.undo_depth(), 2);

        h.undo();
        assert_eq!(h.undo_depth(), 1);

        h.undo();
        assert_eq!(h.undo_depth(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn redo_follows_last_branch() {
        let mut h = History::new();
        let text = Text::from("hello");
        h.push(make_inverse(5), &text);
        h.push(make_inverse(5), &text);

        h.undo();
        assert!(h.can_redo());

        let child_idx = h.redo();
        assert!(child_idx.is_some());
        assert_eq!(h.undo_depth(), 2);
    }

    #[test]
    fn branching_preserves_old_timeline() {
        let mut h = History::new();
        let text = Text::from("hello");

        h.push(make_inverse(5), &text);
        h.push(make_inverse(5), &text);
        let b_idx = h.current();

        h.undo();
        assert_eq!(h.undo_depth(), 1);

        h.push(make_inverse(5), &text);
        let c_idx = h.current();

        assert!(h.node_info(b_idx).is_some());
        assert_eq!(h.current(), c_idx);
        assert_ne!(b_idx, c_idx);

        let a_info = h.node_info(1).unwrap();
        assert_eq!(a_info.children.len(), 2);
    }

    #[test]
    fn redo_prefers_most_recent_branch() {
        let mut h = History::new();
        let text = Text::from("hello");

        h.push(make_inverse(5), &text);
        h.push(make_inverse(5), &text);

        h.undo();
        h.push(make_inverse(5), &text);

        h.undo();

        let child = h.redo().unwrap();
        let c_info = h.node_info(child).unwrap();
        assert!(c_info.children.is_empty());
    }

    #[test]
    fn snapshot_at_interval() {
        let mut h = History::new();
        let text = Text::from("hello");

        for i in 0..SNAPSHOT_INTERVAL {
            let mut txn = Transaction::from_changes(ChangeSet::identity(5));
            // Spread timestamps to avoid grouping
            txn.set_timestamp_for_test(Instant::now() + Duration::from_secs(i as u64 * 2));
            h.push(txn, &text);
        }

        assert!(h.snapshot_at(h.current()).is_some());
    }

    #[test]
    fn node_info_for_visualization() {
        let mut h = History::new();
        let text = Text::from("hello");
        h.push(make_inverse(5), &text);

        let root_info = h.node_info(0).unwrap();
        assert!(!root_info.is_current);
        assert_eq!(root_info.children.len(), 1);

        let a_info = h.node_info(1).unwrap();
        assert!(a_info.is_current);
        assert_eq!(a_info.parent, 0);
    }
}
