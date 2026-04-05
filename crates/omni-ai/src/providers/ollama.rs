//! Ollama provider implementation.

use async_trait::async_trait;

use crate::message::Message;
use crate::provider::{AiError, AiProvider};

/// Ollama local AI provider.
#[derive(Debug)]
pub struct OllamaProvider {
    base_url: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider pointing at the given base URL.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new("http://localhost:11434")
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "Ollama"
    }

    fn is_available(&self) -> bool {
        !self.base_url.is_empty()
    }

    async fn complete(&self, _messages: &[Message], _model: &str) -> Result<String, AiError> {
        // TODO: implement HTTP request to Ollama API
        Err(AiError::NotConfigured)
    }
}
