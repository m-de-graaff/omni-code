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
    /// Whether to format on save.
    pub format_on_save: bool,
    /// Per-language configuration overrides.
    #[serde(default)]
    pub languages: std::collections::HashMap<String, LanguageOverrides>,
}

/// Per-language configuration overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageOverrides {
    pub tab_width: Option<usize>,
    pub use_spaces: Option<bool>,
    pub formatter: Option<String>,
    pub format_on_save: Option<bool>,
}

impl EditorConfig {
    /// Resolve config for a specific language, overlaying any overrides.
    #[must_use]
    pub fn resolve_for_language(&self, lang: Option<&str>) -> Self {
        let mut resolved = self.clone();
        if let Some(lang_id) = lang {
            if let Some(overrides) = self.languages.get(lang_id) {
                if let Some(tw) = overrides.tab_width {
                    resolved.tab_width = tw;
                }
                if let Some(us) = overrides.use_spaces {
                    resolved.use_spaces = us;
                }
                if let Some(fos) = overrides.format_on_save {
                    resolved.format_on_save = fos;
                }
            }
        }
        resolved
    }
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
            format_on_save: false,
            languages: std::collections::HashMap::new(),
        }
    }
}
