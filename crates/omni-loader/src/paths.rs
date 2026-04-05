//! Standard filesystem paths for configuration, data, and cache.

use std::path::PathBuf;

use directories::ProjectDirs;
use thiserror::Error;

/// Errors from path resolution.
#[derive(Debug, Error)]
pub enum PathError {
    #[error("could not determine home directory")]
    NoHomeDir,
}

/// Resolve the application's config directory.
///
/// - Linux: `~/.config/omni-code/`
/// - macOS: `~/Library/Application Support/dev.omnicode.omni-code/`
/// - Windows: `%APPDATA%\omnicode\omni-code\config\`
///
/// # Errors
/// Returns `PathError::NoHomeDir` if the home directory cannot be determined.
pub fn config_dir() -> Result<PathBuf, PathError> {
    project_dirs().map(|dirs| dirs.config_dir().to_path_buf())
}

/// Resolve the application's data directory.
///
/// - Linux: `~/.local/share/omni-code/`
/// - macOS: `~/Library/Application Support/dev.omnicode.omni-code/`
/// - Windows: `%APPDATA%\omnicode\omni-code\data\`
///
/// # Errors
/// Returns `PathError::NoHomeDir` if the home directory cannot be determined.
pub fn data_dir() -> Result<PathBuf, PathError> {
    project_dirs().map(|dirs| dirs.data_dir().to_path_buf())
}

/// Resolve the log directory (`<data_dir>/logs/`).
///
/// # Errors
/// Returns `PathError::NoHomeDir` if the home directory cannot be determined.
pub fn log_dir() -> Result<PathBuf, PathError> {
    data_dir().map(|d| d.join("logs"))
}

fn project_dirs() -> Result<ProjectDirs, PathError> {
    ProjectDirs::from("dev", "omnicode", "omni-code").ok_or(PathError::NoHomeDir)
}
