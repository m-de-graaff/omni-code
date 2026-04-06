//! Filesystem tree model with lazy directory loading.
//!
//! Uses the `ignore` crate for `.gitignore`-aware directory traversal.
//! Directories are loaded on-demand when expanded (depth=1 per expansion).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tui_tree_widget::TreeItem;

/// A filesystem tree with lazy-loaded directories.
#[derive(Debug)]
pub struct FileTree {
    /// Root directory path.
    root: PathBuf,
    /// Arena of file/directory nodes.
    nodes: Vec<FileNode>,
    /// Parent → child index mapping.
    children: HashMap<usize, Vec<usize>>,
}

/// A single node in the file tree.
#[derive(Debug, Clone)]
pub struct FileNode {
    /// File or directory name.
    pub name: String,
    /// Absolute path.
    pub path: PathBuf,
    /// Whether this is a file or directory.
    pub kind: NodeKind,
    /// Whether this directory's children have been loaded.
    pub loaded: bool,
    /// Tree depth (0 = root's direct children).
    pub depth: usize,
    /// Index of parent node (root children have parent=0).
    pub parent: usize,
}

/// Node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Directory,
    File,
}

impl FileTree {
    /// Create a file tree rooted at `path`, loading only the first level.
    #[must_use]
    pub fn from_root(path: &Path) -> Self {
        let mut tree = Self {
            root: path.to_path_buf(),
            nodes: Vec::new(),
            children: HashMap::new(),
        };

        // Create the root node
        let root_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string();
        tree.nodes.push(FileNode {
            name: root_name,
            path: path.to_path_buf(),
            kind: NodeKind::Directory,
            loaded: false,
            depth: 0,
            parent: 0,
        });

        // Eagerly load the root's children
        tree.expand(0);
        tree
    }

    /// The root directory path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get a node by index.
    #[must_use]
    pub fn node(&self, idx: usize) -> Option<&FileNode> {
        self.nodes.get(idx)
    }

    /// Get children indices for a node.
    #[must_use]
    pub fn children_of(&self, idx: usize) -> &[usize] {
        self.children.get(&idx).map_or(&[], Vec::as_slice)
    }

    /// Total number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Lazy-load a directory's children (one level deep).
    ///
    /// Does nothing if the node is not a directory or is already loaded.
    pub fn expand(&mut self, node_idx: usize) {
        let node = &self.nodes[node_idx];
        if node.kind != NodeKind::Directory || node.loaded {
            return;
        }

        let dir_path = node.path.clone();
        let depth = node.depth + 1;

        // Use ignore crate for .gitignore-aware traversal
        let mut dirs: Vec<(String, PathBuf)> = Vec::new();
        let mut files: Vec<(String, PathBuf)> = Vec::new();

        let walker = ignore::WalkBuilder::new(&dir_path)
            .max_depth(Some(1))
            .hidden(true)       // skip hidden files
            .git_ignore(true)   // respect .gitignore
            .git_global(true)
            .git_exclude(true)
            .sort_by_file_name(std::cmp::Ord::cmp)
            .build();

        for entry in walker.flatten() {
            let entry_path = entry.path();
            // Skip the directory itself (depth 0 entry)
            if entry_path == dir_path {
                continue;
            }
            let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                dirs.push((name.to_string(), entry_path.to_path_buf()));
            } else {
                files.push((name.to_string(), entry_path.to_path_buf()));
            }
        }

        // Add directories first, then files (sorted within each group)
        let mut child_indices = Vec::new();

        for (name, path) in dirs {
            let idx = self.nodes.len();
            self.nodes.push(FileNode {
                name,
                path,
                kind: NodeKind::Directory,
                loaded: false,
                depth,
                parent: node_idx,
            });
            child_indices.push(idx);
        }

