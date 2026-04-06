//! Document model combining text buffer with metadata.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use omni_core::{DocumentId, History, LineEnding, Selection, Text, Transaction};
use omni_syntax::{HighlightSpan, SyntaxTree};

use crate::view_tree::NodeKey;

/// A document represents an open file with its text buffer, edit history,
/// syntax state, and metadata.
#[derive(Debug)]
pub struct Document {
    /// Unique document identifier.
    pub id: DocumentId,
    text: Text,
    /// Per-view selections (keyed by the view's `NodeKey`).
    selections: HashMap<NodeKey, Selection>,
    history: History,
    /// Parsed syntax tree (updated incrementally by the highlighter).
    pub syntax: Option<SyntaxTree>,
    /// Language identifier (e.g. `"rust"`, `"python"`).
    pub language: Option<String>,
    /// File path, if saved.
    pub path: Option<PathBuf>,
    /// Whether the document has unsaved changes.
    pub modified: bool,
    /// Lines that have been touched by AI edits (for gutter indicators).
    pub ai_touched_lines: HashSet<usize>,
    /// LSP document version, incremented on each edit.
    pub version: u64,
    /// Detected line ending style.
    pub line_ending: LineEnding,
    /// Detected character encoding.
    pub encoding: &'static encoding_rs::Encoding,
    /// Raw file size in bytes (for large-file detection).
    pub file_size: usize,
    /// Cached highlight spans from the last parse (sorted by byte offset).
    pub highlight_spans: Vec<HighlightSpan>,
    /// Per-line git diff status (empty if not computed).
    pub diff_status: Vec<omni_vcs::diff::LineDiffStatus>,
}

