//! Welcome/startup screen shown when no files are open.

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Action emitted when a startup screen item is activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    OpenFolder,
    NewFile,
    AiChat,
    CommandPalette,
    OpenRecent(usize),
}

/// Quick action definition for rendering and hit-testing.
struct QuickAction {
    label: &'static str,
    shortcut: &'static str,
    action: StartupAction,
}

const QUICK_ACTIONS: &[QuickAction] = &[
    QuickAction { label: "Open Folder", shortcut: "Ctrl+O", action: StartupAction::OpenFolder },
    QuickAction { label: "New File", shortcut: "Ctrl+N", action: StartupAction::NewFile },
    QuickAction { label: "AI Chat", shortcut: "Ctrl+Shift+A", action: StartupAction::AiChat },
    QuickAction {
        label: "Command Palette",
        shortcut: "Ctrl+P",
        action: StartupAction::CommandPalette,
    },
];

const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+B", "Toggle Sidebar"),
    ("Ctrl+J", "Toggle Bottom Panel"),
    ("Ctrl+Tab", "Cycle Focus"),
    ("Ctrl+Q", "Quit"),
];

const LOGO: &[&str] = &[
    "  \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2557}   \u{2588}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2588}\u{2557}   \u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2557}     \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}",
    " \u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557} \u{2588}\u{2588}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}    \u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}",
    " \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}    \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}  ",
    " \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}\u{255a}\u{2588}\u{2588}\u{2554}\u{255d}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}\u{255a}\u{2588}\u{2588}\u{2557}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}    \u{2588}\u{2588}\u{2551}     \u{2588}\u{2588}\u{2551}   \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}  \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2554}\u{2550}\u{2550}\u{255d}  ",
    " \u{255a}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255d}\u{2588}\u{2588}\u{2551} \u{255a}\u{2550}\u{255d} \u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551} \u{255a}\u{2588}\u{2588}\u{2588}\u{2588}\u{2551}\u{2588}\u{2588}\u{2551}    \u{255a}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}\u{255a}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255d}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2554}\u{255d}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2557}",
    "  \u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d} \u{255a}\u{2550}\u{255d}     \u{255a}\u{2550}\u{255d}\u{255a}\u{2550}\u{255d}  \u{255a}\u{2550}\u{2550}\u{2550}\u{255d}\u{255a}\u{2550}\u{255d}     \u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d} \u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d} \u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d} \u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}",
    "",
    "                  Terminal AI IDE  v0.1.0",
];

const LOGO_COMPACT: &[&str] = &["  O M N I   C O D E", "  Terminal AI IDE"];

/// Welcome screen with keyboard-navigable quick actions.
pub struct StartupScreen {
    selected: usize,
    action_rects: Vec<(StartupAction, Rect)>,
}

impl StartupScreen {
    pub const fn new() -> Self {
        Self { selected: 0, action_rects: Vec::new() }
    }

