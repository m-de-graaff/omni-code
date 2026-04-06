//! Transaction-building functions for text editing operations.
//!
//! Each function creates a [`Transaction`] that can be applied to a document
//! via `Document::apply()`. Multi-cursor operations process all cursors.

use omni_core::{ChangeSet, Range, Selection, Transaction};
use omni_loader::EditorConfig;
use omni_view::Document;
use omni_view::view_tree::NodeKey;

// ── Character insertion ─────────���───────────────────────────────────

/// Insert a character at each cursor position.
///
/// If the character is an auto-closing bracket/quote, inserts the pair
/// and positions the cursor between them.
#[must_use]
pub fn insert_char(doc: &Document, view_id: NodeKey, ch: char) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();

    // Check for auto-closing pair
    let closing = match ch {
        '(' => Some(')'),
        '{' => Some('}'),
        '[' => Some(']'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    };

    let primary = sel.primary();

    if let Some(close) = closing {
        if primary.is_empty() {
            // Insert pair, cursor between
            let pos = primary.head;
            let pair = format!("{ch}{close}");
            let cs = ChangeSet::insert_at(len, pos, &pair);
            let new_sel = Selection::point(pos + 1);
            return Transaction::new(cs, new_sel);
        }
    }

    // Simple character insertion (replaces selection if any)
    if primary.is_empty() {
        let cs = ChangeSet::insert_at(len, primary.head, &ch.to_string());
        let new_sel = Selection::point(primary.head + 1);
        Transaction::new(cs, new_sel)
    } else {
        let cs = ChangeSet::replace_at(len, primary.start(), primary.len(), &ch.to_string());
        let new_sel = Selection::point(primary.start() + 1);
        Transaction::new(cs, new_sel)
    }
}

/// Insert arbitrary text at each cursor position (for paste).
#[must_use]
pub fn insert_text(doc: &Document, view_id: NodeKey, new_text: &str) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();
    let insert_len = new_text.chars().count();

    if primary.is_empty() {
        let cs = ChangeSet::insert_at(len, primary.head, new_text);
        let new_sel = Selection::point(primary.head + insert_len);
        Transaction::new(cs, new_sel)
    } else {
        let cs = ChangeSet::replace_at(len, primary.start(), primary.len(), new_text);
        let new_sel = Selection::point(primary.start() + insert_len);
        Transaction::new(cs, new_sel)
    }
}

// ── Deletion ────��───────────────────────────────────────────────────

/// Delete the character before the cursor (Backspace).
#[must_use]
pub fn delete_backward(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    if !primary.is_empty() {
        // Delete selected text
        let cs = ChangeSet::delete_at(len, primary.start(), primary.len());
        let new_sel = Selection::point(primary.start());
        return Some(Transaction::new(cs, new_sel));
    }

    if primary.head == 0 {
        return None; // at start of document
    }

    let cs = ChangeSet::delete_at(len, primary.head - 1, 1);
    let new_sel = Selection::point(primary.head - 1);
    Some(Transaction::new(cs, new_sel))
}

/// Delete the character after the cursor (Delete key).
#[must_use]
pub fn delete_forward(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    if !primary.is_empty() {
        let cs = ChangeSet::delete_at(len, primary.start(), primary.len());
        let new_sel = Selection::point(primary.start());
        return Some(Transaction::new(cs, new_sel));
    }

    if primary.head >= len {
        return None; // at end of document
    }

    let cs = ChangeSet::delete_at(len, primary.head, 1);
    Some(Transaction::from_changes(cs))
}

/// Delete the word before the cursor (Ctrl+Backspace).
#[must_use]
pub fn delete_word_backward(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    if primary.head == 0 {
        return None;
    }

    let word_start = text.word_boundary_backward(primary.head);
    let delete_count = primary.head - word_start;
    if delete_count == 0 {
        return None;
    }

    let cs = ChangeSet::delete_at(len, word_start, delete_count);
    let new_sel = Selection::point(word_start);
    Some(Transaction::new(cs, new_sel))
}

/// Delete the word after the cursor (Ctrl+Delete).
#[must_use]
pub fn delete_word_forward(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    if primary.head >= len {
        return None;
    }

    let word_end = text.word_boundary_forward(primary.head);
    let delete_count = word_end - primary.head;
    if delete_count == 0 {
        return None;
    }

    let cs = ChangeSet::delete_at(len, primary.head, delete_count);
    Some(Transaction::from_changes(cs))
}

