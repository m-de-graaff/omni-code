//! View state: viewport, scroll offset, and display configuration.

/// A view into a document, managing viewport and scroll state.
#[derive(Debug)]
pub struct View {
    /// The document index this view is displaying.
    pub doc_id: usize,
    /// First visible line.
    pub scroll_offset: usize,
    /// Viewport width in columns.
    pub width: u16,
    /// Viewport height in rows.
    pub height: u16,
}

impl View {
    /// Create a new view for the given document.
    #[must_use]
    pub const fn new(doc_id: usize, width: u16, height: u16) -> Self {
        Self { doc_id, scroll_offset: 0, width, height }
    }

    /// Resize the viewport.
    pub const fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Scroll to ensure the given line is visible.
    pub const fn ensure_visible(&mut self, line: usize) {
        if line < self.scroll_offset {
            self.scroll_offset = line;
        } else if line >= self.scroll_offset + self.height as usize {
            self.scroll_offset = line.saturating_sub(self.height as usize - 1);
        }
    }
}
