//! Right-click context menu popup component.
//!
//! Pushed onto the [`crate::Compositor`] as a modal layer.
//! Clicking outside or pressing Escape dismisses it.

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState};

use omni_event::Action;

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A single item in the context menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Display label.
    pub label: String,
    /// Action emitted when selected.
    pub action: Action,
}

impl MenuItem {
    /// Create a new menu item.
    pub fn new(label: impl Into<String>, action: Action) -> Self {
        Self { label: label.into(), action }
    }
}

/// A popup context menu rendered at a specific terminal position.
///
/// Implements [`Component`] so it can be pushed onto the compositor.
/// Modal: consumes all events. Clicking outside or pressing Escape dismisses.
pub struct ContextMenu {
    items: Vec<MenuItem>,
    position: (u16, u16),
    list_state: ListState,
    area: Option<Rect>,
}

impl ContextMenu {
    /// Create a context menu at the given terminal position.
    pub fn new(items: Vec<MenuItem>, col: u16, row: u16) -> Self {
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }
        Self { items, position: (col, row), list_state, area: None }
    }

    /// Compute the menu rect, clamped to stay within the terminal bounds.
    fn compute_rect(&self, terminal_area: Rect) -> Rect {
        let width = self
            .items
            .iter()
            .map(|i| u16::try_from(i.label.len()).unwrap_or(u16::MAX))
            .max()
            .unwrap_or(10)
            + 4; // border + padding
        let height = u16::try_from(self.items.len()).unwrap_or(u16::MAX) + 2; // border top + bottom

        let (col, row) = self.position;

        // Clamp to stay within terminal bounds
        let x = if col + width > terminal_area.right() {
            terminal_area.right().saturating_sub(width)
        } else {
            col
        };
        let y = if row + height > terminal_area.bottom() {
            terminal_area.bottom().saturating_sub(height)
        } else {
            row
        };

        Rect::new(x, y, width, height)
    }

    /// Return a callback that pops this menu from the compositor.
    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }
}

impl Component for ContextMenu {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let menu_rect = self.compute_rect(area);
        self.area = Some(menu_rect);

        // Clear the area behind the menu
        frame.render_widget(Clear, menu_rect);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(Line::raw(format!(" {} ", item.label))))
            .collect();

        let list = List::new(items)
            .block(Block::bordered().border_style(Style::new().fg(Color::Gray)))
            .highlight_style(
                Style::new().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, menu_rect, &mut self.list_state);
    }

    fn handle_key(
        &mut self,
        event: KeyEvent,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match event.code {
            KeyCode::Esc => Ok(Self::dismiss()),
            KeyCode::Up => {
                let i = self.list_state.selected().unwrap_or(0);
                let new = if i == 0 { self.items.len().saturating_sub(1) } else { i - 1 };
                self.list_state.select(Some(new));
                Ok(EventResult::Consumed)
            }
            KeyCode::Down => {
                let i = self.list_state.selected().unwrap_or(0);
                let new = if i + 1 >= self.items.len() { 0 } else { i + 1 };
                self.list_state.select(Some(new));
                Ok(EventResult::Consumed)
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(item) = self.items.get(idx) {
                        let action = item.action.clone();
                        // Dismiss menu and emit the action
                        return Ok(EventResult::Callback(Box::new(move |compositor| {
                            compositor.pop();
                            // Action will be dispatched by handle_event_result
                            tracing::debug!(?action, "context menu selected");
                        })));
                    }
                }
                Ok(Self::dismiss())
            }
            _ => Ok(EventResult::Consumed), // consume all keys while menu is open
        }
    }

    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        _area: Rect,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                if let Some(menu_rect) = self.area {
                    if menu_rect.contains(pos) {
                        // Click inside menu — select the item at this row
                        let inner_top = menu_rect.y + 1; // skip border
                        let row_idx = event.row.saturating_sub(inner_top) as usize;
                        if row_idx < self.items.len() {
                            let action = self.items[row_idx].action.clone();
                            return Ok(EventResult::Callback(Box::new(move |compositor| {
                                compositor.pop();
                                tracing::debug!(?action, "context menu clicked");
                            })));
                        }
                    }
                }
                // Click outside menu — dismiss
                Ok(Self::dismiss())
            }
            // Consume all other mouse events while menu is open
            _ => Ok(EventResult::Consumed),
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
