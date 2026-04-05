//! # omni-loader
//!
//! Configuration loading, grammar management, and theme parsing.

pub mod config;
pub mod grammar;
pub mod paths;
pub mod theme;

pub use config::EditorConfig;
pub use theme::Theme;
