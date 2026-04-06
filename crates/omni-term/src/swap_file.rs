//! Swap file management for crash recovery.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A swap file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEntry {
    /// Original file path.
    pub path: PathBuf,
    /// Document content at time of swap.
    pub content: String,
    /// Cursor position (char index).
    pub cursor_pos: usize,
}

/// Write a swap file for a document.
pub fn write_swap(original_path: &Path, content: &str, cursor_pos: usize) {
    let Some(swap_dir) = swap_dir() else { return };
    let _ = std::fs::create_dir_all(&swap_dir);

    let entry = SwapEntry {
        path: original_path.to_path_buf(),
        content: content.to_string(),
        cursor_pos,
    };

    let swap_path = swap_path_for(original_path);
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = std::fs::write(swap_path, json);
    }
}

/// Delete the swap file for a given source path.
pub fn delete_swap(original_path: &Path) {
    let path = swap_path_for(original_path);
    let _ = std::fs::remove_file(path);
}

/// List all swap file entries.
pub fn list_swap_files() -> Vec<SwapEntry> {
    let Some(dir) = swap_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "swp") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(swap) = serde_json::from_str::<SwapEntry>(&content) {
                    result.push(swap);
                }
            }
        }
    }
    result
}

/// Delete all swap files.
pub fn clear_all_swap_files() {
    let Some(dir) = swap_dir() else { return };
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "swp") {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn swap_dir() -> Option<PathBuf> {
    omni_loader::paths::data_dir().ok().map(|d| d.join("swap"))
}

fn swap_path_for(original_path: &Path) -> PathBuf {
    let hash = simple_hash(original_path);
    let dir = swap_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join(format!("{hash:016x}.swp"))
}

fn simple_hash(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
