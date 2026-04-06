//! `EditorShell` — the root IDE layout component.
//!
//! Pushed onto the [`crate::Compositor`] as the base layer. Owns all
//! layout state and delegates rendering to sub-widgets.

use std::path::PathBuf;

use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect, Spacing};

use omni_event::Action;
use omni_loader::ThemeColors;

use crate::Component;
use crate::component::{CursorKind, EventResult};
use crate::context::Context;

use super::bottom_panel::BottomPanel;
use super::chat_panel::ChatPanel;
use super::editor_pane::{EditorPane, EditorViewport};
use super::file_tree::file_icon;
use super::hit_map::{DragBorder, HitMap, Panel};
use super::layout_state::{AppMode, LayoutState};
use super::mouse_state::MouseState;
use super::search_bar::SearchBar;
use super::sidebar::{Sidebar, SidebarAction};
use super::startup_screen::{StartupAction, StartupScreen};
use super::status_bar::StatusBar;
use super::tab_bar::{TabAction, TabBar, TabInfo};

/// Focus target within the editor shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Editor,
    Sidebar,
    BottomPanel,
    Chat,
}

/// The root IDE component implementing the full panel arrangement.
pub struct EditorShell {
    layout: LayoutState,
    tab_bar: TabBar,
    status_bar: StatusBar,
    sidebar: Sidebar,
    search_bar: SearchBar,
    startup: StartupScreen,
    hit_map: HitMap,
    mouse_state: MouseState,
    theme: ThemeColors,
    /// Currently focused panel.
    focus: FocusPanel,
    /// Active border drag operation.
    drag: Option<DragBorder>,
    /// Last cursor result from editor pane rendering.
    last_cursor: Option<(u16, u16, CursorKind)>,
    /// Whether any file has been opened (hides startup screen).
    has_opened_file: bool,
    /// Mapping from tab index → document path (for file switching).
    tab_paths: Vec<Option<PathBuf>>,
}

impl EditorShell {
    /// Create a new editor shell with default layout dimensions.
    #[must_use]
    pub fn new(theme: ThemeColors) -> Self {
        Self {
            layout: LayoutState::default(),
            tab_bar: TabBar::new(),
            status_bar: StatusBar::new(),
            sidebar: Sidebar::new(),
            search_bar: SearchBar::new(),
            startup: StartupScreen::new(),
            hit_map: HitMap::default(),
            mouse_state: MouseState::new(),
            theme,
            focus: FocusPanel::Editor,
            drag: None,
            last_cursor: None,
            has_opened_file: false,
            tab_paths: Vec::new(),
        }
    }

    // ── Layout computation ──────────────────────────────────────────

    /// Compute the editor shell layout and render all sub-widgets.
    #[allow(clippy::too_many_lines)]
    fn render_layout(&mut self, frame: &mut Frame, area: Rect, ctx: &Context) {
        self.hit_map.clear();

        // 1. Outer vertical: [main content | status bar (1 row)]
        let outer = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);
        let main_area = outer[0];
        let status_area = outer[1];

        // 2. App mode determines the main layout
        match self.layout.app_mode {
            AppMode::Ide => self.render_ide_mode(frame, main_area, ctx),
            AppMode::Chat => self.render_chat_mode(frame, main_area, ctx),
            AppMode::Split => self.render_split_mode(frame, main_area, ctx),
        }

