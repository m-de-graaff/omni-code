//! Actions that flow through the event system.

use std::path::PathBuf;

/// High-level actions for cross-component communication.
///
/// Actions are the primary mechanism for components to communicate
/// intentions without coupling to each other directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application.
    Quit,

    /// Open a file at the given path.
    OpenFile(PathBuf),

    /// Open a folder as the working directory.
    OpenFolder(PathBuf),

    /// Save the current document.
    Save,

    /// Save the current document to a new path.
    SaveAs(PathBuf),

    /// Close the current document.
    CloseBuffer,

    /// Resize the terminal to the given dimensions.
    Resize { width: u16, height: u16 },

    /// Focus the next component / split.
    FocusNext,

    /// Focus the previous component / split.
    FocusPrev,

    /// Split the view vertically.
    VerticalSplit,

    /// Split the view horizontally.
    HorizontalSplit,

    /// Toggle the sidebar visibility.
    ToggleSidebar,

    /// Toggle the bottom panel visibility.
    ToggleBottomPanel,

    /// Toggle the minimap visibility.
    ToggleMinimap,

    /// Cycle application display mode (IDE → Split → Chat).
    ToggleAppMode,

    /// Switch to the tab at the given index.
    SwitchTab(usize),

    /// Close the tab at the given index.
    CloseTab(usize),

    /// Reorder a tab from one index to another.
    ReorderTab { from: usize, to: usize },

    /// Show the command palette.
    CommandPalette,

    /// Undo the last edit.
    Undo,

    /// Redo the last undone edit.
    Redo,

    // ── Cursor movement ─────────────────────────────────────────────

    /// Move cursor left by one character.
    CursorLeft,
    /// Move cursor right by one character.
    CursorRight,
    /// Move cursor up by one line.
    CursorUp,
    /// Move cursor down by one line.
    CursorDown,
    /// Move cursor left by one word.
    CursorWordLeft,
    /// Move cursor right by one word.
    CursorWordRight,
    /// Move cursor to the start of the line.
    CursorLineStart,
    /// Move cursor to the end of the line.
    CursorLineEnd,
    /// Move cursor to the start of the document.
    CursorDocStart,
    /// Move cursor to the end of the document.
    CursorDocEnd,

    // ── Selection ───────────────────────────────────────────────────

    /// Select all text in the document (Ctrl+A).
    SelectAll,
    /// Select the next occurrence of the current word/selection (Ctrl+D).
    SelectNextOccurrence,
    /// Expand selection to the parent syntax node (tree-sitter).
    ExpandSelection,
    /// Shrink selection to the child syntax node (tree-sitter).
    ShrinkSelection,
    /// Extend selection left by one character.
    SelectLeft,
    /// Extend selection right by one character.
    SelectRight,
    /// Extend selection up by one line.
    SelectUp,
    /// Extend selection down by one line.
    SelectDown,
    /// Extend selection left by one word.
    SelectWordLeft,
    /// Extend selection right by one word.
    SelectWordRight,
    /// Extend selection to line start.
    SelectLineStart,
    /// Extend selection to line end.
    SelectLineEnd,
    /// Select the current word at cursor.
    SelectWord,
    /// Select the current line.
    SelectLine,

    // ── Text editing ────────────────────────────────────────────────

    /// Delete the character before the cursor (Backspace).
    Backspace,
    /// Delete the character after the cursor (Delete key).
    Delete,
    /// Delete the word before the cursor (Ctrl+Backspace).
    DeleteWordBackward,
    /// Delete the word after the cursor (Ctrl+Delete).
    DeleteWordForward,
    /// Insert a newline with auto-indent (Enter).
    InsertNewline,
    /// Insert a tab or spaces (Tab key).
    InsertTab,
    /// Indent the selected lines (Tab with selection).
    IndentSelection,
    /// Outdent the selected lines (Shift+Tab).
    OutdentSelection,
    /// Duplicate the current line (Ctrl+Shift+D).
    DuplicateLine,
    /// Move the current line up (Alt+Up).
    MoveLineUp,
    /// Move the current line down (Alt+Down).
    MoveLineDown,
    /// Toggle line comment (Ctrl+/).
    ToggleComment,
    /// Cut selection to clipboard (Ctrl+X).
    Cut,
    /// Copy selection to clipboard (Ctrl+C).
    Copy,
    /// Paste from clipboard (Ctrl+V).
    Paste,

    // ── Search ──────────────────────────────────────────────────────

    /// Open in-buffer search (Ctrl+F).
    Find,
    /// Find next match (F3).
    FindNext,
    /// Find previous match (Shift+F3).
    FindPrev,
    /// Open find and replace (Ctrl+H).
    FindReplace,
    /// Replace current match.
    ReplaceOne,
    /// Replace all matches.
    ReplaceAll,
    /// Open project-wide search (Ctrl+Shift+F).
    ProjectSearch,
    /// Go to a specific line number (Ctrl+G).
    GotoLine,
    /// Go to a symbol in the current file (Ctrl+Shift+O).
    GotoSymbol,

    // ── Scroll ──────────────────────────────────────────────────────

    /// Scroll the editor up by one line.
    ScrollUp,
    /// Scroll the editor down by one line.
    ScrollDown,
    /// Scroll the editor up by half a page.
    PageUp,
    /// Scroll the editor down by half a page.
    PageDown,

    /// Show the file picker.
    FilePicker,

    /// Execute a named command (for extensibility).
    Command(String),

    /// No-op / placeholder.
    Noop,
}
