//! Registry of language server configurations and running instances.

use crate::LspClient;

/// Registry managing available and active language servers.
#[derive(Debug, Default)]
pub struct ServerRegistry {
    clients: Vec<LspClient>,
}

impl ServerRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a client.
    pub fn add(&mut self, client: LspClient) {
        self.clients.push(client);
    }

    /// All registered clients.
    #[must_use]
    pub fn clients(&self) -> &[LspClient] {
        &self.clients
    }
}
