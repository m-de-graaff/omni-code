//! Compositor — a layered stack of components (Helix pattern).
//!
//! Events propagate front-to-back (topmost popup first).
//! Rendering goes back-to-front (base layer first, popups on top).

use crossterm::event::{Event, KeyEvent, MouseEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::Component;
use crate::component::{CursorKind, EventResult};
use crate::context::Context;

/// Manages a stack of UI component layers.
///
/// The last element in the `Vec` is the topmost layer (receives events first,
/// renders last / on top).
pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    area: Rect,
    needs_redraw: bool,
}

impl Compositor {
    /// Create an empty compositor.
    #[must_use]
    pub fn new() -> Self {
        Self { layers: Vec::new(), area: Rect::default(), needs_redraw: true }
    }

    /// The current terminal area.
    #[must_use]
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// Whether the compositor needs to be redrawn.
    #[must_use]
    pub const fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    /// Reset the redraw flag after rendering.
    pub const fn mark_redrawn(&mut self) {
        self.needs_redraw = false;
    }

    /// Push a component layer on top and initialize it.
    ///
    /// # Errors
    /// Returns an error if the component's `init` fails.
    pub fn push(&mut self, mut component: Box<dyn Component>) -> color_eyre::Result<()> {
        component.init(self.area)?;
        self.layers.push(component);
        self.needs_redraw = true;
        Ok(())
    }

    /// Pop the topmost layer.
    pub fn pop(&mut self) -> Option<Box<dyn Component>> {
        let layer = self.layers.pop();
        if layer.is_some() {
            self.needs_redraw = true;
        }
        layer
    }

    /// Number of layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Whether the compositor has no layers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Notify all layers of a terminal resize and update the area.
    ///
    /// # Errors
    /// Returns an error if any component's `init` fails during resize.
    pub fn resize(&mut self, area: Rect) -> color_eyre::Result<()> {
        self.area = area;
        for layer in &mut self.layers {
            layer.init(area)?;
        }
        self.needs_redraw = true;
        Ok(())
    }

    /// Handle a terminal event by dispatching to layers front-to-back.
    ///
    /// Returns any [`EventResult`] that requires action by the event loop
    /// (quit, global action, or callback).
    ///
    /// # Errors
    /// Returns an error if a component's event handler fails.
    pub fn handle_event(
        &mut self,
        event: &Event,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        let result = match event {
            Event::Key(key_event) => self.handle_key(*key_event, ctx)?,
            Event::Mouse(mouse_event) => self.handle_mouse(*mouse_event, ctx)?,
            Event::Paste(text) => self.handle_paste(text, ctx)?,
            Event::Resize(w, h) => {
                let area = Rect::new(0, 0, *w, *h);
                self.resize(area)?;
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        };

        if !matches!(result, EventResult::Ignored) {
            self.needs_redraw = true;
        }

        Ok(result)
    }

    /// Render all layers back-to-front, then set cursor from the topmost
    /// focused component.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.area = area;

        // Render back-to-front (base layer first)
        for layer in &mut self.layers {
            layer.render(frame, area);
        }

        // Cursor from the topmost component that provides one
        for layer in self.layers.iter().rev() {
            if let Some((x, y, kind)) = layer.cursor() {
                if kind != CursorKind::Hidden {
                    frame.set_cursor_position((x, y));
                }
                break;
            }
        }
    }

    /// Dispatch a key event front-to-back.
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut Context) -> color_eyre::Result<EventResult> {
        for layer in self.layers.iter_mut().rev() {
            let result = layer.handle_key(key, ctx)?;
            if !matches!(result, EventResult::Ignored) {
                return Ok(result);
            }
        }
        Ok(EventResult::Ignored)
    }

    /// Dispatch a mouse event front-to-back.
    fn handle_mouse(
        &mut self,
        mouse: MouseEvent,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        let area = self.area;
        for layer in self.layers.iter_mut().rev() {
            let result = layer.handle_mouse(mouse, area, ctx)?;
            if !matches!(result, EventResult::Ignored) {
                return Ok(result);
            }
        }
        Ok(EventResult::Ignored)
    }

    /// Dispatch a paste event front-to-back.
    fn handle_paste(&mut self, text: &str, ctx: &mut Context) -> color_eyre::Result<EventResult> {
        for layer in self.layers.iter_mut().rev() {
            let result = layer.handle_paste(text, ctx)?;
            if !matches!(result, EventResult::Ignored) {
                return Ok(result);
            }
        }
        Ok(EventResult::Ignored)
    }
}

impl Default for Compositor {
    fn default() -> Self {
        Self::new()
    }
}
