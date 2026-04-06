//! Load and merge keybinding configurations.
//!
//! Default keybindings are compiled into the binary. User overrides are loaded
//! from `{config_dir}/keybindings.toml` and merged on top.

use std::path::Path;

use omni_core::keymap::{Keymap, KeymapMode, KeySequence};
use omni_event::Action;
use serde::Deserialize;

/// Errors from keybinding loading.
#[derive(Debug, thiserror::Error)]
pub enum KeymapError {
    #[error("failed to read keybindings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse keybindings TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid key sequence in keybindings: {0}")]
    Parse(String),
}

// ── TOML schema ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct KeybindingsFile {
    #[serde(default)]
    bind: Vec<BindingEntry>,
}

#[derive(Debug, Deserialize)]
struct BindingEntry {
    key: String,
    action: String,
    #[serde(default)]
    mode: Option<String>,
}

// ── Default keybindings ─────────────────────────────────────────────

/// The compiled-in default keybindings (VS Code-style).
const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    // Application
    ("ctrl+q", "quit"),
    ("ctrl+s", "save"),
    ("ctrl+b", "toggle_sidebar"),
    ("ctrl+j", "toggle_bottom_panel"),
    ("ctrl+tab", "focus_next"),
    ("ctrl+shift+tab", "focus_prev"),
    ("ctrl+o", "open_folder"),
    ("ctrl+p", "command_palette"),
    ("ctrl+n", "new_file"),
    ("ctrl+shift+a", "toggle_app_mode"),
    ("ctrl+w", "close_buffer"),
    // Undo/Redo
    ("ctrl+z", "undo"),
    ("ctrl+shift+z", "redo"),
    // Cursor movement
    ("left", "cursor_left"),
    ("right", "cursor_right"),
    ("up", "cursor_up"),
    ("down", "cursor_down"),
    ("ctrl+left", "cursor_word_left"),
    ("ctrl+right", "cursor_word_right"),
    ("home", "cursor_line_start"),
    ("end", "cursor_line_end"),
    ("ctrl+home", "cursor_doc_start"),
    ("ctrl+end", "cursor_doc_end"),
    // Selection
    ("shift+left", "select_left"),
    ("shift+right", "select_right"),
    ("shift+up", "select_up"),
    ("shift+down", "select_down"),
    ("ctrl+shift+left", "select_word_left"),
    ("ctrl+shift+right", "select_word_right"),
    ("shift+home", "select_line_start"),
    ("shift+end", "select_line_end"),
    ("ctrl+a", "select_all"),
    ("ctrl+d", "select_word"),
    ("ctrl+l", "select_line"),
    // Text editing
    ("backspace", "backspace"),
    ("delete", "delete"),
    ("ctrl+backspace", "delete_word_backward"),
    ("ctrl+delete", "delete_word_forward"),
    ("enter", "insert_newline"),
    ("tab", "insert_tab"),
    ("shift+tab", "outdent_selection"),
    // Line operations
    ("ctrl+shift+d", "duplicate_line"),
    ("alt+up", "move_line_up"),
    ("alt+down", "move_line_down"),
    ("ctrl+/", "toggle_comment"),
    // Clipboard
    ("ctrl+x", "cut"),
    ("ctrl+c", "copy"),
    ("ctrl+v", "paste"),
    // View / splits
    ("ctrl+shift+m", "toggle_minimap"),
    ("ctrl+\\", "vertical_split"),
    ("ctrl+-", "horizontal_split"),
    // Navigation
    ("ctrl+g", "goto_line"),
    ("ctrl+shift+o", "goto_symbol"),
    // Search
    ("ctrl+f", "find"),
    ("f3", "find_next"),
    ("shift+f3", "find_prev"),
    ("ctrl+h", "find_replace"),
    // Scroll
    ("pageup", "page_up"),
    ("pagedown", "page_down"),
    // Chord bindings
    ("ctrl+k ctrl+c", "toggle_line_comment"),
    ("ctrl+k ctrl+u", "uncomment"),
];

