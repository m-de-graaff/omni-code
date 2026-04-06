//! Type-safe document identifier.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A unique identifier for an open document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    /// Generate the next unique document ID.
    ///
    /// IDs are monotonically increasing and unique within a process lifetime.
    pub fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Return the raw numeric value (useful for display/logging).
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "doc:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let a = DocumentId::next();
        let b = DocumentId::next();
        assert_ne!(a, b);
        assert!(b.raw() > a.raw());
    }

    #[test]
    fn display_format() {
        let id = DocumentId::next();
        let s = format!("{id}");
        assert!(s.starts_with("doc:"));
    }
}
