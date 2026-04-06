//! Git repository abstraction.

use std::path::PathBuf;

use thiserror::Error;

/// Errors from git operations.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),
    #[error("git error: {0}")]
    Git(String),
}

/// A handle to a git repository.
#[derive(Debug)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Open a repository at the given path.
    ///
    /// # Errors
    /// Returns `GitError::NotARepo` if the path is not a git repository.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, GitError> {
        let path = path.into();
        let _ = gix::open(&path).map_err(|e| GitError::Git(e.to_string()))?;
        Ok(Self { path })
    }

    /// The repository's working directory.
    #[must_use]
    pub const fn workdir(&self) -> &PathBuf {
        &self.path
    }

    /// Get the current branch name, or short commit hash if detached HEAD.
    ///
    /// Returns `None` if the path is not a git repo or HEAD can't be read.
    #[must_use]
    pub fn current_branch(workdir: &std::path::Path) -> Option<String> {
        let repo = gix::open(workdir).ok()?;
        let head = repo.head().ok()?;
        // Try to get the branch name from the reference
        head.referent_name()
            .map(|r| r.shorten().to_string())
            .or_else(|| {
                // Detached HEAD — show short commit hash
                head.id().map(|id| format!("{:.7}", id))
            })
    }
}
