//! Document model combining text buffer with metadata.

use std::path::PathBuf;

use omni_core::{History, Selection, Text};

/// A document represents an open file with its text buffer, edit history,
/// syntax state, and metadata.
#[derive(Debug)]
pub struct Document {
    text: Text,
    selection: Selection,
    history: History,
    path: Option<PathBuf>,
    modified: bool,
}

impl Document {
    /// Create a new empty document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: Text::new(),
            selection: Selection::point(0),
            history: History::new(),
            path: None,
            modified: false,
        }
    }

    /// Create a document from a string with an optional path.
    #[must_use]
    pub fn from_str(content: &str, path: Option<PathBuf>) -> Self {
        Self {
            text: Text::from(content),
            selection: Selection::point(0),
            history: History::new(),
            path,
            modified: false,
        }
    }

    /// The text buffer.
    #[must_use]
    pub const fn text(&self) -> &Text {
        &self.text
    }

    /// Mutable access to the text buffer.
    pub const fn text_mut(&mut self) -> &mut Text {
        self.modified = true;
        &mut self.text
    }

    /// The current selection.
    #[must_use]
    pub const fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Set the selection.
    pub fn set_selection(&mut self, sel: Selection) {
        self.selection = sel;
    }

    /// The edit history.
    #[must_use]
    pub const fn history(&self) -> &History {
        &self.history
    }

    /// Mutable access to the edit history.
    pub const fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    /// File path, if saved.
    #[must_use]
    pub const fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// Whether the document has unsaved changes.
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the document as saved.
    pub const fn mark_saved(&mut self) {
        self.modified = false;
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
