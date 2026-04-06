//! Theme definition, parsing, and color resolution.
//!
//! Themes are defined as TOML with hex color strings. At startup,
//! a `Theme` is resolved into `ThemeColors` (ratatui `Color` values)
//! based on the terminal's color capability.

use omni_syntax::HighlightScope;
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};

/// Terminal color capability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCapability {
    /// 24-bit true color (16M colors).
    TrueColor,
    /// 256-color palette.
    Color256,
    /// Basic 16-color ANSI.
    Basic,
}

/// Detect terminal color capability from environment variables.
pub fn detect_color_capability() -> ColorCapability {
    match std::env::var("COLORTERM").as_deref() {
        Ok("truecolor" | "24bit") => ColorCapability::TrueColor,
        _ => match std::env::var("TERM").as_deref() {
            Ok(t) if t.contains("256color") => ColorCapability::Color256,
            _ => {
                // Windows Terminal and most modern terminals support truecolor
                if cfg!(windows) { ColorCapability::TrueColor } else { ColorCapability::Basic }
            }
        },
    }
}

/// A color theme defined as hex strings, serializable to/from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    // Base
    pub background: String,
    pub foreground: String,
    pub sidebar_bg: String,
    pub tab_bar_bg: String,
    pub status_bar_bg: String,
    pub panel_bg: String,
    // Borders
    pub border: String,
    pub border_focused: String,
    pub border_drag: String,
    // Text
    pub text_muted: String,
    pub text_accent: String,
    // Tabs
    pub tab_active_bg: String,
    pub tab_active_fg: String,
    pub tab_inactive_fg: String,
    pub tab_close: String,
    // Status bar modes
    pub mode_normal_bg: String,
    pub mode_normal_fg: String,
    pub mode_insert_bg: String,
    pub mode_insert_fg: String,
    pub mode_ai_bg: String,
    pub mode_ai_fg: String,
    // Indicators
    pub modified: String,
    pub ai_marker: String,
    // Selection & highlights
    pub selection_bg: String,
    pub hover_bg: String,
    pub cursor: String,
    // Gutter
    pub gutter_fg: String,
    // Context menu
    pub menu_bg: String,
    pub menu_fg: String,
    pub menu_selected_bg: String,
    pub menu_selected_fg: String,
    // Search highlights
    pub search_match_bg: String,
    pub search_current_bg: String,
    // Syntax highlighting
    pub syntax_keyword: String,
    pub syntax_function: String,
    pub syntax_type: String,
    pub syntax_string: String,
    pub syntax_number: String,
    pub syntax_comment: String,
    pub syntax_operator: String,
    pub syntax_variable: String,
    pub syntax_constant: String,
    pub syntax_property: String,
    pub syntax_attribute: String,
    pub syntax_namespace: String,
    pub syntax_punctuation: String,
    pub syntax_escape: String,
    pub syntax_tag: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Zed-inspired dark theme (Catppuccin Mocha palette).
    #[must_use]
    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            background: "#1e1e2e".into(),
            foreground: "#cdd6f4".into(),
            sidebar_bg: "#252535".into(),
            tab_bar_bg: "#181825".into(),
            status_bar_bg: "#1a1a2e".into(),
            panel_bg: "#1e1e2e".into(),
            border: "#45475a".into(),
            border_focused: "#89b4fa".into(),
            border_drag: "#74c7ec".into(),
            text_muted: "#6c7086".into(),
            text_accent: "#89b4fa".into(),
            tab_active_bg: "#313244".into(),
            tab_active_fg: "#cdd6f4".into(),
            tab_inactive_fg: "#6c7086".into(),
            tab_close: "#f38ba8".into(),
            mode_normal_bg: "#89b4fa".into(),
            mode_normal_fg: "#1e1e2e".into(),
            mode_insert_bg: "#a6e3a1".into(),
            mode_insert_fg: "#1e1e2e".into(),
            mode_ai_bg: "#cba6f7".into(),
            mode_ai_fg: "#1e1e2e".into(),
            modified: "#fab387".into(),
            ai_marker: "#e0a526".into(),
            selection_bg: "#45475a".into(),
            hover_bg: "#313244".into(),
            cursor: "#f5e0dc".into(),
            gutter_fg: "#45475a".into(),
            menu_bg: "#313244".into(),
            menu_fg: "#cdd6f4".into(),
            menu_selected_bg: "#89b4fa".into(),
            menu_selected_fg: "#1e1e2e".into(),
            // Catppuccin Mocha syntax colors
            search_match_bg: "#45475a".into(),
            search_current_bg: "#fab387".into(),
            syntax_keyword: "#cba6f7".into(),
            syntax_function: "#89b4fa".into(),
            syntax_type: "#f9e2af".into(),
            syntax_string: "#a6e3a1".into(),
            syntax_number: "#fab387".into(),
            syntax_comment: "#6c7086".into(),
            syntax_operator: "#89dceb".into(),
            syntax_variable: "#cdd6f4".into(),
            syntax_constant: "#fab387".into(),
            syntax_property: "#b4befe".into(),
            syntax_attribute: "#f9e2af".into(),
            syntax_namespace: "#89b4fa".into(),
            syntax_punctuation: "#9399b2".into(),
            syntax_escape: "#f2cdcd".into(),
            syntax_tag: "#89b4fa".into(),
        }
    }

    /// Light theme alternative.
    #[must_use]
    pub fn light() -> Self {
        Self {
            name: "light".into(),
            background: "#eff1f5".into(),
            foreground: "#4c4f69".into(),
            sidebar_bg: "#e6e9ef".into(),
            tab_bar_bg: "#dce0e8".into(),
            status_bar_bg: "#ccd0da".into(),
            panel_bg: "#eff1f5".into(),
            border: "#9ca0b0".into(),
            border_focused: "#1e66f5".into(),
            border_drag: "#209fb5".into(),
            text_muted: "#9ca0b0".into(),
            text_accent: "#1e66f5".into(),
            tab_active_bg: "#ccd0da".into(),
            tab_active_fg: "#4c4f69".into(),
            tab_inactive_fg: "#9ca0b0".into(),
            tab_close: "#d20f39".into(),
            mode_normal_bg: "#1e66f5".into(),
            mode_normal_fg: "#eff1f5".into(),
            mode_insert_bg: "#40a02b".into(),
            mode_insert_fg: "#eff1f5".into(),
            mode_ai_bg: "#8839ef".into(),
            mode_ai_fg: "#eff1f5".into(),
            modified: "#fe640b".into(),
            ai_marker: "#df8e1d".into(),
            selection_bg: "#acb0be".into(),
            hover_bg: "#ccd0da".into(),
            cursor: "#dc8a78".into(),
            gutter_fg: "#9ca0b0".into(),
            menu_bg: "#ccd0da".into(),
            menu_fg: "#4c4f69".into(),
            menu_selected_bg: "#1e66f5".into(),
            menu_selected_fg: "#eff1f5".into(),
            // Catppuccin Latte syntax colors
            search_match_bg: "#acb0be".into(),
            search_current_bg: "#fe640b".into(),
            syntax_keyword: "#8839ef".into(),
            syntax_function: "#1e66f5".into(),
            syntax_type: "#df8e1d".into(),
            syntax_string: "#40a02b".into(),
            syntax_number: "#fe640b".into(),
            syntax_comment: "#9ca0b0".into(),
            syntax_operator: "#04a5e5".into(),
            syntax_variable: "#4c4f69".into(),
            syntax_constant: "#fe640b".into(),
            syntax_property: "#7287fd".into(),
            syntax_attribute: "#df8e1d".into(),
            syntax_namespace: "#1e66f5".into(),
            syntax_punctuation: "#8c8fa1".into(),
            syntax_escape: "#dd7878".into(),
            syntax_tag: "#1e66f5".into(),
        }
    }

    /// Load a theme by name. Falls back to the dark theme.
    #[must_use]
    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }
}

