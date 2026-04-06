//! Keybinding data model: key chords, sequences, modes, and the keymap.
//!
//! All types are framework-agnostic — no dependency on crossterm or ratatui.
//! Conversion from terminal-specific types happens in `omni-term`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use smallvec::SmallVec;

// ── Modifiers ───────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Keyboard modifier flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const CTRL  = 0b0001;
        const SHIFT = 0b0010;
        const ALT   = 0b0100;
        const SUPER = 0b1000;
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut write_mod = |name: &str| -> fmt::Result {
            if !first {
                f.write_str("+")?;
            }
            first = false;
            f.write_str(name)
        };
        if self.contains(Self::CTRL) {
            write_mod("Ctrl")?;
        }
        if self.contains(Self::SHIFT) {
            write_mod("Shift")?;
        }
        if self.contains(Self::ALT) {
            write_mod("Alt")?;
        }
        if self.contains(Self::SUPER) {
            write_mod("Super")?;
        }
        Ok(())
    }
}

// ── KeyCodeRepr ─────────────────────────────────────────────────────

/// Framework-agnostic key code representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyCodeRepr {
    Char(char),
    F(u8),
    Enter,
    Esc,
    Backspace,
    Tab,
    BackTab,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Insert,
    Space,
}

impl fmt::Display for KeyCodeRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{}", c.to_uppercase()),
            Self::F(n) => write!(f, "F{n}"),
            Self::Enter => f.write_str("Enter"),
            Self::Esc => f.write_str("Esc"),
            Self::Backspace => f.write_str("Backspace"),
            Self::Tab => f.write_str("Tab"),
            Self::BackTab => f.write_str("BackTab"),
            Self::Delete => f.write_str("Delete"),
            Self::Home => f.write_str("Home"),
            Self::End => f.write_str("End"),
            Self::PageUp => f.write_str("PageUp"),
            Self::PageDown => f.write_str("PageDown"),
            Self::Up => f.write_str("Up"),
            Self::Down => f.write_str("Down"),
            Self::Left => f.write_str("Left"),
            Self::Right => f.write_str("Right"),
            Self::Insert => f.write_str("Insert"),
            Self::Space => f.write_str("Space"),
        }
    }
}

// ── KeyChord ────────────────────────────────────────────────────────

/// A single key press with modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCodeRepr,
    pub modifiers: Modifiers,
}

impl KeyChord {
    /// Create a chord with no modifiers.
    #[must_use]
    pub const fn key(code: KeyCodeRepr) -> Self {
        Self { code, modifiers: Modifiers::empty() }
    }

    /// Create a Ctrl+key chord.
    #[must_use]
    pub const fn ctrl(code: KeyCodeRepr) -> Self {
        Self { code, modifiers: Modifiers::CTRL }
    }

    /// Create a Ctrl+Shift+key chord.
    #[must_use]
    pub const fn ctrl_shift(code: KeyCodeRepr) -> Self {
        Self { code, modifiers: Modifiers::CTRL.union(Modifiers::SHIFT) }
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}+{}", self.modifiers, self.code)
        }
    }
}

/// Parse a chord string like `"ctrl+k"`, `"ctrl+shift+a"`, `"enter"`.
impl FromStr for KeyChord {
    type Err = KeymapParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('+').collect();
        let mut modifiers = Modifiers::empty();
        let mut key_part = None;

        for part in &parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= Modifiers::CTRL,
                "shift" => modifiers |= Modifiers::SHIFT,
                "alt" => modifiers |= Modifiers::ALT,
                "super" | "meta" | "win" => modifiers |= Modifiers::SUPER,
                other => {
                    if key_part.is_some() {
                        return Err(KeymapParseError::InvalidKey(s.to_string()));
                    }
                    key_part = Some(other.to_string());
                }
            }
        }

        let key_str = key_part.ok_or_else(|| KeymapParseError::InvalidKey(s.to_string()))?;
        let code = parse_key_code(&key_str)?;

        Ok(Self { code, modifiers })
    }
}

