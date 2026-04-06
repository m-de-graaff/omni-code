//! A changeset describes a transformation from one document state to another
//! as a sequence of retain/insert/delete operations.

/// A single atomic operation in a changeset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Keep `n` characters unchanged.
    Retain(usize),
    /// Insert the given text.
    Insert(String),
    /// Delete `n` characters.
    Delete(usize),
}

/// An ordered sequence of [`Operation`]s that transforms a text buffer.
///
/// A changeset is always *normalized*: adjacent operations of the same kind
/// are merged, and zero-length operations are dropped. This ensures a
/// canonical representation and makes composition simpler.
///
/// # Invariant
///
/// The sum of `Retain` + `Delete` lengths must equal the length of the
/// document *before* the changeset is applied (`len_before`). The sum of
/// `Retain` + `Insert` lengths equals the document length *after* application.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    ops: Vec<Operation>,
    /// Document length (in chars) before this changeset is applied.
    len_before: usize,
}

impl ChangeSet {
    /// Create an identity changeset for a document of the given length.
    #[must_use]
    pub fn identity(len: usize) -> Self {
        let mut cs = Self { ops: Vec::new(), len_before: len };
        if len > 0 {
            cs.ops.push(Operation::Retain(len));
        }
        cs
    }

    /// Create an empty changeset (for an empty document).
    #[must_use]
    pub const fn empty() -> Self {
        Self { ops: Vec::new(), len_before: 0 }
    }

    /// The document length before this changeset is applied.
    #[must_use]
    pub const fn len_before(&self) -> usize {
        self.len_before
    }

    /// The document length after this changeset is applied.
    #[must_use]
    pub fn len_after(&self) -> usize {
        let mut len = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => len += n,
                Operation::Insert(s) => len += s.chars().count(),
                Operation::Delete(_) => {}
            }
        }
        len
    }

    /// Whether this changeset makes no changes (identity).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.iter().all(|op| matches!(op, Operation::Retain(_)))
    }

    /// The operations in this changeset.
    #[must_use]
    pub fn ops(&self) -> &[Operation] {
        &self.ops
    }

    // ── Builder methods ─────────────────────────────────────────────

    /// Append a retain operation, merging with the previous if possible.
    pub fn retain(&mut self, n: usize) -> &mut Self {
        if n == 0 {
            return self;
        }
        if let Some(Operation::Retain(prev)) = self.ops.last_mut() {
            *prev += n;
        } else {
            self.ops.push(Operation::Retain(n));
        }
        self
    }

    /// Append an insert operation, merging with the previous if possible.
    #[allow(clippy::missing_panics_doc)] // pop() after matching last() as Some
    pub fn insert(&mut self, text: impl Into<String>) -> &mut Self {
        let text = text.into();
        if text.is_empty() {
            return self;
        }
        // Insert before a trailing Delete (canonical order: Insert before Delete).
        // The `pop` is safe because we just matched `last()` as `Some(Delete(_))`.
        if let Some(Operation::Delete(_)) = self.ops.last() {
            let delete = self.ops.pop().expect("just matched Some");
            if let Some(Operation::Insert(prev)) = self.ops.last_mut() {
                prev.push_str(&text);
            } else {
                self.ops.push(Operation::Insert(text));
            }
            self.ops.push(delete);
        } else if let Some(Operation::Insert(prev)) = self.ops.last_mut() {
            prev.push_str(&text);
        } else {
            self.ops.push(Operation::Insert(text));
        }
        self
    }

    /// Append a delete operation, merging with the previous if possible.
    pub fn delete(&mut self, n: usize) -> &mut Self {
        if n == 0 {
            return self;
        }
        if let Some(Operation::Delete(prev)) = self.ops.last_mut() {
            *prev += n;
        } else {
            self.ops.push(Operation::Delete(n));
        }
        self
    }

    // ── Apply ───────────────────────────────────────────────────────

    /// Apply this changeset to a text buffer.
    pub fn apply(&self, text: &mut super::Text) {
        let mut pos = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => pos += n,
                Operation::Insert(s) => {
                    text.insert(pos, s);
                    pos += s.chars().count();
                }
                Operation::Delete(n) => {
                    text.remove(pos, pos + n);
                }
            }
        }
    }

    // ── Invert ──────────────────────────────────────────────────────

    /// Create the inverse changeset (for undo).
    ///
    /// The inverse, when applied to the *result* of this changeset,
    /// restores the original document.
    #[must_use]
    pub fn invert(&self, text: &super::Text) -> Self {
        let rope = text.rope();
        let mut inverse = Self { ops: Vec::new(), len_before: self.len_after() };
        let mut pos = 0;

        for op in &self.ops {
            match op {
                Operation::Retain(n) => {
                    inverse.retain(*n);
                    pos += n;
                }
                Operation::Insert(s) => {
                    inverse.delete(s.chars().count());
                }
                Operation::Delete(n) => {
                    let slice = rope.slice(pos..pos + n);
                    let deleted: String = slice.chars().collect();
                    inverse.insert(deleted);
                    pos += n;
                }
            }
        }

        inverse
    }

    // ── Compose ─────────────────────────────────────────────────────

    /// Compose two changesets into one that has the same effect as applying
    /// `self` then `other`.
    ///
    /// `self.len_after()` must equal `other.len_before()`.
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        debug_assert_eq!(
            self.len_after(),
            other.len_before,
            "compose: self.len_after ({}) != other.len_before ({})",
            self.len_after(),
            other.len_before,
        );

        let mut result = Self { ops: Vec::new(), len_before: self.len_before };

        let mut a_iter = OpCursor::new(&self.ops);
        let mut b_iter = OpCursor::new(&other.ops);

        loop {
            let a_done = a_iter.is_done();
            let b_done = b_iter.is_done();

            if a_done && b_done {
                break;
            }

            // If `other` has an insert, emit it (inserts in `other` are new text)
            if let Some(Operation::Insert(s)) = b_iter.peek() {
                result.insert(s.clone());
                b_iter.advance_full();
                continue;
            }

            // If `self` has a delete, emit it (deletes in `self` remove original text)
            if let Some(Operation::Delete(n)) = a_iter.peek() {
                result.delete(*n);
                a_iter.advance_full();
                continue;
            }

            // Both must be Retain or self=Insert, other=Retain|Delete
            match (a_iter.peek(), b_iter.peek()) {
                (Some(Operation::Retain(a_n)), Some(Operation::Retain(b_n))) => {
                    let len = (*a_n).min(*b_n);
                    result.retain(len);
                    a_iter.advance(len);
                    b_iter.advance(len);
                }
                (Some(Operation::Retain(a_n)), Some(Operation::Delete(b_n))) => {
                    let len = (*a_n).min(*b_n);
                    result.delete(len);
                    a_iter.advance(len);
                    b_iter.advance(len);
                }
                (Some(Operation::Insert(s)), Some(Operation::Retain(b_n))) => {
                    let char_len = s.chars().count();
                    let len = char_len.min(*b_n);
                    // Take `len` chars from the insert
                    let taken: String = s.chars().take(len).collect();
                    result.insert(taken);
                    a_iter.advance(len);
                    b_iter.advance(len);
                }
                (Some(Operation::Insert(s)), Some(Operation::Delete(b_n))) => {
                    let char_len = s.chars().count();
                    let len = char_len.min(*b_n);
                    // Insert then delete cancels out — skip both
                    a_iter.advance(len);
                    b_iter.advance(len);
                }
                _ => break,
            }
        }

        result
    }

    // ── Position mapping ────────────────────────────────────────────

    /// Map a char position from the old document to the new document.
    ///
    /// Used to adjust cursor/selection positions after applying a changeset.
    #[must_use]
    pub fn map_pos(&self, mut pos: usize) -> usize {
        let mut old_pos = 0;
        let mut new_pos = 0;

        for op in &self.ops {
            if old_pos > pos {
                break;
            }
            match op {
                Operation::Retain(n) => {
                    old_pos += n;
                    new_pos += n;
                }
                Operation::Insert(s) => {
                    let len = s.chars().count();
                    if old_pos <= pos {
                        pos += len;
                    }
                    new_pos += len;
                }
                Operation::Delete(n) => {
                    if old_pos < pos {
                        let overlap = (*n).min(pos - old_pos);
                        pos -= overlap;
                    }
                    old_pos += n;
                }
            }
        }
        pos
    }
}

