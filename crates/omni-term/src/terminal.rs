//! Terminal mode management — RAII guard for mouse capture and bracketed paste.
//!
//! `ratatui::init()` handles raw mode and alternate screen. This module
//! adds the extra modes needed for a full IDE experience.

use std::io::stdout;

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;

/// RAII guard that enables mouse capture and bracketed paste on creation,
/// and disables them on drop — even during panics or early returns.
pub struct TerminalModeGuard;

impl TerminalModeGuard {
    /// Enable mouse capture and bracketed paste mode.
    ///
    /// # Errors
    /// Returns an error if the terminal commands fail.
    pub fn enable() -> color_eyre::Result<Self> {
        execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
        Ok(Self)
    }
}

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableBracketedPaste, DisableMouseCapture);
    }
}
