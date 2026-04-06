//! Rope-backed text buffer.

use std::ops::Range;

use ropey::{Rope, RopeSlice};

/// A text buffer backed by a rope data structure for efficient editing.
#[derive(Debug, Clone)]
pub struct Text {
    rope: Rope,
}

impl Text {
    /// Create an empty text buffer.
    #[must_use]
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    /// Create a text buffer from a string slice.
    #[must_use]
    pub fn from(s: &str) -> Self {
        Self { rope: Rope::from_str(s) }
    }

    /// Return the underlying rope.
    #[must_use]
    pub const fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Return a mutable reference to the underlying rope.
    pub const fn rope_mut(&mut self) -> &mut Rope {
        &mut self.rope
    }

    // ── Size queries ────────────────────────────────────────────────

    /// Total number of characters.
    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Total number of lines.
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Total number of bytes.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    // ── Mutations ───────────────────────────────────────────────────

    /// Insert text at the given character index.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
    }

    /// Remove the character range `[start..end)`.
    pub fn remove(&mut self, start: usize, end: usize) {
        self.rope.remove(start..end);
    }

    // ── Line ↔ char index conversions ───────────────────────────────

    /// Return the char index of the start of the given line (0-based).
    #[must_use]
    pub fn line_to_char(&self, line: usize) -> usize {
        self.rope.line_to_char(line)
    }

    /// Return the line number containing the given char index.
    #[must_use]
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    // ── Byte ↔ char conversions (for tree-sitter) ───────────────────

    /// Convert a char index to a byte index.
    #[must_use]
    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        self.rope.char_to_byte(char_idx)
    }

    /// Convert a byte index to a char index.
    #[must_use]
    pub fn byte_to_char(&self, byte_idx: usize) -> usize {
        self.rope.byte_to_char(byte_idx)
    }

    // ── UTF-16 conversions (for LSP interop) ────────────────────────

    /// Convert a char offset to a UTF-16 code-unit column offset within its line.
    ///
    /// LSP uses UTF-16 code units for column positions. This method takes an
    /// absolute char index and returns the UTF-16 column offset from the start
    /// of that char's line.
    #[must_use]
    pub fn char_to_utf16_cu(&self, char_idx: usize) -> usize {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let line_slice = self.rope.line(line);
        let char_col = char_idx - line_start;

        line_slice
            .chars()
            .take(char_col)
            .map(char::len_utf16)
            .sum()
    }

    /// Convert a UTF-16 code-unit column on a given line to an absolute char index.
    ///
    /// This is the inverse of [`char_to_utf16_cu`](Self::char_to_utf16_cu) — it
    /// takes an LSP-style `(line, utf16_col)` position and returns the char index
    /// into the buffer.
    #[must_use]
    pub fn utf16_cu_to_char(&self, line: usize, utf16_col: usize) -> usize {
        let line_start = self.rope.line_to_char(line);
        let line_slice = self.rope.line(line);

        let mut remaining = utf16_col;
        let mut char_col = 0;
        for ch in line_slice.chars() {
            if remaining == 0 {
                break;
            }
            let cu = ch.len_utf16();
            if cu > remaining {
                break;
            }
            remaining -= cu;
            char_col += 1;
        }

        line_start + char_col
    }

    // ── Zero-copy slice extraction ──────────────────────────────────

    /// Return a zero-copy slice of the buffer by char range.
    #[must_use]
    pub fn slice(&self, char_range: Range<usize>) -> RopeSlice<'_> {
        self.rope.slice(char_range)
    }

    /// Return a zero-copy slice of a single line (0-based).
    #[must_use]
    pub fn line(&self, line_idx: usize) -> RopeSlice<'_> {
        self.rope.line(line_idx)
    }

    /// Return an iterator over zero-copy line slices in the range `[start_line..end_line)`.
    pub fn lines_range(
        &self,
        start_line: usize,
        end_line: usize,
    ) -> impl Iterator<Item = RopeSlice<'_>> {
        let start = self.rope.line_to_char(start_line);
        let end = if end_line >= self.rope.len_lines() {
            self.rope.len_chars()
        } else {
            self.rope.line_to_char(end_line)
        };
        self.rope.slice(start..end).lines()
    }

    // ── Word boundaries ─────────────────────────────────────────────

    /// Find the start of the word at or before `pos` (Unicode word boundaries).
    ///
    /// A "word" is a contiguous run of alphanumeric/underscore characters.
    #[must_use]
    pub fn word_start(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut p = pos.min(self.rope.len_chars());
        // If we're past the end or on a non-word char, step back first
        if p > 0 {
            let ch = self.char_at(p.saturating_sub(1));
            if !is_word_char(ch) && p == pos {
                // At the start of a non-word — step back to find previous word
                p = p.saturating_sub(1);
                while p > 0 && !is_word_char(self.char_at(p.saturating_sub(1))) {
                    p -= 1;
                }
            }
        }
        // Walk backward while still on word chars
        while p > 0 && is_word_char(self.char_at(p - 1)) {
            p -= 1;
        }
        p
    }

    /// Find the end of the word at or after `pos` (Unicode word boundaries).
    #[must_use]
    pub fn word_end(&self, pos: usize) -> usize {
        let len = self.rope.len_chars();
        let mut p = pos.min(len);
        while p < len && is_word_char(self.char_at(p)) {
            p += 1;
        }
        p
    }

    /// Find the word boundary forward from `pos` (for Ctrl+Right).
    /// Skips the current word, then skips whitespace, landing at the start
    /// of the next word.
    #[must_use]
    pub fn word_boundary_forward(&self, pos: usize) -> usize {
        let len = self.rope.len_chars();
        let mut p = pos.min(len);
        // Skip current word
        if p < len && is_word_char(self.char_at(p)) {
            while p < len && is_word_char(self.char_at(p)) {
                p += 1;
            }
        }
        // Skip non-word, non-newline chars (whitespace, punctuation)
        while p < len {
            let ch = self.char_at(p);
            if is_word_char(ch) || ch == '\n' {
                break;
            }
            p += 1;
        }
        p
    }

    /// Find the word boundary backward from `pos` (for Ctrl+Left).
    #[must_use]
    pub fn word_boundary_backward(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut p = pos.min(self.rope.len_chars());
        // Skip non-word chars before
        while p > 0 && !is_word_char(self.char_at(p - 1)) {
            p -= 1;
        }
        // Skip word chars
        while p > 0 && is_word_char(self.char_at(p - 1)) {
            p -= 1;
        }
        p
    }

    /// Return the Range covering the word at `pos`. If `pos` is not on a word
    /// character, returns a zero-width range at `pos`.
    #[must_use]
    pub fn word_at(&self, pos: usize) -> super::selection::Range {
        let p = pos.min(self.rope.len_chars().saturating_sub(1));
        if self.rope.len_chars() == 0 {
            return super::selection::Range::point(0);
        }
        let ch = self.char_at(p);
        if !is_word_char(ch) {
            return super::selection::Range::point(pos);
        }
        let start = self.word_start(p);
        let end = self.word_end(p);
        super::selection::Range::new(start, end)
    }

    // ── Line selection ──────────────────────────────────────────────

    /// Return a Range covering the full line that contains `pos`.
    /// Includes the trailing newline if present.
    #[must_use]
    pub fn select_line(&self, pos: usize) -> super::selection::Range {
        let line = self.rope.char_to_line(pos.min(self.rope.len_chars().saturating_sub(1)));
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        super::selection::Range::new(start, end)
    }

    /// The length of a given line (in chars, excluding trailing newline).
    #[must_use]
    pub fn line_len_no_newline(&self, line: usize) -> usize {
        let slice = self.rope.line(line);
        let len = slice.len_chars();
        // Strip trailing \n or \r\n
        if len > 0 {
            let last = slice.char(len - 1);
            if last == '\n' {
                if len > 1 && slice.char(len - 2) == '\r' {
                    return len - 2;
                }
                return len - 1;
            }
        }
        len
    }

    // ── Search ──────────────────────────────────────────────────────

    /// Find the next occurrence of `needle` starting after `from` (char index).
    /// Wraps around to the beginning if not found before end.
    /// Returns the char range `(start, end)` if found.
    #[must_use]
    pub fn find_next(&self, needle: &str, from: usize) -> Option<(usize, usize)> {
        if needle.is_empty() {
            return None;
        }
        let needle_len = needle.chars().count();
        let len = self.rope.len_chars();

        // Search from `from` to end
        if let Some(pos) = self.find_in_range(needle, from, len) {
            return Some((pos, pos + needle_len));
        }
        // Wrap around: search from 0 to `from`
        if from > 0 {
            if let Some(pos) = self.find_in_range(needle, 0, from) {
                return Some((pos, pos + needle_len));
            }
        }
        None
    }

    /// Find all non-overlapping occurrences of `needle` in the document.
    ///
    /// Returns a vector of `(start_char, end_char)` pairs, sorted by position.
    /// If `case_sensitive` is false, comparison is done case-insensitively.
    #[must_use]
    pub fn find_all(&self, needle: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
        if needle.is_empty() {
            return Vec::new();
        }

        let needle_lower: String;
        let effective_needle = if case_sensitive {
            needle
        } else {
            needle_lower = needle.to_lowercase();
            &needle_lower
        };
        let needle_len = effective_needle.chars().count();
        let len = self.rope.len_chars();

        let mut results = Vec::new();
        let mut pos = 0;

        while pos + needle_len <= len {
            let found = if case_sensitive {
                self.find_in_range(effective_needle, pos, len)
            } else {
                self.find_in_range_case_insensitive(effective_needle, pos, len)
            };
            match found {
                Some(start) => {
                    results.push((start, start + needle_len));
                    pos = start + needle_len; // skip past this match
                }
                None => break,
            }
        }

        results
    }

    /// Search with a regex pattern. Returns char-index range pairs.
    ///
    /// # Errors
    /// Returns `regex::Error` if the pattern is invalid.
    pub fn find_all_regex(
        &self,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<Vec<(usize, usize)>, regex::Error> {
        let re = regex::RegexBuilder::new(pattern)
            .case_insensitive(!case_sensitive)
            .build()?;

        let text_str = self.to_string();
        let mut results = Vec::new();

        for m in re.find_iter(&text_str) {
            let start_byte = m.start();
            let end_byte = m.end();
            let start_char = self.byte_to_char(start_byte);
            let end_char = self.byte_to_char(end_byte);
            if start_char != end_char {
                results.push((start_char, end_char));
            }
        }

        Ok(results)
    }

    /// Search for `needle` in the char range [start..end).
    fn find_in_range(&self, needle: &str, start: usize, end: usize) -> Option<usize> {
        if start >= end || needle.is_empty() {
            return None;
        }
        let slice = self.rope.slice(start..end);
        // Build needle chars for comparison
        let needle_chars: Vec<char> = needle.chars().collect();
        let needle_len = needle_chars.len();

        if needle_len > slice.len_chars() {
            return None;
        }

        // Simple linear search over the rope slice
        'outer: for i in 0..=(slice.len_chars() - needle_len) {
            for (j, &nc) in needle_chars.iter().enumerate() {
                if slice.char(i + j) != nc {
                    continue 'outer;
                }
            }
            return Some(start + i);
        }
        None
    }

    /// Case-insensitive search in char range [start..end).
    /// The `needle` must already be lowercased.
    fn find_in_range_case_insensitive(&self, needle: &str, start: usize, end: usize) -> Option<usize> {
        if start >= end || needle.is_empty() {
            return None;
        }
        let slice = self.rope.slice(start..end);
        let needle_chars: Vec<char> = needle.chars().collect();
        let needle_len = needle_chars.len();

        if needle_len > slice.len_chars() {
            return None;
        }

        'outer: for i in 0..=(slice.len_chars() - needle_len) {
            for (j, &nc) in needle_chars.iter().enumerate() {
                let original = slice.char(i + j);
                let sc = original.to_lowercase().next().unwrap_or(original);
                if sc != nc {
                    continue 'outer;
                }
            }
            return Some(start + i);
        }
        None
    }

    // ── Char access ─────────────────────────────────────────────────

    /// Get the character at the given char index.
    #[must_use]
    pub fn char_at(&self, char_idx: usize) -> char {
        self.rope.char(char_idx)
    }
}

