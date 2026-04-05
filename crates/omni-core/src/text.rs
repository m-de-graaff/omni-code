//! Rope-backed text buffer.

use ropey::Rope;

/// A text buffer backed by a rope data structure for efficient editing.
#[derive(Debug, Clone)]
pub struct Text {
    rope: Rope,
}

impl Text {
    /// Create an empty text buffer.
    #[must_use]
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    /// Create a text buffer from a string slice.
    #[must_use]
    pub fn from(s: &str) -> Self {
        Self { rope: Rope::from_str(s) }
    }

    /// Return the underlying rope.
    #[must_use]
    pub const fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Return a mutable reference to the underlying rope.
    pub const fn rope_mut(&mut self) -> &mut Rope {
        &mut self.rope
    }

    /// Total number of characters.
    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Total number of lines.
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Insert text at the given character index.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
    }

    /// Remove the character range `[start..end)`.
    pub fn remove(&mut self, start: usize, end: usize) {
        self.rope.remove(start..end);
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.rope.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}
