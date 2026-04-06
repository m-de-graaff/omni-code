//! Go-to-line popup: Ctrl+G opens a floating input to jump to a line number.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A floating input popup for jumping to a specific line number.
pub struct GotoLinePopup {
    input: String,
    theme: ThemeColors,
    total_lines: usize,
}

impl GotoLinePopup {
    /// Create a new go-to-line popup.
    #[must_use]
    pub const fn new(theme: ThemeColors, total_lines: usize) -> Self {
        Self {
            input: String::new(),
            theme,
            total_lines,
        }
    }

    fn compute_rect(area: Rect) -> Rect {
        let width = 40.min(area.width.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + area.height / 4;
        Rect::new(x, y, width, 3)
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }
}

impl Component for GotoLinePopup {
    fn handle_key(
        &mut self,
        event: KeyEvent,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        if event.kind != KeyEventKind::Press {
            return Ok(EventResult::Consumed);
        }

        match event.code {
            KeyCode::Esc => Ok(Self::dismiss()),
            KeyCode::Enter => {
                if let Ok(line_num) = self.input.parse::<usize>() {
                    let line = line_num.saturating_sub(1).min(self.total_lines.saturating_sub(1));

                    // Set cursor to the beginning of the target line
                    if let Some(focus_key) = ctx.view_tree.focus() {
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get(focus_key)
                        {
                            if let Some(doc) = ctx.documents.get_mut(view.doc_id) {
                                let char_pos = doc.text().line_to_char(line);
                                doc.set_selection(
                                    focus_key,
                                    omni_core::Selection::point(char_pos),
                                );
                            }
                        }
                        // Scroll to make the line visible
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get_mut(focus_key)
                        {
                            view.ensure_visible(line);
                        }
                    }
                }
                Ok(Self::dismiss())
            }
            KeyCode::Backspace => {
                self.input.pop();
                Ok(EventResult::Consumed)
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.input.push(c);
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Consumed),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let popup = Self::compute_rect(area);

        frame.render_widget(Clear, popup);

        let block = Block::bordered()
            .title(" Go to Line ")
            .border_style(Style::new().fg(self.theme.border_focused))
            .style(Style::new().bg(self.theme.panel_bg));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let hint = format!(" (1–{})", self.total_lines);
        let line = Line::from(vec![
            Span::styled(
                &self.input,
                Style::new()
                    .fg(self.theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "\u{2588}",
                Style::new().fg(self.theme.cursor),
            ),
            Span::styled(hint, Style::new().fg(self.theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(line), inner);
    }
}
