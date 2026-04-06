//! File tree sidebar with tree-widget rendering, keyboard navigation, and filtering.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use omni_loader::ThemeColors;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};
use tui_tree_widget::{Tree, TreeState};

use super::file_tree::{FileTree, NodeKind};

/// File tree sidebar panel with interactive navigation.
pub struct Sidebar {
    /// The filesystem tree model.
    file_tree: Option<FileTree>,
    /// Widget state for tui-tree-widget (tracks open/selected nodes).
    tree_state: TreeState<String>,
    /// Whether the sidebar filter input is active.
    filter_active: bool,
    /// Filter text for narrowing the file tree.
    pub filter_text: String,
    /// Cached area from the last render (for mouse hit-testing).
    area: Option<Rect>,
}

impl Sidebar {
    /// Create a new empty sidebar.
    pub fn new() -> Self {
        Self {
            file_tree: None,
            tree_state: TreeState::default(),
            filter_active: false,
            filter_text: String::new(),
            area: None,
        }
    }

    /// Set the workspace root, loading the file tree.
    pub fn set_root(&mut self, path: &std::path::Path) {
        self.file_tree = Some(FileTree::from_root(path));
        self.tree_state = TreeState::default();
    }

    /// Whether a workspace root has been set.
    pub fn has_tree(&self) -> bool {
        self.file_tree.is_some()
    }

    /// Activate the filter input.
    pub fn activate_filter(&mut self) {
        self.filter_active = true;
    }

    /// Deactivate the filter input.
    pub fn deactivate_filter(&mut self) {
        self.filter_active = false;
        self.filter_text.clear();
    }

    /// Whether the filter input is active.
    pub fn is_filter_active(&self) -> bool {
        self.filter_active
    }

    /// The path of the currently selected node, if any.
    pub fn selected_path(&self) -> Option<PathBuf> {
        let tree = self.file_tree.as_ref()?;
        let selected = self.tree_state.selected();
        if selected.is_empty() {
            return None;
        }
        tree.path_for_id(selected)
    }

    /// Render the sidebar into the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &ThemeColors) {
        self.area = Some(area);

