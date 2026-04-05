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
        // TODO: validate with gix::open
        Ok(Self { path })
    }

    /// The repository's working directory.
    #[must_use]
    pub const fn workdir(&self) -> &PathBuf {
        &self.path
    }
}
