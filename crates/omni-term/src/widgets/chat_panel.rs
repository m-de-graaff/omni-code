//! AI chat panel placeholder.

use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};

/// Renders the AI chat panel.
pub struct ChatPanel;

impl ChatPanel {
    pub fn render(frame: &mut Frame, area: Rect, is_focused: bool, theme: &ThemeColors) {
        let border_color = if is_focused { theme.border_focused } else { theme.border };
        let block = Block::bordered()
            .title(" \u{2726} AI Chat ")
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new("Type a message to start chatting with AI...")
            .style(Style::new().fg(theme.text_muted).add_modifier(Modifier::ITALIC));
        frame.render_widget(placeholder, inner);
    }
}
