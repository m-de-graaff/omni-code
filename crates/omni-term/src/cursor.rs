//! Cursor movement and selection extension functions.
//!
//! All functions are pure: they take a `(&Text, &Selection)` and return a
//! new `Selection`. Multi-cursor is handled via `Selection::map_ranges`.

use omni_core::{Range, Selection, Text};

// ── Movement (collapse selection, move cursor) ──────────────────────

/// Move all cursors left by one character. Collapses active selections first.
#[must_use]
pub fn move_left(_text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        if r.is_empty() {
            Range::point(r.head.saturating_sub(1))
        } else {
            Range::point(r.start())
        }
    })
}

/// Move all cursors right by one character. Collapses active selections first.
#[must_use]
pub fn move_right(text: &Text, sel: &Selection) -> Selection {
    let len = text.len_chars();
    sel.map_ranges(|r| {
        if r.is_empty() {
            Range::point((r.head + 1).min(len))
        } else {
            Range::point(r.end())
        }
    })
}

/// Move all cursors up by one line.
#[must_use]
pub fn move_up(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let pos = r.head;
        let line = text.char_to_line(pos);
        if line == 0 {
            return Range::point(0);
        }
        let col = pos - text.line_to_char(line);
        let prev_line_start = text.line_to_char(line - 1);
        let prev_line_len = text.line_len_no_newline(line - 1);
        Range::point(prev_line_start + col.min(prev_line_len))
    })
}

/// Move all cursors down by one line.
#[must_use]
pub fn move_down(text: &Text, sel: &Selection) -> Selection {
    let total_lines = text.len_lines();
    sel.map_ranges(|r| {
        let pos = r.head;
        let line = text.char_to_line(pos);
        if line + 1 >= total_lines {
            return Range::point(text.len_chars());
        }
        let col = pos - text.line_to_char(line);
        let next_line_start = text.line_to_char(line + 1);
        let next_line_len = text.line_len_no_newline(line + 1);
        Range::point(next_line_start + col.min(next_line_len))
    })
}

/// Move cursor to the previous word boundary.
#[must_use]
pub fn move_word_left(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| Range::point(text.word_boundary_backward(r.head)))
}

/// Move cursor to the next word boundary.
#[must_use]
pub fn move_word_right(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| Range::point(text.word_boundary_forward(r.head)))
}

/// Move cursor to the start of the current line.
#[must_use]
pub fn move_line_start(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        Range::point(text.line_to_char(line))
    })
}

/// Move cursor to the end of the current line.
#[must_use]
pub fn move_line_end(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        let line_start = text.line_to_char(line);
        let line_len = text.line_len_no_newline(line);
        Range::point(line_start + line_len)
    })
}

/// Move cursor to the start of the document.
#[must_use]
pub fn move_doc_start(_sel: &Selection) -> Selection {
    Selection::point(0)
}

/// Move cursor to the end of the document.
#[must_use]
pub fn move_doc_end(text: &Text, _sel: &Selection) -> Selection {
    Selection::point(text.len_chars())
}

// ── Selection extension (keep anchor, move head) ────────────────────

/// Extend selection left by one character.
#[must_use]
pub fn select_left(_text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| r.extend_to(r.head.saturating_sub(1)))
}

/// Extend selection right by one character.
#[must_use]
pub fn select_right(text: &Text, sel: &Selection) -> Selection {
    let len = text.len_chars();
    sel.map_ranges(|r| r.extend_to((r.head + 1).min(len)))
}

/// Extend selection up by one line.
#[must_use]
pub fn select_up(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        if line == 0 {
            return r.extend_to(0);
        }
        let col = r.head - text.line_to_char(line);
        let prev_start = text.line_to_char(line - 1);
        let prev_len = text.line_len_no_newline(line - 1);
        r.extend_to(prev_start + col.min(prev_len))
    })
}

