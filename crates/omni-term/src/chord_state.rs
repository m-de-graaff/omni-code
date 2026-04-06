//! Multi-key chord state machine and crossterm key conversion.
//!
//! Accumulates partial key sequences and resolves them against the [`Keymap`].
//! Handles the timeout for incomplete chord sequences (e.g., pressing `Ctrl+K`
//! but not following up within 1 second).

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use omni_core::keymap::{
    KeyChord, KeyCodeRepr, KeySequence, Keymap, KeymapMode, KeymapResult, Modifiers,
};

/// How long to wait for the second key of a multi-key chord.
const CHORD_TIMEOUT: Duration = Duration::from_millis(1000);

/// The outcome of feeding a key chord into the state machine.
#[derive(Debug)]
pub enum ChordOutcome {
    /// A complete binding was matched.
    Matched(String),
    /// Waiting for the next key in a multi-key chord.
    Pending(KeyChord),
    /// No binding found — pass the key through to the compositor.
    PassThrough,
}

/// Tracks the state of an in-progress multi-key chord sequence.
#[derive(Debug, Default)]
pub struct ChordState {
    /// The first chord of a pending multi-key sequence, if any.
    pending: Option<(KeyChord, Instant)>,
}

impl ChordState {
    /// Create a new (idle) chord state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a key chord into the state machine and get the outcome.
    pub fn feed(&mut self, chord: KeyChord, keymap: &Keymap, mode: KeymapMode) -> ChordOutcome {
        // If we have a pending first chord, try the two-chord sequence
        if let Some((first, timestamp)) = self.pending.take() {
            if timestamp.elapsed() < CHORD_TIMEOUT {
                let seq = KeySequence::from_pair(first, chord.clone());
                if let KeymapResult::Matched(action) = keymap.lookup(mode, &seq) {
                    return ChordOutcome::Matched(action);
                }
            }
            // Timeout expired or two-chord sequence didn't match.
            // Fall through to try the second chord as a fresh single-key binding.
        }

        // Try single-chord lookup
        let seq = KeySequence::from_single(chord.clone());
        match keymap.lookup(mode, &seq) {
            KeymapResult::Matched(action) => ChordOutcome::Matched(action),
            KeymapResult::Pending => {
                self.pending = Some((chord.clone(), Instant::now()));
                ChordOutcome::Pending(chord)
            }
            KeymapResult::NotFound => ChordOutcome::PassThrough,
        }
    }

    /// Cancel any pending chord (e.g., on Escape or when a modal consumes a key).
    pub const fn cancel(&mut self) {
        self.pending = None;
    }

    /// Check if a pending chord has timed out. Returns `true` if it was
    /// pending and is now cancelled.
    pub fn check_timeout(&mut self) -> bool {
        if let Some((_, ts)) = &self.pending {
            if ts.elapsed() >= CHORD_TIMEOUT {
                self.pending = None;
                return true;
            }
        }
        false
    }

    /// Whether a chord is pending (for status bar display).
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The pending chord, if any (for status bar display).
    #[must_use]
    pub fn pending_chord(&self) -> Option<&KeyChord> {
        self.pending.as_ref().map(|(chord, _)| chord)
    }
}

// ── Crossterm → KeyChord conversion ─────────────────────────────────

