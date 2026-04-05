//! `OpenAI` provider implementation.

use async_trait::async_trait;

use crate::message::Message;
use crate::provider::{AiError, AiProvider};

/// `OpenAI` API provider.
#[derive(Debug)]
pub struct OpenAiProvider {
    api_key: Option<String>,
}

impl OpenAiProvider {
    /// Create a new `OpenAI` provider with an optional API key.
    #[must_use]
    pub const fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    async fn complete(&self, _messages: &[Message], _model: &str) -> Result<String, AiError> {
        // TODO: implement HTTP request to OpenAI API
        Err(AiError::NotConfigured)
    }
}
