//! Main editor pane: viewport rendering with gutters, syntax highlighting,
//! cursor, selection overlays, and scrollbar.

use std::collections::HashSet;

use omni_core::{Selection, Text};
use omni_loader::{EditorConfig, ThemeColors};
use omni_syntax::HighlightSpan;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::component::CursorKind;

/// All data needed to render the editor viewport for one view.
pub struct EditorViewport<'a> {
    pub text: &'a Text,
    pub highlight_spans: &'a [HighlightSpan],
    pub ai_touched_lines: &'a HashSet<usize>,
    pub selection: Selection,
    pub scroll_offset: usize,
    pub col_offset: usize,
    pub total_lines: usize,
    pub config: &'a EditorConfig,
    /// Search match ranges (`start_char`, `end_char`) for highlighting.
    pub search_matches: &'a [(usize, usize)],
    /// Index of the currently active search match (distinct highlight).
    pub current_match_idx: Option<usize>,
}

/// Result of rendering the editor pane — contains cursor info for the shell.
pub struct EditorRenderResult {
    pub cursor: Option<(u16, u16, CursorKind)>,
}

/// Renders the editor content area, optionally split with a minimap.
pub struct EditorPane;

impl EditorPane {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        viewport: Option<&EditorViewport<'_>>,
        minimap_visible: bool,
        minimap_width: u16,
        is_focused: bool,
        theme: &ThemeColors,
    ) -> EditorRenderResult {
        let border_color = if is_focused { theme.border_focused } else { theme.border };

        let editor_area = if minimap_visible {
            let chunks =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(minimap_width)])
                    .split(area);
            if let Some(vp) = viewport {
                super::minimap::Minimap::render(frame, chunks[1], vp, theme);
            } else {
                Self::render_minimap_placeholder(frame, chunks[1], theme);
            }
            chunks[0]
        } else {
            area
        };

        Self::render_editor(frame, editor_area, viewport, border_color, is_focused, theme)
    }

    #[allow(clippy::too_many_lines)]
    fn render_editor(
        frame: &mut Frame,
        area: Rect,
        viewport: Option<&EditorViewport<'_>>,
        border_color: Color,
        is_focused: bool,
        theme: &ThemeColors,
    ) -> EditorRenderResult {
        let block = Block::bordered()
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(theme.background));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(vp) = viewport else {
            // No document open — show placeholder
            let placeholder =
                Paragraph::new("(editor)").style(Style::new().fg(theme.text_muted));
            frame.render_widget(placeholder, inner);
            return EditorRenderResult { cursor: None };
        };

        if inner.width == 0 || inner.height == 0 {
            return EditorRenderResult { cursor: None };
        }

        // Compute gutter widths
        let line_gutter_width = gutter_width(vp.total_lines);
        let has_ai_gutter = !vp.ai_touched_lines.is_empty();
        let ai_gutter_width: u16 = if has_ai_gutter { 2 } else { 0 };

        // Layout: [line_gutter | ai_gutter | code_area]
        let mut constraints = vec![Constraint::Length(line_gutter_width)];
        if has_ai_gutter {
            constraints.push(Constraint::Length(ai_gutter_width));
        }
        constraints.push(Constraint::Fill(1));

        let chunks = Layout::horizontal(constraints).split(inner);
        let (gutter_area, ai_area, code_area) = if has_ai_gutter {
            (chunks[0], Some(chunks[1]), chunks[2])
        } else {
            (chunks[0], None, chunks[1])
        };

        // Determine cursor position
        let cursor_head = vp.selection.primary().head;
        let cursor_line = if vp.text.len_chars() > 0 {
            vp.text.char_to_line(cursor_head.min(vp.text.len_chars().saturating_sub(1)))
        } else {
            0
        };
        let cursor_col = cursor_head.saturating_sub(vp.text.line_to_char(cursor_line));

        let visible_height = inner.height as usize;

        // Render each visible line
        for row in 0..visible_height {
            let line_idx = vp.scroll_offset + row;
            #[allow(clippy::cast_possible_truncation)]
            let y = gutter_area.y + row as u16;

            if line_idx >= vp.total_lines {
                // Past end of file — render tilde
                let tilde = Span::styled("~", Style::new().fg(theme.gutter_fg));
                frame.render_widget(
                    Paragraph::new(Line::from(tilde)),
                    Rect::new(gutter_area.x, y, gutter_area.width, 1),
                );
                continue;
            }

            let is_cursor_line = line_idx == cursor_line;

            // ── Line number gutter ──
            let num_style = if is_cursor_line {
                Style::new().fg(theme.foreground).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.gutter_fg)
            };
            let num_str = format!("{:>width$} ", line_idx + 1, width = (line_gutter_width - 1) as usize);
            frame.render_widget(
                Paragraph::new(Span::styled(num_str, num_style)),
                Rect::new(gutter_area.x, y, gutter_area.width, 1),
            );

            // ── AI marker gutter ──
            if let Some(ai_rect) = ai_area {
                if vp.ai_touched_lines.contains(&line_idx) {
                    let marker = Span::styled("\u{2726} ", Style::new().fg(theme.ai_marker));
                    frame.render_widget(
                        Paragraph::new(Line::from(marker)),
                        Rect::new(ai_rect.x, y, ai_rect.width, 1),
                    );
                }
            }

            // ── Code area: syntax-highlighted text ──
            let line_spans = build_styled_line(
                vp.text,
                line_idx,
                vp.col_offset,
                code_area.width as usize,
                vp.highlight_spans,
                &theme.syntax,
                theme.foreground,
                vp.config.tab_width,
            );

            // Cursor line highlight
            let line_bg = if is_cursor_line && is_focused {
                Style::new().bg(theme.hover_bg)
            } else {
                Style::default()
            };

            let styled_line = Line::from(line_spans);
            frame.render_widget(
                Paragraph::new(styled_line).style(line_bg),
                Rect::new(code_area.x, y, code_area.width, 1),
            );
        }

        // ── Scrollbar ──
        if vp.total_lines > visible_height {
            let mut scrollbar_state = ScrollbarState::new(vp.total_lines)
                .position(vp.scroll_offset)
                .viewport_content_length(visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                code_area,
                &mut scrollbar_state,
            );
        }

        // ── Cursor position ──
        let cursor = if is_focused
            && cursor_line >= vp.scroll_offset
            && cursor_line < vp.scroll_offset + visible_height
            && cursor_col >= vp.col_offset
            && (cursor_col - vp.col_offset) < code_area.width as usize
        {
            #[allow(clippy::cast_possible_truncation)]
            let x = code_area.x + (cursor_col - vp.col_offset) as u16;
            #[allow(clippy::cast_possible_truncation)]
            let y = code_area.y + (cursor_line - vp.scroll_offset) as u16;
            Some((x, y, CursorKind::Bar))
        } else {
            None
        };

        EditorRenderResult { cursor }
    }

    fn render_minimap_placeholder(frame: &mut Frame, area: Rect, theme: &ThemeColors) {
        let block = Block::bordered()
            .title(" Map ")
            .border_style(Style::new().fg(theme.border))
            .style(Style::new().bg(theme.background));
        frame.render_widget(block, area);
    }
}