/// Parse a hex color string (e.g., `"#1e1e2e"`) into RGB components.
fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Find the nearest 256-color ANSI index for an RGB color.
///
/// Uses the 6x6x6 color cube (indices 16-231) for the best approximation.
#[allow(clippy::cast_possible_truncation)] // Result is always 0..=5
fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    // Map each channel to the 6-level cube (0-5)
    let ri = ((u16::from(r) * 5 + 127) / 255) as u8;
    let gi = ((u16::from(g) * 5 + 127) / 255) as u8;
    let bi = ((u16::from(b) * 5 + 127) / 255) as u8;
    16 + 36 * ri + 6 * gi + bi
}

/// Resolve a hex color string to a ratatui `Color` based on capability.
fn resolve_color(hex: &str, capability: ColorCapability) -> Color {
    let Some((r, g, b)) = parse_hex(hex) else {
        return Color::Reset;
    };
    match capability {
        ColorCapability::TrueColor => Color::Rgb(r, g, b),
        ColorCapability::Color256 => Color::Indexed(nearest_256(r, g, b)),
        ColorCapability::Basic => approx_basic(r, g, b),
    }
}

/// Approximate an RGB color to the basic 16-color ANSI palette.
fn approx_basic(r: u8, g: u8, b: u8) -> Color {
    let lum = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
    if lum < 40 {
        Color::Black
    } else if lum < 100 {
        Color::DarkGray
    } else if lum < 180 {
        Color::Gray
    } else {
        Color::White
    }
}

