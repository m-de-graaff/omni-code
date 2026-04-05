//! Diff computation between file versions.

/// A line-level diff hunk.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Starting line in the old version.
    pub old_start: usize,
    /// Number of lines in the old version.
    pub old_lines: usize,
    /// Starting line in the new version.
    pub new_start: usize,
    /// Number of lines in the new version.
    pub new_lines: usize,
}
