//! Core AI provider trait.

use async_trait::async_trait;
use thiserror::Error;

use crate::message::Message;

/// Errors from AI provider operations.
#[derive(Debug, Error)]
pub enum AiError {
    #[error("request failed: {0}")]
    Request(String),
    #[error("authentication failed")]
    Auth,
    #[error("rate limited")]
    RateLimit,
    #[error("provider not configured")]
    NotConfigured,
}

/// Trait that all AI providers must implement.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// The provider's display name (e.g., "Ollama", "`OpenAI`").
    fn name(&self) -> &str;

    /// Whether the provider is configured and ready.
    fn is_available(&self) -> bool;

    /// Send a chat completion request and return the full response.
    async fn complete(&self, messages: &[Message], model: &str) -> Result<String, AiError>;
}