fn parse_key_code(s: &str) -> Result<KeyCodeRepr, KeymapParseError> {
    // Check for function keys first
    if let Some(n) = s.strip_prefix('f').or_else(|| s.strip_prefix('F')) {
        if let Ok(num) = n.parse::<u8>() {
            if (1..=24).contains(&num) {
                return Ok(KeyCodeRepr::F(num));
            }
        }
    }

    match s.to_lowercase().as_str() {
        "enter" | "return" | "cr" => Ok(KeyCodeRepr::Enter),
        "esc" | "escape" => Ok(KeyCodeRepr::Esc),
        "backspace" | "bs" => Ok(KeyCodeRepr::Backspace),
        "tab" => Ok(KeyCodeRepr::Tab),
        "backtab" => Ok(KeyCodeRepr::BackTab),
        "delete" | "del" => Ok(KeyCodeRepr::Delete),
        "home" => Ok(KeyCodeRepr::Home),
        "end" => Ok(KeyCodeRepr::End),
        "pageup" | "pgup" => Ok(KeyCodeRepr::PageUp),
        "pagedown" | "pgdn" => Ok(KeyCodeRepr::PageDown),
        "up" => Ok(KeyCodeRepr::Up),
        "down" => Ok(KeyCodeRepr::Down),
        "left" => Ok(KeyCodeRepr::Left),
        "right" => Ok(KeyCodeRepr::Right),
        "insert" | "ins" => Ok(KeyCodeRepr::Insert),
        "space" => Ok(KeyCodeRepr::Space),
        other => {
            let mut chars = other.chars();
            let ch = chars.next().ok_or_else(|| KeymapParseError::InvalidKey(other.to_string()))?;
            if chars.next().is_some() {
                return Err(KeymapParseError::InvalidKey(other.to_string()));
            }
            Ok(KeyCodeRepr::Char(ch))
        }
    }
}

// ── KeySequence ─────────────────────────────────────────────────────

/// A sequence of 1–2 key chords (e.g., `Ctrl+K Ctrl+C`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence(pub SmallVec<[KeyChord; 2]>);

impl KeySequence {
    /// Create a single-chord sequence.
    #[must_use]
    pub fn from_single(chord: KeyChord) -> Self {
        Self(SmallVec::from_elem(chord, 1))
    }

    /// Create a two-chord sequence.
    #[must_use]
    pub fn from_pair(first: KeyChord, second: KeyChord) -> Self {
        let mut v = SmallVec::new();
        v.push(first);
        v.push(second);
        Self(v)
    }

    /// Number of chords in the sequence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the sequence is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, chord) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{chord}")?;
        }
        Ok(())
    }
}

/// Parse a sequence string like `"ctrl+k ctrl+c"` (space-separated chords).
impl FromStr for KeySequence {
    type Err = KeymapParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() || parts.len() > 2 {
            return Err(KeymapParseError::InvalidSequence(s.to_string()));
        }
        let mut chords = SmallVec::new();
        for part in parts {
            chords.push(part.parse()?);
        }
        Ok(Self(chords))
    }
}

// ── KeymapMode ──────────────────────────────────────────────────────

/// An editing mode that determines which keymap layer is active.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum KeymapMode {
    /// VS Code-style modeless editing (the default).
    #[default]
    Normal,
    // Future vim support:
    // VimNormal,
    // VimInsert,
    // VimVisual,
    // VimCommand,
}

impl FromStr for KeymapMode {
    type Err = KeymapParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "normal" | "" => Ok(Self::Normal),
            _ => Err(KeymapParseError::InvalidMode(s.to_string())),
        }
    }
}

// ── KeymapResult ────────────────────────────────────────────────────

/// The result of looking up a key sequence in the keymap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapResult {
    /// The sequence matched a binding — here is the action name.
    Matched(String),
    /// The sequence is a valid prefix of a multi-key binding — wait for more input.
    Pending,
    /// No binding matches this sequence.
    NotFound,
}

// ── Keymap ──────────────────────────────────────────────────────────