/// Convert a crossterm `KeyEvent` to our framework-agnostic `KeyChord`.
///
/// Returns `None` for key events we don't handle (media keys, etc.).
///
/// Normalizes `Char` to lowercase — the `SHIFT` modifier flag captures
/// the shift state separately.
#[must_use]
pub fn crossterm_to_chord(key: &KeyEvent) -> Option<KeyChord> {
    let code = match key.code {
        KeyCode::Char(' ') => KeyCodeRepr::Space,
        KeyCode::Char(c) => {
            // Normalize to lowercase; SHIFT is tracked via modifiers
            KeyCodeRepr::Char(c.to_ascii_lowercase())
        }
        KeyCode::F(n) => KeyCodeRepr::F(n),
        KeyCode::Enter => KeyCodeRepr::Enter,
        KeyCode::Esc => KeyCodeRepr::Esc,
        KeyCode::Backspace => KeyCodeRepr::Backspace,
        KeyCode::Tab => KeyCodeRepr::Tab,
        KeyCode::BackTab => KeyCodeRepr::BackTab,
        KeyCode::Delete => KeyCodeRepr::Delete,
        KeyCode::Home => KeyCodeRepr::Home,
        KeyCode::End => KeyCodeRepr::End,
        KeyCode::PageUp => KeyCodeRepr::PageUp,
        KeyCode::PageDown => KeyCodeRepr::PageDown,
        KeyCode::Up => KeyCodeRepr::Up,
        KeyCode::Down => KeyCodeRepr::Down,
        KeyCode::Left => KeyCodeRepr::Left,
        KeyCode::Right => KeyCodeRepr::Right,
        KeyCode::Insert => KeyCodeRepr::Insert,
        _ => return None,
    };

    let mut modifiers = Modifiers::empty();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        modifiers |= Modifiers::CTRL;
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        modifiers |= Modifiers::SHIFT;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        modifiers |= Modifiers::ALT;
    }
    if key.modifiers.contains(KeyModifiers::SUPER) {
        modifiers |= Modifiers::SUPER;
    }

    // BackTab already implies Shift — don't double-count
    if matches!(code, KeyCodeRepr::BackTab) {
        modifiers.remove(Modifiers::SHIFT);
    }

    Some(KeyChord { code, modifiers })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    #[test]
    fn convert_ctrl_s() {
        let key = make_key(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let chord = crossterm_to_chord(&key).unwrap();
        assert_eq!(chord.code, KeyCodeRepr::Char('s'));
        assert_eq!(chord.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn convert_ctrl_shift_a() {
        // Some terminals send uppercase char with SHIFT
        let key = make_key(KeyCode::Char('A'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let chord = crossterm_to_chord(&key).unwrap();
        assert_eq!(chord.code, KeyCodeRepr::Char('a')); // normalized to lowercase
        assert!(chord.modifiers.contains(Modifiers::CTRL));
        assert!(chord.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn convert_ctrl_tab() {
        let key = make_key(KeyCode::Tab, KeyModifiers::CONTROL);
        let chord = crossterm_to_chord(&key).unwrap();
        assert_eq!(chord.code, KeyCodeRepr::Tab);
        assert_eq!(chord.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn convert_backtab_strips_shift() {
        // Ctrl+Shift+Tab arrives as BackTab with CONTROL|SHIFT
        let key = make_key(KeyCode::BackTab, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let chord = crossterm_to_chord(&key).unwrap();
        assert_eq!(chord.code, KeyCodeRepr::BackTab);
        // SHIFT should be stripped since BackTab already implies it
        assert_eq!(chord.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn chord_state_single_key_match() {
        let mut state = ChordState::new();
        let mut km = Keymap::new();
        km.bind(KeymapMode::Normal, "ctrl+s".parse().unwrap(), "save");

        let chord: KeyChord = "ctrl+s".parse().unwrap();
        match state.feed(chord, &km, KeymapMode::Normal) {
            ChordOutcome::Matched(action) => assert_eq!(action, "save"),
            other => panic!("expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn chord_state_multi_key_pending_then_match() {
        let mut state = ChordState::new();
        let mut km = Keymap::new();
        km.bind(
            KeymapMode::Normal,
            "ctrl+k ctrl+c".parse().unwrap(),
            "comment",
        );

        // First key → Pending
        let first: KeyChord = "ctrl+k".parse().unwrap();
        match state.feed(first, &km, KeymapMode::Normal) {
            ChordOutcome::Pending(_) => {} // expected
            other => panic!("expected Pending, got {other:?}"),
        }

        // Second key → Matched
        let second: KeyChord = "ctrl+c".parse().unwrap();
        match state.feed(second, &km, KeymapMode::Normal) {
            ChordOutcome::Matched(action) => assert_eq!(action, "comment"),
            other => panic!("expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn chord_state_passthrough() {
        let state_machine = &mut ChordState::new();
        let km = Keymap::new(); // empty keymap

        let chord: KeyChord = "ctrl+x".parse().unwrap();
        assert!(matches!(
            state_machine.feed(chord, &km, KeymapMode::Normal),
            ChordOutcome::PassThrough
        ));
    }
}