// ── Newline and indentation ───────────��─────────────────────────────

/// Insert a newline with auto-indent (copies leading whitespace from current line).
#[must_use]
pub fn insert_newline(doc: &Document, view_id: NodeKey) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    // Get current line's leading whitespace
    let line = text.char_to_line(primary.head);
    let line_start = text.line_to_char(line);
    let line_len = text.line_len_no_newline(line);
    let mut indent = String::new();
    for i in 0..line_len {
        let ch = text.char_at(line_start + i);
        if ch == ' ' || ch == '\t' {
            indent.push(ch);
        } else {
            break;
        }
    }

    let insert = format!("\n{indent}");
    let insert_len = insert.chars().count();

    if primary.is_empty() {
        let cs = ChangeSet::insert_at(len, primary.head, &insert);
        let new_sel = Selection::point(primary.head + insert_len);
        Transaction::new(cs, new_sel)
    } else {
        let cs = ChangeSet::replace_at(len, primary.start(), primary.len(), &insert);
        let new_sel = Selection::point(primary.start() + insert_len);
        Transaction::new(cs, new_sel)
    }
}

/// Insert a tab (spaces or tab character based on config).
#[must_use]
pub fn insert_tab(doc: &Document, view_id: NodeKey, config: &EditorConfig) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let primary = sel.primary();

    // If there's a selection, indent instead
    if !primary.is_empty() {
        return indent_lines(doc, view_id, config);
    }

    let tab_str = if config.use_spaces {
        " ".repeat(config.tab_width)
    } else {
        "\t".to_string()
    };

    let len = text.len_chars();
    let tab_len = tab_str.chars().count();
    let cs = ChangeSet::insert_at(len, primary.head, &tab_str);
    let new_sel = Selection::point(primary.head + tab_len);
    Transaction::new(cs, new_sel)
}

/// Indent all lines covered by the selection.
#[must_use]
pub fn indent_lines(doc: &Document, view_id: NodeKey, config: &EditorConfig) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let primary = sel.primary();

    let indent = if config.use_spaces {
        " ".repeat(config.tab_width)
    } else {
        "\t".to_string()
    };
    let indent_len = indent.chars().count();

    let start_line = text.char_to_line(primary.start());
    let end_line = text.char_to_line(primary.end().saturating_sub(1).max(primary.start()));

    let len = text.len_chars();

    // Build the full replacement text
    let mut new_text = String::new();
    for line in 0..text.len_lines() {
        let line_slice = text.line(line);
        let line_str: String = line_slice.chars().collect();
        if line >= start_line && line <= end_line {
            new_text.push_str(&indent);
        }
        new_text.push_str(&line_str);
    }

    // Build as a full replace (simple but correct for multi-line)
    let cs = ChangeSet::replace_at(len, 0, len, &new_text);
    let new_sel = Selection::single(Range::new(
        primary.start() + indent_len,
        primary.end() + (end_line - start_line + 1) * indent_len,
    ));
    Transaction::new(cs, new_sel)
}

/// Outdent all lines covered by the selection.
#[must_use]
pub fn outdent_lines(doc: &Document, view_id: NodeKey, config: &EditorConfig) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    let start_line = text.char_to_line(primary.start());
    let end_line = text.char_to_line(primary.end().saturating_sub(1).max(primary.start()));

    let mut new_text = String::new();
    let mut removed = 0;
    for line in 0..text.len_lines() {
        let line_slice = text.line(line);
        let line_str: String = line_slice.chars().collect();
        if line >= start_line && line <= end_line {
            // Remove up to tab_width leading spaces or one leading tab
            let stripped = if config.use_spaces {
                let spaces = line_str.chars().take_while(|&c| c == ' ').count();
                let remove = spaces.min(config.tab_width);
                removed += remove;
                &line_str[remove..]
            } else if let Some(rest) = line_str.strip_prefix('\t') {
                removed += 1;
                rest
            } else {
                &line_str
            };
            new_text.push_str(stripped);
        } else {
            new_text.push_str(&line_str);
        }
    }

    let cs = ChangeSet::replace_at(len, 0, len, &new_text);
    let new_sel = Selection::point(primary.head.saturating_sub(removed));
    Transaction::new(cs, new_sel)
}

// ── Line operations ─────────────────────────────────────────────────

