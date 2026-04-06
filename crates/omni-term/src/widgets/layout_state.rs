//! Runtime-mutable layout dimensions for the IDE shell.

/// Minimum sidebar width in columns.
pub const MIN_SIDEBAR_WIDTH: u16 = 15;
/// Maximum sidebar width in columns (absolute cap before terminal-relative clamping).
pub const MAX_SIDEBAR_WIDTH: u16 = 80;
/// Minimum bottom panel height in rows.
pub const MIN_BOTTOM_HEIGHT: u16 = 3;
/// Maximum bottom panel height in rows (absolute cap before terminal-relative clamping).
pub const MAX_BOTTOM_HEIGHT: u16 = 40;
/// Minimum split ratio percentage.
pub const MIN_SPLIT_RATIO: u16 = 20;
/// Maximum split ratio percentage.
pub const MAX_SPLIT_RATIO: u16 = 80;

/// Application display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppMode {
    /// Full IDE: sidebar + tabs + editor + bottom panel.
    #[default]
    Ide,
    /// Full-width AI conversation view.
    Chat,
    /// Editor and AI chat side-by-side.
    Split,
}

impl AppMode {
    /// Display label for the status bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ide => "IDE",
            Self::Chat => "CHAT",
            Self::Split => "SPLIT",
        }
    }
}

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
    /// Width to restore when re-expanding sidebar.
    sidebar_restore_width: u16,
    /// Current application display mode.
    pub app_mode: AppMode,
    /// Split ratio as percentage (0-100) for the editor portion in split mode.
    pub split_ratio: u16,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            sidebar_width: 30,
            sidebar_visible: false,
            bottom_height: 12,
            bottom_visible: false,
            minimap_visible: false,
            minimap_width: 12,
            sidebar_restore_width: 30,
            app_mode: AppMode::Ide,
            split_ratio: 50,
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

    /// Set sidebar width, clamped to sensible bounds for the given terminal width.
    pub fn set_sidebar_width(&mut self, width: u16, terminal_width: u16) {
        let max = terminal_width.saturating_sub(20).min(MAX_SIDEBAR_WIDTH);
        self.sidebar_width = width.clamp(MIN_SIDEBAR_WIDTH, max);
    }

    /// Set bottom panel height, clamped to sensible bounds for the given terminal height.
    pub fn set_bottom_height(&mut self, height: u16, terminal_height: u16) {
        let max = terminal_height.saturating_sub(6).min(MAX_BOTTOM_HEIGHT);
        self.bottom_height = height.clamp(MIN_BOTTOM_HEIGHT, max);
    }

    /// Set split ratio, clamped to valid range.
    pub fn set_split_ratio(&mut self, ratio: u16) {
        self.split_ratio = ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO);
    }

    /// Cycle through app modes: Ide → Split → Chat → Ide.
    pub const fn cycle_mode(&mut self) {
        self.app_mode = match self.app_mode {
            AppMode::Ide => AppMode::Split,
            AppMode::Split => AppMode::Chat,
            AppMode::Chat => AppMode::Ide,
        };
    }

    /// Toggle sidebar visibility (instant, shadcn-style).
    pub const fn toggle_sidebar(&mut self) {
        if self.sidebar_visible {
            self.sidebar_restore_width = self.sidebar_width;
            self.sidebar_visible = false;
        } else {
            self.sidebar_visible = true;
            self.sidebar_width = self.sidebar_restore_width;
        }
    }

    /// Toggle bottom panel visibility.
    pub const fn toggle_bottom_panel(&mut self) {
        self.bottom_visible = !self.bottom_visible;
    }

    /// Toggle minimap visibility.
    pub const fn toggle_minimap(&mut self) {
        self.minimap_visible = !self.minimap_visible;
    }
}
