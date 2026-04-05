//! LSP client that communicates with a language server process.

use thiserror::Error;

/// Errors from LSP communication.
#[derive(Debug, Error)]
pub enum LspError {
    #[error("server not running")]
    NotRunning,
    #[error("transport error: {0}")]
    Transport(String),
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// Client for a single language server instance.
#[derive(Debug)]
pub struct LspClient {
    /// The language server name.
    pub server_name: String,
    /// Whether the server is initialized.
    initialized: bool,
}

impl LspClient {
    /// Create a new (unstarted) LSP client.
    #[must_use]
    pub const fn new(server_name: String) -> Self {
        Self { server_name, initialized: false }
    }

    /// Whether the server has completed initialization.
    #[must_use]
    pub const fn is_initialized(&self) -> bool {
        self.initialized
    }
}
