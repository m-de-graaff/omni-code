//! Editor configuration loaded from TOML files.

use serde::{Deserialize, Serialize};

/// Top-level editor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    /// Tab width in spaces.
    pub tab_width: usize,
    /// Whether to use spaces for indentation.
    pub use_spaces: bool,
    /// Whether to show line numbers.
    pub line_numbers: bool,
    /// Whether to enable word wrap.
    pub word_wrap: bool,
    /// Theme name.
    pub theme: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            use_spaces: true,
            line_numbers: true,
            word_wrap: false,
            theme: "default".to_string(),
        }
    }
}
