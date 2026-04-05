//! Theme definition and parsing.

use serde::{Deserialize, Serialize};

/// A color theme for the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name.
    pub name: String,
    /// Background color (hex).
    pub background: String,
    /// Foreground / default text color (hex).
    pub foreground: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            background: "#1e1e2e".to_string(),
            foreground: "#cdd6f4".to_string(),
        }
    }
}
