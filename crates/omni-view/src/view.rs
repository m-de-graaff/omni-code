//! View state: viewport, scroll offset, and display configuration.

use omni_core::DocumentId;

/// A view into a document, managing viewport and scroll state.
#[derive(Debug)]
pub struct View {
    /// The document this view is displaying.
    pub doc_id: DocumentId,
    /// First visible line (vertical scroll offset).
    pub scroll_offset: usize,
    /// First visible column (horizontal scroll offset).
    pub col_offset: usize,
    /// Viewport width in columns.
    pub width: u16,
    /// Viewport height in rows.
    pub height: u16,
}

impl View {
    /// Create a new view for the given document.
    #[must_use]
    pub const fn new(doc_id: DocumentId, width: u16, height: u16) -> Self {
        Self { doc_id, scroll_offset: 0, col_offset: 0, width, height }
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

    /// Scroll to ensure the given column is visible within the code area.
    pub const fn ensure_col_visible(&mut self, col: usize, code_width: usize) {
        if code_width == 0 {
            return;
        }
        if col < self.col_offset {
            self.col_offset = col;
        } else if col >= self.col_offset + code_width {
            self.col_offset = col.saturating_sub(code_width - 1);
        }
    }

    /// Scroll up by `n` lines, clamped to 0.
    pub const fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines, clamped to `total_lines - 1`.
    pub fn scroll_down(&mut self, n: usize, total_lines: usize) {
        self.scroll_offset = (self.scroll_offset + n).min(total_lines.saturating_sub(1));
    }

    /// Scroll up by half a page.
    pub fn page_up(&mut self) {
        let half = (self.height as usize) / 2;
        self.scroll_up(half.max(1));
    }

    /// Scroll down by half a page.
    pub fn page_down(&mut self, total_lines: usize) {
        let half = (self.height as usize) / 2;
        self.scroll_down(half.max(1), total_lines);
    }
}
