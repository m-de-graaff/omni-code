//! Custom TUI widgets for the editor.

pub(crate) mod bottom_panel;
pub(crate) mod chat_panel;
pub(crate) mod command_palette;
pub(crate) mod context_menu;
pub(crate) mod editor_pane;
pub(crate) mod file_tree;
pub(crate) mod folder_picker;
pub(crate) mod goto_line;
pub(crate) mod hit_map;
pub(crate) mod layout_state;
pub(crate) mod minimap;
pub(crate) mod mouse_state;
pub(crate) mod search_bar;
pub(crate) mod sidebar;
pub(crate) mod startup_screen;
pub(crate) mod status_bar;
pub(crate) mod symbol_picker;
pub(crate) mod tab_bar;

mod editor_shell;
pub use editor_shell::EditorShell;
