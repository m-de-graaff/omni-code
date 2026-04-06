//! Language registry: maps language IDs to tree-sitter grammars and queries.

use std::collections::HashMap;

use crate::highlighter::Highlighter;
use crate::language::LanguageConfig;

/// A registered language with its grammar and highlight query.
pub struct LanguageEntry {
    /// Language identifier (e.g., `"rust"`).
    pub id: &'static str,
    /// The tree-sitter language grammar.
    pub ts_language: tree_sitter::Language,
    /// The highlights.scm query source.
    pub highlights_query: &'static str,
    /// Language configuration (comment tokens, indent, etc.).
    pub config: LanguageConfig,
}

impl std::fmt::Debug for LanguageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageEntry")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Registry of all available languages.
///
/// Languages are registered at construction time via feature-gated code.
/// Each language provides a tree-sitter grammar and a highlights.scm query.
#[derive(Debug)]
pub struct LanguageRegistry {
    languages: HashMap<&'static str, LanguageEntry>,
}

impl LanguageRegistry {
    /// Create a new registry with all feature-gated languages.
    #[must_use]
    pub fn new() -> Self {
        let mut languages = HashMap::new();

        #[cfg(feature = "lang-rust")]
        {
            languages.insert(
                "rust",
                LanguageEntry {
                    id: "rust",
                    ts_language: tree_sitter_rust::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/rust/highlights.scm"),
                    config: LanguageConfig {
                        name: "Rust".to_string(),
                        language_id: "rust".to_string(),
                        file_extensions: vec!["rs".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "    ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-toml")]
        {
            languages.insert(
                "toml",
                LanguageEntry {
                    id: "toml",
                    ts_language: tree_sitter_toml_ng::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/toml/highlights.scm"),
                    config: LanguageConfig {
                        name: "TOML".to_string(),
                        language_id: "toml".to_string(),
                        file_extensions: vec!["toml".to_string()],
                        comment_token: Some("#".to_string()),
                        block_comment_tokens: None,
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-markdown")]
        {
            languages.insert(
                "markdown",
                LanguageEntry {
                    id: "markdown",
                    ts_language: tree_sitter_md::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/markdown/highlights.scm"),
                    config: LanguageConfig {
                        name: "Markdown".to_string(),
                        language_id: "markdown".to_string(),
                        file_extensions: vec!["md".to_string(), "markdown".to_string()],
                        comment_token: None,
                        block_comment_tokens: None,
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-python")]
        {
            languages.insert(
                "python",
                LanguageEntry {
                    id: "python",
                    ts_language: tree_sitter_python::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/python/highlights.scm"),
                    config: LanguageConfig {
                        name: "Python".to_string(),
                        language_id: "python".to_string(),
                        file_extensions: vec!["py".to_string(), "pyi".to_string()],
                        comment_token: Some("#".to_string()),
                        block_comment_tokens: None,
                        indent_unit: "    ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-javascript")]
        {
            languages.insert(
                "javascript",
                LanguageEntry {
                    id: "javascript",
                    ts_language: tree_sitter_javascript::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/javascript/highlights.scm"),
                    config: LanguageConfig {
                        name: "JavaScript".to_string(),
                        language_id: "javascript".to_string(),
                        file_extensions: vec!["js".to_string(), "mjs".to_string(), "cjs".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
            // JSX uses the same grammar as JavaScript
            languages.insert(
                "jsx",
                LanguageEntry {
                    id: "jsx",
                    ts_language: tree_sitter_javascript::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/javascript/highlights.scm"),
                    config: LanguageConfig {
                        name: "JSX".to_string(),
                        language_id: "jsx".to_string(),
                        file_extensions: vec!["jsx".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-typescript")]
        {
            languages.insert(
                "typescript",
                LanguageEntry {
                    id: "typescript",
                    ts_language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                    highlights_query: include_str!("../queries/typescript/highlights.scm"),
                    config: LanguageConfig {
                        name: "TypeScript".to_string(),
                        language_id: "typescript".to_string(),
                        file_extensions: vec!["ts".to_string(), "mts".to_string(), "cts".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
            languages.insert(
                "tsx",
                LanguageEntry {
                    id: "tsx",
                    ts_language: tree_sitter_typescript::LANGUAGE_TSX.into(),
                    highlights_query: include_str!("../queries/tsx/highlights.scm"),
                    config: LanguageConfig {
                        name: "TSX".to_string(),
                        language_id: "tsx".to_string(),
                        file_extensions: vec!["tsx".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-json")]
        {
            languages.insert(
                "json",
                LanguageEntry {
                    id: "json",
                    ts_language: tree_sitter_json::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/json/highlights.scm"),
                    config: LanguageConfig {
                        name: "JSON".to_string(),
                        language_id: "json".to_string(),
                        file_extensions: vec!["json".to_string(), "jsonc".to_string()],
                        comment_token: None,
                        block_comment_tokens: None,
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-css")]
        {
            languages.insert(
                "css",
                LanguageEntry {
                    id: "css",
                    ts_language: tree_sitter_css::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/css/highlights.scm"),
                    config: LanguageConfig {
                        name: "CSS".to_string(),
                        language_id: "css".to_string(),
                        file_extensions: vec!["css".to_string()],
                        comment_token: None,
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
            // SCSS uses same grammar for basic highlighting
            languages.insert(
                "scss",
                LanguageEntry {
                    id: "scss",
                    ts_language: tree_sitter_css::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/css/highlights.scm"),
                    config: LanguageConfig {
                        name: "SCSS".to_string(),
                        language_id: "scss".to_string(),
                        file_extensions: vec!["scss".to_string(), "sass".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-html")]
        {
            languages.insert(
                "html",
                LanguageEntry {
                    id: "html",
                    ts_language: tree_sitter_html::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/html/highlights.scm"),
                    config: LanguageConfig {
                        name: "HTML".to_string(),
                        language_id: "html".to_string(),
                        file_extensions: vec!["html".to_string(), "htm".to_string()],
                        comment_token: None,
                        block_comment_tokens: Some(("<!--".to_string(), "-->".to_string())),
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-go")]
        {
            languages.insert(
                "go",
                LanguageEntry {
                    id: "go",
                    ts_language: tree_sitter_go::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/go/highlights.scm"),
                    config: LanguageConfig {
                        name: "Go".to_string(),
                        language_id: "go".to_string(),
                        file_extensions: vec!["go".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "\t".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-c")]
        {
            languages.insert(
                "c",
                LanguageEntry {
                    id: "c",
                    ts_language: tree_sitter_c::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/c/highlights.scm"),
                    config: LanguageConfig {
                        name: "C".to_string(),
                        language_id: "c".to_string(),
                        file_extensions: vec!["c".to_string(), "h".to_string()],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "    ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-cpp")]
        {
            languages.insert(
                "cpp",
                LanguageEntry {
                    id: "cpp",
                    ts_language: tree_sitter_cpp::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/cpp/highlights.scm"),
                    config: LanguageConfig {
                        name: "C++".to_string(),
                        language_id: "cpp".to_string(),
                        file_extensions: vec![
                            "cpp".to_string(), "cc".to_string(), "cxx".to_string(),
                            "hpp".to_string(), "hxx".to_string(),
                        ],
                        comment_token: Some("//".to_string()),
                        block_comment_tokens: Some(("/*".to_string(), "*/".to_string())),
                        indent_unit: "    ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-bash")]
        {
            languages.insert(
                "bash",
                LanguageEntry {
                    id: "bash",
                    ts_language: tree_sitter_bash::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/bash/highlights.scm"),
                    config: LanguageConfig {
                        name: "Bash".to_string(),
                        language_id: "bash".to_string(),
                        file_extensions: vec!["sh".to_string(), "bash".to_string(), "zsh".to_string()],
                        comment_token: Some("#".to_string()),
                        block_comment_tokens: None,
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        #[cfg(feature = "lang-yaml")]
        {
            languages.insert(
                "yaml",
                LanguageEntry {
                    id: "yaml",
                    ts_language: tree_sitter_yaml::LANGUAGE.into(),
                    highlights_query: include_str!("../queries/yaml/highlights.scm"),
                    config: LanguageConfig {
                        name: "YAML".to_string(),
                        language_id: "yaml".to_string(),
                        file_extensions: vec!["yaml".to_string(), "yml".to_string()],
                        comment_token: Some("#".to_string()),
                        block_comment_tokens: None,
                        indent_unit: "  ".to_string(),
                    },
                },
            );
        }

        Self { languages }
    }

    /// Look up a language by its identifier.
    #[must_use]
    pub fn get(&self, language_id: &str) -> Option<&LanguageEntry> {
        self.languages.get(language_id)
    }

    /// Create a [`Highlighter`] for the given language.
    ///
    /// Returns `None` if the language isn't registered, or if the query
    /// fails to compile.
    #[must_use]
    pub fn create_highlighter(&self, language_id: &str) -> Option<Highlighter> {
        let entry = self.get(language_id)?;
        Highlighter::new(&entry.ts_language, entry.highlights_query).ok()
    }

    /// All registered language IDs.
    pub fn language_ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.languages.keys().copied()
    }

    /// Number of registered languages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.languages.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_languages() {
        let reg = LanguageRegistry::new();
        // With default features, at least Rust should be registered
        #[cfg(feature = "lang-rust")]
        {
            assert!(reg.get("rust").is_some());
            assert!(reg.len() >= 1);
        }
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn create_rust_highlighter() {
        let reg = LanguageRegistry::new();
        let hl = reg.create_highlighter("rust");
        assert!(hl.is_some(), "should create a Rust highlighter");
    }

    #[test]
    fn unknown_language_returns_none() {
        let reg = LanguageRegistry::new();
        assert!(reg.get("nonexistent").is_none());
        assert!(reg.create_highlighter("nonexistent").is_none());
    }
}