/// Resolved theme colors using ratatui `Color` values.
///
/// Created once at startup from a `Theme` + `ColorCapability`.
/// Passed to all widgets through `Context`.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Base
    pub background: Color,
    pub foreground: Color,
    pub sidebar_bg: Color,
    pub tab_bar_bg: Color,
    pub status_bar_bg: Color,
    pub panel_bg: Color,
    // Borders
    pub border: Color,
    pub border_focused: Color,
    pub border_drag: Color,
    // Text
    pub text_muted: Color,
    pub text_accent: Color,
    // Tabs
    pub tab_active_bg: Color,
    pub tab_active_fg: Color,
    pub tab_inactive_fg: Color,
    pub tab_close: Color,
    // Status bar modes
    pub mode_normal_bg: Color,
    pub mode_normal_fg: Color,
    pub mode_insert_bg: Color,
    pub mode_insert_fg: Color,
    pub mode_ai_bg: Color,
    pub mode_ai_fg: Color,
    // Indicators
    pub modified: Color,
    pub ai_marker: Color,
    // Selection & highlights
    pub selection_bg: Color,
    pub hover_bg: Color,
    pub cursor: Color,
    // Gutter
    pub gutter_fg: Color,
    // Context menu
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub menu_selected_bg: Color,
    pub menu_selected_fg: Color,
    // Search highlights
    pub search_match_bg: Color,
    pub search_current_bg: Color,
    // Syntax highlighting
    pub syntax: SyntaxColors,
}

/// Resolved syntax highlight colors.
#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub keyword: Color,
    pub function: Color,
    pub r#type: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub operator: Color,
    pub variable: Color,
    pub constant: Color,
    pub property: Color,
    pub attribute: Color,
    pub namespace: Color,
    pub punctuation: Color,
    pub escape: Color,
    pub tag: Color,
}

impl SyntaxColors {
    /// Get the foreground [`Style`] for a given highlight scope.
    #[must_use]
    pub const fn style_for_scope(&self, scope: HighlightScope) -> Style {
        Style::new().fg(self.color_for_scope(scope))
    }

    /// Get the foreground [`Color`] for a given highlight scope.
    #[must_use]
    pub const fn color_for_scope(&self, scope: HighlightScope) -> Color {
        match scope {
            HighlightScope::Keyword
            | HighlightScope::KeywordFunction
            | HighlightScope::KeywordReturn
            | HighlightScope::KeywordOperator
            | HighlightScope::KeywordControl => self.keyword,

            HighlightScope::Function
            | HighlightScope::FunctionMethod
            | HighlightScope::FunctionBuiltin
            | HighlightScope::FunctionMacro => self.function,

            HighlightScope::Type | HighlightScope::TypeBuiltin => self.r#type,

            HighlightScope::Variable
            | HighlightScope::VariableBuiltin
            | HighlightScope::VariableParameter
            | HighlightScope::Label => self.variable,

            HighlightScope::String | HighlightScope::StringSpecial => self.string,

            HighlightScope::Number | HighlightScope::Boolean => self.number,

            HighlightScope::Comment | HighlightScope::CommentDoc => self.comment,

            HighlightScope::Operator => self.operator,

            HighlightScope::Punctuation
            | HighlightScope::PunctuationBracket
            | HighlightScope::PunctuationDelimiter => self.punctuation,

            HighlightScope::Constant | HighlightScope::ConstantBuiltin => self.constant,
            HighlightScope::Property => self.property,
            HighlightScope::Namespace => self.namespace,
            HighlightScope::Attribute => self.attribute,
            HighlightScope::Tag => self.tag,
            HighlightScope::Escape => self.escape,
        }
    }
}

