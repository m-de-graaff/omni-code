//! Git working tree status.

use std::path::PathBuf;

/// Status of a file in the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// Unmodified.
    Clean,
    /// Modified but not staged.
    Modified,
    /// Staged for commit.
    Staged,
    /// Untracked file.
    Untracked,
    /// Deleted.
    Deleted,
    /// Renamed.
    Renamed { from: PathBuf },
}
