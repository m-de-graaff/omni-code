//! Tab bar displaying open buffer tabs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Renders the tab bar row above the editor area.
pub struct TabBar;

impl TabBar {
    pub fn render(frame: &mut Frame, area: Rect) {
        let tabs = Line::from(vec![
            Span::styled(" [scratch] ", Style::new().fg(Color::White).bg(Color::DarkGray)),
            Span::styled(" [untitled] ", Style::new().fg(Color::DarkGray)),
        ]);
        let bar = Paragraph::new(tabs).style(Style::new().bg(Color::Black));
        frame.render_widget(bar, area);
    }
}
