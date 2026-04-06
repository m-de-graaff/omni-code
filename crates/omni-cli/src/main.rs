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

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
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

    // Ensure Nerd Font is installed (downloads on first run)
    if let Err(e) = omni_loader::font::ensure_installed().await {
        tracing::warn!(?e, "Nerd Font auto-install failed — icons may not render correctly");
    }

    // Initialize the event bus
    let bus = omni_event::EventBus::new(256);
    let mut action_rx = bus.subscribe();
    let action_tx = bus.sender();

    // Callback channel for compositor mutations
    let (callback_tx, mut callback_rx) = tokio::sync::mpsc::unbounded_channel();

    // Load configuration and theme
    let config = omni_loader::EditorConfig::default();
    let theme_def = omni_loader::Theme::by_name(&config.theme);
    let capability = omni_loader::detect_color_capability();
    let theme_colors = omni_loader::ThemeColors::from_theme(&theme_def, capability);
    tracing::info!(theme = %theme_def.name, ?capability, "theme loaded");

    // Load keybindings (defaults + user overrides)
    let keymap = omni_loader::load_keymap().unwrap_or_else(|err| {
        tracing::warn!(?err, "failed to load keybindings, using defaults");
        omni_loader::keymap_loader::default_keymap()
    });
    tracing::info!("keybindings loaded");

    // Initialize language registry for syntax highlighting
    let language_registry = omni_syntax::LanguageRegistry::new();
    tracing::info!(languages = language_registry.len(), "language registry loaded");

    // Initialize editor state
    let mut view_tree = omni_view::ViewTree::new();
    let mut documents = omni_view::DocumentStore::new();

    // Build the application context
    let mut ctx = omni_term::Context::new(
        &mut view_tree,
        &mut documents,
        &config,
        &theme_colors,
        &keymap,
        &language_registry,
        action_tx,
        callback_tx,
    );

    // Set up the terminal and run
    let mut terminal = ratatui::init();
    let mut compositor = omni_term::Compositor::new();
    compositor.push(Box::new(omni_term::EditorShell::new(theme_colors.clone())))?;

    let result = omni_term::event_loop::run(
        &mut terminal,
        &mut compositor,
        &mut ctx,
        &mut action_rx,
        &mut callback_rx,
    )
    .await;

    ratatui::restore();

    result
}
