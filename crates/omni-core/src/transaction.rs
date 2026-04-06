//! Edit transactions: a changeset bundled with an optional selection update.

use std::time::Instant;

use smallvec::SmallVec;

use crate::changeset::ChangeSet;
use crate::{Range, Selection};

/// A transaction bundles a [`ChangeSet`] (text mutations) with an optional
/// [`Selection`] update and a timestamp.
///
/// Transactions are the **only** way to mutate a document's text. This
/// ensures every edit is invertible, composable, and recorded in the
/// undo history.
#[derive(Debug, Clone)]
pub struct Transaction {
    /// The text changes.
    changes: ChangeSet,
    /// If `Some`, the selection to apply after the changeset. If `None`,
    /// the existing selection is mapped through the changeset.
    selection: Option<Selection>,
    /// When the transaction was created (for grouping rapid edits).
    timestamp: Instant,
}

impl Transaction {
    /// Create a transaction from a changeset (no explicit selection update).
    #[must_use]
    pub fn from_changes(changes: ChangeSet) -> Self {
        Self { changes, selection: None, timestamp: Instant::now() }
    }

    /// Create a transaction with both a changeset and a selection.
    #[must_use]
    pub fn new(changes: ChangeSet, selection: Selection) -> Self {
        Self { changes, selection: Some(selection), timestamp: Instant::now() }
    }

    /// The text changeset.
    #[must_use]
    pub const fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    /// The explicit selection update, if any.
    #[must_use]
    pub const fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// When this transaction was created.
    #[must_use]
    pub const fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Override the timestamp (for testing time-dependent behaviour).
    #[cfg(test)]
    pub fn set_timestamp_for_test(&mut self, ts: Instant) {
        self.timestamp = ts;
    }

    // ── Convenience constructors ────────────────────────────────────

    /// Insert text at a position in a document of length `doc_len`.
    #[must_use]
    pub fn insert_at(doc_len: usize, pos: usize, text: &str) -> Self {
        Self::from_changes(ChangeSet::insert_at(doc_len, pos, text))
    }

    /// Delete `count` chars at `pos` in a document of length `doc_len`.
    #[must_use]
    pub fn delete_at(doc_len: usize, pos: usize, count: usize) -> Self {
        Self::from_changes(ChangeSet::delete_at(doc_len, pos, count))
    }

    /// Replace `count` chars at `pos` with `text`.
    #[must_use]
    pub fn replace_at(doc_len: usize, pos: usize, count: usize, text: &str) -> Self {
        Self::from_changes(ChangeSet::replace_at(doc_len, pos, count, text))
    }

    // ── Apply ───────────────────────────────────────────────────────

    /// Apply this transaction's changeset to a text buffer.
    pub fn apply(&self, text: &mut crate::Text) {
        self.changes.apply(text);
    }

    // ── Invert ──────────────────────────────────────────────────────

    /// Create the inverse transaction (for undo).
    ///
    /// The inverse changeset restores the original text; the inverse
    /// selection is the *current* selection (so undo restores the cursor
    /// position from before the edit).
    #[must_use]
    pub fn invert(&self, text: &crate::Text, current_selection: &Selection) -> Self {
        Self {
            changes: self.changes.invert(text),
            selection: Some(current_selection.clone()),
            timestamp: self.timestamp,
        }
    }

    // ── Compose ─────────────────────────────────────────────────────

    /// Compose two transactions into one. The resulting transaction has the
    /// same effect as applying `self` then `other`.
    ///
    /// The composed selection is `other`'s selection (or `self`'s mapped
    /// through `other`'s changes, or `None` if neither has one).
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        let changes = self.changes.compose(&other.changes);

        // The selection after composing is the later transaction's selection,
        // falling back to mapping the earlier selection through the later changes.
        let selection = other.selection.clone().or_else(|| {
            self.selection
                .as_ref()
                .map(|sel| map_selection(&other.changes, sel))
        });

        Self {
            changes,
            selection,
            // Use the later timestamp for grouping purposes
            timestamp: other.timestamp,
        }
    }

    // ── Selection mapping ───────────────────────────────────────────

    /// Map a selection through this transaction's changeset.
    ///
    /// If the transaction carries an explicit selection, returns that.
    /// Otherwise maps each range through the changeset's position mapping.
    #[must_use]
    pub fn map_selection(&self, sel: &Selection) -> Selection {
        if let Some(ref explicit) = self.selection {
            return explicit.clone();
        }
        map_selection(&self.changes, sel)
    }
}

/// Map a selection through a changeset's position mapping.
fn map_selection(changes: &ChangeSet, sel: &Selection) -> Selection {
    let ranges: SmallVec<[Range; 1]> = sel
        .ranges()
        .iter()
        .map(|r| Range::new(changes.map_pos(r.anchor), changes.map_pos(r.head)))
        .collect();

    if ranges.is_empty() {
        return Selection::point(0);
    }

    Selection::from_ranges(ranges, sel.primary_index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn insert_and_apply() {
        let mut text = Text::from("hello");
        let txn = Transaction::insert_at(5, 5, " world");
        txn.apply(&mut text);
        assert_eq!(text.to_string(), "hello world");
    }

    #[test]
    fn delete_and_apply() {
        let mut text = Text::from("hello world");
        let txn = Transaction::delete_at(11, 5, 6);
        txn.apply(&mut text);
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn invert_roundtrip() {
        let text = Text::from("hello");
        let sel = Selection::point(5);
        let txn = Transaction::insert_at(5, 5, " world");
        let inv = txn.invert(&text, &sel);

        let mut t2 = text.clone();
        txn.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello world");

        inv.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello");

        // The inverse should restore the original selection
        assert_eq!(inv.selection(), Some(&sel));
    }

    #[test]
    fn map_selection_insert_before_cursor() {
        let txn = Transaction::insert_at(10, 3, "ab");
        let sel = Selection::point(5);
        let mapped = txn.map_selection(&sel);
        // Cursor was at 5, insert of 2 chars at 3 pushes it to 7
        assert_eq!(mapped.primary().head, 7);
    }

    #[test]
    fn explicit_selection_overrides_mapping() {
        let changes = ChangeSet::insert_at(5, 0, "x");
        let explicit_sel = Selection::point(42);
        let txn = Transaction::new(changes, explicit_sel.clone());

        let sel = Selection::point(3);
        let result = txn.map_selection(&sel);
        // Should return the explicit selection, not the mapped one
        assert_eq!(result, explicit_sel);
    }

    #[test]
    fn compose_two_transactions() {
        let a = Transaction::insert_at(3, 0, "x"); // "abc" → "xabc"
        let b = Transaction::insert_at(4, 4, "y"); // "xabc" → "xabcy"
        let composed = a.compose(&b);

        let mut text = Text::from("abc");
        composed.apply(&mut text);
        assert_eq!(text.to_string(), "xabcy");
    }

    #[test]
    fn map_selection_multi_cursor() {
        let txn = Transaction::insert_at(10, 0, "xx");
        let ranges = smallvec::smallvec![
            Range::point(2),
            Range::point(5),
        ];
        let sel = Selection::from_ranges(ranges, 1);
        let mapped = txn.map_selection(&sel);

        // Both cursors should shift by 2
        assert_eq!(mapped.ranges()[0].head, 4);
        assert_eq!(mapped.ranges()[1].head, 7);
        // Primary index preserved
        assert_eq!(mapped.primary_index(), 1);
    }
}