        let border_color = theme.border;
        let block = Block::bordered()
            .title(" Files ")
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(theme.background));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let Some(tree) = &self.file_tree else {
            let placeholder =
                Paragraph::new("  Open a folder (Ctrl+O)").style(Style::new().fg(theme.text_muted));
            frame.render_widget(placeholder, inner);
            return;
        };

        // Reserve space for filter bar when active
        let (tree_area, filter_area) = if self.filter_active && inner.height > 2 {
            let filter_h = 1;
            let tree_h = inner.height - filter_h;
            (
                Rect::new(inner.x, inner.y, inner.width, tree_h),
                Some(Rect::new(inner.x, inner.y + tree_h, inner.width, filter_h)),
            )
        } else {
            (inner, None)
        };

        // Build tree items (filtered or full)
        let items = if self.filter_text.is_empty() {
            tree.to_tree_items()
        } else {
            tree.to_tree_items_filtered(&self.filter_text)
        };

        // Render the tree widget
        let highlight_style = Style::new().fg(theme.foreground).bg(theme.selection_bg);
        let tree_widget = Tree::new(&items)
            .expect("tree items should be valid")
            .highlight_style(highlight_style)
            .node_closed_symbol("\u{25b8} ")  // ▸
            .node_open_symbol("\u{25be} ")    // ▾
            .node_no_children_symbol("  ");

        frame.render_stateful_widget(tree_widget, tree_area, &mut self.tree_state);

        // Filter input
        if let Some(fa) = filter_area {
            let filter_display = format!(" \u{f002} {}\u{2588}", self.filter_text); // nf-fa-search + cursor block
            let filter_line = Paragraph::new(filter_display)
                .style(Style::new().fg(theme.foreground).bg(theme.selection_bg));
            frame.render_widget(filter_line, fa);
        }
    }

    /// Handle a key event. Returns `true` if consumed.
    pub fn handle_key(&mut self, event: KeyEvent) -> SidebarAction {
        // Filter input mode
        if self.filter_active {
            return match event.code {
                KeyCode::Esc => {
                    self.deactivate_filter();
                    SidebarAction::Consumed
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    SidebarAction::Consumed
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    SidebarAction::Consumed
                }
                KeyCode::Enter => {
                    self.deactivate_filter();
                    // Try to open the selected file
                    self.try_open_selected()
                }
                _ => SidebarAction::Consumed,
            };
        }

        match event.code {
            KeyCode::Esc => SidebarAction::FocusEditor,
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree_state.key_up();
                SidebarAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.tree_state.key_down();
                SidebarAction::Consumed
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.tree_state.key_left();
                SidebarAction::Consumed
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.tree_state.key_right();
                // Expand directory if needed
                self.expand_selected();
                SidebarAction::Consumed
            }
            KeyCode::Enter => self.try_open_selected(),
            KeyCode::Char('/') => {
                self.activate_filter();
                SidebarAction::Consumed
            }
            _ => SidebarAction::Ignored,
        }
    }

    /// Handle a mouse event. Returns a sidebar action.
    pub fn handle_mouse(&mut self, event: MouseEvent) -> SidebarAction {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let pos = Position::new(event.column, event.row);
                // click_at selects the node; if already selected, toggles open/close
                let changed = self.tree_state.click_at(pos);
                if !changed {
                    return SidebarAction::Consumed;
                }

                // After click, check if a file was selected — open it
                let Some(tree) = &mut self.file_tree else {
                    return SidebarAction::Consumed;
                };
                let selected = self.tree_state.selected();
                if selected.is_empty() {
                    return SidebarAction::Consumed;
                }

                match tree.kind_for_id(selected) {
                    Some(NodeKind::File) => {
                        if let Some(path) = tree.path_for_id(selected) {
                            return SidebarAction::OpenFile(path);
                        }
                        SidebarAction::Consumed
                    }
                    Some(NodeKind::Directory) => {
                        // Expand the directory so children become visible
                        if let Some(idx) = tree.find_node_by_id(selected) {
                            tree.expand(idx);
                        }
                        SidebarAction::Consumed
                    }
                    None => SidebarAction::Consumed,
                }
            }
            MouseEventKind::ScrollUp => {
                self.tree_state.scroll_up(3);
                SidebarAction::Consumed
            }
            MouseEventKind::ScrollDown => {
                self.tree_state.scroll_down(3);
                SidebarAction::Consumed
            }
            _ => SidebarAction::Consumed,
        }
    }

    /// Try to open the selected node (file → OpenFile action, dir → expand).
    fn try_open_selected(&mut self) -> SidebarAction {
        let Some(tree) = &mut self.file_tree else {
            return SidebarAction::Consumed;
        };

        let selected = self.tree_state.selected();
        if selected.is_empty() {
            return SidebarAction::Consumed;
        }

        let kind = tree.kind_for_id(selected);
        let path = tree.path_for_id(selected);

        match kind {
            Some(NodeKind::File) => {
                if let Some(p) = path {
                    SidebarAction::OpenFile(p)
                } else {
                    SidebarAction::Consumed
                }
            }
            Some(NodeKind::Directory) => {
                // Expand the directory
                if let Some(idx) = tree.find_node_by_id(selected) {
                    tree.expand(idx);
                }
                self.tree_state.toggle_selected();
                SidebarAction::Consumed
            }
            None => SidebarAction::Consumed,
        }
    }

    /// Expand the currently selected directory node.
    fn expand_selected(&mut self) {
        let Some(tree) = &mut self.file_tree else {
            return;
        };
        let selected = self.tree_state.selected();
        if selected.is_empty() {
            return;
        }
        if let Some(idx) = tree.find_node_by_id(selected) {
            tree.expand(idx);
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that the sidebar can emit to the editor shell.
#[derive(Debug)]
pub enum SidebarAction {
    /// The event was consumed internally.
    Consumed,
    /// The event was not handled.
    Ignored,
    /// Request to open a file at the given path.
    OpenFile(PathBuf),
    /// Return focus to the editor.
    FocusEditor,
}