/// Duplicate the current line.
#[must_use]
pub fn duplicate_line(doc: &Document, view_id: NodeKey) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    let line_range = text.select_line(primary.head);
    let line_text: String = text.slice(line_range.start()..line_range.end()).chars().collect();

    // Insert a copy of the line after it
    let insert_pos = line_range.end();
    let insert_text = if line_text.ends_with('\n') {
        line_text
    } else {
        format!("\n{line_text}")
    };
    let insert_len = insert_text.chars().count();

    let cs = ChangeSet::insert_at(len, insert_pos, &insert_text);
    let new_sel = Selection::point(primary.head + insert_len);
    Transaction::new(cs, new_sel)
}

/// Move the current line up by one.
#[must_use]
pub fn move_line_up(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let primary = sel.primary();

    let line = text.char_to_line(primary.head);
    if line == 0 {
        return None; // already at first line
    }

    let cur_range = text.select_line(primary.head);
    let prev_range = text.select_line(text.line_to_char(line - 1));

    let cur_text: String = text.slice(cur_range.start()..cur_range.end()).chars().collect();
    let prev_text: String = text.slice(prev_range.start()..prev_range.end()).chars().collect();

    // Swap: replace prev+cur with cur+prev
    let combined_start = prev_range.start();
    let combined_end = cur_range.end();
    let combined_len = combined_end - combined_start;
    let new_text = format!("{cur_text}{prev_text}");

    let len = text.len_chars();
    let cs = ChangeSet::replace_at(len, combined_start, combined_len, &new_text);

    // Move cursor up by the length of the previous line
    let col = primary.head - cur_range.start();
    let new_pos = prev_range.start() + col;
    let new_sel = Selection::point(new_pos);
    Some(Transaction::new(cs, new_sel))
}

/// Move the current line down by one.
#[must_use]
pub fn move_line_down(doc: &Document, view_id: NodeKey) -> Option<Transaction> {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let primary = sel.primary();

    let line = text.char_to_line(primary.head);
    if line + 1 >= text.len_lines() {
        return None; // already at last line
    }

    let cur_range = text.select_line(primary.head);
    let next_start = text.line_to_char(line + 1);
    let next_range = text.select_line(next_start);

    let cur_text: String = text.slice(cur_range.start()..cur_range.end()).chars().collect();
    let next_text: String = text.slice(next_range.start()..next_range.end()).chars().collect();

    let combined_start = cur_range.start();
    let combined_end = next_range.end();
    let combined_len = combined_end - combined_start;
    let new_text = format!("{next_text}{cur_text}");

    let len = text.len_chars();
    let cs = ChangeSet::replace_at(len, combined_start, combined_len, &new_text);

    let col = primary.head - cur_range.start();
    let new_pos = combined_start + next_text.chars().count() + col;
    let new_sel = Selection::point(new_pos);
    Some(Transaction::new(cs, new_sel))
}

// ── Comment toggle ──────────────────────────────────────────────────

/// Toggle line comment on all lines in the selection.
#[must_use]
pub fn toggle_comment(
    doc: &Document,
    view_id: NodeKey,
    comment_token: &str,
) -> Transaction {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    let start_line = text.char_to_line(primary.start());
    let end_line = text.char_to_line(primary.end().saturating_sub(1).max(primary.start()));

    // Check if all lines are already commented
    let all_commented = (start_line..=end_line).all(|line| {
        let start = text.line_to_char(line);
        let line_len = text.line_len_no_newline(line);
        let line_str: String = (0..line_len).map(|i| text.char_at(start + i)).collect();
        line_str.trim_start().starts_with(comment_token)
    });

    let mut new_text = String::new();
    for line in 0..text.len_lines() {
        let line_slice = text.line(line);
        let line_str: String = line_slice.chars().collect();
        if line >= start_line && line <= end_line {
            if all_commented {
                // Remove comment
                if let Some(rest) = line_str.trim_start().strip_prefix(comment_token) {
                    let indent: String = line_str.chars().take_while(|c| c.is_whitespace()).collect();
                    let rest = rest.strip_prefix(' ').unwrap_or(rest);
                    new_text.push_str(&indent);
                    new_text.push_str(rest);
                    // Preserve newline
                    if line_str.ends_with('\n') && !rest.ends_with('\n') {
                        new_text.push('\n');
                    }
                } else {
                    new_text.push_str(&line_str);
                }
            } else {
                // Add comment
                let indent: String = line_str.chars().take_while(|c| c.is_whitespace()).collect();
                let content = &line_str[indent.len()..];
                new_text.push_str(&indent);
                new_text.push_str(comment_token);
                new_text.push(' ');
                new_text.push_str(content);
            }
        } else {
            new_text.push_str(&line_str);
        }
    }

    let cs = ChangeSet::replace_at(len, 0, len, &new_text);
    Transaction::from_changes(cs)
}

