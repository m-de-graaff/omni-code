//! In-buffer search bar rendered at the bottom of the editor pane.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use omni_core::Text;
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// In-buffer search state and UI.
pub struct SearchBar {
    /// The search query.
    pub query: String,
    /// All match positions as `(start_char, end_char)` pairs.
    pub matches: Vec<(usize, usize)>,
    /// Index of the currently selected match.
    pub current_match: usize,
    /// Whether the search is case-sensitive.
    pub case_sensitive: bool,
    /// Whether the search bar is active/visible.
    pub active: bool,
    /// Whether regex search mode is active.
    pub regex_mode: bool,
    /// Whether replace mode is active.
    pub replace_mode: bool,
    /// The replacement text.
    pub replace_text: String,
    /// Whether the replace input is focused (vs search input).
    replace_focused: bool,
    /// Regex compilation error message (when regex_mode is on and pattern is invalid).
    pub regex_error: Option<String>,
    /// Document version when matches were last computed (for cache invalidation).
    doc_version: u64,
}

impl SearchBar {
    /// Create a new inactive search bar.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current_match: 0,
            case_sensitive: false,
            active: false,
            regex_mode: false,
            replace_mode: false,
            replace_text: String::new(),
            replace_focused: false,
            regex_error: None,
            doc_version: u64::MAX,
        }
    }

    /// Activate the search bar (Ctrl+F).
    pub const fn activate(&mut self) {
        self.active = true;
        // Select all text in query for easy replacement
    }

    /// Deactivate and reset.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.regex_mode = false;
        self.replace_mode = false;
        self.replace_focused = false;
        self.matches.clear();
        self.query.clear();
        self.replace_text.clear();
    }

    /// Update matches if the query or document changed.
    pub fn update_matches(&mut self, text: &Text, doc_version: u64) {
        if self.query.is_empty() {
            self.matches.clear();
            self.current_match = 0;
            self.doc_version = doc_version;
            return;
        }
        if doc_version == self.doc_version && !self.matches.is_empty() {
            return; // cached
        }
        self.regex_error = None;
        self.matches = if self.regex_mode {
            match text.find_all_regex(&self.query, self.case_sensitive) {
                Ok(m) => m,
                Err(e) => {
                    self.regex_error = Some(e.to_string());
                    Vec::new()
                }
            }
        } else {
            text.find_all(&self.query, self.case_sensitive)
        };
        if self.current_match >= self.matches.len() {
            self.current_match = 0;
        }
        self.doc_version = doc_version;
    }

    /// Force a re-search (after query change).
    pub fn force_update(&mut self, text: &Text, doc_version: u64) {
        self.doc_version = u64::MAX; // invalidate cache
        self.update_matches(text, doc_version);
    }

    /// Move to the next match.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Move to the previous match.
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.matches.len() - 1
            } else {
                self.current_match - 1
            };
        }
    }

    /// The char position of the current match (for scrolling to it).
    #[must_use]
    pub fn current_match_pos(&self) -> Option<usize> {
        self.matches.get(self.current_match).map(|&(start, _)| start)
    }

    /// Handle a key event. Returns action to take.
    pub fn handle_key(&mut self, event: KeyEvent) -> SearchBarAction {
        match event.code {
            KeyCode::Esc => {
                self.deactivate();
                SearchBarAction::Consumed
            }
            KeyCode::Tab if self.replace_mode => {
                self.replace_focused = !self.replace_focused;
                SearchBarAction::Consumed
            }
            KeyCode::Enter => {
                if self.replace_focused {
                    // In replace field: Enter = replace current match
                    if event.modifiers.contains(KeyModifiers::CONTROL) {
                        SearchBarAction::ReplaceAll
                    } else {
                        SearchBarAction::ReplaceOne
                    }
                } else if event.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_match();
                    SearchBarAction::Consumed
                } else {
                    self.next_match();
                    SearchBarAction::Consumed
                }
            }
            KeyCode::Backspace => {
                if self.replace_focused {
                    self.replace_text.pop();
                } else {
                    self.query.pop();
                }
                SearchBarAction::Consumed
            }
            KeyCode::Char(c) => {
                if event.modifiers.contains(KeyModifiers::ALT) && (c == 'r' || c == 'R') {
                    self.regex_mode = !self.regex_mode;
                } else if event.modifiers.contains(KeyModifiers::ALT) && (c == 'c' || c == 'C') {
                    self.case_sensitive = !self.case_sensitive;
                } else if !event.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.replace_focused {
                        self.replace_text.push(c);
                    } else {
                        self.query.push(c);
                    }
                } else {
                    return SearchBarAction::Ignored;
                }
                SearchBarAction::Consumed
            }
            _ => SearchBarAction::Ignored,
        }
    }

    /// Render the search bar into the given 2-row area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &ThemeColors) {
        if area.height == 0 || area.width < 10 {
            return;
        }

        // Background
        let bg_style = Style::new().bg(theme.panel_bg);
        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                if let Some(cell) = frame.buffer_mut().cell_mut((col, row)) {
                    cell.set_style(bg_style);
                    cell.set_char(' ');
                }
            }
        }

        // Search input line
        let case_indicator = if self.case_sensitive { "[Aa]" } else { "[aa]" };
        let regex_indicator = if self.regex_mode { "[.*]" } else { "[  ]" };
        let match_info = if let Some(ref err) = self.regex_error {
            format!("Regex error: {}", err.chars().take(30).collect::<String>())
        } else if self.matches.is_empty() {
            if self.query.is_empty() {
                String::new()
            } else {
                "No results".to_string()
            }
        } else {
            format!("{} of {}", self.current_match + 1, self.matches.len())
        };

        let line = Line::from(vec![
            Span::styled(" \u{f002} ", Style::new().fg(theme.text_accent)), // nf-fa-search
            Span::styled(&self.query, Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("\u{2588}", Style::new().fg(theme.cursor)), // cursor block
            Span::styled(format!("  {match_info}  "), Style::new().fg(theme.text_muted)),
            Span::styled(case_indicator, Style::new().fg(if self.case_sensitive { theme.text_accent } else { theme.text_muted })),
        ]);

        let search_style = if !self.replace_focused {
            Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.foreground)
        };
        let search_cursor = if !self.replace_focused {
            Span::styled("\u{2588}", Style::new().fg(theme.cursor))
        } else {
            Span::raw("")
        };

        let line = Line::from(vec![
            Span::styled(" \u{f002} ", Style::new().fg(theme.text_accent)),
            Span::styled(&self.query, search_style),
            search_cursor,
            Span::styled(format!("  {match_info}  "), Style::new().fg(theme.text_muted)),
            Span::styled(case_indicator, Style::new().fg(if self.case_sensitive { theme.text_accent } else { theme.text_muted })),
            Span::styled(" ", Style::new()),
            Span::styled(regex_indicator, Style::new().fg(if self.regex_mode { theme.text_accent } else { theme.text_muted })),
        ]);

        frame.render_widget(
            Paragraph::new(line),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // Replace row (if in replace mode)
        if self.replace_mode && area.height > 1 {
            let replace_style = if self.replace_focused {
                Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.foreground)
            };
            let replace_cursor = if self.replace_focused {
                Span::styled("\u{2588}", Style::new().fg(theme.cursor))
            } else {
                Span::raw("")
            };

            let replace_line = Line::from(vec![
                Span::styled(" \u{f061} ", Style::new().fg(theme.text_accent)), // arrow right
                Span::styled(&self.replace_text, replace_style),
                replace_cursor,
                Span::styled("  Enter:replace  Ctrl+Enter:all", Style::new().fg(theme.text_muted)),
            ]);
            frame.render_widget(
                Paragraph::new(replace_line),
                Rect::new(area.x, area.y + 1, area.width, 1),
            );
        }
    }
}

/// Actions emitted by the search bar key handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBarAction {
    Consumed,
    Ignored,
    ReplaceOne,
    ReplaceAll,
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}
