//! Shared application state passed to components during event handling.

use tokio::sync::{broadcast, mpsc};

use std::path::PathBuf;

use omni_core::keymap::{Keymap, KeymapMode};
use omni_event::Action;
use omni_loader::{EditorConfig, ThemeColors};
use omni_syntax::LanguageRegistry;
use omni_view::{DocumentStore, ViewTree};

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

    /// All open documents.
    pub documents: &'a mut DocumentStore,

    /// Editor configuration.
    pub config: &'a EditorConfig,

    /// Resolved theme colors for the current terminal.
    pub theme: &'a ThemeColors,

    /// The active keymap.
    pub keymap: &'a Keymap,

    /// Current keymap mode.
    pub keymap_mode: KeymapMode,

    /// Language registry for syntax highlighting.
    pub language_registry: &'a LanguageRegistry,

    /// The workspace root directory (set on `Action::OpenFolder`).
    pub workspace_root: Option<PathBuf>,

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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        view_tree: &'a mut ViewTree,
        documents: &'a mut DocumentStore,
        config: &'a EditorConfig,
        theme: &'a ThemeColors,
        keymap: &'a Keymap,
        language_registry: &'a LanguageRegistry,
        action_tx: broadcast::Sender<Action>,
        callback_tx: mpsc::UnboundedSender<Callback>,
    ) -> Self {
        Self {
            view_tree,
            documents,
            config,
            theme,
            keymap,
            keymap_mode: KeymapMode::default(),
            language_registry,
            workspace_root: None,
            should_quit: false,
            action_tx,
            callback_tx,
            needs_redraw: true,
        }
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