/// Calculate gutter width based on total line count.
#[allow(clippy::cast_possible_truncation)]
const fn gutter_width(total_lines: usize) -> u16 {
    let digits = if total_lines == 0 {
        1
    } else {
        total_lines.ilog10() as u16 + 1
    };
    digits + 2 // 1 char padding on each side
}

/// Build a vector of styled `Span`s for a single line, applying syntax highlighting
/// and horizontal scrolling.
#[allow(clippy::too_many_arguments)]
fn build_styled_line(
    text: &Text,
    line_idx: usize,
    col_offset: usize,
    visible_cols: usize,
    highlight_spans: &[HighlightSpan],
    syntax: &omni_loader::SyntaxColors,
    default_fg: Color,
    tab_width: usize,
) -> Vec<Span<'static>> {
    let line_start_char = text.line_to_char(line_idx);
    let line_len = text.line_len_no_newline(line_idx);

    if line_len == 0 {
        return vec![];
    }

    let line_start_byte = text.char_to_byte(line_start_char);
    let _line_end_byte = text.char_to_byte(line_start_char + line_len);

    // Find highlight spans that overlap this line using binary search
    let span_start = highlight_spans
        .partition_point(|s| s.end_byte <= line_start_byte);

    // Build character-level styles
    let mut chars_and_styles: Vec<(char, Style)> = Vec::with_capacity(line_len);
    let mut byte_offset = line_start_byte;
    let mut span_idx = span_start;

    for i in 0..line_len {
        let ch = text.char_at(line_start_char + i);
        let char_byte_len = ch.len_utf8();

        // Find the applicable highlight span for this byte offset
        while span_idx < highlight_spans.len()
            && highlight_spans[span_idx].end_byte <= byte_offset
        {
            span_idx += 1;
        }

        let style = if span_idx < highlight_spans.len()
            && highlight_spans[span_idx].start_byte <= byte_offset
            && byte_offset < highlight_spans[span_idx].end_byte
        {
            syntax.style_for_scope(highlight_spans[span_idx].scope)
        } else {
            Style::new().fg(default_fg)
        };

        // Expand tabs
        if ch == '\t' {
            for _ in 0..tab_width {
                chars_and_styles.push((' ', style));
            }
        } else {
            chars_and_styles.push((ch, style));
        }

        byte_offset += char_byte_len;
    }

    // Apply horizontal scrolling
    let visible_start = col_offset.min(chars_and_styles.len());
    let visible_end = (col_offset + visible_cols).min(chars_and_styles.len());
    let visible = &chars_and_styles[visible_start..visible_end];

    // Group consecutive chars with same style into Spans
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_text = String::new();
    let mut current_style = Style::default();

    for &(ch, style) in visible {
        if style == current_style && !current_text.is_empty() {
            current_text.push(ch);
        } else {
            if !current_text.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut current_text), current_style));
            }
            current_text.push(ch);
            current_style = style;
        }
    }
    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gutter_width_adapts() {
        assert_eq!(gutter_width(1), 3);     // 1 digit + 2 padding
        assert_eq!(gutter_width(99), 4);    // 2 digits + 2 padding
        assert_eq!(gutter_width(999), 5);   // 3 digits + 2 padding
        assert_eq!(gutter_width(10000), 7); // 5 digits + 2 padding
    }

    #[test]
    fn gutter_width_zero() {
        assert_eq!(gutter_width(0), 3); // at least 1 digit
    }
}