    /// Render the startup screen centered in the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &ThemeColors, recent_files: &[std::path::PathBuf]) {
        self.render_inner(frame, area, theme, recent_files);
    }

    fn render_inner(&mut self, frame: &mut Frame, area: Rect, theme: &ThemeColors, recent_files: &[std::path::PathBuf]) {
        self.action_rects.clear();

        let logo = if area.width >= 72 { LOGO } else { LOGO_COMPACT };

        if area.width < 30 || area.height < 12 {
            let text = Paragraph::new(logo[0])
                .alignment(Alignment::Center)
                .style(Style::new().fg(theme.text_accent).add_modifier(Modifier::BOLD));
            frame.render_widget(text, area);
            return;
        }

        let content_lines = logo.len() + 1 + 1 + QUICK_ACTIONS.len() + 1 + 1 + SHORTCUTS.len();
        #[allow(clippy::cast_possible_truncation)]
        let start_y = area.y + area.height.saturating_sub(content_lines as u16) / 2;
        let center_x = area.x + area.width / 2;

        let mut y = start_y;

        // -- Logo --
        for logo_line in logo {
            let line = Line::from(Span::styled(
                *logo_line,
                Style::new().fg(theme.text_accent).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center);
            frame.render_widget(Paragraph::new(line), Rect::new(area.x, y, area.width, 1));
            y += 1;
        }
        y += 1;

        // -- Quick Actions header --
        let header = Line::from(Span::styled(
            "Quick Actions",
            Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(Paragraph::new(header), Rect::new(area.x, y, area.width, 1));
        y += 1;

        // -- Quick Action items (with keyboard selection highlight) --
        for (i, qa) in QUICK_ACTIONS.iter().enumerate() {
            let label_width = 20u16;
            let shortcut_width = 14u16;
            let total_w = label_width + shortcut_width;
            let item_x = center_x.saturating_sub(total_w / 2);

            let is_selected = i == self.selected;
            let arrow = if is_selected { "\u{25b6} " } else { "  " }; // ▶ or space
            let label_style = if is_selected {
                Style::new().fg(theme.foreground).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.foreground)
            };
            let shortcut_style = if is_selected {
                Style::new().fg(theme.text_muted).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.text_muted)
            };
            let arrow_style = if is_selected {
                Style::new()
                    .fg(theme.text_accent)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.text_accent)
            };

            let line = Line::from(vec![
                Span::styled(arrow, arrow_style),
                Span::styled(
                    format!("{:<width$}", qa.label, width = (label_width - 2) as usize),
                    label_style,
                ),
                Span::styled(qa.shortcut, shortcut_style),
            ]);

            let item_rect = Rect::new(item_x, y, total_w, 1);
            frame.render_widget(Paragraph::new(line), item_rect);
            self.action_rects.push((qa.action, item_rect));
            y += 1;
        }
        y += 1;

        // -- Keyboard Shortcuts header --
        #[allow(clippy::cast_possible_truncation)]
        if y < area.bottom().saturating_sub(SHORTCUTS.len() as u16 + 1) {
            let header = Line::from(Span::styled(
                "Keyboard Shortcuts",
                Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center);
            frame.render_widget(Paragraph::new(header), Rect::new(area.x, y, area.width, 1));
            y += 1;

            for (key, desc) in SHORTCUTS {
                if y >= area.bottom() {
                    break;
                }
                let total_w = 30u16;
                let item_x = center_x.saturating_sub(total_w / 2);
                let line = Line::from(vec![
                    Span::styled(format!("{key:<12}"), Style::new().fg(theme.text_accent)),
                    Span::styled(*desc, Style::new().fg(theme.text_muted)),
                ]);
                frame.render_widget(Paragraph::new(line), Rect::new(item_x, y, total_w, 1));
                y += 1;
            }
        }

        // -- Recent Files --
        if !recent_files.is_empty() && y + 2 < area.bottom() {
            y += 1;
            let header = Line::from(Span::styled(
                "Recent Files",
                Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center);
            frame.render_widget(Paragraph::new(header), Rect::new(area.x, y, area.width, 1));
            y += 1;

            for (i, path) in recent_files.iter().take(8).enumerate() {
                if y >= area.bottom() {
                    break;
                }
                let display = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?");
                let dir = path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let total_w = 50u16;
                let item_x = center_x.saturating_sub(total_w / 2);
                let label = format!("  {display}  ({dir})");
                let style = Style::new().fg(theme.text_muted);
                let rect = Rect::new(item_x, y, total_w, 1);
                frame.render_widget(Paragraph::new(Line::from(Span::styled(label, style))), rect);
                self.action_rects.push((StartupAction::OpenRecent(i), rect));
                y += 1;
            }
        }
    }

    /// Handle a key event on the startup screen.
    ///
    /// Returns the selected action on Enter, or `None` for navigation keys.
    pub fn handle_key(&mut self, event: KeyEvent) -> Option<StartupAction> {
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                } else {
                    self.selected = QUICK_ACTIONS.len() - 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % QUICK_ACTIONS.len();
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => Some(QUICK_ACTIONS[self.selected].action),
            _ => None,
        }
    }

    /// Handle a mouse event on the startup screen.
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Option<StartupAction> {
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return None;
        }
        let pos = Position::new(event.column, event.row);
        for (i, &(action, rect)) in self.action_rects.iter().enumerate() {
            if rect.contains(pos) {
                self.selected = i;
                return Some(action);
            }
        }
        None
    }
}

impl Default for StartupScreen {
    fn default() -> Self {
        Self::new()
    }
}
