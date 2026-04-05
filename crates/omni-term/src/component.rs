//! Component trait — the contract for all TUI elements.

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::context::Context;

/// Result of handling an event.
pub enum EventResult {
    /// The event was consumed — stop propagation.
    Consumed,
    /// The event was ignored — propagate to the next layer.
    Ignored,
    /// The event produced an action to dispatch globally.
    Action(omni_event::Action),
    /// The event produced a callback to run against the compositor.
    ///
    /// This is useful for pushing/popping layers (e.g., opening a popup)
    /// without the component needing a reference to the compositor.
    Callback(Box<dyn FnOnce(&mut crate::Compositor) + Send>),
}

/// Cursor shape variants for the terminal cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    /// Standard blinking block cursor.
    Block,
    /// Thin vertical bar cursor (insert mode).
    Bar,
    /// Underline cursor.
    Underline,
    /// Hide the cursor entirely.
    Hidden,
}

/// A renderable, interactive UI component.
///
/// Components form the building blocks of the TUI. They are stacked
/// in the [`crate::Compositor`] and receive events front-to-back
/// (topmost first), while rendering happens back-to-front.
pub trait Component: Send {
    /// Initialize the component with its allocated area.
    ///
    /// Called once when the component is first pushed onto the compositor,
    /// and again on terminal resize. Default implementation does nothing.
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    fn init(&mut self, _area: Rect) -> color_eyre::Result<()> {
        Ok(())
    }

    /// Handle a key event.
    ///
    /// Return [`EventResult::Consumed`] to stop propagation to layers below.
    ///
    /// # Errors
    /// Returns an error if event handling fails.
    fn handle_key(
        &mut self,
        _event: KeyEvent,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        Ok(EventResult::Ignored)
    }

    /// Handle a mouse event.
    ///
    /// The `area` parameter is the region this component occupies, so the
    /// component can determine if the click is within its bounds.
    ///
    /// # Errors
    /// Returns an error if event handling fails.
    fn handle_mouse(
        &mut self,
        _event: MouseEvent,
        _area: Rect,
        _ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        Ok(EventResult::Ignored)
    }

    /// Render the component into the given frame area.
    ///
    /// Takes `&mut self` so components can update cached layout state
    /// during rendering (e.g., scroll position adjustments).
    fn render(&mut self, frame: &mut Frame, area: Rect);

    /// Return the cursor position and shape, if this component wants to show one.
    ///
    /// The coordinates are absolute (frame-relative), not area-relative.
    fn cursor(&self) -> Option<(u16, u16, CursorKind)> {
        None
    }

    /// Whether this component can receive focus.
    ///
    /// Non-focusable components (e.g., status bars) still render but
    /// never receive key events.
    fn focusable(&self) -> bool {
        false
    }
}
