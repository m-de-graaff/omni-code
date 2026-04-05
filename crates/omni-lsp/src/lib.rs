//! # omni-lsp
//!
//! LSP client implementation for connecting to language servers.

pub mod client;
pub mod registry;
pub mod transport;

pub use client::LspClient;
pub use registry::ServerRegistry;
pub use transport::Transport;
