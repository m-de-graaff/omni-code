//! Interactive tab bar with click, close, scroll, drag-to-reorder, and indicators.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Per-tab metadata.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub label: String,
    pub modified: bool,
    pub ai_modified: bool,
    pub icon: &'static str,
}

impl TabInfo {
    pub fn new(label: impl Into<String>, icon: &'static str) -> Self {
        Self { label: label.into(), modified: false, ai_modified: false, icon }
    }

    fn rendered_width(&self) -> u16 {
        let icon_w = if self.icon.is_empty() { 0 } else { 2 };
        let indicator_w = if self.modified || self.ai_modified { 2 } else { 0 };
        let label_w = u16::try_from(self.label.len()).unwrap_or(u16::MAX);
        1 + icon_w + label_w + indicator_w + 2
    }
}

/// Semantic action returned by tab bar mouse handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    Switch(usize),
    Close(usize),
    Reorder { from: usize, to: usize },
    Handled,
}

#[derive(Debug, Clone, Copy, Default)]
enum TabDrag {
    #[default]
    None,
    Dragging {
        from_idx: usize,
    },
}

/// Interactive tab bar widget with hit-testing and drag support.
pub struct TabBar {
    tabs: Vec<TabInfo>,
    active: usize,
    scroll_offset: usize,
    tab_rects: Vec<(usize, Rect)>,
    close_rects: Vec<(usize, Rect)>,
    drag: TabDrag,
}

impl TabBar {
    pub const fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            scroll_offset: 0,
            tab_rects: Vec::new(),
            close_rects: Vec::new(),
            drag: TabDrag::None,
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// The currently active tab index.
    pub const fn active_index(&self) -> usize {
        self.active
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn set_active(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
            self.ensure_active_visible();
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active > idx {
            self.active -= 1;
        }
        self.ensure_active_visible();
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if self.active == from {
            self.active = to;
        } else if from < self.active && to >= self.active {
            self.active -= 1;
        } else if from > self.active && to <= self.active {
            self.active += 1;
        }
    }

    #[allow(dead_code)]
    pub fn add_tab(&mut self, info: TabInfo) {
        self.tabs.push(info);
        self.active = self.tabs.len() - 1;
        self.ensure_active_visible();
    }

    const fn ensure_active_visible(&mut self) {
        if self.active < self.scroll_offset {
            self.scroll_offset = self.active;
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &ThemeColors) {
        self.tab_rects.clear();
        self.close_rects.clear();

        if area.width == 0 || area.height == 0 || self.tabs.is_empty() {
            return;
        }

        let mut spans: Vec<Span> = Vec::new();
        let mut x = area.x;
        let max_x = area.right();
        let overflow_width: u16 = 6;

        let total = self.tabs.len();

        for (rendered_count, (i, tab)) in
            self.tabs.iter().enumerate().skip(self.scroll_offset).enumerate()
        {
            let tab_w = tab.rendered_width();
            let remaining_tabs = total - i - 1;
            let needs_overflow = remaining_tabs > 0;
            let available =
                if needs_overflow { max_x.saturating_sub(overflow_width) } else { max_x };

            if x + tab_w > available && rendered_count > 0 {
                let overflow_count = total - i;
                let overflow_text = format!(" \u{2026} +{overflow_count} ");
                spans.push(Span::styled(overflow_text, Style::new().fg(theme.text_muted)));
                break;
            }

            let is_active = i == self.active;
            let style = if is_active {
                Style::new()
                    .fg(theme.tab_active_fg)
                    .bg(theme.tab_active_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.tab_inactive_fg).bg(theme.tab_bar_bg)
            };
            let close_style = if is_active {
                Style::new().fg(theme.tab_close).bg(theme.tab_active_bg)
            } else {
                Style::new().fg(theme.text_muted).bg(theme.tab_bar_bg)
            };

            let mut tab_text = String::with_capacity(tab_w as usize);
            tab_text.push(' ');
            if !tab.icon.is_empty() {
                tab_text.push_str(tab.icon);
                tab_text.push(' ');
            }
            tab_text.push_str(&tab.label);
            if tab.ai_modified {
                tab_text.push_str(" \u{2726}");
            } else if tab.modified {
                tab_text.push_str(" \u{25cf}");
            }
            tab_text.push(' ');

            spans.push(Span::styled(tab_text, style));
            spans.push(Span::styled("\u{00d7} ", close_style));

            let tab_rect = Rect::new(x, area.y, tab_w, 1);
            let close_rect = Rect::new(x + tab_w - 2, area.y, 2, 1);
            self.tab_rects.push((i, tab_rect));
            self.close_rects.push((i, close_rect));

            x += tab_w;
        }

        if x < max_x {
            let fill_w = max_x - x;
            spans
                .push(Span::styled(" ".repeat(fill_w as usize), Style::new().bg(theme.tab_bar_bg)));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::new().bg(theme.tab_bar_bg));
        frame.render_widget(bar, area);
    }

    pub fn handle_mouse(&mut self, event: MouseEvent) -> Option<TabAction> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                for &(idx, rect) in &self.close_rects {
                    if rect.contains(pos) {
                        return Some(TabAction::Close(idx));
                    }
                }
                for &(idx, rect) in &self.tab_rects {
                    if rect.contains(pos) {
                        self.drag = TabDrag::Dragging { from_idx: idx };
                        return Some(TabAction::Switch(idx));
                    }
                }
                None
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                let pos = Position::new(event.column, event.row);
                for &(idx, rect) in &self.tab_rects {
                    if rect.contains(pos) {
                        return Some(TabAction::Close(idx));
                    }
                }
                None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let TabDrag::Dragging { from_idx } = self.drag {
                    let pos = Position::new(event.column, event.row);
                    for &(idx, rect) in &self.tab_rects {
                        if rect.contains(pos) && idx != from_idx {
                            self.drag = TabDrag::Dragging { from_idx: idx };
                            return Some(TabAction::Reorder { from: from_idx, to: idx });
                        }
                    }
                }
                Some(TabAction::Handled)
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if !matches!(self.drag, TabDrag::None) {
                    self.drag = TabDrag::None;
                    return Some(TabAction::Handled);
                }
                None
            }
            MouseEventKind::ScrollUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                    return Some(TabAction::Handled);
                }
                None
            }
            MouseEventKind::ScrollDown => {
                if self.scroll_offset + 1 < self.tabs.len() {
                    self.scroll_offset += 1;
                    return Some(TabAction::Handled);
                }
                None
            }
            _ => None,
        }
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}