        // 3. Status bar (always visible)
        self.hit_map.set(Panel::StatusBar, status_area);
        self.update_status_bar_state(ctx);
        self.status_bar.render(frame, status_area, &self.theme);
    }

    fn render_ide_mode(&mut self, frame: &mut Frame, main_area: Rect, ctx: &Context) {
        // Top-level vertical: [tab bar (1) | body (fill)]
        let outer = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .split(main_area);
        let tab_area = outer[0];
        let body_area = outer[1];

        // Render tab bar spanning full width
        self.hit_map.set(Panel::TabBar, tab_area);
        self.tab_bar.render(frame, tab_area, &self.theme);

        // Body horizontal: [sidebar | right region]
        let sidebar_w = self.layout.effective_sidebar_width();
        let (sidebar_area, right_area) = if sidebar_w > 0 {
            let h = Layout::horizontal([Constraint::Length(sidebar_w), Constraint::Fill(1)])
                .spacing(Spacing::Overlap(1))
                .split(body_area);
            (Some(h[0]), h[1])
        } else {
            (None, body_area)
        };

        // Right vertical: [editor (fill) | search? | bottom panel?]
        let bottom_h = self.layout.effective_bottom_height();
        let search_h: u16 = if self.search_bar.active { 1 } else { 0 };

        let mut constraints = vec![Constraint::Fill(1)];
        if search_h > 0 {
            constraints.push(Constraint::Length(search_h));
        }
        if bottom_h > 0 {
            constraints.push(Constraint::Length(bottom_h));
        }

        let right_chunks = Layout::vertical(constraints)
            .spacing(Spacing::Overlap(1))
            .split(right_area);

        let editor_area = right_chunks[0];

        let search_area = if search_h > 0 { Some(right_chunks[1]) } else { None };
        let bottom_area = if bottom_h > 0 {
            Some(right_chunks[if search_h > 0 { 2 } else { 1 }])
        } else {
            None
        };

        // Record hit map
        self.hit_map.set(Panel::EditorPane, editor_area);

        // Render sidebar
        if let Some(sb) = sidebar_area {
            self.hit_map.set(Panel::Sidebar, sb);
            self.sidebar.render(frame, sb, &self.theme);
        }

        // Render editor pane or startup screen
        if self.has_opened_file {
            let viewport = self.build_viewport(ctx);
            let result = EditorPane::render(
                frame,
                editor_area,
                viewport.as_ref(),
                self.layout.minimap_visible,
                self.layout.minimap_width,
                self.focus == FocusPanel::Editor,
                &self.theme,
            );
            self.last_cursor = result.cursor;
        } else {
            self.startup.render(frame, editor_area, &self.theme);
            self.last_cursor = None;
        }

        // Render search bar
        if let Some(sa) = search_area {
            self.search_bar.render(frame, sa, &self.theme);
        }

        // Render bottom panel
        if let Some(bp) = bottom_area {
            self.hit_map.set(Panel::Bottom, bp);
            BottomPanel::render(frame, bp, self.focus == FocusPanel::BottomPanel, &self.theme);
        }
    }

    fn render_chat_mode(&mut self, frame: &mut Frame, main_area: Rect, ctx: &Context) {
        self.hit_map.set(Panel::Chat, main_area);
        ChatPanel::render(frame, main_area, true, &self.theme);
        let _ = ctx; // suppress unused warning
    }

    fn render_split_mode(&mut self, frame: &mut Frame, main_area: Rect, ctx: &Context) {
        let ratio = self.layout.split_ratio;
        let editor_pct = Constraint::Percentage(ratio);
        let chat_pct = Constraint::Percentage(100 - ratio);
        let chunks = Layout::horizontal([editor_pct, Constraint::Length(1), chat_pct])
            .split(main_area);

        let editor_region = chunks[0];
        let divider = chunks[1];
        let chat_region = chunks[2];

        self.hit_map.set(Panel::EditorPane, editor_region);
        self.hit_map.set_split_divider(divider);
        self.hit_map.set(Panel::Chat, chat_region);

        // Editor side: tab bar + editor
        let editor_chunks = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .split(editor_region);

        self.hit_map.set(Panel::TabBar, editor_chunks[0]);
        self.tab_bar.render(frame, editor_chunks[0], &self.theme);

        if self.has_opened_file {
            let viewport = self.build_viewport(ctx);
            let result = EditorPane::render(
                frame,
                editor_chunks[1],
                viewport.as_ref(),
                self.layout.minimap_visible,
                self.layout.minimap_width,
                self.focus == FocusPanel::Editor,
                &self.theme,
            );
            self.last_cursor = result.cursor;
        } else {
            self.startup.render(frame, editor_chunks[1], &self.theme);
            self.last_cursor = None;
        }

        // Divider
        let divider_widget = ratatui::widgets::Paragraph::new("\u{2502}".repeat(divider.height as usize))
            .style(ratatui::style::Style::new().fg(self.theme.border));
        frame.render_widget(divider_widget, divider);

        // Chat side
        ChatPanel::render(frame, chat_region, self.focus == FocusPanel::Chat, &self.theme);
    }

    // ── Viewport builder ────────────────────────────────────────────

    fn build_viewport<'a>(&'a self, ctx: &'a Context<'a>) -> Option<EditorViewport<'a>> {
        let focus_key = ctx.view_tree.focus()?;
        let node = ctx.view_tree.get(focus_key)?;
        let omni_view::view_tree::Node::Leaf(view) = node else {
            return None;
        };
        let doc = ctx.documents.get(view.doc_id)?;

        Some(EditorViewport {
            text: doc.text(),
            highlight_spans: &doc.highlight_spans,
            ai_touched_lines: &doc.ai_touched_lines,
            selection: doc.selection(focus_key),
            scroll_offset: view.scroll_offset,
            col_offset: view.col_offset,
            total_lines: doc.text().len_lines(),
            config: ctx.config,
            search_matches: &self.search_bar.matches,
            current_match_idx: if self.search_bar.active {
                Some(self.search_bar.current_match)
            } else {
                None
            },
        })
    }

    // ── Status bar state sync ───────────────────────────────────────

    fn update_status_bar_state(&mut self, ctx: &Context) {
        self.status_bar.state.app_mode = self.layout.app_mode;

        if let Some(focus_key) = ctx.view_tree.focus() {
            if let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) {
                if let Some(doc) = ctx.documents.get(view.doc_id) {
                    let text = doc.text();
                    let sel = doc.selection(focus_key);
                    let head = sel.primary().head;
                    let line = if text.len_chars() > 0 {
                        text.char_to_line(head.min(text.len_chars().saturating_sub(1)))
                    } else {
                        0
                    };
                    let col = head.saturating_sub(text.line_to_char(line));

                    self.status_bar.state.cursor_line = line + 1;
                    self.status_bar.state.cursor_col = col + 1;
                    self.status_bar.state.modified = doc.is_modified();

                    if let Some(path) = &doc.path {
                        self.status_bar.state.filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("[unnamed]")
                            .to_string();
                    } else {
                        self.status_bar.state.filename = "[scratch]".to_string();
                    }

                    if let Some(ref lang) = doc.language {
                        self.status_bar.state.language = lang.clone();
                    }
                }
            }
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────

    fn open_folder_picker(&self) -> EventResult {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let theme = self.theme.clone();
        EventResult::Callback(Box::new(move |compositor| {
            let picker = super::folder_picker::FolderPicker::new(start, theme);
            let _ = compositor.push(Box::new(picker));
        }))
    }

    fn open_command_palette(&self) -> EventResult {
        let theme = self.theme.clone();
        EventResult::Callback(Box::new(move |compositor| {
            let palette = super::command_palette::CommandPalette::new(theme);
            let _ = compositor.push(Box::new(palette));
        }))
    }

    fn open_goto_line(&self, ctx: &Context) -> EventResult {
        let total_lines = ctx
            .view_tree
            .focus()
            .and_then(|k| ctx.view_tree.get(k))
            .and_then(|node| {
                if let omni_view::view_tree::Node::Leaf(view) = node {
                    ctx.documents.get(view.doc_id)
                } else {
                    None
                }
            })
            .map_or(1, |doc| doc.text().len_lines());

        let theme = self.theme.clone();
        EventResult::Callback(Box::new(move |compositor| {
            let popup = super::goto_line::GotoLinePopup::new(theme, total_lines);
            let _ = compositor.push(Box::new(popup));
        }))
    }

    fn open_goto_symbol(&self, ctx: &Context) -> EventResult {
        let symbols = ctx
            .view_tree
            .focus()
            .and_then(|k| ctx.view_tree.get(k))
            .and_then(|node| {
                if let omni_view::view_tree::Node::Leaf(view) = node {
                    ctx.documents.get(view.doc_id)
                } else {
                    None
                }
            })
            .and_then(|doc| {
                doc.syntax.as_ref().map(|syn| {
                    syn.tree().map(|t| omni_syntax::extract_symbols(t, doc.text())).unwrap_or_default()
                })
            })
            .unwrap_or_default();

        let theme = self.theme.clone();
        EventResult::Callback(Box::new(move |compositor| {
            let popup = super::symbol_picker::SymbolPickerPopup::new(symbols, theme);
            let _ = compositor.push(Box::new(popup));
        }))
    }

    fn process_startup_action(&mut self, action: StartupAction) -> EventResult {
        match action {
            StartupAction::OpenFolder => self.open_folder_picker(),
            StartupAction::NewFile => {
                self.tab_bar.add_tab(TabInfo::new("untitled", "\u{f15c}"));
                self.tab_paths.push(None);
                self.has_opened_file = true;
                EventResult::Consumed
            }
            StartupAction::AiChat => {
                self.layout.cycle_mode();
                EventResult::Consumed
            }
            StartupAction::CommandPalette => self.open_command_palette(),
        }
    }

    fn open_file_in_tab(&mut self, path: &std::path::Path, ctx: &mut Context) {
        // Check if already open in a tab
        for (i, tab_path) in self.tab_paths.iter().enumerate() {
            if tab_path.as_deref() == Some(path) {
                self.tab_bar.set_active(i);
                self.switch_to_tab(i, ctx);
                return;
            }
        }

        // Load the document from disk
        let mut doc = match omni_view::Document::from_file(path) {
            Ok(doc) => doc,
            Err(e) => {
                tracing::error!(?path, ?e, "failed to open file");
                return;
            }
        };

        let doc_id = doc.id;
        let language = doc.language.clone();
        tracing::info!(?path, ?doc_id, ?language, "opened file");

        // Parse syntax highlighting (skip for large files)
        if !doc.is_large_file(ctx.config.large_file_threshold) {
            if let Some(ref lang_id) = language {
                if let Some(mut highlighter) = ctx.language_registry.create_highlighter(lang_id) {
                    if let Some((tree, spans)) = highlighter.parse_full(doc.text()) {
                        doc.syntax = Some(omni_syntax::SyntaxTree::from_tree(tree));
                        doc.highlight_spans = spans;
                        tracing::info!(?doc_id, spans = doc.highlight_spans.len(), "syntax parsed");
                    }
                }
            }
        }

        // Insert into document store
        ctx.documents.insert(doc);

        // Create a view for this document
        let view = omni_view::View::new(doc_id, 80, 24);
        ctx.view_tree.set_root(view);

        // Add the tab
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed");
        let icon = file_icon(name, super::file_tree::NodeKind::File);
        self.tab_bar.add_tab(TabInfo::new(name, icon));
        self.tab_paths.push(Some(path.to_path_buf()));
        self.has_opened_file = true;

        ctx.request_redraw();
    }

    fn switch_to_tab(&mut self, idx: usize, ctx: &mut Context) {
        self.tab_bar.set_active(idx);

        // Find the document for this tab and set it as the active view
        if let Some(Some(path)) = self.tab_paths.get(idx) {
            if let Some(doc_id) = ctx.documents.find_by_path(path) {
                // Try to update the existing focused view
                if let Some(focus_key) = ctx.view_tree.focus() {
                    if let Some(omni_view::view_tree::Node::Leaf(view)) =
                        ctx.view_tree.get_mut(focus_key)
                    {
                        view.doc_id = doc_id;
                        return;
                    }
                }
                // No existing view — create one
                let view = omni_view::View::new(doc_id, 80, 24);
                ctx.view_tree.set_root(view);
            }
        }
    }

    fn close_tab(&mut self, idx: usize, ctx: &mut Context) {
        if idx >= self.tab_paths.len() {
            return;
        }
        self.tab_paths.remove(idx);
        self.tab_bar.close_tab(idx);

        if self.tab_bar.is_empty() {
            self.has_opened_file = false;
        } else {
            let new_active = self.tab_bar.active_index();
            self.switch_to_tab(new_active, ctx);
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPanel::Editor => {
                if self.layout.sidebar_visible {
                    FocusPanel::Sidebar
                } else if self.layout.bottom_visible {
                    FocusPanel::BottomPanel
                } else {
                    FocusPanel::Editor
                }
            }
            FocusPanel::Sidebar => {
                if self.layout.bottom_visible {
                    FocusPanel::BottomPanel
                } else {
                    FocusPanel::Editor
                }
            }
            FocusPanel::BottomPanel => FocusPanel::Editor,
            FocusPanel::Chat => FocusPanel::Editor,
        };
    }
}