/// Extend selection down by one line.
#[must_use]
pub fn select_down(text: &Text, sel: &Selection) -> Selection {
    let total_lines = text.len_lines();
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        if line + 1 >= total_lines {
            return r.extend_to(text.len_chars());
        }
        let col = r.head - text.line_to_char(line);
        let next_start = text.line_to_char(line + 1);
        let next_len = text.line_len_no_newline(line + 1);
        r.extend_to(next_start + col.min(next_len))
    })
}

/// Extend selection left by one word.
#[must_use]
pub fn select_word_left(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| r.extend_to(text.word_boundary_backward(r.head)))
}

/// Extend selection right by one word.
#[must_use]
pub fn select_word_right(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| r.extend_to(text.word_boundary_forward(r.head)))
}

/// Extend selection to line start.
#[must_use]
pub fn select_line_start(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        r.extend_to(text.line_to_char(line))
    })
}

/// Extend selection to line end.
#[must_use]
pub fn select_line_end(text: &Text, sel: &Selection) -> Selection {
    sel.map_ranges(|r| {
        let line = text.char_to_line(r.head);
        let line_start = text.line_to_char(line);
        r.extend_to(line_start + text.line_len_no_newline(line))
    })
}

// ── Special selections ──────────────────────────────────────────────

/// Select the word at the primary cursor position.
#[must_use]
pub fn select_word(text: &Text, sel: &Selection) -> Selection {
    let word_range = text.word_at(sel.primary().head);
    if word_range.is_empty() {
        return sel.clone();
    }
    Selection::single(word_range)
}

/// Select the entire line at the primary cursor.
#[must_use]
pub fn select_line(text: &Text, sel: &Selection) -> Selection {
    let line_range = text.select_line(sel.primary().head);
    Selection::single(line_range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_left_at_start() {
        let text = Text::from("hello");
        let sel = Selection::point(0);
        let result = move_left(&text, &sel);
        assert_eq!(result.primary().head, 0);
    }

    #[test]
    fn move_left_collapses_selection() {
        let text = Text::from("hello");
        let sel = Selection::single(Range::new(1, 4));
        let result = move_left(&text, &sel);
        assert_eq!(result.primary().head, 1); // collapses to start
        assert!(result.primary().is_empty());
    }

    #[test]
    fn move_right_at_end() {
        let text = Text::from("hello");
        let sel = Selection::point(5);
        let result = move_right(&text, &sel);
        assert_eq!(result.primary().head, 5);
    }

    #[test]
    fn move_up_first_line() {
        let text = Text::from("hello\nworld");
        let sel = Selection::point(3);
        let result = move_up(&text, &sel);
        assert_eq!(result.primary().head, 0); // goes to start of doc
    }

    #[test]
    fn move_down_preserves_column() {
        let text = Text::from("hello\nworld\nfoo");
        let sel = Selection::point(3); // "hel|lo"
        let result = move_down(&text, &sel);
        // Should be at position 9 = "wor|ld" (line 1, col 3)
        assert_eq!(result.primary().head, 9);
    }

    #[test]
    fn move_down_clamps_to_shorter_line() {
        let text = Text::from("hello\nhi");
        let sel = Selection::point(4); // "hell|o"
        let result = move_down(&text, &sel);
        // "hi" is only 2 chars, so col 4 → col 2
        assert_eq!(result.primary().head, 8); // "hi" end
    }

    #[test]
    fn select_left_extends() {
        let text = Text::from("hello");
        let sel = Selection::point(3);
        let result = select_left(&text, &sel);
        assert_eq!(result.primary().anchor, 3);
        assert_eq!(result.primary().head, 2);
    }

    #[test]
    fn select_word_selects_word() {
        let text = Text::from("hello world");
        let sel = Selection::point(7); // on 'o' in "world"
        let result = select_word(&text, &sel);
        assert_eq!(result.primary().start(), 6);
        assert_eq!(result.primary().end(), 11);
    }

    #[test]
    fn move_line_start_end() {
        let text = Text::from("hello\nworld");
        let sel = Selection::point(8); // "wo|rld"
        let start = move_line_start(&text, &sel);
        assert_eq!(start.primary().head, 6);
        let end = move_line_end(&text, &sel);
        assert_eq!(end.primary().head, 11);
    }
}
