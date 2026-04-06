//! # omni-core
//!
//! Text primitives for the Omni Code editor: rope-backed text buffers,
//! selections, transactions, and undo/redo history.

pub mod changeset;
pub mod doc_id;
pub mod history;
pub mod keymap;
pub mod line_ending;
pub mod selection;
pub mod text;
pub mod transaction;

pub use changeset::{ChangeSet, Operation};
pub use doc_id::DocumentId;
pub use history::{History, NodeInfo};
pub use keymap::{KeyChord, KeyCodeRepr, KeySequence, Keymap, KeymapMode, KeymapResult, Modifiers};
pub use line_ending::LineEnding;
pub use selection::{Range, Selection};
pub use text::Text;
pub use transaction::Transaction;
