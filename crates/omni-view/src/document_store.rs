//! Collection of open documents.

use std::collections::HashMap;
use std::path::Path;

use omni_core::DocumentId;

use crate::Document;

/// A collection of all open documents in the editor.
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<DocumentId, Document>,
}

impl DocumentStore {
    /// Create an empty document store.
    #[must_use]
    pub fn new() -> Self {
        Self { documents: HashMap::new() }
    }

    /// Insert a document into the store. Returns its ID.
    pub fn insert(&mut self, doc: Document) -> DocumentId {
        let id = doc.id;
        self.documents.insert(id, doc);
        id
    }

    /// Get a reference to a document by ID.
    #[must_use]
    pub fn get(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    /// Get a mutable reference to a document by ID.
    pub fn get_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(&id)
    }

    /// Remove a document from the store, returning it if it existed.
    pub fn remove(&mut self, id: DocumentId) -> Option<Document> {
        self.documents.remove(&id)
    }

    /// Find a document by its file path. Returns `None` if no document
    /// with that path is open, avoiding duplicate opens of the same file.
    #[must_use]
    pub fn find_by_path(&self, path: &Path) -> Option<DocumentId> {
        self.documents
            .values()
            .find(|doc| doc.path.as_deref() == Some(path))
            .map(|doc| doc.id)
    }

    /// Iterate over all documents.
    pub fn iter(&self) -> impl Iterator<Item = (&DocumentId, &Document)> {
        self.documents.iter()
    }

    /// Number of open documents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether there are no open documents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut store = DocumentStore::new();
        let doc = Document::from_str("hello", None);
        let id = doc.id;
        store.insert(doc);

        assert_eq!(store.len(), 1);
        assert!(store.get(id).is_some());
    }

    #[test]
    fn find_by_path_returns_existing() {
        let mut store = DocumentStore::new();
        let path = Path::new("/tmp/test.rs");
        let doc = Document::from_str("fn main() {}", Some(path.to_path_buf()));
        let id = doc.id;
        store.insert(doc);

        assert_eq!(store.find_by_path(path), Some(id));
        assert_eq!(store.find_by_path(Path::new("/other")), None);
    }

    #[test]
    fn remove_document() {
        let mut store = DocumentStore::new();
        let doc = Document::new();
        let id = doc.id;
        store.insert(doc);

        assert!(store.remove(id).is_some());
        assert!(store.is_empty());
    }
}
