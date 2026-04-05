//! # omni-syntax
//!
//! Syntax highlighting via tree-sitter and language configuration management.

pub mod highlight;
pub mod language;
pub mod tree;

pub use highlight::{HighlightConfig, HighlightEvent};
pub use language::LanguageConfig;
pub use tree::SyntaxTree;
