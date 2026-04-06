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
    /// Tick rate in milliseconds for periodic UI refresh (animations, spinners).
    pub tick_rate_ms: u64,
    /// File size in bytes above which syntax highlighting is disabled.
    pub large_file_threshold: usize,
    /// Auto-save interval in milliseconds. 0 = disabled.
    pub auto_save_ms: u64,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            use_spaces: true,
            line_numbers: true,
            word_wrap: false,
            theme: "default".to_string(),
            tick_rate_ms: 100,
            large_file_threshold: 10_485_760, // 10 MB
            auto_save_ms: 0,                  // disabled
        }
    }
}
