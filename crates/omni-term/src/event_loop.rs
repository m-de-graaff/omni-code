//! Main event loop: async `tokio::select!` with event-driven rendering.

use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use ratatui::layout::Rect;
use tokio::sync::{broadcast, mpsc};

use omni_event::Action;

use crate::Compositor;
use crate::chord_state::{ChordOutcome, ChordState, crossterm_to_chord};
use crate::component::EventResult;
use crate::context::{Callback, Context};
use crate::terminal::TerminalModeGuard;

use omni_syntax::LanguageRegistry;

/// Run the main async event loop.
///
/// Uses `tokio::select!` to multiplex five event sources:
/// 1. Terminal events (keyboard, mouse, resize, paste)
/// 2. Actions from background tasks (AI, LSP, file watcher)
/// 3. Compositor callbacks
/// 4. Periodic tick for animations/spinners
/// 5. Ctrl+C signal for graceful shutdown
///
/// Rendering is event-driven: the terminal is only redrawn when state changes.
///
/// # Errors
/// Returns an error if terminal I/O or event handling fails.
#[allow(clippy::too_many_lines)]
pub async fn run(
    terminal: &mut DefaultTerminal,
    compositor: &mut Compositor,
    ctx: &mut Context<'_>,
    action_rx: &mut broadcast::Receiver<Action>,
    callback_rx: &mut mpsc::UnboundedReceiver<Callback>,
) -> color_eyre::Result<()> {
    // Enable mouse capture + bracketed paste (cleaned up on drop)
    let _mode_guard = TerminalModeGuard::enable()?;

    // Initialize compositor with the terminal size
    let size = terminal.size()?;
    compositor.resize(Rect::new(0, 0, size.width, size.height))?;

    // Chord state machine for multi-key sequences
    let mut chord_state = ChordState::new();

    // Async terminal event stream
    let mut terminal_events = EventStream::new();

    // Tick interval for animations/spinners
    let tick_rate = Duration::from_millis(ctx.config.tick_rate_ms);
    let mut tick_interval = tokio::time::interval(tick_rate);

    // Swap file timer (every 30 seconds)
    let mut swap_interval = tokio::time::interval(Duration::from_secs(30));
    swap_interval.tick().await; // consume first immediate tick

    // File watcher for external change detection
    let (fw_tx, mut fw_rx) = mpsc::unbounded_channel::<std::path::PathBuf>();
    let mut file_watcher = setup_file_watcher(fw_tx);

    // Ctrl+C signal for graceful shutdown
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // Initial render
    terminal.draw(|frame| compositor.render(frame, ctx))?;
    ctx.needs_redraw = false;

    loop {
        tokio::select! {
            // Branch 1: Terminal events (keyboard, mouse, resize, paste)
            maybe_event = terminal_events.next() => {
                let Some(event_result) = maybe_event else {
                    // Stream ended — terminal closed
                    break;
                };
                let event = event_result?;

                // Handle paste events directly
                if let Event::Paste(ref paste_text) = event {
                    handle_paste_event(paste_text, ctx);
                    ctx.needs_redraw = true;
                    continue;
                }

                // Let the compositor handle the event (modals get priority)
                let result = compositor.handle_event(&event, ctx)?;

                match result {
                    EventResult::Consumed => {
                        // A modal or component consumed the key — cancel any pending chord
                        chord_state.cancel();
                        ctx.needs_redraw = true;
                    }
                    EventResult::Ignored => {
                        // No component consumed it — try the keymap
                        if let Event::Key(ref key_event) = event {
                            if key_event.kind == KeyEventKind::Press {
                                if let Some(chord) = crossterm_to_chord(key_event) {
                                    match chord_state.feed(chord, ctx.keymap, ctx.keymap_mode) {
                                        ChordOutcome::Matched(action_name) => {
                                            if let Some(action) = omni_loader::resolve_action(&action_name) {
                                                handle_action(&action, compositor, ctx)?;
                                            } else {
                                                tracing::warn!(action = %action_name, "unknown action in keymap");
                                            }
                                        }
                                        ChordOutcome::Pending(_) => {
                                            // Swallow key, wait for next chord
                                        }
                                        ChordOutcome::PassThrough => {
                                            // No binding — try character insertion
                                            if let Event::Key(ref ke) = event {
                                                try_insert_char(ke, ctx);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    EventResult::Action(action) => {
                        chord_state.cancel();
                        if let Err(err) = handle_action(&action, compositor, ctx) {
                            tracing::error!(?err, ?action, "failed to handle action");
                        }
                        ctx.needs_redraw = true;
                    }
                    EventResult::Callback(cb) => {
                        chord_state.cancel();
                        cb(compositor);
                        ctx.needs_redraw = true;
                    }
                }
            }

            // Branch 2: Actions from background tasks (AI, LSP, file watcher)
            result = action_rx.recv() => {
                match result {
                    Ok(action) => {
                        handle_action(&action, compositor, ctx)?;
                        ctx.needs_redraw = true;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "action receiver lagged, some events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("action bus closed, shutting down");
                        break;
                    }
                }
            }

            // Branch 3: Compositor callbacks
            Some(callback) = callback_rx.recv() => {
                callback(compositor);
                ctx.needs_redraw = true;
            }

            // Branch 4: Periodic tick for animations/spinners
            _ = tick_interval.tick() => {
                // Check for chord timeout
                if chord_state.check_timeout() {
                    ctx.needs_redraw = true;
                }
                // Redraw on tick for status message auto-dismiss and other time-based UI
                // (only when there's something that might need updating)
                ctx.needs_redraw = true; // TODO: optimize to only redraw when status message is active
            }

            // Branch 5: Periodic swap file writes
            _ = swap_interval.tick() => {
                write_swap_files(ctx);
            }

            // Branch 6: File watcher events
            Some(changed_path) = fw_rx.recv() => {
                handle_action(&Action::FileChanged(changed_path), compositor, ctx)?;
                ctx.needs_redraw = true;
            }

            // Branch 6: Graceful shutdown on Ctrl+C
            _ = &mut ctrl_c => {
                tracing::info!("received Ctrl+C, shutting down");
                break;
            }
        }

        // Check quit condition
        if ctx.should_quit {
            break;
        }

        // Watch workspace root if newly set
        if let Some(ref root) = ctx.workspace_root {
            if let Some(ref mut watcher) = file_watcher {
                use notify::{RecursiveMode, Watcher};
                let _ = watcher.watch(root, RecursiveMode::Recursive);
            }
        }

        // Event-driven rendering: only redraw when state changed
        if ctx.needs_redraw {
            terminal.draw(|frame| compositor.render(frame, ctx))?;
            compositor.mark_redrawn();
            ctx.needs_redraw = false;
        }
    }

    Ok(())
}

/// Process a global action.
///
/// # Errors
/// Returns an error if action handling fails.
fn handle_action(
    action: &Action,
    compositor: &mut Compositor,
    ctx: &mut Context,
) -> color_eyre::Result<()> {
    match action {
        Action::Quit => ctx.quit(),
        Action::Resize { width, height } => {
            let area = Rect::new(0, 0, *width, *height);
            compositor.resize(area)?;
        }
        Action::OpenFile(path) => {
            // EditorShell handles document loading, tab creation, and view setup
            let result = compositor.dispatch_action(action, ctx)?;
            if let EventResult::Action(nested) = result {
                handle_action(&nested, compositor, ctx)?;
            }
            // Watch the file for external changes (path is borrowed from the Action)
            // We can't pass the watcher here, so we store the path for watching below
        }
        Action::Save => {
            handle_save(ctx);
        }
        Action::SaveAs(path) => {
            handle_save_as(path, ctx);
        }
        Action::Undo => {
            handle_undo_redo(ctx, true);
        }
        Action::Redo => {
            handle_undo_redo(ctx, false);
        }
        Action::ScrollUp | Action::ScrollDown | Action::PageUp | Action::PageDown => {
            handle_scroll(action, ctx);
        }
        // Cursor movement actions
        Action::CursorLeft | Action::CursorRight | Action::CursorUp | Action::CursorDown
        | Action::CursorWordLeft | Action::CursorWordRight
        | Action::CursorLineStart | Action::CursorLineEnd
        | Action::CursorDocStart | Action::CursorDocEnd => {
            handle_cursor_action(action, ctx);
        }
        // Selection actions
        Action::SelectLeft | Action::SelectRight | Action::SelectUp | Action::SelectDown
        | Action::SelectWordLeft | Action::SelectWordRight
        | Action::SelectLineStart | Action::SelectLineEnd
        | Action::SelectWord | Action::SelectLine | Action::SelectAll => {
            handle_selection_action(action, ctx);
        }
        // Text editing actions
        Action::Backspace | Action::Delete
        | Action::DeleteWordBackward | Action::DeleteWordForward
        | Action::InsertNewline | Action::InsertTab
        | Action::IndentSelection | Action::OutdentSelection
        | Action::DuplicateLine | Action::MoveLineUp | Action::MoveLineDown
        | Action::ToggleComment
        | Action::Cut | Action::Copy | Action::Paste => {
            handle_editing_action(action, ctx);
        }
        // Actions routable to component layers
        Action::FocusNext
        | Action::FocusPrev
        | Action::NextTab
        | Action::PrevTab
        | Action::NavigateBack
        | Action::NavigateForward
        | Action::FilePicker
        | Action::ProjectSearch
        | Action::ToggleSidebar
        | Action::ToggleBottomPanel
        | Action::ToggleMinimap
        | Action::ToggleAppMode
        | Action::OpenFolder(_)
        | Action::CommandPalette
        | Action::CloseBuffer
        | Action::SelectNextOccurrence
        | Action::ExpandSelection
        | Action::ShrinkSelection
        | Action::Find
        | Action::FindNext
        | Action::FindPrev
        | Action::FindReplace
        | Action::ReplaceOne
        | Action::ReplaceAll
        | Action::ProjectSearch
        | Action::GotoLine
        | Action::GotoSymbol
        | Action::Command(_) => {
            let result = compositor.dispatch_action(action, ctx)?;
            if let EventResult::Action(nested) = result {
                handle_action(&nested, compositor, ctx)?;
            }
        }
        Action::FileChanged(path) => {
            handle_file_changed(path, ctx);
        }
        _ => {
            tracing::debug!(?action, "unhandled action");
        }
    }
    Ok(())
}

/// Try to insert a character when the keymap returns `PassThrough`.
/// Only inserts for plain Char keys (no modifiers except Shift).
fn try_insert_char(key: &crossterm::event::KeyEvent, ctx: &mut Context) {
    // Only handle plain character keys (no Ctrl/Alt/Super)
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_super = key.modifiers.contains(KeyModifiers::SUPER);
    if has_ctrl || has_alt || has_super {
        return;
    }

    let KeyCode::Char(ch) = key.code else {
        return;
    };

    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(_)) = ctx.view_tree.get(focus_key) else {
        return;
    };

    let (doc_id, txn) = {
        let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
            return;
        };
        let Some(doc) = ctx.documents.get(view.doc_id) else {
            return;
        };
        (view.doc_id, crate::editing::insert_char(doc, focus_key, ch))
    };

    if let Some(doc) = ctx.documents.get_mut(doc_id) {
        doc.apply(&txn, focus_key);
        rehighlight(doc, ctx.language_registry);
    }
    ensure_cursor_in_view(ctx);
    ctx.request_redraw();
}

/// Handle paste events by inserting the pasted text.
fn handle_paste_event(text: &str, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };

    let (doc_id, txn) = {
        let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
            return;
        };
        let Some(doc) = ctx.documents.get(view.doc_id) else {
            return;
        };
        (view.doc_id, crate::editing::insert_text(doc, focus_key, text))
    };

    if let Some(doc) = ctx.documents.get_mut(doc_id) {
        doc.apply(&txn, focus_key);
        rehighlight(doc, ctx.language_registry);
    }
    ensure_cursor_in_view(ctx);
    ctx.request_redraw();
}

/// Handle cursor movement actions.
fn handle_cursor_action(action: &Action, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let Some(doc) = ctx.documents.get(view.doc_id) else {
        return;
    };

    let text = doc.text();
    let sel = doc.selection(focus_key);

    let new_sel = match action {
        Action::CursorLeft => crate::cursor::move_left(text, &sel),
        Action::CursorRight => crate::cursor::move_right(text, &sel),
        Action::CursorUp => crate::cursor::move_up(text, &sel),
        Action::CursorDown => crate::cursor::move_down(text, &sel),
        Action::CursorWordLeft => crate::cursor::move_word_left(text, &sel),
        Action::CursorWordRight => crate::cursor::move_word_right(text, &sel),
        Action::CursorLineStart => crate::cursor::move_line_start(text, &sel),
        Action::CursorLineEnd => crate::cursor::move_line_end(text, &sel),
        Action::CursorDocStart => crate::cursor::move_doc_start(&sel),
        Action::CursorDocEnd => crate::cursor::move_doc_end(text, &sel),
        _ => return,
    };

    let doc_id = view.doc_id;
    if let Some(doc) = ctx.documents.get_mut(doc_id) {
        doc.set_selection(focus_key, new_sel);
    }
    ensure_cursor_in_view(ctx);
    ctx.request_redraw();
}

/// Handle selection extension actions.
fn handle_selection_action(action: &Action, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let Some(doc) = ctx.documents.get(view.doc_id) else {
        return;
    };

    let text = doc.text();
    let sel = doc.selection(focus_key);

    let new_sel = match action {
        Action::SelectLeft => crate::cursor::select_left(text, &sel),
        Action::SelectRight => crate::cursor::select_right(text, &sel),
        Action::SelectUp => crate::cursor::select_up(text, &sel),
        Action::SelectDown => crate::cursor::select_down(text, &sel),
        Action::SelectWordLeft => crate::cursor::select_word_left(text, &sel),
        Action::SelectWordRight => crate::cursor::select_word_right(text, &sel),
        Action::SelectLineStart => crate::cursor::select_line_start(text, &sel),
        Action::SelectLineEnd => crate::cursor::select_line_end(text, &sel),
        Action::SelectWord => crate::cursor::select_word(text, &sel),
        Action::SelectLine => crate::cursor::select_line(text, &sel),
        Action::SelectAll => omni_core::Selection::select_all(text.len_chars()),
        _ => return,
    };

    let doc_id = view.doc_id;
    if let Some(doc) = ctx.documents.get_mut(doc_id) {
        doc.set_selection(focus_key, new_sel);
    }
    ensure_cursor_in_view(ctx);
    ctx.request_redraw();
}

/// Handle text editing actions (mutations that create transactions).
#[allow(clippy::too_many_lines)]
fn handle_editing_action(action: &Action, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let doc_id = view.doc_id;
    let Some(doc) = ctx.documents.get(doc_id) else {
        return;
    };

    match action {
        Action::Backspace => {
            if let Some(txn) = crate::editing::delete_backward(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::Delete => {
            if let Some(txn) = crate::editing::delete_forward(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::DeleteWordBackward => {
            if let Some(txn) = crate::editing::delete_word_backward(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::DeleteWordForward => {
            if let Some(txn) = crate::editing::delete_word_forward(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::InsertNewline => {
            let txn = crate::editing::insert_newline(doc, focus_key, ctx.config);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::InsertTab => {
            let txn = crate::editing::insert_tab(doc, focus_key, ctx.config);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::IndentSelection => {
            let txn = crate::editing::indent_lines(doc, focus_key, ctx.config);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::OutdentSelection => {
            let txn = crate::editing::outdent_lines(doc, focus_key, ctx.config);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::DuplicateLine => {
            let txn = crate::editing::duplicate_line(doc, focus_key);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::MoveLineUp => {
            if let Some(txn) = crate::editing::move_line_up(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::MoveLineDown => {
            if let Some(txn) = crate::editing::move_line_down(doc, focus_key) {
                if let Some(d) = ctx.documents.get_mut(doc_id) {
                    d.apply(&txn, focus_key);
                    rehighlight(d, ctx.language_registry);
                }
            }
        }
        Action::ToggleComment => {
            // Use language config for comment token, fallback to hardcoded
            let comment = doc
                .language
                .as_ref()
                .and_then(|lang| {
                    ctx.language_registry.get(lang)
                        .and_then(|entry| entry.config.comment_token.as_deref())
                })
                .unwrap_or("//");
            let txn = crate::editing::toggle_comment(doc, focus_key, comment);
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::Cut => {
            let (txn, cut_text) = crate::editing::cut_selection(doc, focus_key);
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(&cut_text);
            }
            if let Some(d) = ctx.documents.get_mut(doc_id) {
                d.apply(&txn, focus_key);
                rehighlight(d, ctx.language_registry);
            }
        }
        Action::Copy => {
            let copied = crate::editing::copy_selection(doc, focus_key);
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(&copied);
            }
        }
        Action::Paste => {
            // Read from system clipboard and insert
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    let txn = crate::editing::insert_text(doc, focus_key, &text);
                    if let Some(d) = ctx.documents.get_mut(doc_id) {
                        d.apply(&txn, focus_key);
                        rehighlight(d, ctx.language_registry);
                    }
                }
            }
        }
        _ => {}
    }
    ensure_cursor_in_view(ctx);
    ctx.request_redraw();
}

/// Save the focused document to its current path.
fn handle_save(ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let doc_id = view.doc_id;

    let Some(doc) = ctx.documents.get_mut(doc_id) else {
        return;
    };

    if doc.path.is_none() {
        tracing::info!("save: document has no path, needs save-as");
        return;
    }

    // Format on save
    let resolved = ctx.config.resolve_for_language(doc.language.as_deref());
    if resolved.format_on_save {
        if let Some(ref lang) = doc.language {
            if let Some(overrides) = ctx.config.languages.get(lang.as_str()) {
                if let Some(ref formatter_cmd) = overrides.formatter {
                    let content = doc.text().to_string();
                    match crate::formatter::format_buffer(&content, formatter_cmd, doc.path.as_deref()) {
                        Ok(formatted) => {
                            doc.reload_from_string(&formatted);
                            rehighlight(doc, ctx.language_registry);
                            tracing::info!(?doc_id, "formatted before save");
                        }
                        Err(e) => {
                            tracing::warn!(?e, ?doc_id, "formatter failed, saving unformatted");
                        }
                    }
                }
            }
        }
    }

    match doc.save() {
        Ok(()) => {
            tracing::info!(?doc_id, "document saved");
            // Delete swap file on clean save
            if let Some(ref path) = doc.path {
                crate::swap_file::delete_swap(path);
            }
        }
        Err(e) => {
            tracing::error!(?e, ?doc_id, "failed to save document");
        }
    }
    ctx.request_redraw();
}

/// Save the focused document to a new path.
fn handle_save_as(path: &std::path::Path, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let doc_id = view.doc_id;

    let Some(doc) = ctx.documents.get_mut(doc_id) else {
        return;
    };

    match doc.save_as(path.to_path_buf()) {
        Ok(()) => {
            tracing::info!(?doc_id, ?path, "document saved as");
        }
        Err(e) => {
            tracing::error!(?e, ?doc_id, ?path, "failed to save-as");
        }
    }
    ctx.request_redraw();
}

/// Handle scroll actions on the focused view.
fn handle_scroll(action: &Action, ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get_mut(focus_key) else {
        return;
    };

    // Get total lines from the document
    let total_lines = ctx
        .documents
        .get(view.doc_id)
        .map_or(1, |doc| doc.text().len_lines());

    match action {
        Action::ScrollUp => view.scroll_up(1),
        Action::ScrollDown => view.scroll_down(1, total_lines),
        Action::PageUp => view.page_up(),
        Action::PageDown => view.page_down(total_lines),
        _ => {}
    }

    ctx.request_redraw();
}

/// Perform undo or redo on the focused document.
fn handle_undo_redo(ctx: &mut Context, is_undo: bool) {
    let Some(focus_key) = ctx.view_tree.focus() else {
        return;
    };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else {
        return;
    };
    let doc_id = view.doc_id;

    let Some(doc) = ctx.documents.get_mut(doc_id) else {
        return;
    };

    let performed = if is_undo {
        doc.undo(focus_key)
    } else {
        doc.redo(focus_key)
    };

    if performed {
        rehighlight(doc, ctx.language_registry);
        ctx.request_redraw();
    }
}

/// Write swap files for all modified documents.
fn write_swap_files(ctx: &Context) {
    for (_id, doc) in ctx.documents.iter() {
        if doc.is_modified() {
            if let Some(ref path) = doc.path {
                crate::swap_file::write_swap(path, &doc.text().to_string(), 0);
            }
        }
    }
}

/// Ensure the cursor is visible in the viewport (both vertical and horizontal scroll).
fn ensure_cursor_in_view(ctx: &mut Context) {
    let Some(focus_key) = ctx.view_tree.focus() else { return };
    let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get(focus_key) else { return };
    let doc_id = view.doc_id;
    let Some(doc) = ctx.documents.get(doc_id) else { return };

    let text = doc.text();
    let head = doc.selection(focus_key).primary().head;
    let line = if text.len_chars() > 0 {
        text.char_to_line(head.min(text.len_chars().saturating_sub(1)))
    } else {
        0
    };
    let col = head.saturating_sub(text.line_to_char(line));

    // Re-borrow mutably to update view
    if let Some(omni_view::view_tree::Node::Leaf(view)) = ctx.view_tree.get_mut(focus_key) {
        view.ensure_visible(line);
        view.ensure_col_visible(col, view.width.saturating_sub(6) as usize); // approximate code width
    }
}

/// Re-parse syntax highlighting for a document after an edit.
fn rehighlight(doc: &mut omni_view::Document, lang_reg: &LanguageRegistry) {
    let lang_id = match doc.language {
        Some(ref id) => id.clone(),
        None => return,
    };
    let Some(mut hl) = lang_reg.create_highlighter(&lang_id) else {
        return;
    };
    if let Some((tree, spans)) = hl.parse_full(doc.text()) {
        doc.syntax = Some(omni_syntax::SyntaxTree::from_tree(tree));
        doc.highlight_spans = spans;
    }
}

/// Handle a file-changed notification from the file watcher.
fn handle_file_changed(path: &std::path::Path, ctx: &mut Context) {
    let Some(doc_id) = ctx.documents.find_by_path(path) else {
        return;
    };
    let Some(doc) = ctx.documents.get_mut(doc_id) else {
        return;
    };

    // Don't reload if the user has unsaved edits
    if doc.is_modified() {
        tracing::info!(?path, "external change detected but document has unsaved edits, skipping reload");
        return;
    }

    // Reload the file content
    match std::fs::read_to_string(path) {
        Ok(content) => {
            doc.reload_from_string(&content);
            rehighlight(doc, ctx.language_registry);
            tracing::info!(?path, "reloaded externally changed file");
            ctx.request_redraw();
        }
        Err(e) => {
            tracing::warn!(?path, ?e, "failed to reload changed file");
        }
    }
}

/// Set up a file watcher that sends changed paths to the channel.
fn setup_file_watcher(
    tx: mpsc::UnboundedSender<std::path::PathBuf>,
) -> Option<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher, event::ModifyKind};

    let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            if matches!(
                event.kind,
                notify::EventKind::Modify(ModifyKind::Data(_))
                    | notify::EventKind::Modify(ModifyKind::Any)
            ) {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        }
    });

    match watcher {
        Ok(w) => {
            tracing::info!("file watcher initialized");
            Some(w)
        }
        Err(e) => {
            tracing::warn!(?e, "failed to create file watcher");
            None
        }
    }
}