        for (name, path) in files {
            let idx = self.nodes.len();
            self.nodes.push(FileNode {
                name,
                path,
                kind: NodeKind::File,
                loaded: true, // files are always "loaded"
                depth,
                parent: node_idx,
            });
            child_indices.push(idx);
        }

        self.children.insert(node_idx, child_indices);
        self.nodes[node_idx].loaded = true;
    }

    /// Convert the tree to `tui-tree-widget` items for rendering.
    #[must_use]
    pub fn to_tree_items(&self) -> Vec<TreeItem<'static, String>> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        // Build tree starting from root's children
        let root_children = self.children_of(0);
        self.build_items(root_children)
    }

    /// Convert the tree to filtered items (name contains query, case-insensitive).
    #[must_use]
    pub fn to_tree_items_filtered(&self, query: &str) -> Vec<TreeItem<'static, String>> {
        if query.is_empty() {
            return self.to_tree_items();
        }
        let query_lower = query.to_lowercase();
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let root_children = self.children_of(0);
        self.build_items_filtered(root_children, &query_lower)
    }

    /// Find a node index by tui-tree-widget identifier path.
    ///
    /// The identifier path is the names of nodes from root to target.
    #[must_use]
    pub fn find_node_by_id(&self, id_path: &[String]) -> Option<usize> {
        if id_path.is_empty() {
            return None;
        }

        // Start from root's children
        let mut current_children = self.children_of(0);

        for (i, name) in id_path.iter().enumerate() {
            let found = current_children
                .iter()
                .find(|&&idx| self.nodes[idx].name == *name);

            let &idx = found?;

            if i == id_path.len() - 1 {
                return Some(idx);
            }

            current_children = self.children_of(idx);
        }
        None
    }

    /// Get the path for a node identified by the tui-tree-widget identifier path.
    #[must_use]
    pub fn path_for_id(&self, id_path: &[String]) -> Option<PathBuf> {
        self.find_node_by_id(id_path)
            .map(|idx| self.nodes[idx].path.clone())
    }

    /// Get the kind for a node identified by the tui-tree-widget identifier path.
    #[must_use]
    pub fn kind_for_id(&self, id_path: &[String]) -> Option<NodeKind> {
        self.find_node_by_id(id_path)
            .map(|idx| self.nodes[idx].kind)
    }

    // ── Private helpers ─────────────────────────────────────────────

    fn build_items(&self, indices: &[usize]) -> Vec<TreeItem<'static, String>> {
        indices
            .iter()
            .filter_map(|&idx| self.node_to_item(idx))
            .collect()
    }

    fn node_to_item(&self, idx: usize) -> Option<TreeItem<'static, String>> {
        let node = &self.nodes[idx];
        let display = format!("{} {}", file_icon(&node.name, node.kind), node.name);

        if node.kind == NodeKind::Directory {
            let children = self.build_items(self.children_of(idx));
            TreeItem::new(node.name.clone(), display, children).ok()
        } else {
            Some(TreeItem::new_leaf(node.name.clone(), display))
        }
    }

    fn build_items_filtered(
        &self,
        indices: &[usize],
        query: &str,
    ) -> Vec<TreeItem<'static, String>> {
        indices
            .iter()
            .filter_map(|&idx| self.node_to_item_filtered(idx, query))
            .collect()
    }

    fn node_to_item_filtered(
        &self,
        idx: usize,
        query: &str,
    ) -> Option<TreeItem<'static, String>> {
        let node = &self.nodes[idx];
        let display = format!("{} {}", file_icon(&node.name, node.kind), node.name);

        if node.kind == NodeKind::Directory {
            let children = self.build_items_filtered(self.children_of(idx), query);
            if children.is_empty() {
                // Only include empty dirs if their name matches
                if node.name.to_lowercase().contains(query) {
                    return TreeItem::new(node.name.clone(), display, vec![]).ok();
                }
                return None;
            }
            TreeItem::new(node.name.clone(), display, children).ok()
        } else if node.name.to_lowercase().contains(query) {
            Some(TreeItem::new_leaf(node.name.clone(), display))
        } else {
            None
        }
    }
}

