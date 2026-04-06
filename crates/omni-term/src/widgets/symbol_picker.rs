//! Symbol picker popup: Ctrl+Shift+O lists document symbols for navigation.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use omni_loader::ThemeColors;
use omni_syntax::DocumentSymbol;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph};

use crate::Component;
use crate::component::EventResult;
use crate::context::Context;

/// A modal popup listing document symbols for quick navigation.
pub struct SymbolPickerPopup {
    symbols: Vec<DocumentSymbol>,
    filtered: Vec<usize>, // indices into `symbols`
    query: String,
    list_state: ListState,
    theme: ThemeColors,
}

impl SymbolPickerPopup {
    /// Create a new symbol picker with the given symbols.
    #[must_use]
    pub fn new(symbols: Vec<DocumentSymbol>, theme: ThemeColors) -> Self {
        let filtered: Vec<usize> = (0..symbols.len()).collect();
        let mut picker = Self {
            symbols,
            filtered,
            query: String::new(),
            list_state: ListState::default(),
            theme,
        };
        if !picker.filtered.is_empty() {
            picker.list_state.select(Some(0));
        }
        picker
    }

    fn compute_rect(area: Rect) -> Rect {
        let width = 60.min(area.width.saturating_sub(4));
        let height = 20.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 4;
        Rect::new(x, y, width, height)
    }

    fn dismiss() -> EventResult {
        EventResult::Callback(Box::new(|compositor| {
            compositor.pop();
        }))
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.symbols.len()).collect();
        } else {
            let query_lower = self.query.to_lowercase();
            self.filtered = self
                .symbols
                .iter()
                .enumerate()
                .filter(|(_, s)| s.name.to_lowercase().contains(&query_lower))
                .map(|(i, _)| i)
                .collect();
        }
        self.list_state.select(if self.filtered.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    fn selected_symbol(&self) -> Option<&DocumentSymbol> {
        let sel = self.list_state.selected()?;
        let &idx = self.filtered.get(sel)?;
        self.symbols.get(idx)
    }
}

impl Component for SymbolPickerPopup {
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
            KeyCode::Enter => {
                if let Some(symbol) = self.selected_symbol() {
                    let line = symbol.line;
                    // Jump to symbol's line
                    if let Some(focus_key) = ctx.view_tree.focus() {
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get(focus_key)
                        {
                            if let Some(doc) = ctx.documents.get_mut(view.doc_id) {
                                let char_pos = doc.text().line_to_char(line);
                                doc.set_selection(
                                    focus_key,
                                    omni_core::Selection::point(char_pos),
                                );
                            }
                        }
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get_mut(focus_key)
                        {
                            view.ensure_visible(line);
                        }
                    }
                }
                Ok(Self::dismiss())
            }
            KeyCode::Up => {
                let sel = self.list_state.selected().unwrap_or(0);
                if sel > 0 {
                    self.list_state.select(Some(sel - 1));
                }
                Ok(EventResult::Consumed)
            }
            KeyCode::Down => {
                let sel = self.list_state.selected().unwrap_or(0);
                if sel + 1 < self.filtered.len() {
                    self.list_state.select(Some(sel + 1));
                }
                Ok(EventResult::Consumed)
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

    fn render(&mut self, frame: &mut Frame, area: Rect, _ctx: &Context) {
        let popup = Self::compute_rect(area);

        frame.render_widget(Clear, popup);

        let block = Block::bordered()
            .title(" Go to Symbol ")
            .border_style(Style::new().fg(self.theme.border_focused))
            .style(Style::new().bg(self.theme.panel_bg));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        if inner.height < 2 {
            return;
        }

        // Input line
        let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
        let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);

        let input_line = Line::from(vec![
            Span::styled(
                &self.query,
                Style::new()
                    .fg(self.theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("\u{2588}", Style::new().fg(self.theme.cursor)),
            Span::styled(
                format!("  {} symbols", self.filtered.len()),
                Style::new().fg(self.theme.text_muted),
            ),
        ]);
        frame.render_widget(Paragraph::new(input_line), input_area);

        // Symbol list
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&idx| {
                let sym = &self.symbols[idx];
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", sym.kind.icon()),
                        Style::new().fg(self.theme.text_accent),
                    ),
                    Span::styled(&sym.name, Style::new().fg(self.theme.foreground)),
                    Span::styled(
                        format!("  :{}", sym.line + 1),
                        Style::new().fg(self.theme.text_muted),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let highlight_style = Style::new()
            .fg(self.theme.foreground)
            .bg(self.theme.selection_bg);

        let list = List::new(items).highlight_style(highlight_style);
        frame.render_stateful_widget(list, list_area, &mut self.list_state);
    }
}
