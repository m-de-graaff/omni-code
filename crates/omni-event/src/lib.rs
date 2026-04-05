//! # omni-event
//!
//! Central event system for Omni Code: broadcast bus, hooks, and async actions.

pub mod action;
pub mod bus;
pub mod hook;

pub use action::Action;
pub use bus::EventBus;
pub use hook::{Hook, HookRegistry};