impl Default for EditorShell {
    fn default() -> Self {
        let theme_def = omni_loader::Theme::by_name("onedark");
        let capability = omni_loader::detect_color_capability();
        Self::new(ThemeColors::from_theme(&theme_def, capability))
    }
}

impl Component for EditorShell {
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &Context) {
        self.render_layout(frame, area, ctx);
    }

    fn cursor(&self) -> Option<(u16, u16, CursorKind)> {
        self.last_cursor
    }

    #[allow(clippy::too_many_lines)]
    fn handle_key(
        &mut self,
        event: KeyEvent,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        // Search bar intercepts keys when active
        if self.search_bar.active {
            if self.search_bar.handle_key(event) {
                // Update search matches
                if let Some(focus_key) = ctx.view_tree.focus() {
                    if let Some(omni_view::view_tree::Node::Leaf(view)) =
                        ctx.view_tree.get(focus_key)
                    {
                        if let Some(doc) = ctx.documents.get(view.doc_id) {
                            self.search_bar.force_update(doc.text(), doc.version);
                        }
                    }
                }
                return Ok(EventResult::Consumed);
            }
        }

        // Sidebar intercepts keys when focused
        if self.focus == FocusPanel::Sidebar {
            match self.sidebar.handle_key(event) {
                SidebarAction::Consumed => return Ok(EventResult::Consumed),
                SidebarAction::OpenFile(path) => {
                    self.focus = FocusPanel::Editor;
                    return Ok(EventResult::Action(Action::OpenFile(path)));
                }
                SidebarAction::FocusEditor => {
                    self.focus = FocusPanel::Editor;
                    return Ok(EventResult::Consumed);
                }
                SidebarAction::Ignored => {}
            }
        }

        // Startup screen intercepts keys when visible
        if !self.has_opened_file {
            if let Some(action) = self.startup.handle_key(event) {
                return Ok(self.process_startup_action(action));
            }
        }

        // Global keybindings are handled by the keymap in the event loop.
        // Here we only handle keys not covered by the keymap.
        Ok(EventResult::Ignored)
    }

    #[allow(clippy::too_many_lines)]
    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        _area: Rect,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        // Handle drag in progress
        if let Some(border) = self.drag {
            match event.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    match border {
                        DragBorder::SidebarRight => {
                            let terminal_w = _area.width;
                            self.layout.set_sidebar_width(event.column, terminal_w);
                        }
                        DragBorder::BottomTop => {
                            let terminal_h = _area.height;
                            let new_h = terminal_h.saturating_sub(event.row).saturating_sub(1);
                            self.layout.set_bottom_height(new_h, terminal_h);
                        }
                        DragBorder::SplitDivider => {
                            let ratio = ((event.column as u32) * 100 / _area.width as u32) as u16;
                            self.layout.set_split_ratio(ratio);
                        }
                    }
                    return Ok(EventResult::Consumed);
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.drag = None;
                    return Ok(EventResult::Consumed);
                }
                _ => {}
            }
        }

        // Check for border drag initiation
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            if let Some(border) = self.hit_map.border_at(event.column, event.row) {
                self.drag = Some(border);
                return Ok(EventResult::Consumed);
            }
        }

        // Dispatch to panels based on hit map
        let panel = self.hit_map.panel_at(event.column, event.row);

        match panel {
            Some(Panel::TabBar) => {
                if let Some(action) = self.tab_bar.handle_mouse(event) {
                    match action {
                        TabAction::Switch(idx) => {
                            self.switch_to_tab(idx, ctx);
                        }
                        TabAction::Close(idx) => {
                            self.close_tab(idx, ctx);
                        }
                        TabAction::Reorder { from, to } => {
                            self.tab_bar.reorder(from, to);
                            // Also reorder tab_paths
                            if from < self.tab_paths.len() && to < self.tab_paths.len() {
                                let item = self.tab_paths.remove(from);
                                self.tab_paths.insert(to, item);
                            }
                        }
                        TabAction::Handled => {}
                    }
                    return Ok(EventResult::Consumed);
                }
            }
            Some(Panel::Sidebar) => {
                self.focus = FocusPanel::Sidebar;
                let action = self.sidebar.handle_mouse(event);
                match action {
                    SidebarAction::OpenFile(path) => {
                        return Ok(EventResult::Action(Action::OpenFile(path)));
                    }
                    _ => return Ok(EventResult::Consumed),
                }
            }
            Some(Panel::EditorPane) => {
                self.focus = FocusPanel::Editor;
                // Handle scroll events in editor
                match event.kind {
                    MouseEventKind::ScrollUp => {
                        return Ok(EventResult::Action(Action::ScrollUp));
                    }
                    MouseEventKind::ScrollDown => {
                        return Ok(EventResult::Action(Action::ScrollDown));
                    }
                    MouseEventKind::Down(MouseButton::Right) => {
                        // Context menu
                        let items = vec![
                            super::context_menu::MenuItem::new("Cut", Action::Cut),
                            super::context_menu::MenuItem::new("Copy", Action::Copy),
                            super::context_menu::MenuItem::new("Paste", Action::Paste),
                            super::context_menu::MenuItem::new("Select All", Action::SelectAll),
                        ];
                        let menu =
                            super::context_menu::ContextMenu::new(items, event.column, event.row);
                        return Ok(EventResult::Callback(Box::new(move |compositor| {
                            let _ = compositor.push(Box::new(menu));
                        })));
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let _click_count =
                            self.mouse_state.record_click(MouseButton::Left, event.column, event.row);
                        // TODO: position cursor at click location, handle double/triple click
                        return Ok(EventResult::Consumed);
                    }
                    _ => return Ok(EventResult::Consumed),
                }
            }
            Some(Panel::Bottom) => {
                self.focus = FocusPanel::BottomPanel;
                return Ok(EventResult::Consumed);
            }
            Some(Panel::StatusBar) => {
                if let Some(action) = self.status_bar.handle_mouse(event) {
                    match action {
                        super::status_bar::StatusBarAction::CycleAppMode => {
                            self.layout.cycle_mode();
                        }
                        _ => {}
                    }
                    return Ok(EventResult::Consumed);
                }
            }
            Some(Panel::Chat) => {
                self.focus = FocusPanel::Chat;
                return Ok(EventResult::Consumed);
            }
            None => {}
        }

        // Startup screen mouse handling
        if !self.has_opened_file {
            if let Some(action) = self.startup.handle_mouse(event) {
                return Ok(self.process_startup_action(action));
            }
        }

        Ok(EventResult::Ignored)
    }

    fn handle_action(
        &mut self,
        action: &Action,
        ctx: &mut Context,
    ) -> color_eyre::Result<EventResult> {
        match action {
            Action::ToggleSidebar => {
                if self.layout.sidebar_visible && self.focus != FocusPanel::Sidebar {
                    // Sidebar visible but not focused → focus it
                    self.focus = FocusPanel::Sidebar;
                } else if self.layout.sidebar_visible && self.focus == FocusPanel::Sidebar {
                    // Sidebar visible and focused → hide it, return to editor
                    self.layout.toggle_sidebar();
                    self.focus = FocusPanel::Editor;
                } else {
                    // Sidebar hidden → show and focus it
                    self.layout.toggle_sidebar();
                    self.focus = FocusPanel::Sidebar;
                }
                Ok(EventResult::Consumed)
            }
            Action::ToggleBottomPanel => {
                self.layout.toggle_bottom_panel();
                if !self.layout.bottom_visible && self.focus == FocusPanel::BottomPanel {
                    self.focus = FocusPanel::Editor;
                }
                Ok(EventResult::Consumed)
            }
            Action::ToggleMinimap => {
                self.layout.toggle_minimap();
                Ok(EventResult::Consumed)
            }
            Action::ToggleAppMode => {
                self.layout.cycle_mode();
                Ok(EventResult::Consumed)
            }
            Action::FocusNext => {
                self.cycle_focus();
                Ok(EventResult::Consumed)
            }
            Action::FocusPrev => {
                // Reverse cycle
                self.focus = match self.focus {
                    FocusPanel::Editor => {
                        if self.layout.bottom_visible {
                            FocusPanel::BottomPanel
                        } else if self.layout.sidebar_visible {
                            FocusPanel::Sidebar
                        } else {
                            FocusPanel::Editor
                        }
                    }
                    FocusPanel::Sidebar => FocusPanel::Editor,
                    FocusPanel::BottomPanel => {
                        if self.layout.sidebar_visible {
                            FocusPanel::Sidebar
                        } else {
                            FocusPanel::Editor
                        }
                    }
                    FocusPanel::Chat => FocusPanel::Editor,
                };
                Ok(EventResult::Consumed)
            }
            Action::OpenFile(path) => {
                self.open_file_in_tab(path, ctx);
                Ok(EventResult::Consumed)
            }
            Action::OpenFolder(path) => {
                self.sidebar.set_root(path);
                self.layout.sidebar_visible = true;
                ctx.workspace_root = Some(path.clone());
                Ok(EventResult::Consumed)
            }
            Action::CloseBuffer => {
                let idx = self.tab_bar.active_index();
                self.close_tab(idx, ctx);
                Ok(EventResult::Consumed)
            }
            Action::SwitchTab(idx) => {
                self.switch_to_tab(*idx, ctx);
                Ok(EventResult::Consumed)
            }
            Action::CloseTab(idx) => {
                self.close_tab(*idx, ctx);
                Ok(EventResult::Consumed)
            }
            Action::ReorderTab { from, to } => {
                self.tab_bar.reorder(*from, *to);
                if *from < self.tab_paths.len() && *to < self.tab_paths.len() {
                    let item = self.tab_paths.remove(*from);
                    self.tab_paths.insert(*to, item);
                }
                Ok(EventResult::Consumed)
            }
            Action::CommandPalette => Ok(self.open_command_palette()),
            Action::Find => {
                self.search_bar.activate();
                Ok(EventResult::Consumed)
            }
            Action::FindNext => {
                self.search_bar.next_match();
                // Scroll to match
                if let Some(pos) = self.search_bar.current_match_pos() {
                    if let Some(focus_key) = ctx.view_tree.focus() {
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get(focus_key)
                        {
                            if let Some(doc) = ctx.documents.get(view.doc_id) {
                                let line = doc.text().char_to_line(pos);
                                if let Some(omni_view::view_tree::Node::Leaf(v)) =
                                    ctx.view_tree.get_mut(focus_key)
                                {
                                    v.ensure_visible(line);
                                }
                            }
                        }
                    }
                }
                Ok(EventResult::Consumed)
            }
            Action::FindPrev => {
                self.search_bar.prev_match();
                if let Some(pos) = self.search_bar.current_match_pos() {
                    if let Some(focus_key) = ctx.view_tree.focus() {
                        if let Some(omni_view::view_tree::Node::Leaf(view)) =
                            ctx.view_tree.get(focus_key)
                        {
                            if let Some(doc) = ctx.documents.get(view.doc_id) {
                                let line = doc.text().char_to_line(pos);
                                if let Some(omni_view::view_tree::Node::Leaf(v)) =
                                    ctx.view_tree.get_mut(focus_key)
                                {
                                    v.ensure_visible(line);
                                }
                            }
                        }
                    }
                }
                Ok(EventResult::Consumed)
            }
            Action::GotoLine => Ok(self.open_goto_line(ctx)),
            Action::GotoSymbol => Ok(self.open_goto_symbol(ctx)),
            Action::Command(cmd) => {
                match cmd.as_str() {
                    "new_file" => {
                        self.tab_bar.add_tab(TabInfo::new("untitled", "\u{f15c}"));
                        self.tab_paths.push(None);
                        self.has_opened_file = true;
                    }
                    "open_folder" => return Ok(self.open_folder_picker()),
                    _ => tracing::debug!(%cmd, "unhandled command"),
                }
                Ok(EventResult::Consumed)
            }
            _ => Ok(EventResult::Ignored),
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
