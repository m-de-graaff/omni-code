//! Project-wide search panel (Ctrl+Shift+F).

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

/// A search result from project-wide search.
#[derive(Debug, Clone)]
struct SearchResult {
    path: PathBuf,
    line_number: usize,
    line_text: String,
}

/// Project-wide search modal.
pub struct ProjectSearchPanel {
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
    scroll_offset: usize,
    list_area: Option<Rect>,
    theme: ThemeColors,
    workspace_root: PathBuf,
}

impl ProjectSearchPanel {
    pub fn new(workspace_root: PathBuf, theme: ThemeColors) -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            list_area: None,
            theme,
            workspace_root,
        }
    }

    fn search(&mut self) {
        self.results.clear();
        self.selected = 0;
        self.scroll_offset = 0;

        if self.query.is_empty() {
            return;
        }

        let query = &self.query;
        let walker = ignore::WalkBuilder::new(&self.workspace_root)
            .hidden(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            // Limit results to avoid excessive memory use
            if self.results.len() >= 500 {
                break;
            }
            let path = entry.into_path();
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(query.as_str()) {
                        self.results.push(SearchResult {
                            path: path.clone(),
                            line_number: i + 1,
                            line_text: line.to_string(),
                        });
                        if self.results.len() >= 500 {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    fn compute_rect(area: Rect) -> Rect {
        let width = (area.width * 4 / 5).clamp(50, 100);
        let height = (area.height * 3 / 4).clamp(10, 35);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 4;
        Rect::new(x, y, width, height)
    }

    fn ensure_visible(&mut self, visible_rows: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if visible_rows > 0 && self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected - visible_rows + 1;
        }
    }

    fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}

impl Component for ProjectSearchPanel {
    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let modal = Self::compute_rect(area);

        let height_est = modal.height.saturating_sub(5) as usize;
        self.ensure_visible(height_est);

        let theme = &self.theme;

        frame.render_widget(Clear, modal);

        let block = Block::bordered()
            .title(" Project Search ")
            .border_style(Style::new().fg(theme.border_focused))
            .style(Style::new().bg(theme.panel_bg));
        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        if inner.height < 2 {
            return;
        }

        // Input line
        let result_info = if self.results.is_empty() && !self.query.is_empty() {
            " No results".to_string()
        } else if !self.results.is_empty() {
            format!(" {} results", self.results.len())
        } else {
            String::new()
        };

        let input_line = Line::from(vec![
            Span::styled("\u{f002} ", Style::new().fg(theme.text_accent)),
            Span::styled(&self.query, Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("\u{2588}", Style::new().fg(theme.cursor)),
            Span::styled(result_info, Style::new().fg(theme.text_muted)),
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

        // Results list
        let list_top = sep_y + 1;
        let list_height = inner.bottom().saturating_sub(list_top);
        let list_rect = Rect::new(inner.x, list_top, inner.width, list_height);
        self.list_area = Some(list_rect);

        let visible_rows = list_height as usize;

        for (vi, result) in self
            .results
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(visible_rows)
        {
            #[allow(clippy::cast_possible_truncation)]
            let row_y = list_top + (vi - self.scroll_offset) as u16;
            let is_selected = vi == self.selected;

            let rel_path = self.relative_path(&result.path);
            let preview = result.line_text.trim();
            let max_preview = (inner.width as usize).saturating_sub(rel_path.len() + 8);
            let preview_truncated = if preview.len() > max_preview {
                &preview[..max_preview]
            } else {
                preview
            };

            let path_style = if is_selected {
                Style::new().fg(theme.text_accent).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.text_accent)
            };
            let line_style = if is_selected {
                Style::new().fg(theme.text_muted).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.text_muted)
            };
            let text_style = if is_selected {
                Style::new().fg(theme.foreground).bg(theme.selection_bg)
            } else {
                Style::new().fg(theme.foreground)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {rel_path}"), path_style),
                Span::styled(format!(":{}", result.line_number), line_style),
                Span::styled(format!(" {preview_truncated}"), text_style),
            ]);
            frame.render_widget(
                Paragraph::new(line),
                Rect::new(inner.x, row_y, inner.width, 1),
            );
        }

        if self.results.is_empty() && !self.query.is_empty() {
            frame.render_widget(
                Paragraph::new("  No matching results")
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
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Enter => {
                if let Some(result) = self.results.get(self.selected) {
                    let path = result.path.clone();
                    let _ = ctx.action_tx.send(Action::OpenFile(path));
                }
                Ok(Self::dismiss())
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.search();
                Ok(EventResult::Consumed)
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.search();
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
                        if row_idx < self.results.len() {
                            let path = self.results[row_idx].path.clone();
                            let _ = ctx.action_tx.send(Action::OpenFile(path));
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
                if self.selected + 1 < self.results.len() {
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
