//! Anthropic provider implementation.

use async_trait::async_trait;

use crate::message::Message;
use crate::provider::{AiError, AiProvider};

/// Anthropic Claude API provider.
#[derive(Debug)]
pub struct AnthropicProvider {
    api_key: Option<String>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with an optional API key.
    #[must_use]
    pub const fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Anthropic"
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    async fn complete(&self, _messages: &[Message], _model: &str) -> Result<String, AiError> {
        // TODO: implement HTTP request to Anthropic API
        Err(AiError::NotConfigured)
    }
}
