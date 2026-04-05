//! Broadcast-based event bus.

use tokio::sync::broadcast;

use crate::Action;

/// Central event bus using tokio broadcast channels.
///
/// Components publish actions to the bus, and any number of subscribers
/// receive a copy.
#[derive(Debug)]
pub struct EventBus {
    sender: broadcast::Sender<Action>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an action to all subscribers.
    ///
    /// Returns the number of active receivers.
    pub fn publish(&self, action: Action) -> usize {
        self.sender.send(action).unwrap_or(0)
    }

    /// Get a clone of the underlying sender for direct publishing.
    ///
    /// Useful for passing into components or background tasks that need
    /// to publish actions without holding the entire `EventBus`.
    #[must_use]
    pub fn sender(&self) -> broadcast::Sender<Action> {
        self.sender.clone()
    }

    /// Subscribe to events. Returns a receiver handle.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Action> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
