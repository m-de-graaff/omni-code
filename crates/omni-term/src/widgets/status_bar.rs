//! Status bar displaying mode, file, cursor position, and branch info.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Renders the status bar pinned at the bottom of the terminal.
pub struct StatusBar;

impl StatusBar {
    pub fn render(frame: &mut Frame, area: Rect) {
        let line = Line::from(vec![
            Span::styled(" NORMAL ", Style::new().fg(Color::Black).bg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("[scratch]", Style::new().fg(Color::White)),
            Span::raw("  "),
            Span::styled("1:1", Style::new().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("main", Style::new().fg(Color::Yellow)),
        ]);
        let bar = Paragraph::new(line).style(Style::new().bg(Color::DarkGray));
        frame.render_widget(bar, area);
    }
}
