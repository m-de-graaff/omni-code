//! Tree-sitter syntax tree wrapper.

/// A parsed syntax tree for a document.
#[derive(Debug)]
pub struct SyntaxTree {
    tree: Option<tree_sitter::Tree>,
}

impl SyntaxTree {
    /// Create an empty (unparsed) syntax tree.
    #[must_use]
    pub const fn empty() -> Self {
        Self { tree: None }
    }

    /// Create a syntax tree from a parsed tree-sitter tree.
    #[must_use]
    pub const fn from_tree(tree: tree_sitter::Tree) -> Self {
        Self { tree: Some(tree) }
    }

    /// Return the inner tree-sitter tree, if parsed.
    #[must_use]
    pub const fn tree(&self) -> Option<&tree_sitter::Tree> {
        self.tree.as_ref()
    }

    /// Replace the inner tree.
    pub fn set_tree(&mut self, tree: tree_sitter::Tree) {
        self.tree = Some(tree);
    }

    /// Clear the syntax tree.
    pub fn clear(&mut self) {
        self.tree = None;
    }
}

impl Default for SyntaxTree {
    fn default() -> Self {
        Self::empty()
    }
}
