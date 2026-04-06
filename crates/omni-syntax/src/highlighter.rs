//! Tree-sitter-based syntax highlighter.
//!
//! Parses source code and runs highlight queries to produce [`HighlightSpan`]s.

use tree_sitter::{InputEdit, Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

use crate::highlight::{HighlightScope, HighlightSpan};

/// Errors from the highlighter.
#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    #[error("failed to set parser language: {0}")]
    Language(String),
    #[error("failed to compile highlight query: {0}")]
    Query(String),
}

/// A syntax highlighter for a single language.
///
/// Holds a tree-sitter [`Parser`] and compiled [`Query`] for highlight captures.
/// Each document gets its own `Highlighter` instance (parser state is per-document).
pub struct Highlighter {
    parser: Parser,
    query: Query,
    /// Pre-computed scope for each capture index in the query.
    capture_scopes: Vec<Option<HighlightScope>>,
}

impl std::fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Highlighter")
            .field("capture_count", &self.capture_scopes.len())
            .finish_non_exhaustive()
    }
}

impl Highlighter {
    /// Create a new highlighter for the given language and query source.
    ///
    /// # Errors
    ///
    /// Returns an error if the language cannot be set or the query fails to compile.
    pub fn new(language: &Language, query_source: &str) -> Result<Self, HighlightError> {
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .map_err(|e| HighlightError::Language(e.to_string()))?;

        let query = Query::new(language, query_source)
            .map_err(|e| HighlightError::Query(e.to_string()))?;

        // Pre-compute scope for each capture index
        let capture_scopes: Vec<Option<HighlightScope>> = query
            .capture_names()
            .iter()
            .map(|name| HighlightScope::from_capture(name))
            .collect();

        Ok(Self {
            parser,
            query,
            capture_scopes,
        })
    }

    /// Full parse of a text buffer. Returns the tree and highlight spans.
    pub fn parse_full(&mut self, text: &omni_core::Text) -> Option<(Tree, Vec<HighlightSpan>)> {
        let source = text.to_string();
        let tree = self.parser.parse(source.as_bytes(), None)?;
        let spans = self.compute_highlights(&tree, source.as_bytes());
        Some((tree, spans))
    }

    /// Incremental re-parse after an edit.
    ///
    /// The caller must call `old_tree.edit(&input_edit)` before calling this.
    pub fn parse_incremental(
        &mut self,
        old_tree: &mut Tree,
        text: &omni_core::Text,
        edit: &InputEdit,
    ) -> Option<(Tree, Vec<HighlightSpan>)> {
        old_tree.edit(edit);
        let source = text.to_string();
        let tree = self.parser.parse(source.as_bytes(), Some(old_tree))?;
        let spans = self.compute_highlights(&tree, source.as_bytes());
        Some((tree, spans))
    }

    /// Run highlight queries on a parsed tree, returning sorted spans.
    fn compute_highlights(&self, tree: &Tree, source: &[u8]) -> Vec<HighlightSpan> {
        let mut cursor = QueryCursor::new();
        let root = tree.root_node();

        // Collect all captures with their pattern index (for priority)
        let mut raw_spans: Vec<(usize, HighlightSpan)> = Vec::new();

        let mut matches = cursor.matches(&self.query, root, source);
        while let Some(m) = matches.next() {
            let pattern_idx = m.pattern_index;
            for capture in m.captures {
                let idx = capture.index as usize;
                if let Some(Some(scope)) = self.capture_scopes.get(idx) {
                    let node = capture.node;
                    raw_spans.push((
                        pattern_idx,
                        HighlightSpan {
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            scope: *scope,
                        },
                    ));
                }
            }
        }

        // Sort by start position, then by end position descending, then by
        // pattern index descending (later patterns = higher priority in tree-sitter)
        raw_spans.sort_by(|a, b| {
            a.1.start_byte
                .cmp(&b.1.start_byte)
                .then(b.1.end_byte.cmp(&a.1.end_byte))
                .then(b.0.cmp(&a.0))
        });

        // Dedup: for spans at the same position, keep the highest priority (first after sort)
        let mut spans = Vec::with_capacity(raw_spans.len());
        let mut last_start = usize::MAX;
        let mut last_end = usize::MAX;

        for (_, span) in raw_spans {
            if span.start_byte == last_start && span.end_byte == last_end {
                continue; // duplicate position, lower priority — skip
            }
            last_start = span.start_byte;
            last_end = span.end_byte;
            spans.push(span);
        }

        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_core::Text;

    #[cfg(feature = "lang-rust")]
    #[test]
    fn parse_rust_code() {
        let query_src = include_str!("../queries/rust/highlights.scm");
        let lang = tree_sitter_rust::LANGUAGE.into();
        let mut hl = Highlighter::new(&lang, query_src).expect("query should compile");

        let text = Text::from("fn main() {\n    let x = 42;\n}\n");
        let (tree, spans) = hl.parse_full(&text).expect("parse should succeed");

        assert!(!spans.is_empty(), "should produce highlight spans");
        assert!(tree.root_node().child_count() > 0);

        // Check that we have highlight spans for identifiers/functions
        let has_function = spans.iter().any(|s| matches!(s.scope, HighlightScope::Function));
        assert!(has_function, "should have at least one function highlight, got: {spans:?}");
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn spans_are_sorted() {
        let query_src = include_str!("../queries/rust/highlights.scm");
        let lang = tree_sitter_rust::LANGUAGE.into();
        let mut hl = Highlighter::new(&lang, query_src).unwrap();

        let text = Text::from("use std::collections::HashMap;\nfn foo() {}\n");
        let (_, spans) = hl.parse_full(&text).unwrap();

        for window in spans.windows(2) {
            assert!(
                window[0].start_byte <= window[1].start_byte,
                "spans should be sorted by start_byte"
            );
        }
    }
}
