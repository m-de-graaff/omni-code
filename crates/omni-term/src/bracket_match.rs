//! Stack-based bracket matching.

use omni_core::Text;

/// Find the matching bracket for the character at `pos`.
///
/// Returns `None` if the character is not a bracket or no match is found.
#[must_use]
pub fn find_matching_bracket(text: &Text, pos: usize) -> Option<usize> {
    let len = text.len_chars();
    if pos >= len {
        return None;
    }

    let ch = text.char_at(pos);
    match ch {
        '(' => scan_forward(text, pos, '(', ')'),
        '{' => scan_forward(text, pos, '{', '}'),
        '[' => scan_forward(text, pos, '[', ']'),
        ')' => scan_backward(text, pos, ')', '('),
        '}' => scan_backward(text, pos, '}', '{'),
        ']' => scan_backward(text, pos, ']', '['),
        _ => None,
    }
}

fn scan_forward(text: &Text, start: usize, open: char, close: char) -> Option<usize> {
    let len = text.len_chars();
    let mut depth: usize = 0;
    for i in start..len {
        let ch = text.char_at(i);
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn scan_backward(text: &Text, start: usize, close: char, open: char) -> Option<usize> {
    let mut depth: usize = 0;
    for i in (0..=start).rev() {
        let ch = text.char_at(i);
        if ch == close {
            depth += 1;
        } else if ch == open {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Text {
        Text::from(s)
    }

    #[test]
    fn match_simple_parens() {
        let t = text("(hello)");
        assert_eq!(find_matching_bracket(&t, 0), Some(6));
        assert_eq!(find_matching_bracket(&t, 6), Some(0));
    }

    #[test]
    fn match_nested_braces() {
        let t = text("{a{b}c}");
        assert_eq!(find_matching_bracket(&t, 0), Some(6));
        assert_eq!(find_matching_bracket(&t, 2), Some(4));
    }

    #[test]
    fn no_match_unbalanced() {
        let t = text("(hello");
        assert_eq!(find_matching_bracket(&t, 0), None);
    }

    #[test]
    fn match_square_brackets() {
        let t = text("[a[b]]");
        assert_eq!(find_matching_bracket(&t, 0), Some(5));
        assert_eq!(find_matching_bracket(&t, 2), Some(4));
    }

    #[test]
    fn non_bracket_returns_none() {
        let t = text("hello");
        assert_eq!(find_matching_bracket(&t, 0), None);
    }
}
