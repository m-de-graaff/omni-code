//! Streaming response handling for AI completions.

/// A chunk of a streaming AI response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A text delta.
    Delta(String),
    /// The stream has completed.
    Done,
    /// An error occurred during streaming.
    Error(String),
}
