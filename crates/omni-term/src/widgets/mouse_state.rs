//! Mouse state tracking for double-click detection.

use std::time::Instant;

use crossterm::event::MouseButton;

/// Threshold for double-click detection in milliseconds.
const DOUBLE_CLICK_MS: u128 = 300;

/// Maximum drift in cells between clicks to count as multi-click.
const MULTI_CLICK_MAX_DRIFT: u16 = 2;

/// Maximum click count tracked (triple-click for line selection).
const MAX_CLICK_COUNT: u8 = 3;

/// Tracks mouse interaction state across events for multi-click detection.
#[derive(Debug)]
pub struct MouseState {
    last_click_time: Option<Instant>,
    last_click_pos: (u16, u16),
    last_click_button: MouseButton,
    click_count: u8,
}

impl MouseState {
    /// Create a new mouse state tracker.
    pub const fn new() -> Self {
        Self {
            last_click_time: None,
            last_click_pos: (0, 0),
            last_click_button: MouseButton::Left,
            click_count: 0,
        }
    }

    /// Record a mouse-down event and return the resolved click count.
    ///
    /// Returns 1 for single-click, 2 for double-click, 3 for triple-click.
    pub fn record_click(&mut self, button: MouseButton, col: u16, row: u16) -> u8 {
        let now = Instant::now();

        let is_continuation = self.last_click_time.is_some_and(|t| {
            let elapsed = now.duration_since(t).as_millis();
            let drift = col.abs_diff(self.last_click_pos.0) + row.abs_diff(self.last_click_pos.1);
            elapsed <= DOUBLE_CLICK_MS
                && drift <= MULTI_CLICK_MAX_DRIFT
                && button == self.last_click_button
        });

        self.click_count =
            if is_continuation { (self.click_count + 1).min(MAX_CLICK_COUNT) } else { 1 };

        self.last_click_time = Some(now);
        self.last_click_pos = (col, row);
        self.last_click_button = button;

        self.click_count
    }

    /// The current click count from the last recorded click.
    #[allow(dead_code)]
    pub const fn click_count(&self) -> u8 {
        self.click_count
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}