/// Maps `(mode, key sequence)` → action name string.
///
/// Action names are strings (e.g., `"quit"`, `"toggle_sidebar"`) because
/// omni-core does not depend on omni-event. The caller resolves names to
/// `Action` enum values.
#[derive(Debug, Default)]
pub struct Keymap {
    /// Per-mode binding tables.
    bindings: HashMap<KeymapMode, HashMap<KeySequence, String>>,
    /// Per-mode prefix sets: first chords of multi-key sequences.
    prefixes: HashMap<KeymapMode, HashSet<KeyChord>>,
}

impl Keymap {
    /// Create an empty keymap.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding. Automatically updates the prefix set for multi-key sequences.
    pub fn bind(&mut self, mode: KeymapMode, seq: KeySequence, action: impl Into<String>) {
        if seq.len() > 1 {
            // Register the first chord as a prefix
            self.prefixes
                .entry(mode)
                .or_default()
                .insert(seq.0[0].clone());
        }
        self.bindings
            .entry(mode)
            .or_default()
            .insert(seq, action.into());
    }

    /// Remove a binding.
    pub fn unbind(&mut self, mode: KeymapMode, seq: &KeySequence) {
        if let Some(bindings) = self.bindings.get_mut(&mode) {
            bindings.remove(seq);
        }
        // Note: we don't clean up prefixes because other bindings may
        // still use the same first chord.
    }

    /// Look up a key sequence in the given mode.
    #[must_use]
    pub fn lookup(&self, mode: KeymapMode, seq: &KeySequence) -> KeymapResult {
        // Check for exact match
        if let Some(bindings) = self.bindings.get(&mode) {
            if let Some(action) = bindings.get(seq) {
                return KeymapResult::Matched(action.clone());
            }
        }

        // Check if this is a prefix (single chord that starts a multi-key sequence)
        if seq.len() == 1 {
            if let Some(prefixes) = self.prefixes.get(&mode) {
                if prefixes.contains(&seq.0[0]) {
                    return KeymapResult::Pending;
                }
            }
        }

        KeymapResult::NotFound
    }

    /// Check if a single chord is the first key of any multi-key sequence.
    #[must_use]
    pub fn is_prefix(&self, mode: KeymapMode, chord: &KeyChord) -> bool {
        self.prefixes
            .get(&mode)
            .is_some_and(|p| p.contains(chord))
    }

    /// Merge another keymap on top. Bindings from `other` override `self`.
    /// Empty action strings in `other` unbind the key.
    pub fn merge(&mut self, other: &Self) {
        for (mode, bindings) in &other.bindings {
            for (seq, action) in bindings {
                if action.is_empty() {
                    self.unbind(*mode, seq);
                } else {
                    self.bind(*mode, seq.clone(), action.clone());
                }
            }
        }
    }

    /// Get the display string for an action in the given mode (for UI).
    /// Returns the first matching key sequence, if any.
    #[must_use]
    pub fn display_for_action(&self, mode: KeymapMode, action_name: &str) -> Option<String> {
        self.bindings
            .get(&mode)?
            .iter()
            .find(|(_, a)| a.as_str() == action_name)
            .map(|(seq, _)| seq.to_string())
    }
}

// ── Errors ──────────────────────────────────────────────────────────

