//! # omni-view
//!
//! Frontend-agnostic editor state: documents, views, and view tree layout.

pub mod document;
pub mod view;
pub mod view_tree;

pub use document::Document;
pub use view::View;
pub use view_tree::ViewTree;
