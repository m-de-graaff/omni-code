//! Bottom panel placeholder (terminal / AI chat).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

/// Renders the bottom panel (terminal or AI chat area).
pub struct BottomPanel;

impl BottomPanel {
    pub fn render(frame: &mut Frame, area: Rect) {
        let block =
            Block::bordered().title(" Terminal ").border_style(Style::new().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder =
            Paragraph::new("(terminal / AI chat)").style(Style::new().fg(Color::DarkGray));
        frame.render_widget(placeholder, inner);
    }
}
