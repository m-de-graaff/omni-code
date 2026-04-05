//! Cursor positions and multi-cursor selections.

use smallvec::SmallVec;

/// A single cursor range with anchor and head positions (character indices).
///
/// When `anchor == head`, the range represents a simple cursor with no selection.
/// When they differ, the text between them is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// The fixed end of the selection.
    pub anchor: usize,
    /// The moving end (cursor position).
    pub head: usize,
}

impl Range {
    /// Create a new range (cursor) at the given position.
    #[must_use]
    pub const fn point(pos: usize) -> Self {
        Self { anchor: pos, head: pos }
    }

    /// Create a new range spanning from anchor to head.
    #[must_use]
    pub const fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// The leftmost position.
    #[must_use]
    pub fn from(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The rightmost position.
    #[must_use]
    pub fn to(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Whether this range is a simple cursor (no text selected).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// A set of non-overlapping, sorted ranges representing a multi-cursor selection.
///
/// Uses `SmallVec` to avoid heap allocation for the common single-cursor case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: SmallVec<[Range; 1]>,
    /// Index of the primary cursor within `ranges`.
    primary: usize,
}

impl Selection {
    /// Create a selection with a single cursor at the given position.
    #[must_use]
    pub fn point(pos: usize) -> Self {
        Self { ranges: SmallVec::from_elem(Range::point(pos), 1), primary: 0 }
    }

    /// Create a selection from a single range.
    #[must_use]
    pub fn single(range: Range) -> Self {
        Self { ranges: SmallVec::from_elem(range, 1), primary: 0 }
    }

    /// The primary (active) range.
    #[must_use]
    pub fn primary(&self) -> Range {
        self.ranges[self.primary]
    }

    /// All ranges in the selection.
    #[must_use]
    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    /// Number of cursors/ranges.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Whether the selection is empty (should never be, but for completeness).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}
