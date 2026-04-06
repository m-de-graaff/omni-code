//! Folder picker modal for selecting a working directory.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use omni_event::Action;

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A directory entry in the folder picker.
struct DirEntry {
    name: String,
    is_dir: bool,
    path: PathBuf,
}

/// Modal folder picker for selecting a working directory.
///
/// Pushed onto the compositor as a modal layer. Navigate with arrow keys,
/// Enter to descend/confirm, Backspace to go up, Escape to cancel.
pub struct FolderPicker {
    current_dir: PathBuf,
    entries: Vec<DirEntry>,
    selected: usize,
    scroll_offset: usize,
    area: Option<Rect>,
    list_area: Option<Rect>,
    theme: ThemeColors,
}

impl FolderPicker {
    /// Create a new folder picker starting at the given directory.
    pub fn new(start_dir: PathBuf, theme: ThemeColors) -> Self {
        let mut picker = Self {
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            area: None,
            list_area: None,
            theme,
        };
        picker.refresh_entries();
        picker
    }

    /// Read the current directory and populate entries.
    fn refresh_entries(&mut self) {
        self.entries.clear();
        self.selected = 0;
        self.scroll_offset = 0;

        let Ok(read_dir) = std::fs::read_dir(&self.current_dir) else {
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files/dirs
            if name.starts_with('.') {
                continue;
            }
            let de = DirEntry { name, is_dir: metadata.is_dir(), path: entry.path() };
            if de.is_dir {
                dirs.push(de);
            } else {
                files.push(de);
            }
        }

        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        self.entries.extend(dirs);
        self.entries.extend(files);
    }

    /// Navigate into a subdirectory.
    fn descend(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.refresh_entries();
            }
        }
    }

    /// Navigate to the parent directory.
    fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(Path::to_path_buf) {
            self.current_dir = parent;
            self.refresh_entries();
        }
    }

    /// Dismiss the picker.
    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    /// Compute the modal rect centered in the terminal.
    fn compute_rect(terminal: Rect) -> Rect {
        let width = (terminal.width * 4 / 5).min(80);
        let height = (terminal.height * 7 / 10).min(30);
        let x = terminal.x + (terminal.width.saturating_sub(width)) / 2;
        let y = terminal.y + (terminal.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Ensure selected item is visible in the scroll window.
    const fn ensure_visible(&mut self, visible_rows: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected.saturating_sub(visible_rows - 1);
        }
    }
}

impl Component for FolderPicker {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let modal = Self::compute_rect(area);
        self.area = Some(modal);

        // Pre-compute scroll before borrowing theme
        let list_height_estimate = modal.height.saturating_sub(6) as usize;
        self.ensure_visible(list_height_estimate);

        frame.render_widget(Clear, modal);

        let theme = &self.theme;
        let block = Block::bordered()
            .title(" Open Folder ")
            .border_style(Style::new().fg(theme.border_focused))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        if inner.height < 3 {
            return;
        }

        // Header: current path
        let path_str = self.current_dir.to_string_lossy();
        let header = Line::from(vec![
            Span::styled(" \u{f07b} ", Style::new().fg(theme.text_accent)),
            Span::styled(
                path_str.as_ref(),
                Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(header), Rect::new(inner.x, inner.y, inner.width, 1));

        // Separator
        let sep_y = inner.y + 1;
        let sep = Line::from("\u{2500}".repeat(inner.width as usize));
        frame.render_widget(
            Paragraph::new(sep).style(Style::new().fg(theme.border)),
            Rect::new(inner.x, sep_y, inner.width, 1),
        );

        // Footer: instructions
        let footer_y = inner.bottom().saturating_sub(1);
        let footer = Line::from(vec![
            Span::styled(" Enter", Style::new().fg(theme.text_accent)),
            Span::styled(":Open  ", Style::new().fg(theme.text_muted)),
            Span::styled("Backspace", Style::new().fg(theme.text_accent)),
            Span::styled(":Up  ", Style::new().fg(theme.text_muted)),
            Span::styled("Ctrl+Enter", Style::new().fg(theme.text_accent)),
            Span::styled(":Select  ", Style::new().fg(theme.text_muted)),
            Span::styled("Esc", Style::new().fg(theme.text_accent)),
            Span::styled(":Cancel", Style::new().fg(theme.text_muted)),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(Paragraph::new(footer), Rect::new(inner.x, footer_y, inner.width, 1));

        // Entry list area
        let list_top = sep_y + 1;
        let list_height = footer_y.saturating_sub(list_top);
        let list_area = Rect::new(inner.x, list_top, inner.width, list_height);
        self.list_area = Some(list_area);

        let visible_rows = list_height as usize;

        // Render entries
        for (i, entry) in
            self.entries.iter().enumerate().skip(self.scroll_offset).take(visible_rows)
        {
            #[allow(clippy::cast_possible_truncation)]
            let row_y = list_top + (i - self.scroll_offset) as u16;
            let is_selected = i == self.selected;

            let icon = if entry.is_dir { "\u{f07b} " } else { "\u{f15c} " };
            let style = if is_selected {
                Style::new().fg(theme.foreground).bg(theme.selection_bg)
            } else if entry.is_dir {
                Style::new().fg(theme.text_accent)
            } else {
                Style::new().fg(theme.text_muted)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {icon}"), style),
                Span::styled(&entry.name, style),
            ]);
            frame.render_widget(Paragraph::new(line), Rect::new(inner.x, row_y, inner.width, 1));
        }

        // Empty directory message
        if self.entries.is_empty() {
            let msg = Paragraph::new("  (empty directory)")
                .style(Style::new().fg(theme.text_muted).add_modifier(Modifier::ITALIC));
            frame.render_widget(msg, list_area);
        }
    }

    fn handle_key(
        &mut self,
        event: KeyEvent,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        // Only handle key press, not release or repeat
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
                if self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Enter if event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                // Ctrl+Enter: select current directory
                let path = self.current_dir.clone();
                let _ = ctx.action_tx.send(Action::OpenFolder(path));
                Ok(Self::dismiss())
            }
            KeyCode::Enter => {
                // Enter on directory: descend. Enter on file: ignore.
                if let Some(entry) = self.entries.get(self.selected) {
                    if entry.is_dir {
                        self.descend(self.selected);
                    }
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Backspace => {
                self.go_up();
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Consumed), // modal: consume all keys
        }
    }

    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        _area: Rect,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                // Check if click is in the list area
                if let Some(la) = self.list_area {
                    if la.contains(pos) {
                        let row_idx = (event.row - la.y) as usize + self.scroll_offset;
                        if row_idx < self.entries.len() {
                            if self.selected == row_idx && self.entries[row_idx].is_dir {
                                // Double-click effect: descend on re-click of selected dir
                                self.descend(row_idx);
                            } else {
                                self.selected = row_idx;
                            }
                            return Ok(EventResult::Consumed);
                        }
                    }
                }
                // Click outside modal: dismiss
                if let Some(modal) = self.area {
                    if !modal.contains(pos) {
                        return Ok(Self::dismiss());
                    }
                }
                Ok(EventResult::Consumed)
            }
            MouseEventKind::ScrollUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Ok(EventResult::Consumed)
            }
            MouseEventKind::ScrollDown => {
                if self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Consumed), // modal: consume all
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
