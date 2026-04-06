//! Cursor positions and multi-cursor selections.

use smallvec::SmallVec;

// ── Range ───────────────────────────────────────────────────────────

/// A single cursor range with anchor and head positions (character indices).
///
/// When `anchor == head`, the range represents a simple cursor with no selection.
/// When they differ, the text between them is selected.  The *direction* matters:
/// `anchor` is the fixed end, `head` is where the cursor moves.
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

    /// The leftmost (start) position.
    #[must_use]
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The rightmost (end) position.
    #[must_use]
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Number of characters in this range.
    #[must_use]
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Whether this range is a simple cursor (no text selected).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Whether this range is "forward" (anchor <= head).
    #[must_use]
    pub const fn is_forward(&self) -> bool {
        self.anchor <= self.head
    }

    /// Return the same span with the direction flipped.
    #[must_use]
    pub const fn flip(self) -> Self {
        Self { anchor: self.head, head: self.anchor }
    }

    /// Whether the given char position falls within this range.
    #[must_use]
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start() && pos < self.end()
    }

    /// Whether two ranges overlap (share at least one character).
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start() < other.end() && other.start() < self.end()
    }

    /// Whether two ranges are adjacent (touching but not overlapping)
    /// or overlapping — i.e., they can be merged.
    #[must_use]
    pub fn touches_or_overlaps(&self, other: &Self) -> bool {
        self.start() <= other.end() && other.start() <= self.end()
    }

    /// Merge two ranges into one that covers both spans.
    /// The direction follows `self` (anchor side preserved).
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        let start = self.start().min(other.start());
        let end = self.end().max(other.end());
        if self.is_forward() {
            Self::new(start, end)
        } else {
            Self::new(end, start)
        }
    }

    /// Extend the head to a new position (for shift+arrow selection).
    #[must_use]
    pub const fn extend_to(self, new_head: usize) -> Self {
        Self { anchor: self.anchor, head: new_head }
    }
}

// ── Selection ───────────────────────────────────────────────────────

/// A set of non-overlapping, sorted ranges representing a multi-cursor selection.
///
/// Uses `SmallVec` to avoid heap allocation for the common single-cursor case.
///
/// # Invariants (enforced by [`normalize`](Self::normalize))
/// - Ranges are sorted by `start()`.
/// - No two ranges overlap or are adjacent.
/// - There is always at least one range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: SmallVec<[Range; 1]>,
    /// Index of the primary cursor within `ranges`.
    primary: usize,
}

impl Selection {
    // ── Constructors ────────────────────────────────────────────────

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

    /// Create a selection from multiple ranges with a primary index.
    ///
    /// # Panics
    ///
    /// Panics if `ranges` is empty or `primary >= ranges.len()`.
    #[must_use]
    pub fn from_ranges(ranges: SmallVec<[Range; 1]>, primary: usize) -> Self {
        assert!(!ranges.is_empty(), "Selection must have at least one range");
        assert!(primary < ranges.len(), "primary index out of bounds");
        Self { ranges, primary }
    }

    /// Select the entire document (0..len).
    #[must_use]
    pub fn select_all(len: usize) -> Self {
        Self::single(Range::new(0, len))
    }

    // ── Accessors ────────────────────────────────────────────���──────

    /// The primary (active) range.
    #[must_use]
    pub fn primary(&self) -> Range {
        self.ranges[self.primary]
    }

