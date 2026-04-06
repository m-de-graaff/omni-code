//! VS Code-style command palette with fuzzy search.
//!
//! Pushed onto the compositor as a modal layer. Type to filter,
//! Up/Down to navigate, Enter to execute, Escape to dismiss.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use omni_event::Action;

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A command palette entry.
struct PaletteEntry {
    label: &'static str,
    shortcut: Option<&'static str>,
    action: Action,
}

/// Build the static command registry.
fn build_commands() -> Vec<PaletteEntry> {
    vec![
        PaletteEntry {
            label: "New File",
            shortcut: Some("Ctrl+N"),
            action: Action::Command("new_file".into()),
        },
        PaletteEntry {
            label: "Open Folder",
            shortcut: Some("Ctrl+O"),
            action: Action::Command("open_folder".into()),
        },
        PaletteEntry { label: "Save", shortcut: Some("Ctrl+S"), action: Action::Save },
        PaletteEntry { label: "Close Tab", shortcut: Some("Ctrl+W"), action: Action::CloseBuffer },
        PaletteEntry {
            label: "Toggle Sidebar",
            shortcut: Some("Ctrl+B"),
            action: Action::ToggleSidebar,
        },
        PaletteEntry {
            label: "Toggle Bottom Panel",
            shortcut: Some("Ctrl+J"),
            action: Action::ToggleBottomPanel,
        },
        PaletteEntry { label: "Toggle Minimap", shortcut: None, action: Action::ToggleMinimap },
        PaletteEntry {
            label: "Cycle App Mode (IDE/Split/Chat)",
            shortcut: Some("Ctrl+Shift+A"),
            action: Action::ToggleAppMode,
        },
        PaletteEntry {
            label: "Focus Next Panel",
            shortcut: Some("Ctrl+Tab"),
            action: Action::FocusNext,
        },
        PaletteEntry {
            label: "Focus Previous Panel",
            shortcut: Some("Ctrl+Shift+Tab"),
            action: Action::FocusPrev,
        },
        PaletteEntry { label: "Vertical Split", shortcut: None, action: Action::VerticalSplit },
        PaletteEntry { label: "Horizontal Split", shortcut: None, action: Action::HorizontalSplit },
        PaletteEntry { label: "File Picker", shortcut: None, action: Action::FilePicker },
        PaletteEntry { label: "Quit", shortcut: Some("Ctrl+Q"), action: Action::Quit },
    ]
}

/// VS Code-style command palette with fuzzy search.
pub struct CommandPalette {
    entries: Vec<PaletteEntry>,
    filtered: Vec<usize>,
    query: String,
    selected: usize,
    scroll_offset: usize,
    list_area: Option<Rect>,
    theme: ThemeColors,
}

impl CommandPalette {
    pub fn new(theme: ThemeColors) -> Self {
        let entries = build_commands();
        let filtered: Vec<usize> = (0..entries.len()).collect();
        Self {
            entries,
            filtered,
            query: String::new(),
            selected: 0,
            scroll_offset: 0,
            list_area: None,
            theme,
        }
    }

    /// Refilter entries based on the current query.
    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| q.is_empty() || e.label.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    const fn ensure_visible(&mut self, visible_rows: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if visible_rows > 0 && self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected - visible_rows + 1;
        }
    }

    fn compute_rect(terminal: Rect) -> Rect {
        let width = (terminal.width * 3 / 5).clamp(40, 70);
        let height = (terminal.height / 2).clamp(8, 20);
        let x = terminal.x + (terminal.width.saturating_sub(width)) / 2;
        let y = terminal.y + terminal.height / 6; // top third
        Rect::new(x, y, width, height)
    }
}

impl Component for CommandPalette {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let modal = Self::compute_rect(area);

        // Pre-compute scroll before borrowing theme
        let height_estimate = modal.height.saturating_sub(5) as usize;
        self.ensure_visible(height_estimate);

        let theme = &self.theme;

        frame.render_widget(Clear, modal);

