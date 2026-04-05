//! Shared application state passed to components during event handling.

use tokio::sync::{broadcast, mpsc};

use omni_event::Action;
use omni_loader::EditorConfig;
use omni_view::ViewTree;

/// A boxed closure that mutates the compositor.
///
/// Used by components to enqueue layer operations (push/pop) without
/// holding a reference to the compositor during event handling.
pub type Callback = Box<dyn FnOnce(&mut crate::Compositor) + Send>;

/// Shared application context passed to components during event handling.
///
/// Holds mutable references to the core application state so components
/// can read and modify documents, views, and configuration without owning them.
pub struct Context<'a> {
    /// The view tree managing editor splits and focus.
    pub view_tree: &'a mut ViewTree,

    /// Editor configuration.
    pub config: &'a EditorConfig,

    /// Whether the application should quit after this event cycle.
    pub should_quit: bool,

    /// Sender to publish actions to the event bus.
    pub action_tx: broadcast::Sender<Action>,

    /// Sender to enqueue compositor callbacks.
    pub callback_tx: mpsc::UnboundedSender<Callback>,

    /// Whether the UI needs to be redrawn.
    pub needs_redraw: bool,
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(
        view_tree: &'a mut ViewTree,
        config: &'a EditorConfig,
        action_tx: broadcast::Sender<Action>,
        callback_tx: mpsc::UnboundedSender<Callback>,
    ) -> Self {
        Self { view_tree, config, should_quit: false, action_tx, callback_tx, needs_redraw: true }
    }

    /// Signal that the application should quit.
    pub const fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Request a UI redraw on the next event loop iteration.
    pub const fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }
}