    /// The index of the primary range.
    #[must_use]
    pub const fn primary_index(&self) -> usize {
        self.primary
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

    /// Whether the selection has no ranges (should never happen in practice).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Whether this is a single cursor (no text selected, one range).
    #[must_use]
    pub fn is_single_cursor(&self) -> bool {
        self.ranges.len() == 1 && self.ranges[0].is_empty()
    }

    /// Whether this selection has multiple cursors.
    #[must_use]
    pub fn is_multi_cursor(&self) -> bool {
        self.ranges.len() > 1
    }

    // ── Multi-cursor manipulation ───────────────────────────────────

    /// Add a new cursor/range. The new range becomes the primary.
    /// Call [`normalize`](Self::normalize) afterward if the range may overlap.
    pub fn push_range(&mut self, range: Range) {
        self.ranges.push(range);
        self.primary = self.ranges.len() - 1;
    }

    /// Add a cursor at the given position. It becomes primary.
    pub fn add_cursor_at(&mut self, pos: usize) {
        self.push_range(Range::point(pos));
        self.normalize();
    }

    /// Remove the primary cursor. If it's the only one, this is a no-op.
    pub fn remove_primary(&mut self) {
        if self.ranges.len() <= 1 {
            return;
        }
        self.ranges.remove(self.primary);
        if self.primary >= self.ranges.len() {
            self.primary = self.ranges.len() - 1;
        }
    }

    /// Cycle the primary cursor to the next one.
    pub fn cycle_primary_forward(&mut self) {
        if self.ranges.len() > 1 {
            self.primary = (self.primary + 1) % self.ranges.len();
        }
    }

    /// Cycle the primary cursor to the previous one.
    pub fn cycle_primary_backward(&mut self) {
        if self.ranges.len() > 1 {
            self.primary = if self.primary == 0 {
                self.ranges.len() - 1
            } else {
                self.primary - 1
            };
        }
    }

    /// Collapse all ranges to their head positions (remove selections, keep cursors).
    #[must_use]
    pub fn collapse_to_cursors(&self) -> Self {
        let ranges: SmallVec<[Range; 1]> = self
            .ranges
            .iter()
            .map(|r| Range::point(r.head))
            .collect();
        Self { ranges, primary: self.primary }
    }

    /// Apply a function to each range and return a new selection.
    #[must_use]
    pub fn map_ranges(&self, f: impl Fn(&Range) -> Range) -> Self {
        let ranges: SmallVec<[Range; 1]> = self.ranges.iter().map(&f).collect();
        let mut sel = Self { ranges, primary: self.primary };
        sel.normalize();
        sel
    }

    // ── Normalization ───────────────────────────────────────────────

    /// Sort ranges by start position and merge any that overlap or touch.
    /// The primary index is updated to track its original range.
    #[allow(clippy::missing_panics_doc)] // unwrap after is_some_and check
    pub fn normalize(&mut self) {
        if self.ranges.len() <= 1 {
            return;
        }

        // Tag each range with its original index so we can track primary
        let mut tagged: SmallVec<[(usize, Range); 1]> = self
            .ranges
            .iter()
            .enumerate()
            .map(|(i, r)| (i, *r))
            .collect();

        // Sort by start position
        tagged.sort_by_key(|(_, r)| r.start());

        // Merge overlapping/adjacent ranges
        let mut merged: SmallVec<[(usize, Range); 1]> = SmallVec::new();
        for (idx, range) in tagged {
            let should_merge = merged
                .last()
                .is_some_and(|(_, last)| last.touches_or_overlaps(&range));

            if should_merge {
                let (tag, last) = merged.last_mut().unwrap();
                *last = last.merge(range);
                // Keep the primary tag if this merge absorbs the primary
                if idx == self.primary {
                    *tag = idx;
                }
                continue;
            }
            merged.push((idx, range));
        }

        // Find new primary index
        let old_primary = self.primary;
        let new_primary = merged
            .iter()
            .position(|(orig_idx, _)| *orig_idx == old_primary)
            .unwrap_or(0);

        self.ranges = merged.into_iter().map(|(_, r)| r).collect();
        self.primary = new_primary;
    }

    // ── Block/column selection ───────────────────────────────────────

    /// Create a block (column) selection spanning multiple lines.
    ///
    /// Given a start and end position as (line, col) pairs, creates one
    /// cursor per line with the same column span.
    #[must_use]
    pub fn block_selection(
        start_line: usize,
        end_line: usize,
        start_col: usize,
        end_col: usize,
        line_to_char: impl Fn(usize) -> usize,
        line_len: impl Fn(usize) -> usize,
    ) -> Self {
        let (from_line, to_line) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };

        let mut ranges = SmallVec::new();
        for line in from_line..=to_line {
            let line_start = line_to_char(line);
            let len = line_len(line);
            let col_a = start_col.min(len);
            let col_b = end_col.min(len);
            ranges.push(Range::new(line_start + col_a, line_start + col_b));
        }

        if ranges.is_empty() {
            return Self::point(0);
        }

        Self { primary: ranges.len() - 1, ranges }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Range tests ─────────────────────────────────────────────────

    #[test]
    fn range_start_end() {
        let fwd = Range::new(2, 5);
        assert_eq!(fwd.start(), 2);
        assert_eq!(fwd.end(), 5);
        assert_eq!(fwd.len(), 3);

        let bwd = Range::new(5, 2);
        assert_eq!(bwd.start(), 2);
        assert_eq!(bwd.end(), 5);
        assert_eq!(bwd.len(), 3);
    }

    #[test]
    fn range_contains() {
        let r = Range::new(2, 5);
        assert!(r.contains(2));
        assert!(r.contains(4));
        assert!(!r.contains(5)); // exclusive end
        assert!(!r.contains(1));
    }

    #[test]
    fn range_overlaps() {
        let a = Range::new(2, 5);
        let b = Range::new(4, 7);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));

        let c = Range::new(5, 8); // adjacent, not overlapping
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn range_touches_or_overlaps() {
        let a = Range::new(2, 5);
        let c = Range::new(5, 8); // adjacent
        assert!(a.touches_or_overlaps(&c));
    }

