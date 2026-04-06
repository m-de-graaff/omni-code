//! Cached panel rectangles from the last render pass.
//!
//! Updated by [`super::EditorShell::render()`] each frame.
//! Read by [`super::EditorShell::handle_mouse()`] for hit-testing.

use ratatui::layout::{Position, Rect};

/// Identifies a panel region within the `EditorShell` layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    Sidebar,
    TabBar,
    EditorPane,
    Bottom,
    StatusBar,
    Chat,
}

/// Identifies a draggable border between panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragBorder {
    /// The right border of the sidebar (vertical, resizes sidebar width).
    SidebarRight,
    /// The top border of the bottom panel (horizontal, resizes bottom height).
    BottomTop,
    /// The divider between editor and chat in split mode.
    SplitDivider,
}

/// Stores the `Rect` allocated to each panel during the last render.
#[derive(Debug, Default)]
pub struct HitMap {
    sidebar: Option<Rect>,
    tab_bar: Option<Rect>,
    editor_pane: Option<Rect>,
    bottom_panel: Option<Rect>,
    status_bar: Option<Rect>,
    chat_panel: Option<Rect>,
    split_divider: Option<Rect>,
}

impl HitMap {
    /// Reset all regions (called at the start of each render).
    pub const fn clear(&mut self) {
        self.sidebar = None;
        self.tab_bar = None;
        self.editor_pane = None;
        self.bottom_panel = None;
        self.status_bar = None;
        self.chat_panel = None;
        self.split_divider = None;
    }

    /// Record the rect for a panel.
    pub const fn set(&mut self, panel: Panel, rect: Rect) {
        match panel {
            Panel::Sidebar => self.sidebar = Some(rect),
            Panel::TabBar => self.tab_bar = Some(rect),
            Panel::EditorPane => self.editor_pane = Some(rect),
            Panel::Bottom => self.bottom_panel = Some(rect),
            Panel::StatusBar => self.status_bar = Some(rect),
            Panel::Chat => self.chat_panel = Some(rect),
        }
    }

    /// Set the split divider rect for drag detection.
    pub const fn set_split_divider(&mut self, rect: Rect) {
        self.split_divider = Some(rect);
    }

    /// Find which panel contains the given (column, row) coordinate.
    ///
    /// Priority: `Sidebar` > `TabBar` > `EditorPane` > `Chat` > `Bottom` > `StatusBar`.
    pub fn panel_at(&self, col: u16, row: u16) -> Option<Panel> {
        let pos = Position::new(col, row);

        if self.sidebar.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::Sidebar);
        }
        if self.tab_bar.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::TabBar);
        }
        if self.editor_pane.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::EditorPane);
        }
        if self.chat_panel.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::Chat);
        }
        if self.bottom_panel.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::Bottom);
        }
        if self.status_bar.is_some_and(|r| r.contains(pos)) {
            return Some(Panel::StatusBar);
        }

        None
    }

    /// The sidebar rect, if visible.
    pub const fn sidebar_rect(&self) -> Option<Rect> {
        self.sidebar
    }

    /// The bottom panel rect, if visible.
    pub const fn bottom_rect(&self) -> Option<Rect> {
        self.bottom_panel
    }

    /// Check if (col, row) is on a draggable border between panels.
    pub const fn border_at(&self, col: u16, row: u16) -> Option<DragBorder> {
        // Split divider (check first — highest priority in split mode)
        if let Some(sd) = self.split_divider {
            if col >= sd.left() && col < sd.right() && row >= sd.top() && row < sd.bottom() {
                return Some(DragBorder::SplitDivider);
            }
        }
        // Sidebar right border
        if let Some(sb) = self.sidebar {
            if col == sb.right().saturating_sub(1) && row >= sb.top() && row < sb.bottom() {
                return Some(DragBorder::SidebarRight);
            }
        }
        // Bottom panel top border
        if let Some(bp) = self.bottom_panel {
            if row == bp.top() && col >= bp.left() && col < bp.right() {
                return Some(DragBorder::BottomTop);
            }
        }
        None
    }
}