/// Errors from parsing key strings.
#[derive(Debug, Clone, thiserror::Error)]
pub enum KeymapParseError {
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("invalid key sequence: {0}")]
    InvalidSequence(String),
    #[error("invalid mode: {0}")]
    InvalidMode(String),
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_chord() {
        let chord: KeyChord = "ctrl+s".parse().unwrap();
        assert_eq!(chord.modifiers, Modifiers::CTRL);
        assert_eq!(chord.code, KeyCodeRepr::Char('s'));
    }

    #[test]
    fn parse_ctrl_shift_chord() {
        let chord: KeyChord = "ctrl+shift+a".parse().unwrap();
        assert_eq!(chord.modifiers, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(chord.code, KeyCodeRepr::Char('a'));
    }

    #[test]
    fn parse_special_key() {
        let chord: KeyChord = "ctrl+tab".parse().unwrap();
        assert_eq!(chord.code, KeyCodeRepr::Tab);
        assert!(chord.modifiers.contains(Modifiers::CTRL));
    }

    #[test]
    fn parse_function_key() {
        let chord: KeyChord = "f5".parse().unwrap();
        assert_eq!(chord.code, KeyCodeRepr::F(5));
        assert!(chord.modifiers.is_empty());
    }

    #[test]
    fn parse_sequence_single() {
        let seq: KeySequence = "ctrl+s".parse().unwrap();
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn parse_sequence_multi() {
        let seq: KeySequence = "ctrl+k ctrl+c".parse().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq.0[0].code, KeyCodeRepr::Char('k'));
        assert_eq!(seq.0[1].code, KeyCodeRepr::Char('c'));
    }

    #[test]
    fn display_roundtrip() {
        let chord = KeyChord::ctrl(KeyCodeRepr::Char('s'));
        let s = chord.to_string();
        assert_eq!(s, "Ctrl+S");
    }

    #[test]
    fn keymap_lookup_single() {
        let mut km = Keymap::new();
        let seq: KeySequence = "ctrl+s".parse().unwrap();
        km.bind(KeymapMode::Normal, seq.clone(), "save");
        assert_eq!(km.lookup(KeymapMode::Normal, &seq), KeymapResult::Matched("save".into()));
    }

    #[test]
    fn keymap_lookup_multi_chord() {
        let mut km = Keymap::new();
        let seq: KeySequence = "ctrl+k ctrl+c".parse().unwrap();
        km.bind(KeymapMode::Normal, seq.clone(), "comment");

        // The full sequence matches
        assert_eq!(
            km.lookup(KeymapMode::Normal, &seq),
            KeymapResult::Matched("comment".into())
        );

        // The first chord alone is Pending
        let first_only = KeySequence::from_single(seq.0[0].clone());
        assert_eq!(km.lookup(KeymapMode::Normal, &first_only), KeymapResult::Pending);
    }

    #[test]
    fn keymap_not_found() {
        let km = Keymap::new();
        let seq: KeySequence = "ctrl+x".parse().unwrap();
        assert_eq!(km.lookup(KeymapMode::Normal, &seq), KeymapResult::NotFound);
    }

    #[test]
    fn keymap_merge_override() {
        let mut base = Keymap::new();
        base.bind(KeymapMode::Normal, "ctrl+s".parse().unwrap(), "save");
        base.bind(KeymapMode::Normal, "ctrl+b".parse().unwrap(), "bold");

        let mut user = Keymap::new();
        user.bind(KeymapMode::Normal, "ctrl+b".parse().unwrap(), "sidebar");

        base.merge(&user);

        let seq_b: KeySequence = "ctrl+b".parse().unwrap();
        assert_eq!(
            base.lookup(KeymapMode::Normal, &seq_b),
            KeymapResult::Matched("sidebar".into())
        );

        // ctrl+s unchanged
        let seq_s: KeySequence = "ctrl+s".parse().unwrap();
        assert_eq!(
            base.lookup(KeymapMode::Normal, &seq_s),
            KeymapResult::Matched("save".into())
        );
    }

    #[test]
    fn keymap_merge_unbind() {
        let mut base = Keymap::new();
        base.bind(KeymapMode::Normal, "ctrl+b".parse().unwrap(), "toggle_sidebar");

        let mut user = Keymap::new();
        // Empty action = unbind
        user.bind(KeymapMode::Normal, "ctrl+b".parse().unwrap(), "");

        base.merge(&user);

        let seq: KeySequence = "ctrl+b".parse().unwrap();
        assert_eq!(base.lookup(KeymapMode::Normal, &seq), KeymapResult::NotFound);
    }

    #[test]
    fn display_for_action() {
        let mut km = Keymap::new();
        km.bind(KeymapMode::Normal, "ctrl+s".parse().unwrap(), "save");
        let display = km.display_for_action(KeymapMode::Normal, "save");
        assert!(display.is_some());
    }
}