impl ThemeColors {
    /// Resolve a `Theme` into concrete `Color` values for the given capability.
    #[must_use]
    pub fn from_theme(theme: &Theme, capability: ColorCapability) -> Self {
        let c = |hex: &str| resolve_color(hex, capability);
        Self {
            background: c(&theme.background),
            foreground: c(&theme.foreground),
            sidebar_bg: c(&theme.sidebar_bg),
            tab_bar_bg: c(&theme.tab_bar_bg),
            status_bar_bg: c(&theme.status_bar_bg),
            panel_bg: c(&theme.panel_bg),
            border: c(&theme.border),
            border_focused: c(&theme.border_focused),
            border_drag: c(&theme.border_drag),
            text_muted: c(&theme.text_muted),
            text_accent: c(&theme.text_accent),
            tab_active_bg: c(&theme.tab_active_bg),
            tab_active_fg: c(&theme.tab_active_fg),
            tab_inactive_fg: c(&theme.tab_inactive_fg),
            tab_close: c(&theme.tab_close),
            mode_normal_bg: c(&theme.mode_normal_bg),
            mode_normal_fg: c(&theme.mode_normal_fg),
            mode_insert_bg: c(&theme.mode_insert_bg),
            mode_insert_fg: c(&theme.mode_insert_fg),
            mode_ai_bg: c(&theme.mode_ai_bg),
            mode_ai_fg: c(&theme.mode_ai_fg),
            modified: c(&theme.modified),
            ai_marker: c(&theme.ai_marker),
            selection_bg: c(&theme.selection_bg),
            hover_bg: c(&theme.hover_bg),
            cursor: c(&theme.cursor),
            gutter_fg: c(&theme.gutter_fg),
            menu_bg: c(&theme.menu_bg),
            menu_fg: c(&theme.menu_fg),
            menu_selected_bg: c(&theme.menu_selected_bg),
            menu_selected_fg: c(&theme.menu_selected_fg),
            search_match_bg: c(&theme.search_match_bg),
            search_current_bg: c(&theme.search_current_bg),
            syntax: SyntaxColors {
                keyword: c(&theme.syntax_keyword),
                function: c(&theme.syntax_function),
                r#type: c(&theme.syntax_type),
                string: c(&theme.syntax_string),
                number: c(&theme.syntax_number),
                comment: c(&theme.syntax_comment),
                operator: c(&theme.syntax_operator),
                variable: c(&theme.syntax_variable),
                constant: c(&theme.syntax_constant),
                property: c(&theme.syntax_property),
                attribute: c(&theme.syntax_attribute),
                namespace: c(&theme.syntax_namespace),
                punctuation: c(&theme.syntax_punctuation),
                escape: c(&theme.syntax_escape),
                tag: c(&theme.syntax_tag),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_valid() {
        assert_eq!(parse_hex("#1e1e2e"), Some((0x1e, 0x1e, 0x2e)));
        assert_eq!(parse_hex("ff0000"), Some((255, 0, 0)));
    }

    #[test]
    fn parse_hex_invalid() {
        assert_eq!(parse_hex("#zzzzzz"), None);
        assert_eq!(parse_hex("#fff"), None);
    }

    #[test]
    fn resolve_truecolor() {
        let c = resolve_color("#1e1e2e", ColorCapability::TrueColor);
        assert_eq!(c, Color::Rgb(0x1e, 0x1e, 0x2e));
    }

    #[test]
    fn resolve_256color() {
        let c = resolve_color("#ff0000", ColorCapability::Color256);
        // Pure red should map to index 196 (5,0,0 in the 6x6x6 cube)
        assert_eq!(c, Color::Indexed(196));
    }

    #[test]
    fn dark_theme_roundtrip() {
        let theme = Theme::dark();
        let colors = ThemeColors::from_theme(&theme, ColorCapability::TrueColor);
        assert_eq!(colors.background, Color::Rgb(0x1e, 0x1e, 0x2e));
    }
}