impl Document {
    /// Create a new empty document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: DocumentId::next(),
            text: Text::new(),
            selections: HashMap::new(),
            history: History::new(),
            syntax: None,
            language: None,
            path: None,
            modified: false,
            ai_touched_lines: HashSet::new(),
            version: 0,
            line_ending: LineEnding::default(),
            encoding: encoding_rs::UTF_8,
            file_size: 0,
            highlight_spans: Vec::new(),
            diff_status: Vec::new(),
        }
    }

    /// Create a document from a string with an optional path.
    #[must_use]
    pub fn from_str(content: &str, path: Option<PathBuf>) -> Self {
        let line_ending = LineEnding::detect(content);
        let language = path.as_ref().and_then(|p| language_from_extension(p));

        Self {
            id: DocumentId::next(),
            text: Text::from(content),
            selections: HashMap::new(),
            history: History::new(),
            syntax: None,
            language,
            path,
            modified: false,
            ai_touched_lines: HashSet::new(),
            version: 0,
            line_ending,
            encoding: encoding_rs::UTF_8,
            file_size: 0,
            highlight_spans: Vec::new(),
            diff_status: Vec::new(),
        }
    }

    /// Load a document from a file path with encoding detection.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_file(path: &Path) -> Result<Self, crate::file_io::FileIoError> {
        let (content, encoding, file_size) = crate::file_io::read_file(path)?;
        let mut doc = Self::from_str(&content, Some(path.to_path_buf()));
        doc.encoding = encoding;
        doc.file_size = file_size;
        Ok(doc)
    }

    /// Save the document to its current path.
    ///
    /// # Errors
    ///
    /// Returns [`FileIoError::NoPath`] if the document has no path (use `save_as`).
    /// Returns I/O errors if the write fails.
    pub fn save(&mut self) -> Result<(), crate::file_io::FileIoError> {
        let path = self.path.as_ref().ok_or(crate::file_io::FileIoError::NoPath)?;
        let path = path.clone();
        crate::file_io::write_file(&path, self.text(), self.encoding, self.line_ending)?;
        self.modified = false;
        Ok(())
    }

    /// Save the document to a new path.
    ///
    /// Updates the document's path, language detection, and modified flag.
    ///
    /// # Errors
    ///
    /// Returns I/O errors if the write fails.
    pub fn save_as(&mut self, path: PathBuf) -> Result<(), crate::file_io::FileIoError> {
        crate::file_io::write_file(&path, self.text(), self.encoding, self.line_ending)?;
        self.language = language_from_extension(&path);
        self.path = Some(path);
        self.modified = false;
        Ok(())
    }

    /// Whether this document exceeds the large-file threshold.
    #[must_use]
    pub const fn is_large_file(&self, threshold: usize) -> bool {
        self.file_size >= threshold
    }

    /// Reload the document content from a string (e.g., after external file change).
    ///
    /// Replaces the entire text buffer and resets edit state.
    pub fn reload_from_string(&mut self, content: &str) {
        self.text = Text::from(content);
        self.modified = false;
        self.version += 1;
        self.selections.clear();
        self.highlight_spans.clear();
        self.syntax = None;
    }

    // ── Text buffer ─────────────────────────────────────────────────

    /// The text buffer (read-only).
    ///
    /// All text mutations must go through [`apply`](Self::apply).
    #[must_use]
    pub const fn text(&self) -> &Text {
        &self.text
    }

    // ── Transaction-based editing ───────────────────────────────────

    /// Apply a transaction to this document.
    ///
    /// This is the **only** way to mutate the text buffer. It:
    /// 1. Computes the inverse transaction (for undo) before applying.
    /// 2. Applies the changeset to the text buffer.
    /// 3. Updates all per-view selections (explicit or mapped).
    /// 4. Pushes the inverse onto the undo history DAG.
    /// 5. Marks the document as modified and increments the LSP version.
    pub fn apply(&mut self, txn: &Transaction, view_id: NodeKey) {
        // 1. Compute inverse before applying (needs original text)
        let current_sel = self.selection(view_id);
        let inverse = txn.invert(&self.text, &current_sel);

        // 2. Apply changeset to text
        txn.apply(&mut self.text);

        // 3. Update selections
        let new_sel = txn.map_selection(&current_sel);
        self.selections.insert(view_id, new_sel);

        for (vid, sel) in &mut self.selections {
            if *vid != view_id {
                let mapped = txn.map_selection(sel);
                *sel = mapped;
            }
        }

        // 4. Push inverse onto undo history DAG (with snapshot text)
        self.history.push(inverse, &self.text);

        // 5. Mark modified + bump version
        self.modified = true;
        self.version += 1;
    }

    /// Undo the most recent edit. Returns `true` if an undo was performed.
    ///
    /// In the DAG history, this moves the current pointer to the parent node
    /// and applies the inverse transaction stored at the current node.
    pub fn undo(&mut self, view_id: NodeKey) -> bool {
        let Some(inverse) = self.history.undo() else {
            return false;
        };

        // Clone the inverse — we need to apply it but history owns it
        let inverse = inverse.clone();

        // Apply the inverse changeset
        inverse.apply(&mut self.text);

        // Restore selection from the inverse transaction
        let current_sel = self.selection(view_id);
        let restored_sel = inverse.map_selection(&current_sel);
        self.selections.insert(view_id, restored_sel);

        for (vid, sel) in &mut self.selections {
            if *vid != view_id {
                let mapped = inverse.map_selection(sel);
                *sel = mapped;
            }
        }

        self.version += 1;
        true
    }

    /// Redo the most recently undone edit. Returns `true` if a redo was performed.
    ///
    /// In the DAG history, this moves the current pointer to the most recently
    /// visited child and applies the *forward* version of that child's inverse.
    #[allow(clippy::missing_panics_doc)] // child node always exists after redo()
    pub fn redo(&mut self, view_id: NodeKey) -> bool {
        // redo() moves current to the child and returns the child index
        let Some(child_idx) = self.history.redo() else {
            return false;
        };

        // The child stores the inverse (child→parent). We need the forward
        // direction (parent→child), so we invert the child's inverse using
        // the current text (which is the parent state).
        let child_inverse = self.history.node_inverse(child_idx)
            .expect("child node must exist")
            .clone();

        let current_sel = self.selection(view_id);
        let forward = child_inverse.invert(&self.text, &current_sel);

        // Apply the forward transaction
        forward.apply(&mut self.text);

        // Update selections
        let new_sel = forward.map_selection(&current_sel);
        self.selections.insert(view_id, new_sel);

        for (vid, sel) in &mut self.selections {
            if *vid != view_id {
                let mapped = forward.map_selection(sel);
                *sel = mapped;
            }
        }

        self.modified = true;
        self.version += 1;
        true
    }

    // ── Selections (per-view) ───────────────────────────────────────

    /// Get the selection for a specific view.
    #[must_use]
    pub fn selection(&self, view_id: NodeKey) -> Selection {
        self.selections
            .get(&view_id)
            .cloned()
            .unwrap_or_else(|| Selection::point(0))
    }

    /// Set the selection for a specific view.
    pub fn set_selection(&mut self, view_id: NodeKey, sel: Selection) {
        self.selections.insert(view_id, sel);
    }

    /// Remove the selection for a view (e.g. when the view is closed).
    pub fn remove_selection(&mut self, view_id: NodeKey) {
        self.selections.remove(&view_id);
    }

    // ── History ─────────────────────────────────────────────────────

    /// The edit history.
    #[must_use]
    pub const fn history(&self) -> &History {
        &self.history
    }

    /// Mutable access to the edit history.
    pub const fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    // ── Metadata ────────────────────────────────────────────────────

    /// Whether the document has unsaved changes.
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the document as saved.
    pub const fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// The file name (without directory), or `"[untitled]"` if unsaved.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[untitled]")
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Infer a language identifier from a file extension.
fn language_from_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let lang = match ext {
        "rs" => "rust",
        "py" | "pyi" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "zig" => "zig",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "sh" | "bash" | "zsh" => "bash",
        "sql" => "sql",
        "xml" => "xml",
        "dart" => "dart",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "r" | "R" => "r",
        "scala" | "sc" => "scala",
        "vim" => "vim",
        _ => return None,
    };
    Some(lang.to_string())
}