/// Whether a character is a "word" character (alphanumeric or underscore).
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.rope.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_to_char_and_back() {
        let t = Text::from("hello\nworld\nfoo");
        assert_eq!(t.line_to_char(0), 0);
        assert_eq!(t.line_to_char(1), 6);
        assert_eq!(t.line_to_char(2), 12);
        assert_eq!(t.char_to_line(0), 0);
        assert_eq!(t.char_to_line(6), 1);
        assert_eq!(t.char_to_line(12), 2);
    }

    #[test]
    fn byte_char_roundtrip() {
        // "café" — 'é' is 2 bytes in UTF-8
        let t = Text::from("café\nbar");
        let char_idx = 3; // 'é'
        let byte_idx = t.char_to_byte(char_idx);
        assert_eq!(t.byte_to_char(byte_idx), char_idx);
    }

    #[test]
    fn utf16_ascii_line() {
        let t = Text::from("hello\nworld");
        // ASCII chars are 1 UTF-16 code unit each
        assert_eq!(t.char_to_utf16_cu(7), 1); // 'o' in "world", col 1
        assert_eq!(t.utf16_cu_to_char(1, 1), 7); // line 1, utf16_col 1 → char 7
    }

    #[test]
    fn utf16_multibyte() {
        // '😀' is 2 UTF-16 code units (surrogate pair), 1 char
        let t = Text::from("a😀b\n");
        // line 0: a(1cu) 😀(2cu) b(1cu) = 4 CU total
        assert_eq!(t.char_to_utf16_cu(1), 1); // after 'a' → 1 CU
        assert_eq!(t.char_to_utf16_cu(2), 3); // after '😀' → 1+2=3 CU
        assert_eq!(t.char_to_utf16_cu(3), 4); // after 'b' → 1+2+1=4 CU

        // Reverse: 3 UTF-16 CU on line 0 → char index 2 (after the emoji)
        assert_eq!(t.utf16_cu_to_char(0, 3), 2);
    }

    #[test]
    fn slice_and_line() {
        let t = Text::from("hello\nworld\nfoo");
        let line1: String = t.line(1).chars().collect();
        assert_eq!(line1, "world\n");

        let slice: String = t.slice(6..11).chars().collect();
        assert_eq!(slice, "world");
    }

    #[test]
    fn lines_range_subset() {
        let t = Text::from("aaa\nbbb\nccc\nddd\n");
        let lines: Vec<String> = t.lines_range(1, 3).map(|s| s.chars().collect()).collect();
        // ropey's .lines() on a slice ending with \n yields the content lines
        // plus a trailing empty slice.
        assert!(lines.len() >= 2);
        assert_eq!(lines[0], "bbb\n");
        assert_eq!(lines[1], "ccc\n");
    }

    #[test]
    fn len_bytes() {
        let t = Text::from("café");
        assert_eq!(t.len_chars(), 4);
        assert_eq!(t.len_bytes(), 5); // 'é' = 2 bytes
    }

    // ── Word boundary tests ─────────────────────────────────────────

    #[test]
    fn word_at_in_word() {
        let t = Text::from("hello world foo");
        let r = t.word_at(7); // on 'o' in "world"
        assert_eq!(r.start(), 6);
        assert_eq!(r.end(), 11);
    }

    #[test]
    fn word_at_on_space() {
        let t = Text::from("hello world");
        let r = t.word_at(5); // on the space
        assert!(r.is_empty());
    }

    #[test]
    fn word_start_end() {
        let t = Text::from("fn main() {");
        assert_eq!(t.word_start(4), 3); // inside "main"
        assert_eq!(t.word_end(3), 7);   // "main" ends at 7
    }

    #[test]
    fn word_boundary_forward() {
        let t = Text::from("hello world_foo bar");
        assert_eq!(t.word_boundary_forward(0), 6);  // skip "hello" + space → "world_foo"
        assert_eq!(t.word_boundary_forward(6), 16); // skip "world_foo" + space → "bar"
    }

    #[test]
    fn word_boundary_backward() {
        let t = Text::from("hello world bar");
        assert_eq!(t.word_boundary_backward(15), 12); // back past "bar" → "bar" start
        assert_eq!(t.word_boundary_backward(12), 6);  // back past "world" → "world" start
    }

    // ── Line selection tests ────────────────────────────────────────

    #[test]
    fn select_line_middle() {
        let t = Text::from("aaa\nbbb\nccc");
        let r = t.select_line(5); // on 'b' in second line
        assert_eq!(r.start(), 4);
        assert_eq!(r.end(), 8); // includes trailing \n
    }

    #[test]
    fn select_line_last() {
        let t = Text::from("aaa\nbbb");
        let r = t.select_line(5); // last line, no trailing \n
        assert_eq!(r.start(), 4);
        assert_eq!(r.end(), 7);
    }

    #[test]
    fn line_len_no_newline_basic() {
        let t = Text::from("hello\nworld");
        assert_eq!(t.line_len_no_newline(0), 5);
        assert_eq!(t.line_len_no_newline(1), 5);
    }

    // ── Search tests ────────────────────────────────────────────────

    #[test]
    fn find_next_basic() {
        let t = Text::from("hello hello hello");
        let (start, end) = t.find_next("hello", 0).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 5);

        let (start2, _) = t.find_next("hello", 1).unwrap();
        assert_eq!(start2, 6); // second occurrence
    }

    #[test]
    fn find_next_wraps_around() {
        let t = Text::from("abc def abc");
        // Search from pos 8, should wrap and find at 0
        let (start, _) = t.find_next("abc", 8).unwrap();
        assert_eq!(start, 8); // there's one at 8 too
    }

    #[test]
    fn find_next_not_found() {
        let t = Text::from("hello world");
        assert!(t.find_next("xyz", 0).is_none());
    }

    // ── find_all tests ──────────────────────────────────────────────

    #[test]
    fn find_all_basic() {
        let t = Text::from("hello hello hello");
        let matches = t.find_all("hello", true);
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], (0, 5));
        assert_eq!(matches[1], (6, 11));
        assert_eq!(matches[2], (12, 17));
    }

    #[test]
    fn find_all_no_matches() {
        let t = Text::from("hello world");
        let matches = t.find_all("xyz", true);
        assert!(matches.is_empty());
    }

    #[test]
    fn find_all_case_insensitive() {
        let t = Text::from("Hello HELLO hello");
        let matches = t.find_all("hello", false);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn find_all_empty_needle() {
        let t = Text::from("hello");
        let matches = t.find_all("", true);
        assert!(matches.is_empty());
    }
}
