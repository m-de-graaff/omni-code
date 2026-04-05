//! Edit transactions composed of insert/delete/retain operations.

/// A single atomic operation on a text buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Keep `n` characters unchanged.
    Retain(usize),
    /// Insert the given text.
    Insert(String),
    /// Delete `n` characters.
    Delete(usize),
}

/// A transaction is an ordered sequence of operations that transforms a buffer.
///
/// Transactions are the unit of undo/redo and can be composed or inverted.
#[derive(Debug, Clone, Default)]
pub struct Transaction {
    operations: Vec<Operation>,
}

impl Transaction {
    /// Create an empty transaction.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a retain operation.
    pub fn retain(&mut self, n: usize) -> &mut Self {
        if n > 0 {
            self.operations.push(Operation::Retain(n));
        }
        self
    }

    /// Add an insert operation.
    pub fn insert(&mut self, text: impl Into<String>) -> &mut Self {
        let text = text.into();
        if !text.is_empty() {
            self.operations.push(Operation::Insert(text));
        }
        self
    }

    /// Add a delete operation.
    pub fn delete(&mut self, n: usize) -> &mut Self {
        if n > 0 {
            self.operations.push(Operation::Delete(n));
        }
        self
    }

    /// Return the operations in this transaction.
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// Apply this transaction to a [`super::Text`] buffer.
    pub fn apply(&self, text: &mut super::Text) {
        let mut pos = 0;
        for op in &self.operations {
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

    /// Create the inverse transaction (for undo).
    #[must_use]
    pub fn invert(&self, text: &super::Text) -> Self {
        let rope = text.rope();
        let mut inverse = Self::new();
        let mut pos = 0;

        for op in &self.operations {
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
}
