//! # omni-syntax
//!
//! Syntax highlighting via tree-sitter and language configuration management.

pub mod highlight;
pub mod highlighter;
pub mod language;
pub mod registry;
pub mod symbols;
pub mod tree;

pub use highlight::{HighlightScope, HighlightSpan};
pub use highlighter::Highlighter;
pub use language::LanguageConfig;
pub use registry::LanguageRegistry;
pub use symbols::{DocumentSymbol, SymbolKind, extract_symbols};
pub use tree::SyntaxTree;
