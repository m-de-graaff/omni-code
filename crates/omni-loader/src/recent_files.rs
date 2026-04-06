//! Recent files tracking with JSON persistence.

use std::path::PathBuf;

const MAX_RECENT: usize = 20;

/// Tracks recently opened files.
#[derive(Debug, Default)]
pub struct RecentFiles {
    files: Vec<PathBuf>,
}

impl RecentFiles {
    /// Load recent files from the data directory.
    pub fn load() -> Self {
        let path = match recent_files_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let files: Vec<PathBuf> = serde_json::from_str(&content).unwrap_or_default();
        Self { files }
    }

    /// Save recent files to the data directory.
    pub fn save(&self) {
        let Some(path) = recent_files_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.files) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Add a file to the front of the list (dedup + truncate).
    pub fn push(&mut self, path: PathBuf) {
        self.files.retain(|p| p != &path);
        self.files.insert(0, path);
        self.files.truncate(MAX_RECENT);
    }

    /// Get the list of recent files.
    pub fn list(&self) -> &[PathBuf] {
        &self.files
    }
}

fn recent_files_path() -> Option<PathBuf> {
    crate::paths::data_dir().ok().map(|d| d.join("recent_files.json"))
}