/// Get a Nerd Font icon for a file based on its name/extension.
#[must_use]
pub fn file_icon(name: &str, kind: NodeKind) -> &'static str {
    if kind == NodeKind::Directory {
        return "\u{f07b}"; // nf-fa-folder
    }

    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "\u{e7a8}",        // Rust
        "toml" | "yaml" | "yml" => "\u{e615}", // Config
        "lock" => "\u{f023}",      // Lock
        "md" | "markdown" => "\u{e73e}", // Markdown
        "json" => "\u{e60b}",      // JSON
        "js" | "mjs" | "cjs" => "\u{e74e}", // JavaScript
        "ts" | "mts" | "cts" => "\u{e628}", // TypeScript
        "tsx" | "jsx" => "\u{e7ba}",  // React
        "py" => "\u{e73c}",        // Python
        "go" => "\u{e626}",        // Go
        "c" | "h" => "\u{e61e}",   // C
        "cpp" | "cc" | "cxx" | "hpp" => "\u{e61d}", // C++
        "java" => "\u{e738}",      // Java
        "kt" | "kts" => "\u{e634}",   // Kotlin
        "swift" => "\u{e755}",     // Swift
        "rb" => "\u{e739}",        // Ruby
        "php" => "\u{e73d}",       // PHP
        "html" | "htm" => "\u{e736}", // HTML
        "css" | "scss" | "sass" => "\u{e749}", // CSS
        "sh" | "bash" | "zsh" => "\u{e795}", // Shell
        "lua" => "\u{e620}",       // Lua
        "zig" => "\u{e6a9}",       // Zig
        "sql" => "\u{e706}",       // Database
        "xml" => "\u{e619}",       // XML
        "txt" => "\u{f15c}",       // Text
        "log" => "\u{f18d}",       // Log
        "git" | "gitignore" => "\u{e702}", // Git
        _ => "\u{f15b}",           // Generic file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn from_root_loads_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("world.txt"), "hello").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();

        let tree = FileTree::from_root(dir.path());
        assert!(tree.len() >= 4); // root + 3 entries
        assert!(!tree.is_empty());

        let items = tree.to_tree_items();
        assert_eq!(items.len(), 3); // src/, hello.rs, world.txt
    }

    #[test]
    fn lazy_expand() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("src");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("main.rs"), "fn main() {}").unwrap();

        let mut tree = FileTree::from_root(dir.path());

        // Find the src directory node
        let src_idx = tree.find_node_by_id(&["src".to_string()]).unwrap();
        assert!(!tree.nodes[src_idx].loaded); // not yet loaded (initially only root's direct children loaded, but subdirs start as unloaded)

        // Expand it
        tree.expand(src_idx);
        assert!(tree.nodes[src_idx].loaded);

        let children = tree.children_of(src_idx);
        assert!(!children.is_empty());
    }

    #[test]
    fn find_node_by_id() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test.rs"), "").unwrap();

        let tree = FileTree::from_root(dir.path());
        let idx = tree.find_node_by_id(&["test.rs".to_string()]);
        assert!(idx.is_some());
    }

    #[test]
    fn filter_items() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.rs"), "").unwrap();
        fs::write(dir.path().join("world.txt"), "").unwrap();
        fs::write(dir.path().join("test.rs"), "").unwrap();

        let tree = FileTree::from_root(dir.path());
        let filtered = tree.to_tree_items_filtered("rs");
        // Should include .rs files but not .txt
        assert!(filtered.len() >= 2);
    }

    #[test]
    fn file_icon_returns_icon() {
        assert_eq!(file_icon("test.rs", NodeKind::File), "\u{e7a8}");
        assert_eq!(file_icon("src", NodeKind::Directory), "\u{f07b}");
    }
}