    #[test]
    fn range_merge() {
        let a = Range::new(2, 5);
        let b = Range::new(4, 7);
        let merged = a.merge(b);
        assert_eq!(merged.start(), 2);
        assert_eq!(merged.end(), 7);
        assert!(merged.is_forward());
    }

    #[test]
    fn range_flip() {
        let r = Range::new(2, 5);
        let f = r.flip();
        assert_eq!(f.anchor, 5);
        assert_eq!(f.head, 2);
    }

    #[test]
    fn range_extend_to() {
        let r = Range::new(2, 5);
        let extended = r.extend_to(10);
        assert_eq!(extended.anchor, 2);
        assert_eq!(extended.head, 10);
    }

    // ── Selection tests ─────────────────────────────────────────────

    #[test]
    fn select_all() {
        let sel = Selection::select_all(100);
        assert_eq!(sel.primary().start(), 0);
        assert_eq!(sel.primary().end(), 100);
    }

    #[test]
    fn is_single_cursor() {
        assert!(Selection::point(5).is_single_cursor());
        assert!(!Selection::single(Range::new(2, 5)).is_single_cursor());
    }

    #[test]
    fn add_cursor_at() {
        let mut sel = Selection::point(5);
        sel.add_cursor_at(10);
        assert_eq!(sel.len(), 2);
        assert_eq!(sel.primary().head, 10);
    }

    #[test]
    fn add_cursor_at_merges_overlap() {
        let mut sel = Selection::single(Range::new(2, 8));
        sel.add_cursor_at(5); // inside the existing range
        // Should merge since point(5) is inside [2,8)
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn remove_primary() {
        let ranges = smallvec::smallvec![
            Range::point(1),
            Range::point(5),
            Range::point(10),
        ];
        let mut sel = Selection::from_ranges(ranges, 1);
        sel.remove_primary();
        assert_eq!(sel.len(), 2);
        assert!(sel.primary().head == 1 || sel.primary().head == 10);
    }

    #[test]
    fn normalize_merges_overlapping() {
        let ranges = smallvec::smallvec![
            Range::new(0, 5),
            Range::new(3, 8),  // overlaps with first
            Range::new(10, 15),
        ];
        let mut sel = Selection::from_ranges(ranges, 0);
        sel.normalize();
        assert_eq!(sel.len(), 2);
        assert_eq!(sel.ranges()[0].start(), 0);
        assert_eq!(sel.ranges()[0].end(), 8);
        assert_eq!(sel.ranges()[1].start(), 10);
    }

    #[test]
    fn normalize_sorts_by_start() {
        let ranges = smallvec::smallvec![
            Range::point(10),
            Range::point(2),
            Range::point(5),
        ];
        let mut sel = Selection::from_ranges(ranges, 2);
        sel.normalize();
        assert_eq!(sel.ranges()[0].head, 2);
        assert_eq!(sel.ranges()[1].head, 5);
        assert_eq!(sel.ranges()[2].head, 10);
    }

    #[test]
    fn collapse_to_cursors() {
        let sel = Selection::single(Range::new(2, 10));
        let collapsed = sel.collapse_to_cursors();
        assert!(collapsed.is_single_cursor());
        assert_eq!(collapsed.primary().head, 10);
    }

    #[test]
    fn cycle_primary() {
        let ranges = smallvec::smallvec![
            Range::point(1),
            Range::point(5),
            Range::point(10),
        ];
        let mut sel = Selection::from_ranges(ranges, 0);
        sel.cycle_primary_forward();
        assert_eq!(sel.primary_index(), 1);
        sel.cycle_primary_forward();
        assert_eq!(sel.primary_index(), 2);
        sel.cycle_primary_forward();
        assert_eq!(sel.primary_index(), 0); // wraps
    }

    #[test]
    fn block_selection_basic() {
        // Simulate 3 lines of 10 chars each
        let sel = Selection::block_selection(
            0, 2,  // lines 0..=2
            3, 6,  // cols 3..6
            |line| line * 11, // 10 chars + newline
            |_| 10,
        );
        assert_eq!(sel.len(), 3);
        assert_eq!(sel.ranges()[0], Range::new(3, 6));
        assert_eq!(sel.ranges()[1], Range::new(14, 17));
        assert_eq!(sel.ranges()[2], Range::new(25, 28));
    }

    #[test]
    fn map_ranges() {
        let sel = Selection::single(Range::new(0, 5));
        let shifted = sel.map_ranges(|r| Range::new(r.anchor + 10, r.head + 10));
        assert_eq!(shifted.primary().start(), 10);
        assert_eq!(shifted.primary().end(), 15);
    }
}
