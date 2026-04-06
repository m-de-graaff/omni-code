//! Diff computation between file versions.

use std::path::Path;

/// Line-level diff status for a line in the new (current) version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDiffStatus {
    /// Line is unchanged from HEAD.
    Unchanged,
    /// Line was added (not in HEAD).
    Added,
    /// Line was modified (different content from HEAD).
    Modified,
}

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

/// Compute per-line diff status by comparing old text (HEAD) with new text (buffer).
///
/// Returns a `Vec<LineDiffStatus>` with one entry per line in `new_text`.
#[must_use]
pub fn compute_line_diff(old_text: &str, new_text: &str) -> Vec<LineDiffStatus> {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old_text, new_text);
    let mut result = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => result.push(LineDiffStatus::Unchanged),
            ChangeTag::Insert => result.push(LineDiffStatus::Added),
            ChangeTag::Delete => {
                // Deleted lines don't exist in the new text, but we mark the
                // next line (if any) as modified to show something in the gutter.
                // We'll handle this by marking the last pushed line as modified
                // if there was an insert following a delete.
            }
        }
    }

    // Simpler approach: re-do with grouped ops for better accuracy
    result.clear();
    for op in diff.ops() {
        let new_range = op.new_range();
        let new_count = new_range.len();

        match op.tag() {
            similar::DiffTag::Equal => {
                for _ in 0..new_count {
                    result.push(LineDiffStatus::Unchanged);
                }
            }
            similar::DiffTag::Insert => {
                for _ in 0..new_count {
                    result.push(LineDiffStatus::Added);
                }
            }
            similar::DiffTag::Delete => {
                // Lines deleted from old — no new lines added here
            }
            similar::DiffTag::Replace => {
                for _ in 0..new_count {
                    result.push(LineDiffStatus::Modified);
                }
            }
        }
    }

    result
}

/// Read the HEAD version of a file from the git repository.
///
/// Uses `git show HEAD:<relative_path>` as a simple fallback.
#[must_use]
pub fn read_head_version(workdir: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(workdir).ok()?;
    let relative_str = relative.to_string_lossy().replace('\\', "/");

    let output = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{relative_str}")])
        .current_dir(workdir)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None // File not in HEAD (new file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical() {
        let text = "line1\nline2\nline3\n";
        let result = compute_line_diff(text, text);
        assert!(result.iter().all(|&s| s == LineDiffStatus::Unchanged));
    }

    #[test]
    fn diff_added_lines() {
        let old = "line1\n";
        let new = "line1\nline2\nline3\n";
        let result = compute_line_diff(old, new);
        assert_eq!(result[0], LineDiffStatus::Unchanged);
        assert_eq!(result[1], LineDiffStatus::Added);
        assert_eq!(result[2], LineDiffStatus::Added);
    }

    #[test]
    fn diff_modified_line() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nchanged\nline3\n";
        let result = compute_line_diff(old, new);
        assert_eq!(result[0], LineDiffStatus::Unchanged);
        assert_eq!(result[1], LineDiffStatus::Modified);
        assert_eq!(result[2], LineDiffStatus::Unchanged);
    }

    #[test]
    fn diff_all_new() {
        let old = "";
        let new = "line1\nline2\n";
        let result = compute_line_diff(old, new);
        assert!(result.iter().all(|&s| s == LineDiffStatus::Added));
    }
}