/// Build the default keymap (compiled-in, no file dependency).
#[must_use]
#[allow(clippy::missing_panics_doc)] // default bindings are validated at compile time
pub fn default_keymap() -> Keymap {
    let mut keymap = Keymap::new();
    for &(key_str, action) in DEFAULT_BINDINGS {
        let seq: KeySequence = key_str
            .parse()
            .unwrap_or_else(|_| panic!("invalid default binding: {key_str}"));
        keymap.bind(KeymapMode::Normal, seq, action);
    }
    keymap
}

// ── Loading ─────────────────────────────────────────────────────────

/// Load user keybindings from `{config_dir}/keybindings.toml`.
///
/// Returns `Ok(None)` if the file doesn't exist (not an error).
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load_user_keymap() -> Result<Option<Keymap>, KeymapError> {
    let Ok(config_dir) = crate::paths::config_dir() else {
        return Ok(None);
    };
    let path = config_dir.join("keybindings.toml");
    load_keymap_from_file(&path)
}

/// Load a keymap from a specific TOML file.
///
/// Returns `Ok(None)` if the file doesn't exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load_keymap_from_file(path: &Path) -> Result<Option<Keymap>, KeymapError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let file: KeybindingsFile = toml::from_str(&content)?;

    let mut keymap = Keymap::new();
    for entry in &file.bind {
        let seq: KeySequence = entry
            .key
            .parse()
            .map_err(|e| KeymapError::Parse(format!("{}: {e}", entry.key)))?;

        let mode = entry
            .mode
            .as_deref()
            .unwrap_or("normal")
            .parse()
            .map_err(|e| KeymapError::Parse(format!("{e}")))?;

        keymap.bind(mode, seq, &entry.action);
    }

    Ok(Some(keymap))
}

/// Build the final merged keymap: defaults + user overrides.
///
/// # Errors
///
/// Returns an error if the user keybindings file exists but cannot be parsed.
pub fn load_keymap() -> Result<Keymap, KeymapError> {
    let mut keymap = default_keymap();

    match load_user_keymap() {
        Ok(Some(user)) => {
            tracing::info!("loaded user keybindings");
            keymap.merge(&user);
        }
        Ok(None) => {
            tracing::debug!("no user keybindings file found, using defaults");
        }
        Err(e) => {
            tracing::warn!(?e, "failed to load user keybindings, using defaults");
        }
    }

    Ok(keymap)
}

// ── Action resolution ───────────────────────────────────────────────

