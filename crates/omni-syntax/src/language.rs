//! Language configuration for tree-sitter grammars.

use serde::{Deserialize, Serialize};

/// Configuration for a supported language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Display name (e.g., "Rust").
    pub name: String,
    /// Language identifier (e.g., "rust").
    pub language_id: String,
    /// File extensions that trigger this language (e.g., `["rs"]`).
    pub file_extensions: Vec<String>,
    /// Line comment token (e.g., "//").
    pub comment_token: Option<String>,
    /// Block comment tokens (e.g., `["/*", "*/"]`).
    pub block_comment_tokens: Option<(String, String)>,
    /// Indent unit (e.g., "    " for 4 spaces).
    pub indent_unit: String,
}