// ── Clipboard operations ────��───────────────────────────────────────

/// Cut the selected text, returning the transaction and the cut text.
#[must_use]
pub fn cut_selection(doc: &Document, view_id: NodeKey) -> (Transaction, String) {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let len = text.len_chars();
    let primary = sel.primary();

    if primary.is_empty() {
        // Cut entire line when no selection
        let line_range = text.select_line(primary.head);
        let cut_text: String = text.slice(line_range.start()..line_range.end()).chars().collect();
        let cs = ChangeSet::delete_at(len, line_range.start(), line_range.len());
        let new_sel = Selection::point(line_range.start());
        (Transaction::new(cs, new_sel), cut_text)
    } else {
        let cut_text: String = text.slice(primary.start()..primary.end()).chars().collect();
        let cs = ChangeSet::delete_at(len, primary.start(), primary.len());
        let new_sel = Selection::point(primary.start());
        (Transaction::new(cs, new_sel), cut_text)
    }
}

/// Copy the selected text (no transaction, just returns the text).
#[must_use]
pub fn copy_selection(doc: &Document, view_id: NodeKey) -> String {
    let text = doc.text();
    let sel = doc.selection(view_id);
    let primary = sel.primary();

    if primary.is_empty() {
        // Copy entire line when no selection
        let line_range = text.select_line(primary.head);
        text.slice(line_range.start()..line_range.end()).chars().collect()
    } else {
        text.slice(primary.start()..primary.end()).chars().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(content: &str) -> (Document, NodeKey) {
        let doc = Document::from_str(content, None);
        // Use a dummy NodeKey — we'll use slotmap to get a real one
        // For testing, we set the selection via the document
        let key = omni_view::ViewTree::new().set_root(omni_view::View::new(doc.id, 80, 24));
        (doc, key)
    }

    #[test]
    fn insert_char_basic() {
        let (doc, key) = make_doc("hello");
        // Set cursor at position 5
        let mut doc = doc;
        doc.set_selection(key, Selection::point(5));

        let txn = insert_char(&doc, key, '!');
        doc.apply(&txn, key);
        assert_eq!(doc.text().to_string(), "hello!");
        assert_eq!(doc.selection(key).primary().head, 6);
    }

    #[test]
    fn insert_char_auto_close_bracket() {
        let (doc, key) = make_doc("fn main");
        let mut doc = doc;
        doc.set_selection(key, Selection::point(7));

        let txn = insert_char(&doc, key, '(');
        doc.apply(&txn, key);
        assert_eq!(doc.text().to_string(), "fn main()");
        assert_eq!(doc.selection(key).primary().head, 8); // between parens
    }

    #[test]
    fn delete_backward_basic() {
        let (doc, key) = make_doc("hello");
        let mut doc = doc;
        doc.set_selection(key, Selection::point(5));

        let txn = delete_backward(&doc, key).unwrap();
        doc.apply(&txn, key);
        assert_eq!(doc.text().to_string(), "hell");
    }

    #[test]
    fn delete_backward_at_start_returns_none() {
        let (doc, key) = make_doc("hello");
        let mut doc = doc;
        doc.set_selection(key, Selection::point(0));

        assert!(delete_backward(&doc, key).is_none());
    }

    #[test]
    fn insert_newline_with_indent() {
        let (doc, key) = make_doc("    hello");
        let mut doc = doc;
        doc.set_selection(key, Selection::point(9)); // after "hello"

        let txn = insert_newline(&doc, key);
        doc.apply(&txn, key);
        assert_eq!(doc.text().to_string(), "    hello\n    ");
    }

    #[test]
    fn duplicate_line_basic() {
        let (doc, key) = make_doc("hello\nworld\n");
        let mut doc = doc;
        doc.set_selection(key, Selection::point(1)); // on first line

        let txn = duplicate_line(&doc, key);
        doc.apply(&txn, key);
        assert_eq!(doc.text().to_string(), "hello\nhello\nworld\n");
    }
}
