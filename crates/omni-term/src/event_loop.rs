//! Main event loop: async `tokio::select!` with event-driven rendering.

use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use ratatui::layout::Rect;
use tokio::sync::{broadcast, mpsc};

use omni_event::Action;

use crate::Compositor;
use crate::component::EventResult;
use crate::context::{Callback, Context};
use crate::terminal::TerminalModeGuard;

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

    // Async terminal event stream
    let mut terminal_events = EventStream::new();

    // Tick interval for animations/spinners
    let tick_rate = Duration::from_millis(ctx.config.tick_rate_ms);
    let mut tick_interval = tokio::time::interval(tick_rate);

    // Ctrl+C signal for graceful shutdown
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // Initial render
    terminal.draw(|frame| compositor.render(frame))?;
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

                // Ctrl+Q is always a hard quit, regardless of component handling
                if is_quit_key(&event) {
                    break;
                }

                let result = compositor.handle_event(&event, ctx)?;
                handle_event_result(result, compositor, ctx);
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
                // Mark redraw for any active animations.
                // TODO: Only redraw on tick when animations are actually active.
                ctx.needs_redraw = true;
            }

            // Branch 5: Graceful shutdown on Ctrl+C
            _ = &mut ctrl_c => {
                tracing::info!("received Ctrl+C, shutting down");
                break;
            }
        }

        // Check quit condition
        if ctx.should_quit {
            break;
        }

        // Event-driven rendering: only redraw when state changed
        if ctx.needs_redraw {
            terminal.draw(|frame| compositor.render(frame))?;
            compositor.mark_redrawn();
            ctx.needs_redraw = false;
        }
    }

    Ok(())
}

/// Process an event result from the compositor.
fn handle_event_result(result: EventResult, compositor: &mut Compositor, ctx: &mut Context) {
    match result {
        EventResult::Consumed | EventResult::Ignored => {}
        EventResult::Action(action) => {
            if let Err(err) = handle_action(&action, compositor, ctx) {
                tracing::error!(?err, ?action, "failed to handle action");
            }
        }
        EventResult::Callback(cb) => cb(compositor),
    }
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
        _ => {
            tracing::debug!(?action, "unhandled action");
        }
    }
    Ok(())
}

/// Check if this event is the hard-quit shortcut (Ctrl+Q).
const fn is_quit_key(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::CONTROL, .. })
    )
}
