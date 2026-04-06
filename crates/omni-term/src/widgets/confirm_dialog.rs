//! Confirmation dialog modal for unsaved changes.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use omni_event::Action;
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// What to do after the dialog is dismissed.
#[derive(Debug, Clone)]
enum DialogChoice {
    /// Save the file, then close.
    SaveAndClose,
    /// Close without saving.
    DiscardAndClose,
    /// Cancel (do nothing).
    Cancel,
}

/// A modal confirmation dialog with Save / Don't Save / Cancel buttons.
pub struct ConfirmDialog {
    message: String,
    selected: usize,
    theme: ThemeColors,
    button_rects: Vec<Rect>,
}

impl ConfirmDialog {
    /// Create a new confirmation dialog.
    pub fn new(message: impl Into<String>, theme: ThemeColors) -> Self {
        Self {
            message: message.into(),
            selected: 0,
            theme,
            button_rects: Vec::new(),
        }
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    fn compute_rect(area: Rect) -> Rect {
        let width = 50.min(area.width.saturating_sub(4));
        let height = 7;
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 3;
        Rect::new(x, y, width, height)
    }

    fn execute_choice(&self, choice: DialogChoice, ctx: &mut Context) -> EventResult {
        match choice {
            DialogChoice::SaveAndClose => {
                // Send Save action then CloseBuffer
                let _ = ctx.action_tx.send(Action::Save);
                let _ = ctx.action_tx.send(Action::CloseBuffer);
                Self::dismiss()
            }
            DialogChoice::DiscardAndClose => {
                // Force close: mark doc as not modified so close won't re-prompt
                if let Some(focus_key) = ctx.view_tree.focus() {
                    if let Some(omni_view::view_tree::Node::Leaf(view)) =
                        ctx.view_tree.get(focus_key)
                    {
                        if let Some(doc) = ctx.documents.get_mut(view.doc_id) {
                            doc.modified = false;
                        }
                    }
                }
                let _ = ctx.action_tx.send(Action::CloseBuffer);
                Self::dismiss()
            }
            DialogChoice::Cancel => Self::dismiss(),
        }
    }
}

impl Component for ConfirmDialog {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let popup = Self::compute_rect(area);
        self.button_rects.clear();

        frame.render_widget(Clear, popup);

        let theme = &self.theme;
        let block = Block::bordered()
            .title(" Unsaved Changes ")
            .border_style(Style::new().fg(theme.border_focused))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        if inner.height < 3 {
            return;
        }

        // Message
        let msg = Paragraph::new(&*self.message)
            .style(Style::new().fg(theme.foreground))
            .alignment(Alignment::Center);
        frame.render_widget(msg, Rect::new(inner.x, inner.y + 1, inner.width, 1));

        // Buttons
        let buttons = [" Save ", " Don't Save ", " Cancel "];
        let total_w: u16 = buttons.iter().map(|b| b.len() as u16 + 2).sum();
        let start_x = inner.x + (inner.width.saturating_sub(total_w)) / 2;
        let btn_y = inner.y + inner.height.saturating_sub(2);

        let mut x = start_x;
        for (i, label) in buttons.iter().enumerate() {
            let w = label.len() as u16;
            let style = if i == self.selected {
                Style::new()
                    .fg(theme.foreground)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.text_muted)
            };
            let btn = Paragraph::new(Line::from(Span::styled(*label, style)));
            let rect = Rect::new(x, btn_y, w, 1);
            frame.render_widget(btn, rect);
            self.button_rects.push(rect);
            x += w + 2;
        }
    }

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
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                if self.selected < 2 {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Enter => {
                let choice = match self.selected {
                    0 => DialogChoice::SaveAndClose,
                    1 => DialogChoice::DiscardAndClose,
                    _ => DialogChoice::Cancel,
                };
                Ok(self.execute_choice(choice, ctx))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Ok(self.execute_choice(DialogChoice::SaveAndClose, ctx))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('d') | KeyCode::Char('D') => {
                Ok(self.execute_choice(DialogChoice::DiscardAndClose, ctx))
            }
            _ => Ok(EventResult::Consumed),
        }
    }

    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        _area: Rect,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            let pos = Position::new(event.column, event.row);
            for (i, rect) in self.button_rects.iter().enumerate() {
                if rect.contains(pos) {
                    self.selected = i;
                    let choice = match i {
                        0 => DialogChoice::SaveAndClose,
                        1 => DialogChoice::DiscardAndClose,
                        _ => DialogChoice::Cancel,
                    };
                    return Ok(self.execute_choice(choice, ctx));
                }
            }
        }
        Ok(EventResult::Consumed)
    }

    fn focusable(&self) -> bool {
        true
    }
}
