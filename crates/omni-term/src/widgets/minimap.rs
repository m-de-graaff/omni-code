//! Minimap widget using Unicode Braille characters for code overview.
//!
//! Each Braille character (U+2800–U+28FF) is a 2×4 dot grid, giving ~4:1
//! vertical compression. Non-whitespace characters map to filled dots,
//! and the dominant syntax highlight determines the color.

use omni_loader::ThemeColors;
use omni_syntax::HighlightScope;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::editor_pane::EditorViewport;

/// Braille base codepoint.
const BRAILLE_BASE: u32 = 0x2800;

/// Dots per row of source lines mapped to one Braille character row.
const LINES_PER_ROW: usize = 4;

/// Columns per Braille character (each char spans 2 pixel columns).
const COLS_PER_CHAR: usize = 2;

/// Braille dot bit layout:
/// ```text
/// (0,0)=0x01  (1,0)=0x08
/// (0,1)=0x02  (1,1)=0x10
/// (0,2)=0x04  (1,2)=0x20
/// (0,3)=0x40  (1,3)=0x80
/// ```
const DOT_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // left column: rows 0-3
    [0x08, 0x10, 0x20, 0x80], // right column: rows 0-3
];

/// Renders a Braille minimap of the document.
pub struct Minimap;

impl Minimap {
    /// Render the minimap into the given area.
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        viewport: &EditorViewport<'_>,
        theme: &ThemeColors,
    ) {
        let block = Block::bordered()
            .border_style(Style::new().fg(theme.border))
            .style(Style::new().bg(theme.background));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 || viewport.total_lines == 0 {
            return;
        }

        let text = viewport.text;
        let minimap_cols = inner.width as usize; // each col = 2 source columns
        let minimap_rows = inner.height as usize;
        let source_pixel_cols = minimap_cols * COLS_PER_CHAR;

        // Calculate viewport highlight range (in minimap rows)
        let visible_height = inner.height as usize; // editor visible lines (approximate)
        let vp_start_row = source_line_to_minimap_row(viewport.scroll_offset, viewport.total_lines, minimap_rows);
        let vp_end_row = source_line_to_minimap_row(
            (viewport.scroll_offset + visible_height).min(viewport.total_lines),
            viewport.total_lines,
            minimap_rows,
        );

        // Render each minimap row
        for row in 0..minimap_rows {
            let first_source_line = minimap_row_to_source_line(row, viewport.total_lines, minimap_rows);
            let is_viewport = row >= vp_start_row && row < vp_end_row;

            let mut spans: Vec<Span<'static>> = Vec::with_capacity(minimap_cols);

            for col in 0..minimap_cols {
                let braille = compose_braille_char(
                    text,
                    first_source_line,
                    col * COLS_PER_CHAR,
                    source_pixel_cols,
                    viewport.total_lines,
                );

                // Determine color: use a dimmed foreground for the minimap
                let fg = dominant_color_for_region(
                    text,
                    viewport.highlight_spans,
                    first_source_line,
                    col * COLS_PER_CHAR,
                    viewport.total_lines,
                    &theme.syntax,
                    theme.text_muted,
                );

                let style = if is_viewport {
                    Style::new().fg(fg).bg(theme.selection_bg)
                } else {
                    Style::new().fg(fg)
                };

                spans.push(Span::styled(String::from(braille), style));
            }

            let line = Line::from(spans);
            #[allow(clippy::cast_possible_truncation)]
            let y_offset = row as u16;
            frame.render_widget(
                Paragraph::new(line),
                Rect::new(inner.x, inner.y + y_offset, inner.width, 1),
            );
        }
    }

    /// Calculate which source line a minimap click corresponds to.
    #[must_use]
    pub const fn click_to_source_line(
        click_row: u16,
        minimap_area: Rect,
        total_lines: usize,
    ) -> usize {
        let inner_row = click_row.saturating_sub(minimap_area.y + 1); // +1 for border
        let minimap_rows = minimap_area.height.saturating_sub(2) as usize; // -2 for borders
        if minimap_rows == 0 {
            return 0;
        }
        minimap_row_to_source_line(inner_row as usize, total_lines, minimap_rows)
    }
}

/// Compose a single Braille character from 4 source lines × 2 columns.
fn compose_braille_char(
    text: &omni_core::Text,
    first_line: usize,
    first_col: usize,
    _max_cols: usize,
    total_lines: usize,
) -> char {
    let mut bits: u8 = 0;

    for (sub_row, dot_row) in DOT_BITS[0].iter().enumerate() {
        let line_idx = first_line + sub_row;
        if line_idx >= total_lines {
            break;
        }
        let line_start = text.line_to_char(line_idx);
        let line_len = text.line_len_no_newline(line_idx);

        // Left column
        let char_col_left = first_col;
        if char_col_left < line_len {
            let ch = text.char_at(line_start + char_col_left);
            if !ch.is_whitespace() {
                bits |= dot_row;
            }
        }

        // Right column
        let char_col_right = first_col + 1;
        if char_col_right < line_len {
            let ch = text.char_at(line_start + char_col_right);
            if !ch.is_whitespace() {
                bits |= DOT_BITS[1][sub_row];
            }
        }
    }

    char::from_u32(BRAILLE_BASE + u32::from(bits)).unwrap_or(' ')
}

