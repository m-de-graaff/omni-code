//! Runtime-mutable layout dimensions for the IDE shell.

/// Mutable layout state controlling panel visibility and sizing.
///
/// Owned by [`super::EditorShell`]. Values are read each render pass
/// so mutations take effect immediately on the next frame.
pub struct LayoutState {
    /// Sidebar width in columns.
    pub sidebar_width: u16,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Bottom panel height in rows.
    pub bottom_height: u16,
    /// Whether the bottom panel is visible.
    pub bottom_visible: bool,
    /// Whether to show the minimap.
    pub minimap_visible: bool,
    /// Minimap width in columns.
    pub minimap_width: u16,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            sidebar_width: 30,
            sidebar_visible: true,
            bottom_height: 12,
            bottom_visible: false,
            minimap_visible: false,
            minimap_width: 12,
        }
    }
}

impl LayoutState {
    /// Effective sidebar width (0 when collapsed).
    pub const fn effective_sidebar_width(&self) -> u16 {
        if self.sidebar_visible { self.sidebar_width } else { 0 }
    }

    /// Effective bottom panel height (0 when hidden).
    pub const fn effective_bottom_height(&self) -> u16 {
        if self.bottom_visible { self.bottom_height } else { 0 }
    }

    /// Toggle sidebar visibility.
    pub const fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    /// Toggle bottom panel visibility.
    pub const fn toggle_bottom_panel(&mut self) {
        self.bottom_visible = !self.bottom_visible;
    }
}
