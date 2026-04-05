//! Main editor pane with optional minimap.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};

/// Renders the editor content area, optionally split with a minimap.
pub struct EditorPane;

impl EditorPane {
    pub fn render(frame: &mut Frame, area: Rect, minimap_visible: bool, minimap_width: u16) {
        if minimap_visible {
            let chunks =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(minimap_width)])
                    .split(area);
            Self::render_editor(frame, chunks[0]);
            Self::render_minimap(frame, chunks[1]);
        } else {
            Self::render_editor(frame, area);
        }
    }

    fn render_editor(frame: &mut Frame, area: Rect) {
        let block = Block::bordered().border_style(Style::new().fg(Color::DarkGray));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new("(editor)").style(Style::new().fg(Color::DarkGray));
        frame.render_widget(placeholder, inner);
    }

    fn render_minimap(frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" Map ").border_style(Style::new().fg(Color::DarkGray));
        frame.render_widget(block, area);
    }
}