        let block = Block::bordered()
            .title(" Command Palette ")
            .border_style(Style::new().fg(theme.border_focused))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        if inner.height < 2 {
            return;
        }

        // Input line: "> query_text"
        let input_line = Line::from(vec![
            Span::styled("> ", Style::new().fg(theme.text_accent).add_modifier(Modifier::BOLD)),
            Span::styled(&self.query, Style::new().fg(theme.foreground)),
            Span::styled("\u{2588}", Style::new().fg(theme.cursor)), // block cursor
        ]);
        frame
            .render_widget(Paragraph::new(input_line), Rect::new(inner.x, inner.y, inner.width, 1));

        // Separator
        let sep_y = inner.y + 1;
        if sep_y >= inner.bottom() {
            return;
        }
        let sep = "\u{2500}".repeat(inner.width as usize);
        frame.render_widget(
            Paragraph::new(sep).style(Style::new().fg(theme.border)),
            Rect::new(inner.x, sep_y, inner.width, 1),
        );

        // List area
        let list_top = sep_y + 1;
        let list_height = inner.bottom().saturating_sub(list_top);
        let list_rect = Rect::new(inner.x, list_top, inner.width, list_height);
        self.list_area = Some(list_rect);

        let visible_rows = list_height as usize;

        // Render filtered entries
        for (vi, &entry_idx) in
            self.filtered.iter().enumerate().skip(self.scroll_offset).take(visible_rows)
        {
            let entry = &self.entries[entry_idx];
            #[allow(clippy::cast_possible_truncation)]
            let row_y = list_top + (vi - self.scroll_offset) as u16;
            let is_selected = vi == self.selected;

            let label_style = if is_selected {
                Style::new()
                    .fg(theme.foreground)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.foreground)
            };
            let shortcut_style = if is_selected {
                Style::new().fg(theme.text_muted).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.text_muted)
            };
            let bg_style =
                if is_selected { Style::new().bg(theme.selection_bg) } else { Style::new() };

            let shortcut_text = entry.shortcut.unwrap_or("");
            let label_w =
                inner.width.saturating_sub(u16::try_from(shortcut_text.len()).unwrap_or(0) + 2);

            let mut spans = vec![Span::styled(
                format!(" {:<width$}", entry.label, width = label_w.saturating_sub(1) as usize),
                label_style,
            )];
            if shortcut_text.is_empty() {
                spans.push(Span::styled(" ", bg_style));
            } else {
                spans.push(Span::styled(format!("{shortcut_text} "), shortcut_style));
            }

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(inner.x, row_y, inner.width, 1),
            );
        }

        // Empty state
        if self.filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("  No matching commands")
                    .style(Style::new().fg(theme.text_muted).add_modifier(Modifier::ITALIC)),
                list_rect,
            );
        }
    }

    fn handle_key(
        &mut self,
        event: KeyEvent,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        if event.kind != KeyEventKind::Press {
            return Ok(EventResult::Consumed);
        }
        match event.code {
            KeyCode::Esc => Ok(Self::dismiss()),
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Enter => {
                if let Some(&entry_idx) = self.filtered.get(self.selected) {
                    let action = self.entries[entry_idx].action.clone();
                    let _ = ctx.action_tx.send(action);
                }
                Ok(Self::dismiss())
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.refilter();
                Ok(EventResult::Consumed)
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.refilter();
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Consumed),
        }
    }

    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        _area: Rect,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                if let Some(la) = self.list_area {
                    if la.contains(pos) {
                        let row_idx = (event.row - la.y) as usize + self.scroll_offset;
                        if row_idx < self.filtered.len() {
                            let entry_idx = self.filtered[row_idx];
                            let action = self.entries[entry_idx].action.clone();
                            let _ = ctx.action_tx.send(action);
                            return Ok(Self::dismiss());
                        }
                    }
                }
                // Click outside → dismiss
                Ok(Self::dismiss())
            }
            MouseEventKind::ScrollUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Ok(EventResult::Consumed)
            }
            MouseEventKind::ScrollDown => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Consumed),
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
