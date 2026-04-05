//! Syntax highlight configuration and events.

/// Configuration for highlighting a particular language.
#[derive(Debug)]
pub struct HighlightConfig {
    /// The language name (e.g., "rust", "python").
    pub language_name: String,
    /// Tree-sitter highlight query source.
    pub query_source: String,
}

/// Events emitted during syntax highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightEvent {
    /// A highlight scope starts at this byte offset.
    ScopeStart { byte: usize, scope: String },
    /// A highlight scope ends at this byte offset.
    ScopeEnd { byte: usize },
    /// Unhighlighted text span.
    Source { start: usize, end: usize },
}
