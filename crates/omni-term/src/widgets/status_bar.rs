//! Segmented status bar with mode indicator, file info, and clickable sections.

use super::layout_state::AppMode;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Editor mode for the mode indicator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // Variants will be used when real editing modes are implemented
pub enum EditorMode {
    #[default]
    Normal,
    Insert,
    Visual,
    Ai,
}

impl EditorMode {
    /// Display label for the mode.
    const fn label(self) -> &'static str {
        match self {
            Self::Normal => " NORMAL ",
            Self::Insert => " INSERT ",
            Self::Visual => " VISUAL ",
            Self::Ai => " AI ",
        }
    }
}

/// Clickable section of the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSection {
    Mode,
    File,
    CursorPos,
    Encoding,
    LineEnding,
    Language,
    Branch,
}

/// Action emitted when a status bar section is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarAction {
    ChangeLanguage,
    GitMenu,
    ChangeEncoding,
    ChangeLineEnding,
    CycleAppMode,
}

/// Mutable state displayed in the status bar.
pub struct StatusBarState {
    pub mode: EditorMode,
    pub app_mode: AppMode,
    pub filename: String,
    pub modified: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub encoding: &'static str,
    pub line_ending: &'static str,
    pub language: String,
    pub branch: String,
    /// Temporary notification message (shown on the right side).
    pub message: Option<String>,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            mode: EditorMode::Normal,
            app_mode: AppMode::Ide,
            filename: "[scratch]".into(),
            modified: false,
            cursor_line: 1,
            cursor_col: 1,
            encoding: "UTF-8",
            line_ending: if cfg!(windows) { "CRLF" } else { "LF" },
            language: "Rust".into(),
            branch: "main".into(),
            message: None,
        }
    }
}

/// Segmented status bar with clickable sections.
pub struct StatusBar {
    pub state: StatusBarState,
    section_rects: Vec<(StatusSection, Rect)>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { state: StatusBarState::default(), section_rects: Vec::new() }
    }

    /// Render the status bar with left-aligned and right-aligned sections.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &ThemeColors) {
        self.section_rects.clear();

        if area.width == 0 || area.height == 0 {
            return;
        }

        let bg = Style::new().bg(theme.status_bar_bg);
        let sep = Span::styled("  ", bg);

        // -- Build left spans --
        let mode_style = match self.state.mode {
            EditorMode::Normal => Style::new().fg(theme.mode_normal_fg).bg(theme.mode_normal_bg),
            EditorMode::Insert => Style::new().fg(theme.mode_insert_fg).bg(theme.mode_insert_bg),
            EditorMode::Visual => Style::new().fg(theme.foreground).bg(theme.selection_bg),
            EditorMode::Ai => Style::new().fg(theme.mode_ai_fg).bg(theme.mode_ai_bg),
        };

        let mode_text = format!("{}\u{b7}{}", self.state.mode.label(), self.state.app_mode.label());
        // " NORMAL·IDE " — combined label
        let file_text = if self.state.modified {
            format!("{} \u{25cf}", self.state.filename) // ●
        } else {
            self.state.filename.clone()
        };

        let mut left_spans = vec![
            Span::styled(mode_text.clone(), mode_style),
            sep.clone(),
            Span::styled(
                file_text.clone(),
                Style::new().fg(theme.foreground).bg(theme.status_bar_bg),
            ),
        ];

        // -- Build right spans (rendered right-to-left) --
        let cursor_text = format!("{}:{}", self.state.cursor_line, self.state.cursor_col);
        let right_items: Vec<(&str, StatusSection, Style)> = vec![
            (
                &self.state.branch,
                StatusSection::Branch,
                Style::new().fg(theme.ai_marker).bg(theme.status_bar_bg),
            ),
            (
                &self.state.language,
                StatusSection::Language,
                Style::new().fg(theme.text_accent).bg(theme.status_bar_bg),
            ),
            (
                self.state.line_ending,
                StatusSection::LineEnding,
                Style::new().fg(theme.text_muted).bg(theme.status_bar_bg),
            ),
            (
                self.state.encoding,
                StatusSection::Encoding,
                Style::new().fg(theme.text_muted).bg(theme.status_bar_bg),
            ),
            (
                &cursor_text,
                StatusSection::CursorPos,
                Style::new().fg(theme.text_muted).bg(theme.status_bar_bg),
            ),
        ];

        // Calculate widths
        let left_width = mode_text.len() + 2 + file_text.len();
        let right_width: usize = right_items
            .iter()
            .map(|(text, _, _)| text.len() + 2) // text + separator
            .sum();

        let total_width = area.width as usize;
        let center_gap = total_width.saturating_sub(left_width + right_width);

        // Store left section rects
        let mut x = area.x;
        let mode_w = u16::try_from(mode_text.len()).unwrap_or(0);
        self.section_rects.push((StatusSection::Mode, Rect::new(x, area.y, mode_w, 1)));
        x += mode_w + 2; // mode + sep
        let file_w = u16::try_from(file_text.len()).unwrap_or(0);
        self.section_rects.push((StatusSection::File, Rect::new(x, area.y, file_w, 1)));

        // Center gap
        if center_gap > 0 {
            left_spans.push(Span::styled(" ".repeat(center_gap), bg));
        }

        // Right spans + rects (in display order, left-to-right)
        let mut right_x = area.x + u16::try_from(left_width + center_gap).unwrap_or(0);
        let mut right_spans: Vec<Span> = Vec::new();
        for (text, section, style) in right_items.into_iter().rev() {
            right_spans.push(sep.clone());
            let w = u16::try_from(text.len()).unwrap_or(0);
            right_x += 2; // separator
            self.section_rects.push((section, Rect::new(right_x, area.y, w, 1)));
            right_spans.push(Span::styled(text.to_string(), style));
            right_x += w;
        }

        // Message notification (replaces right side when present)
        if let Some(ref msg) = self.state.message {
            left_spans.push(Span::styled(
                format!("  {msg}  "),
                Style::new().fg(theme.text_accent).bg(theme.status_bar_bg),
            ));
        }

        // Combine all spans
        left_spans.extend(right_spans);
        let line = Line::from(left_spans);
        let bar = Paragraph::new(line).style(bg);
        frame.render_widget(bar, area);
    }

    /// Handle a mouse event in the status bar. Returns an action if a clickable section was hit.
    pub fn handle_mouse(&self, event: MouseEvent) -> Option<StatusBarAction> {
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return None;
        }
        let pos = Position::new(event.column, event.row);
        for &(section, rect) in &self.section_rects {
            if rect.contains(pos) {
                return match section {
                    StatusSection::Mode => Some(StatusBarAction::CycleAppMode),
                    StatusSection::Language => Some(StatusBarAction::ChangeLanguage),
                    StatusSection::Branch => Some(StatusBarAction::GitMenu),
                    StatusSection::Encoding => Some(StatusBarAction::ChangeEncoding),
                    StatusSection::LineEnding => Some(StatusBarAction::ChangeLineEnding),
                    _ => None,
                };
            }
        }
        None
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
