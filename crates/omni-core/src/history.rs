//! Undo/redo history backed by transaction stacks.

use crate::Transaction;

/// Undo/redo history for a document.
///
/// Stores inverse transactions on the undo stack and re-inverse on the redo stack.
#[derive(Debug, Default)]
pub struct History {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
}

impl History {
    /// Create an empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new change. This clears the redo stack.
    pub fn push(&mut self, inverse: Transaction) {
        self.undo_stack.push(inverse);
        self.redo_stack.clear();
    }

    /// Pop the most recent undo transaction, if any.
    pub fn undo(&mut self) -> Option<Transaction> {
        self.undo_stack.pop()
    }

    /// Pop the most recent redo transaction, if any.
    pub fn redo(&mut self) -> Option<Transaction> {
        self.redo_stack.pop()
    }

    /// Push a transaction onto the redo stack (called after undo).
    pub fn push_redo(&mut self, inverse: Transaction) {
        self.redo_stack.push(inverse);
    }

    /// Whether undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
