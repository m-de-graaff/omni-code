//! # omni-loader
//!
//! Configuration loading, grammar management, and theme parsing.

pub mod config;
pub mod font;
pub mod grammar;
pub mod keymap_loader;
pub mod paths;
pub mod recent_files;
pub mod theme;

pub use config::EditorConfig;
pub use keymap_loader::{load_keymap, resolve_action};
pub use theme::{SyntaxColors, Theme, ThemeColors, detect_color_capability};
