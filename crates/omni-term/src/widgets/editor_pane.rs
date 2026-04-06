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
    /// Matching bracket positions: (cursor_bracket_pos, matching_bracket_pos).
    pub bracket_match: Option<(usize, usize)>,
    /// Per-line git diff status for gutter markers.
    pub diff_status: &'a [omni_vcs::diff::LineDiffStatus],
}

/// Result of rendering the editor pane — contains cursor info for the shell.
pub struct EditorRenderResult {
    pub cursor: Option<(u16, u16, CursorKind)>,
    /// The code area rect (for mouse click → text position conversion).
    pub code_area: Option<Rect>,
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
            return EditorRenderResult { cursor: None, code_area: None };
        };

        if inner.width == 0 || inner.height == 0 {
            return EditorRenderResult { cursor: None, code_area: None };
        }

        // Compute gutter widths
        let line_gutter_width = gutter_width(vp.total_lines);
        let has_diff_gutter = !vp.diff_status.is_empty();
        let diff_gutter_width: u16 = if has_diff_gutter { 1 } else { 0 };
        let has_ai_gutter = !vp.ai_touched_lines.is_empty();
        let ai_gutter_width: u16 = if has_ai_gutter { 2 } else { 0 };

        // Layout: [line_gutter | diff_gutter? | ai_gutter? | code_area]
        let mut constraints = vec![Constraint::Length(line_gutter_width)];
        if has_diff_gutter {
            constraints.push(Constraint::Length(diff_gutter_width));
        }
        if has_ai_gutter {
            constraints.push(Constraint::Length(ai_gutter_width));
        }
        constraints.push(Constraint::Fill(1));

        let chunks = Layout::horizontal(constraints).split(inner);
        let mut ci = 0;
        let gutter_area = chunks[ci]; ci += 1;
        let diff_area = if has_diff_gutter { let a = Some(chunks[ci]); ci += 1; a } else { None };
        let ai_area = if has_ai_gutter { let a = Some(chunks[ci]); ci += 1; a } else { None };
        let code_area = chunks[ci];

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

            // ── Line number gutter with diff marker ──
            let diff_marker = vp.diff_status.get(line_idx).copied();
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
            // ── Diff gutter ──
            if let Some(diff_rect) = diff_area {
                match diff_marker {
                    Some(omni_vcs::diff::LineDiffStatus::Added) => {
                        if let Some(cell) = frame.buffer_mut().cell_mut((diff_rect.x, y)) {
                            cell.set_char('\u{2503}');
                            cell.set_fg(Color::Green);
                        }
                    }
                    Some(omni_vcs::diff::LineDiffStatus::Modified) => {
                        if let Some(cell) = frame.buffer_mut().cell_mut((diff_rect.x, y)) {
                            cell.set_char('\u{2503}');
                            cell.set_fg(Color::Yellow);
                        }
                    }
                    _ => {}
                }
            }

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

            // ── Selection highlighting ──
            let line_start_char = vp.text.line_to_char(line_idx);
            let line_end_char = line_start_char + vp.text.line_len_no_newline(line_idx);

            for range in vp.selection.ranges() {
                let sel_start = range.start().max(line_start_char);
                let sel_end = range.end().min(line_end_char);
                if sel_start < sel_end {
                    // Selection overlaps this line
                    let col_start = sel_start - line_start_char;
                    let col_end = sel_end - line_start_char;
                    let vis_start = col_start.saturating_sub(vp.col_offset);
                    let vis_end = col_end.saturating_sub(vp.col_offset).min(code_area.width as usize);
                    if vis_start < vis_end {
                        for col in vis_start..vis_end {
                            #[allow(clippy::cast_possible_truncation)]
                            let cx = code_area.x + col as u16;
                            if let Some(cell) = frame.buffer_mut().cell_mut((cx, y)) {
                                cell.set_bg(theme.selection_bg);
                            }
                        }
                    }
                }
            }
        }

        // ── Bracket match highlights ──
        if let Some((pos_a, pos_b)) = vp.bracket_match {
            for &bracket_pos in &[pos_a, pos_b] {
                let b_line = if vp.text.len_chars() > 0 {
                    vp.text.char_to_line(bracket_pos.min(vp.text.len_chars().saturating_sub(1)))
                } else {
                    continue;
                };
                if b_line >= vp.scroll_offset && b_line < vp.scroll_offset + visible_height {
                    let b_col = bracket_pos.saturating_sub(vp.text.line_to_char(b_line));
                    if b_col >= vp.col_offset && (b_col - vp.col_offset) < code_area.width as usize {
                        #[allow(clippy::cast_possible_truncation)]
                        let bx = code_area.x + (b_col - vp.col_offset) as u16;
                        #[allow(clippy::cast_possible_truncation)]
                        let by = code_area.y + (b_line - vp.scroll_offset) as u16;
                        if let Some(cell) = frame.buffer_mut().cell_mut((bx, by)) {
                            cell.set_bg(theme.selection_bg);
                        }
                    }
                }
            }
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

        EditorRenderResult { cursor, code_area: Some(code_area) }
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
