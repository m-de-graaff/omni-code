//! Omni Code — Terminal AI IDE
//!
//! Binary entry point. Parses CLI arguments, initializes subsystems,
//! and launches the TUI event loop.

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Omni Code — a terminal AI IDE.
#[derive(Parser, Debug)]
#[command(name = "omni", version, about)]
struct Args {
    /// File(s) to open on startup.
    files: Vec<String>,

    /// Log level (error, warn, info, debug, trace).
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    // Log to file, not stdout (stdout is the TUI)
    let log_dir = omni_loader::paths::log_dir().unwrap_or_else(|_| std::env::temp_dir());
    let file_appender = tracing_appender::rolling::daily(&log_dir, "omni.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_new(&args.log_level).unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    tracing::info!("Omni Code starting...");

    // Initialize the event bus
    let _bus = omni_event::EventBus::new(256);

    // Load configuration
    let config = omni_loader::EditorConfig::default();

    // Initialize editor state
    let mut view_tree = omni_view::ViewTree::new();

    // Build the application context
    let mut ctx = omni_term::Context::new(&mut view_tree, &config);

    // Set up the terminal and run
    let mut terminal = ratatui::init();
    let mut compositor = omni_term::Compositor::new();

    let result = omni_term::event_loop::run(&mut terminal, &mut compositor, &mut ctx);

    ratatui::restore();

    result
}
