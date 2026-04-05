//! Actions that flow through the event system.

use std::path::PathBuf;

/// High-level actions for cross-component communication.
///
/// Actions are the primary mechanism for components to communicate
/// intentions without coupling to each other directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application.
    Quit,

    /// Open a file at the given path.
    OpenFile(PathBuf),

    /// Save the current document.
    Save,

    /// Save the current document to a new path.
    SaveAs(PathBuf),

    /// Close the current document.
    CloseBuffer,

    /// Resize the terminal to the given dimensions.
    Resize { width: u16, height: u16 },

    /// Focus the next component / split.
    FocusNext,

    /// Focus the previous component / split.
    FocusPrev,

    /// Split the view vertically.
    VerticalSplit,

    /// Split the view horizontally.
    HorizontalSplit,

    /// Toggle the sidebar visibility.
    ToggleSidebar,

    /// Toggle the bottom panel visibility.
    ToggleBottomPanel,

    /// Toggle the minimap visibility.
    ToggleMinimap,

    /// Show the command palette.
    CommandPalette,

    /// Show the file picker.
    FilePicker,

    /// Execute a named command (for extensibility).
    Command(String),

    /// No-op / placeholder.
    Noop,
}
