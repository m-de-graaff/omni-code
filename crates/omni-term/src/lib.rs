//! # omni-term
//!
//! Terminal UI layer: ratatui compositor, components, and event loop.

pub mod bracket_match;
pub mod chord_state;
pub mod navigation_history;
pub mod component;
pub mod compositor;
pub mod context;
pub mod cursor;
pub mod editing;
pub mod event_loop;
pub mod formatter;
pub mod swap_file;
pub mod terminal;
pub mod widgets;

pub use component::{Component, CursorKind, EventResult};
pub use compositor::Compositor;
pub use context::Context;
pub use widgets::EditorShell;
