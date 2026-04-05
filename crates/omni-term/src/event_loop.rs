//! Main event loop: polls terminal events, dispatches to compositor.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::layout::Rect;

use omni_event::Action;

use crate::Compositor;
use crate::component::EventResult;
use crate::context::Context;

/// Run the main event loop.
///
/// # Errors
/// Returns an error if terminal I/O or event handling fails.
pub fn run(
    terminal: &mut DefaultTerminal,
    compositor: &mut Compositor,
    ctx: &mut Context,
) -> color_eyre::Result<()> {
    // Initialize compositor with the terminal size
    let size = terminal.size()?;
    compositor.resize(Rect::new(0, 0, size.width, size.height))?;

    loop {
        // Render
        terminal.draw(|frame| compositor.render(frame))?;

        // Check for quit signal from context
        if ctx.should_quit {
            return Ok(());
        }

        // Poll for events (~60fps)
        if event::poll(Duration::from_millis(16))? {
            let ev = event::read()?;

            // Ctrl+Q is always a hard quit, regardless of component handling
            if is_quit_key(&ev) {
                return Ok(());
            }

            // Dispatch to compositor
            let result = compositor.handle_event(&ev, ctx)?;
            handle_event_result(result, compositor, ctx);

            // Re-check quit after handling
            if ctx.should_quit {
                return Ok(());
            }
        }
    }
}

/// Process an event result from the compositor.
fn handle_event_result(result: EventResult, compositor: &mut Compositor, ctx: &mut Context) {
    match result {
        EventResult::Consumed | EventResult::Ignored => {}
        EventResult::Action(action) => handle_action(&action, ctx),
        EventResult::Callback(cb) => cb(compositor),
    }
}

/// Process a global action.
fn handle_action(action: &Action, ctx: &mut Context) {
    match action {
        Action::Quit => ctx.quit(),
        Action::Resize { .. } => {
            // Handled by compositor.handle_event directly
        }
        _ => {
            tracing::debug!(?action, "unhandled action");
        }
    }
}

/// Check if this event is the hard-quit shortcut (Ctrl+Q).
const fn is_quit_key(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::CONTROL, .. })
    )
}
