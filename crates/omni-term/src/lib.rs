//! # omni-term
//!
//! Terminal UI layer: ratatui compositor, components, and event loop.

pub mod component;
pub mod compositor;
pub mod context;
pub mod event_loop;
pub mod widgets;

pub use component::{Component, CursorKind, EventResult};
pub use compositor::Compositor;
pub use context::Context;
