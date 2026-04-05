//! Shared application state passed to components during event handling.

use omni_loader::EditorConfig;
use omni_view::ViewTree;

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
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub const fn new(view_tree: &'a mut ViewTree, config: &'a EditorConfig) -> Self {
        Self { view_tree, config, should_quit: false }
    }

    /// Signal that the application should quit.
    pub const fn quit(&mut self) {
        self.should_quit = true;
    }
}
