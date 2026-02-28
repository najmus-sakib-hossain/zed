//! Shadow file synchronization for DXM.
//!
//! Regenerates shadow .md files from .dxm sources to ensure
//! GitHub always shows current content.

use std::fs;
use std::path::{Path, PathBuf};

use super::clean::{CleanFilter, FilterError};

/// Manages synchronization of DXM files to shadow Markdown files.
pub struct SyncManager {
    /// Root directory to search for .dxm files.
    root: PathBuf,
    /// Clean filter for conversion.
    filter: CleanFilter,
}

impl SyncManager {
    /// Create a new sync manager.
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory to search for .dxm files
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            filter: CleanFilter::new(),
        }
    }

    /// Sync all .dxm files in the root directory.
    ///
    /// # Returns
    ///
    /// List of synced file paths (the generated .md files).
    ///
    /// # Errors
    ///
    /// Returns an error if any file fails to sync.
    pub fn sync_all(&self) -> Result<Vec<PathBuf>, SyncError> {
        let mut synced = Vec::new();
        self.sync_directory(&self.root, &mut synced)?;
        Ok(synced)
    }

    /// Sync a single .dxm file.
    ///
    /// # Arguments
    ///
    /// * `dxm_path` - Path to the .dxm file
    ///
    /// # Returns
    ///
    /// Path to the generated .md file.
    ///
    /// # Errors
    ///
    /// Returns an error if conversion fails.
    pub fn sync_file(&self, dxm_path: &Path) -> Result<PathBuf, SyncError> {
        // Verify it's a .dxm file
        if dxm_path.extension().map(|e| e != "dxm").unwrap_or(true) {
            return Err(SyncError::NotDxmFile(dxm_path.to_path_buf()));
        }

        // Read DXM content
        let dxm_content = fs::read_to_string(dxm_path)
            .map_err(|e| SyncError::ReadError(dxm_path.to_path_buf(), e.to_string()))?;

        // Convert to Markdown
        let md_content = self
            .filter
            .dxm_to_markdown(&dxm_content)
            .map_err(|e| SyncError::ConvertError(dxm_path.to_path_buf(), e))?;

        // Generate .md path
        let md_path = dxm_path.with_extension("md");

        // Write Markdown file
        fs::write(&md_path, md_content)
            .map_err(|e| SyncError::WriteError(md_path.clone(), e.to_string()))?;

        Ok(md_path)
    }

    /// Recursively sync all .dxm files in a directory.
    fn sync_directory(&self, dir: &Path, synced: &mut Vec<PathBuf>) -> Result<(), SyncError> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)
            .map_err(|e| SyncError::ReadError(dir.to_path_buf(), e.to_string()))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| SyncError::ReadError(dir.to_path_buf(), e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories and common non-source directories
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    self.sync_directory(&path, synced)?;
                }
            } else if path.extension().map(|e| e == "dxm").unwrap_or(false) {
                let md_path = self.sync_file(&path)?;
                synced.push(md_path);
            }
        }

        Ok(())
    }
}

/// Errors that can occur during synchronization.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// File is not a .dxm file.
    #[error("Not a .dxm file: {0}")]
    NotDxmFile(PathBuf),

    /// Failed to read file.
    #[error("Failed to read {0}: {1}")]
    ReadError(PathBuf, String),

    /// Failed to convert file.
    #[error("Failed to convert {0}: {1}")]
    ConvertError(PathBuf, FilterError),

    /// Failed to write file.
    #[error("Failed to write {0}: {1}")]
    WriteError(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_manager_new() {
        let manager = SyncManager::new("/tmp/test");
        assert_eq!(manager.root, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_sync_file_not_dxm() {
        let manager = SyncManager::new("/tmp");
        let result = manager.sync_file(Path::new("/tmp/test.md"));
        assert!(matches!(result, Err(SyncError::NotDxmFile(_))));
    }
}
