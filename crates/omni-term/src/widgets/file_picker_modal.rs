//! Quick Open / Go-to-File fuzzy finder modal.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use omni_event::Action;
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A modal fuzzy file finder.
pub struct FilePickerModal {
    all_files: Vec<PathBuf>,
    filtered: Vec<usize>,
    query: String,
    selected: usize,
    scroll_offset: usize,
    list_area: Option<Rect>,
    theme: ThemeColors,
    workspace_root: PathBuf,
}

impl FilePickerModal {
    /// Create a new file picker, walking the workspace directory.
    pub fn new(workspace_root: PathBuf, theme: ThemeColors) -> Self {
        let all_files = collect_files(&workspace_root);
        let filtered: Vec<usize> = (0..all_files.len()).collect();
        Self {
            all_files,
            filtered,
            query: String::new(),
            selected: 0,
            scroll_offset: 0,
            list_area: None,
            theme,
            workspace_root,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.all_files.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered = self
                .all_files
                .iter()
                .enumerate()
                .filter(|(_, path)| {
                    let rel = path
                        .strip_prefix(&self.workspace_root)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_lowercase();
                    // Subsequence match
                    let mut qi = q.chars().peekable();
                    for ch in rel.chars() {
                        if qi.peek() == Some(&ch) {
                            qi.next();
                        }
                    }
                    qi.peek().is_none()
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    fn compute_rect(area: Rect) -> Rect {
        let width = (area.width * 3 / 5).clamp(40, 80);
        let height = (area.height / 2).clamp(8, 25);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + area.height / 6;
        Rect::new(x, y, width, height)
    }

    fn ensure_visible(&mut self, visible_rows: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if visible_rows > 0 && self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected - visible_rows + 1;
        }
    }
}

impl Component for FilePickerModal {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let modal = Self::compute_rect(area);

        let height_est = modal.height.saturating_sub(5) as usize;
        self.ensure_visible(height_est);

        let theme = &self.theme;

        frame.render_widget(Clear, modal);

        let block = Block::bordered()
            .title(" Quick Open ")
            .border_style(Style::new().fg(theme.border_focused))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        if inner.height < 2 {
            return;
        }

        // Input line
        let input_line = Line::from(vec![
            Span::styled("> ", Style::new().fg(theme.text_accent).add_modifier(Modifier::BOLD)),
            Span::styled(&self.query, Style::new().fg(theme.foreground)),
            Span::styled("\u{2588}", Style::new().fg(theme.cursor)),
            Span::styled(
                format!("  {} files", self.filtered.len()),
                Style::new().fg(theme.text_muted),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(input_line),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );

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

        for (vi, &file_idx) in self
            .filtered
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_rows)
        {
            let path = &self.all_files[file_idx];
            let display = path
                .strip_prefix(&self.workspace_root)
                .unwrap_or(path)
                .to_string_lossy();

            #[allow(clippy::cast_possible_truncation)]
            let row_y = list_top + (vi - self.scroll_offset) as u16;
            let is_selected = vi == self.selected;

            let style = if is_selected {
                Style::new()
                    .fg(theme.foreground)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.foreground)
            };

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(format!(" {display}"), style))),
                Rect::new(inner.x, row_y, inner.width, 1),
            );
        }

        if self.filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("  No matching files")
                    .style(Style::new().fg(theme.text_muted).add_modifier(Modifier::ITALIC)),
                list_rect,
            );
        }
    }

    fn handle_key(
        &mut self,
        event: KeyEvent,
        _ctx: &mut Context,
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
                if let Some(&file_idx) = self.filtered.get(self.selected) {
                    let path = self.all_files[file_idx].clone();
                    let _ = _ctx.action_tx.send(Action::OpenFile(path));
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
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                if let Some(la) = self.list_area {
                    if la.contains(pos) {
                        let row_idx = (event.row - la.y) as usize + self.scroll_offset;
                        if row_idx < self.filtered.len() {
                            let file_idx = self.filtered[row_idx];
                            let path = self.all_files[file_idx].clone();
                            let _ = _ctx.action_tx.send(Action::OpenFile(path));
                            return Ok(Self::dismiss());
                        }
                    }
                }
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

/// Walk the workspace and collect all file paths (respecting .gitignore).
fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .sort_by_file_name(std::cmp::Ord::cmp)
        .build();

    for entry in walker.flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
    files
}