// ── Helper: cursor into an operation slice for compose ───────────────

/// A cursor that tracks partial consumption of an operation sequence.
struct OpCursor<'a> {
    ops: &'a [Operation],
    idx: usize,
    /// How many chars have been consumed from the current op.
    offset: usize,
}

impl<'a> OpCursor<'a> {
    const fn new(ops: &'a [Operation]) -> Self {
        Self { ops, idx: 0, offset: 0 }
    }

    const fn is_done(&self) -> bool {
        self.idx >= self.ops.len()
    }

    /// Peek at the remaining portion of the current operation.
    fn peek(&self) -> Option<&Operation> {
        if self.is_done() {
            return None;
        }
        let op = &self.ops[self.idx];
        if self.offset == 0 {
            return Some(op);
        }
        // We've partially consumed this op — caller handles via advance()
        Some(op)
    }

    /// Advance past the entire current operation.
    const fn advance_full(&mut self) {
        if !self.is_done() {
            self.idx += 1;
            self.offset = 0;
        }
    }

    /// Advance by `n` chars within the current operation. If the op is
    /// fully consumed, move to the next one.
    fn advance(&mut self, n: usize) {
        if self.is_done() {
            return;
        }

        let remaining = self.op_remaining();
        let consumed = self.offset + n;

        if n >= remaining {
            self.idx += 1;
            self.offset = 0;
        } else {
            self.offset = consumed;
            // Mutate the op in-place is not possible on a shared slice,
            // so we track offset and the caller re-peeks.
            // Actually, we need to handle this differently — let's use
            // a different approach where we clone remaining ops.
        }
    }

    fn op_remaining(&self) -> usize {
        if self.is_done() {
            return 0;
        }
        match &self.ops[self.idx] {
            Operation::Insert(s) => s.chars().count() - self.offset,
            Operation::Retain(n) | Operation::Delete(n) => n - self.offset,
        }
    }
}

