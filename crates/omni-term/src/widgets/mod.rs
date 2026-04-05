//! Custom TUI widgets for the editor.

pub(crate) mod bottom_panel;
pub(crate) mod editor_pane;
pub(crate) mod layout_state;
pub(crate) mod sidebar;
pub(crate) mod status_bar;
pub(crate) mod tab_bar;

mod editor_shell;
pub use editor_shell::EditorShell;
