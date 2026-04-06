//! Cursor position history for back/forward navigation.

use omni_core::DocumentId;

const MAX_HISTORY: usize = 100;

/// A navigation entry: a position in a document.
#[derive(Debug, Clone, Copy)]
pub struct NavEntry {
    pub doc_id: DocumentId,
    pub char_pos: usize,
}

/// Stack-based navigation history with back/forward.
#[derive(Debug, Default)]
pub struct NavigationHistory {
    back: Vec<NavEntry>,
    forward: Vec<NavEntry>,
}

impl NavigationHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new navigation point. Clears the forward stack.
    /// Skips if the position is close to the last entry (same doc, within 10 lines).
    pub fn push(&mut self, entry: NavEntry) {
        // Dedup: skip if same doc and close position
        if let Some(last) = self.back.last() {
            if last.doc_id == entry.doc_id && last.char_pos.abs_diff(entry.char_pos) < 200 {
                return;
            }
        }
        self.back.push(entry);
        if self.back.len() > MAX_HISTORY {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// Go back: push current to forward, pop from back.
    pub fn go_back(&mut self, current: NavEntry) -> Option<NavEntry> {
        let target = self.back.pop()?;
        self.forward.push(current);
        Some(target)
    }

    /// Go forward: push current to back, pop from forward.
    pub fn go_forward(&mut self, current: NavEntry) -> Option<NavEntry> {
        let target = self.forward.pop()?;
        self.back.push(current);
        Some(target)
    }
}