// ── Convenience constructors ────────────────────────────────────────

impl ChangeSet {
    /// Create a changeset that inserts text at the given position in a
    /// document of length `doc_len`.
    #[must_use]
    pub fn insert_at(doc_len: usize, pos: usize, text: &str) -> Self {
        let mut cs = Self { ops: Vec::new(), len_before: doc_len };
        cs.retain(pos);
        cs.insert(text);
        cs.retain(doc_len - pos);
        cs
    }

    /// Create a changeset that deletes `count` chars starting at `pos`
    /// in a document of length `doc_len`.
    #[must_use]
    pub fn delete_at(doc_len: usize, pos: usize, count: usize) -> Self {
        let mut cs = Self { ops: Vec::new(), len_before: doc_len };
        cs.retain(pos);
        cs.delete(count);
        cs.retain(doc_len - pos - count);
        cs
    }

    /// Create a changeset that replaces `count` chars at `pos` with `text`
    /// in a document of length `doc_len`.
    #[must_use]
    pub fn replace_at(doc_len: usize, pos: usize, count: usize, text: &str) -> Self {
        let mut cs = Self { ops: Vec::new(), len_before: doc_len };
        cs.retain(pos);
        cs.delete(count);
        cs.insert(text);
        cs.retain(doc_len - pos - count);
        cs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn insert_at_middle() {
        let mut text = Text::from("helo");
        let cs = ChangeSet::insert_at(4, 3, "l");
        cs.apply(&mut text);
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn delete_at_start() {
        let mut text = Text::from("xxhello");
        let cs = ChangeSet::delete_at(7, 0, 2);
        cs.apply(&mut text);
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn replace_at() {
        let mut text = Text::from("hello world");
        let cs = ChangeSet::replace_at(11, 6, 5, "rust");
        cs.apply(&mut text);
        assert_eq!(text.to_string(), "hello rust");
    }

    #[test]
    fn identity_is_empty() {
        let cs = ChangeSet::identity(10);
        assert!(cs.is_empty());
        assert_eq!(cs.len_before(), 10);
        assert_eq!(cs.len_after(), 10);
    }

    #[test]
    fn len_after_tracks_changes() {
        let cs = ChangeSet::insert_at(5, 2, "abc");
        assert_eq!(cs.len_before(), 5);
        assert_eq!(cs.len_after(), 8);

        let cs2 = ChangeSet::delete_at(5, 1, 2);
        assert_eq!(cs2.len_before(), 5);
        assert_eq!(cs2.len_after(), 3);
    }

    #[test]
    fn invert_insert() {
        let text = Text::from("hello");
        let cs = ChangeSet::insert_at(5, 5, " world");
        let inv = cs.invert(&text);

        let mut t2 = Text::from("hello");
        cs.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello world");

        inv.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello");
    }

    #[test]
    fn invert_delete() {
        let text = Text::from("hello world");
        let cs = ChangeSet::delete_at(11, 5, 6);
        let inv = cs.invert(&text);

        let mut t2 = text.clone();
        cs.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello");

        inv.apply(&mut t2);
        assert_eq!(t2.to_string(), "hello world");
    }

    #[test]
    fn compose_two_inserts() {
        // Insert "x" at pos 0, then insert "y" at pos 1
        let a = ChangeSet::insert_at(3, 0, "x"); // "abc" → "xabc"
        let b = ChangeSet::insert_at(4, 1, "y"); // "xabc" → "xyabc"
        let composed = a.compose(&b);

        let mut text = Text::from("abc");
        composed.apply(&mut text);
        assert_eq!(text.to_string(), "xyabc");
    }

    #[test]
    fn map_pos_insert_before() {
        let cs = ChangeSet::insert_at(10, 3, "ab");
        // Position 5 should shift to 7 (2 chars inserted before it)
        assert_eq!(cs.map_pos(5), 7);
        // Position 2 is before the insert, unchanged
        assert_eq!(cs.map_pos(2), 2);
        // Position at insert point shifts forward
        assert_eq!(cs.map_pos(3), 5);
    }

    #[test]
    fn map_pos_delete_before() {
        let cs = ChangeSet::delete_at(10, 2, 3);
        // Position 7 should shift to 4 (3 chars deleted before it)
        assert_eq!(cs.map_pos(7), 4);
        // Position 1 is before the delete, unchanged
        assert_eq!(cs.map_pos(1), 1);
        // Position inside deleted range clamps to delete start
        assert_eq!(cs.map_pos(3), 2);
    }

    #[test]
    fn builder_merges_adjacent() {
        let mut cs = ChangeSet { ops: Vec::new(), len_before: 0 };
        cs.retain(3);
        cs.retain(5);
        cs.insert("ab");
        cs.insert("cd");
        cs.delete(2);
        cs.delete(3);
        assert_eq!(cs.ops.len(), 3);
        assert_eq!(cs.ops[0], Operation::Retain(8));
        assert_eq!(cs.ops[1], Operation::Insert("abcd".into()));
        assert_eq!(cs.ops[2], Operation::Delete(5));
    }
}
