//! `EditorShell` — the root IDE layout component.
//!
//! Pushed onto the [`crate::Compositor`] as the base layer. Owns all
//! layout state and delegates rendering to sub-widgets.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect, Spacing};

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

use super::bottom_panel::BottomPanel;
use super::editor_pane::EditorPane;
use super::layout_state::LayoutState;
use super::sidebar::Sidebar;
use super::status_bar::StatusBar;
use super::tab_bar::TabBar;

/// The root IDE component implementing the full panel arrangement.
///
/// Layout hierarchy:
/// ```text
/// ┌──────────┬──────────────────────────┐
/// │ Sidebar  │ [Tab1] [Tab2]            │
/// │          │├────────────────┬─────────┤
/// │          ││ Editor Pane    │ Minimap │
/// │          │├───��────────────┴─────────┤
/// │          ││ Bottom Panel             │
/// ├──────────┴──────────────────────────┤
/// │ Status Bar                           │
/// └──────────────────────────────────────┘
/// ```
pub struct EditorShell {
    layout: LayoutState,
}

impl EditorShell {
    /// Create a new editor shell with default layout dimensions.
    #[must_use]
    pub fn new() -> Self {
        Self { layout: LayoutState::default() }
    }
}

impl Default for EditorShell {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for EditorShell {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // 1. Outer vertical: [main content | status bar (1 row)]
        let outer = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);
        let main_area = outer[0];
        let status_area = outer[1];

        // 2. Main horizontal: [sidebar | right region]
        let sidebar_w = self.layout.effective_sidebar_width();
        let (sidebar_area, right_area) = if sidebar_w > 0 {
            let h = Layout::horizontal([Constraint::Length(sidebar_w), Constraint::Fill(1)])
                .spacing(Spacing::Overlap(1))
                .split(main_area);
            (Some(h[0]), h[1])
        } else {
            (None, main_area)
        };

        // 3. Right vertical: [tab bar (1) | editor (fill) | bottom panel?]
        let bottom_h = self.layout.effective_bottom_height();
        let right_chunks = if bottom_h > 0 {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(bottom_h),
            ])
            .spacing(Spacing::Overlap(1))
            .split(right_area)
        } else {
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(right_area)
        };

        let tab_area = right_chunks[0];
        let editor_area = right_chunks[1];

        // 4. Render sub-widgets
        if let Some(sb) = sidebar_area {
            Sidebar::render(frame, sb);
        }

        TabBar::render(frame, tab_area);

        EditorPane::render(
            frame,
            editor_area,
            self.layout.minimap_visible,
            self.layout.minimap_width,
        );

        if bottom_h > 0 {
            BottomPanel::render(frame, right_chunks[2]);
        }

        StatusBar::render(frame, status_area);
    }

    fn handle_key(
        &mut self,
        event: KeyEvent,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match (event.modifiers, event.code) {
            // Ctrl+B: toggle sidebar
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.layout.toggle_sidebar();
                Ok(EventResult::Consumed)
            }
            // Ctrl+J: toggle bottom panel
            (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                self.layout.toggle_bottom_panel();
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Ignored),
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
