//! # omni-ai
//!
//! AI provider abstraction layer with implementations for
//! Ollama, `OpenAI`, and Anthropic.

pub mod message;
pub mod provider;
pub mod providers;
pub mod stream;

pub use message::{Message, Role};
pub use provider::AiProvider;
