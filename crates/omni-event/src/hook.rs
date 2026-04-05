//! Hook system for extensible event handling.

use crate::Action;

/// A hook that can intercept and respond to actions.
pub trait Hook: Send + Sync {
    /// Called when an action is dispatched. Return `true` to consume the action.
    fn on_action(&self, action: &Action) -> bool;
}

/// Registry of active hooks.
#[derive(Default)]
pub struct HookRegistry {
    hooks: Vec<Box<dyn Hook>>,
}

impl HookRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new hook.
    pub fn register(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    /// Dispatch an action through all hooks. Returns `true` if any hook consumed it.
    pub fn dispatch(&self, action: &Action) -> bool {
        self.hooks.iter().any(|h| h.on_action(action))
    }
}
