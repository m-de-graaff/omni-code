//! File tree sidebar placeholder.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

/// Renders the file tree sidebar panel.
pub struct Sidebar;

impl Sidebar {
    pub fn render(frame: &mut Frame, area: Rect) {
        let block =
            Block::bordered().title(" Files ").border_style(Style::new().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new("(file tree)").style(Style::new().fg(Color::DarkGray));
        frame.render_widget(placeholder, inner);
    }
}