/// Get a simplified color for a minimap region based on the dominant highlight.
fn dominant_color_for_region(
    text: &omni_core::Text,
    highlight_spans: &[omni_syntax::HighlightSpan],
    first_line: usize,
    _first_col: usize,
    total_lines: usize,
    syntax: &omni_loader::SyntaxColors,
    default_color: ratatui::style::Color,
) -> ratatui::style::Color {
    if highlight_spans.is_empty() || first_line >= total_lines {
        return default_color;
    }

    // Sample the first source line in this minimap cell to find its dominant scope
    let line_start_byte = text.char_to_byte(text.line_to_char(first_line));
    let line_end_byte = if first_line + 1 < total_lines {
        text.char_to_byte(text.line_to_char(first_line + 1))
    } else {
        text.len_bytes()
    };

    // Find the first highlight span overlapping this line
    let span_idx = highlight_spans.partition_point(|s| s.end_byte <= line_start_byte);
    if span_idx < highlight_spans.len() && highlight_spans[span_idx].start_byte < line_end_byte {
        let scope = highlight_spans[span_idx].scope;
        // Simplify to broad categories for minimap
        return match scope {
            HighlightScope::Comment | HighlightScope::CommentDoc => syntax.comment,
            HighlightScope::String | HighlightScope::StringSpecial => syntax.string,
            HighlightScope::Keyword | HighlightScope::KeywordFunction
            | HighlightScope::KeywordReturn | HighlightScope::KeywordOperator
            | HighlightScope::KeywordControl => syntax.keyword,
            HighlightScope::Function | HighlightScope::FunctionMethod
            | HighlightScope::FunctionMacro | HighlightScope::FunctionBuiltin => syntax.function,
            HighlightScope::Type | HighlightScope::TypeBuiltin => syntax.r#type,
            _ => default_color,
        };
    }

    default_color
}

/// Map a source line index to a minimap row index.
const fn source_line_to_minimap_row(line: usize, total_lines: usize, minimap_rows: usize) -> usize {
    if total_lines == 0 || minimap_rows == 0 {
        return 0;
    }
    (line * minimap_rows) / total_lines
}

/// Map a minimap row index to the first source line it represents.
const fn minimap_row_to_source_line(row: usize, total_lines: usize, minimap_rows: usize) -> usize {
    if minimap_rows == 0 {
        return 0;
    }
    (row * total_lines) / minimap_rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_empty_char() {
        let ch = char::from_u32(BRAILLE_BASE).unwrap();
        assert_eq!(ch, '\u{2800}'); // empty braille
    }

    #[test]
    fn braille_full_char() {
        let full: u8 = 0x01 | 0x02 | 0x04 | 0x40 | 0x08 | 0x10 | 0x20 | 0x80;
        assert_eq!(full, 0xFF);
        let ch = char::from_u32(BRAILLE_BASE + u32::from(full)).unwrap();
        assert_eq!(ch, '\u{28FF}'); // all dots filled
    }

    #[test]
    fn compose_from_text() {
        let text = omni_core::Text::from("ab\ncd\n  \nef\n");
        let ch = compose_braille_char(&text, 0, 0, 4, 4);
        // Lines 0-3: "ab", "cd", "  ", "ef"
        // Col 0: a(dot), c(dot), space(no), e(dot) → bits 0x01 | 0x02 | 0x40 = 0x43
        // Col 1: b(dot), d(dot), space(no), f(dot) → bits 0x08 | 0x10 | 0x80 = 0x98
        // Total: 0x43 | 0x98 = 0xDB
        assert_eq!(ch, char::from_u32(BRAILLE_BASE + 0xDB).unwrap());
    }

    #[test]
    fn source_to_minimap_row_mapping() {
        // 100 source lines, 25 minimap rows → each row = 4 lines
        assert_eq!(source_line_to_minimap_row(0, 100, 25), 0);
        assert_eq!(source_line_to_minimap_row(50, 100, 25), 12);
        assert_eq!(source_line_to_minimap_row(99, 100, 25), 24);
    }

    #[test]
    fn minimap_to_source_line_mapping() {
        assert_eq!(minimap_row_to_source_line(0, 100, 25), 0);
        assert_eq!(minimap_row_to_source_line(12, 100, 25), 48);
        assert_eq!(minimap_row_to_source_line(24, 100, 25), 96);
    }
}
