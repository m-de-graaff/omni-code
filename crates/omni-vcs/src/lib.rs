//! # omni-vcs
//!
//! Git integration powered by gitoxide.

pub mod diff;
pub mod repo;
pub mod status;

pub use repo::Repository;
pub use status::FileStatus;