/// Resolve an action name string to an [`Action`] enum value.
///
/// Returns `None` for unknown action names.
#[must_use]
pub fn resolve_action(name: &str) -> Option<Action> {
    match name {
        "quit" => Some(Action::Quit),
        "save" => Some(Action::Save),
        "close_buffer" => Some(Action::CloseBuffer),
        "toggle_sidebar" => Some(Action::ToggleSidebar),
        "toggle_bottom_panel" => Some(Action::ToggleBottomPanel),
        "toggle_minimap" => Some(Action::ToggleMinimap),
        "toggle_app_mode" => Some(Action::ToggleAppMode),
        "focus_next" => Some(Action::FocusNext),
        "focus_prev" => Some(Action::FocusPrev),
        "vertical_split" => Some(Action::VerticalSplit),
        "horizontal_split" => Some(Action::HorizontalSplit),
        "command_palette" => Some(Action::CommandPalette),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "select_all" => Some(Action::SelectAll),
        "select_next_occurrence" => Some(Action::SelectNextOccurrence),
        "expand_selection" => Some(Action::ExpandSelection),
        "shrink_selection" => Some(Action::ShrinkSelection),
        "file_picker" => Some(Action::FilePicker),
        "scroll_up" => Some(Action::ScrollUp),
        "scroll_down" => Some(Action::ScrollDown),
        "page_up" => Some(Action::PageUp),
        "page_down" => Some(Action::PageDown),
        // Search
        "find" => Some(Action::Find),
        "find_next" => Some(Action::FindNext),
        "find_prev" => Some(Action::FindPrev),
        "find_replace" => Some(Action::FindReplace),
        "replace_one" => Some(Action::ReplaceOne),
        "replace_all" => Some(Action::ReplaceAll),
        "project_search" => Some(Action::ProjectSearch),
        "goto_line" => Some(Action::GotoLine),
        "goto_symbol" => Some(Action::GotoSymbol),
        // Cursor movement
        "cursor_left" => Some(Action::CursorLeft),
        "cursor_right" => Some(Action::CursorRight),
        "cursor_up" => Some(Action::CursorUp),
        "cursor_down" => Some(Action::CursorDown),
        "cursor_word_left" => Some(Action::CursorWordLeft),
        "cursor_word_right" => Some(Action::CursorWordRight),
        "cursor_line_start" => Some(Action::CursorLineStart),
        "cursor_line_end" => Some(Action::CursorLineEnd),
        "cursor_doc_start" => Some(Action::CursorDocStart),
        "cursor_doc_end" => Some(Action::CursorDocEnd),
        // Selection
        "select_left" => Some(Action::SelectLeft),
        "select_right" => Some(Action::SelectRight),
        "select_up" => Some(Action::SelectUp),
        "select_down" => Some(Action::SelectDown),
        "select_word_left" => Some(Action::SelectWordLeft),
        "select_word_right" => Some(Action::SelectWordRight),
        "select_line_start" => Some(Action::SelectLineStart),
        "select_line_end" => Some(Action::SelectLineEnd),
        "select_word" => Some(Action::SelectWord),
        "select_line" => Some(Action::SelectLine),
        // Text editing
        "backspace" => Some(Action::Backspace),
        "delete" => Some(Action::Delete),
        "delete_word_backward" => Some(Action::DeleteWordBackward),
        "delete_word_forward" => Some(Action::DeleteWordForward),
        "insert_newline" => Some(Action::InsertNewline),
        "insert_tab" => Some(Action::InsertTab),
        "indent_selection" => Some(Action::IndentSelection),
        "outdent_selection" => Some(Action::OutdentSelection),
        "duplicate_line" => Some(Action::DuplicateLine),
        "move_line_up" => Some(Action::MoveLineUp),
        "move_line_down" => Some(Action::MoveLineDown),
        "toggle_comment" => Some(Action::ToggleComment),
        "cut" => Some(Action::Cut),
        "copy" => Some(Action::Copy),
        "paste" => Some(Action::Paste),
        // Route through Command for extensible actions
        "new_file" | "open_folder" | "toggle_line_comment" | "uncomment" => {
            Some(Action::Command(name.to_string()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_has_expected_bindings() {
        let km = default_keymap();
        let seq: KeySequence = "ctrl+s".parse().unwrap();
        assert_eq!(
            km.lookup(KeymapMode::Normal, &seq),
            omni_core::KeymapResult::Matched("save".into()),
        );
    }

    #[test]
    fn default_keymap_has_chord() {
        let km = default_keymap();
        let seq: KeySequence = "ctrl+k ctrl+c".parse().unwrap();
        assert_eq!(
            km.lookup(KeymapMode::Normal, &seq),
            omni_core::KeymapResult::Matched("toggle_line_comment".into()),
        );
    }

    #[test]
    fn resolve_known_actions() {
        assert_eq!(resolve_action("quit"), Some(Action::Quit));
        assert_eq!(resolve_action("save"), Some(Action::Save));
        assert_eq!(resolve_action("undo"), Some(Action::Undo));
        assert_eq!(resolve_action("new_file"), Some(Action::Command("new_file".into())));
    }

    #[test]
    fn resolve_unknown_action() {
        assert_eq!(resolve_action("nonexistent"), None);
    }

    #[test]
    fn load_from_nonexistent_file() {
        let result = load_keymap_from_file(Path::new("/nonexistent/keybindings.toml"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
