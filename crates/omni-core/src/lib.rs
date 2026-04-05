//! # omni-core
//!
//! Text primitives for the Omni Code editor: rope-backed text buffers,
//! selections, transactions, and undo/redo history.

pub mod history;
pub mod selection;
pub mod text;
pub mod transaction;

pub use history::History;
pub use selection::{Range, Selection};
pub use text::Text;
pub use transaction::{Operation, Transaction};
