//! Bottom panel placeholder (terminal / AI chat).

use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};

/// Renders the bottom panel (terminal or AI chat area).
pub struct BottomPanel;

impl BottomPanel {
    pub fn render(frame: &mut Frame, area: Rect, is_focused: bool, theme: &ThemeColors) {
        let border_color = if is_focused { theme.border_focused } else { theme.border };
        let block = Block::bordered()
            .title(" Terminal ")
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder =
            Paragraph::new("(terminal / AI chat)").style(Style::new().fg(theme.text_muted));
        frame.render_widget(placeholder, inner);
    }
}
