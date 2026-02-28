//! Incremental Checking Support
//!
//! Tracks file changes to enable incremental checking - only re-checking files
//! that have changed since the last run.
//!
//! **Validates: Requirement 12.3 - Implement incremental checking (only changed files)**

use blake3::Hasher;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// State file name for incremental checking
const STATE_FILE: &str = ".dx-check-state.json";

/// Tracks file states for incremental checking
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IncrementalState {
    /// Version of the state format
    version: u32,
    /// File states keyed by path
    files: HashMap<PathBuf, FileState>,
    /// Last check timestamp
    last_check: Option<u64>,
    /// Configuration hash (to invalidate on config changes)
    config_hash: Option<String>,
}

/// State of a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// Content hash (blake3)
    pub content_hash: String,
    /// Last modified time
    pub modified_time: u64,
    /// File size in bytes
    pub size: u64,
    /// Whether the file had errors on last check
    pub had_errors: bool,
    /// Number of diagnostics on last check
    pub diagnostic_count: usize,
}

/// Result of checking if a file needs re-checking
#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    /// File is unchanged and can be skipped
    Unchanged,
    /// File content has changed
    ContentChanged,
    /// File is new (not in previous state)
    New,
    /// File was deleted
    Deleted,
    /// File metadata changed but content may be same
    MetadataChanged,
}

/// Incremental checker that tracks file changes
pub struct IncrementalChecker {
    /// Current state
    state: RwLock<IncrementalState>,
    /// State file path
    state_path: PathBuf,
    /// Whether to use content hashing (slower but more accurate)
    use_content_hash: bool,
}

impl IncrementalChecker {
    /// Create a new incremental checker
    #[must_use]
    pub fn new(workspace_root: &Path) -> Self {
        let state_path = workspace_root.join(STATE_FILE);
        let state = Self::load_state(&state_path).unwrap_or_default();

        Self {
            state: RwLock::new(state),
            state_path,
            use_content_hash: true,
        }
    }

    /// Create with custom state file path
    #[must_use]
    pub fn with_state_path(state_path: PathBuf) -> Self {
        let state = Self::load_state(&state_path).unwrap_or_default();

        Self {
            state: RwLock::new(state),
            state_path,
            use_content_hash: true,
        }
    }

    /// Load state from disk
    fn load_state(path: &Path) -> io::Result<IncrementalState> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save state to disk
    pub fn save_state(&self) -> io::Result<()> {
        let state = self.state.read();
        let content = serde_json::to_string_pretty(&*state)?;
        fs::write(&self.state_path, content)
    }

    /// Check if a file needs re-checking
    pub fn check_file(&self, path: &Path) -> io::Result<FileChangeStatus> {
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(FileChangeStatus::Deleted);
            }
            Err(e) => return Err(e),
        };

        let state = self.state.read();
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        match state.files.get(&canonical_path) {
            None => Ok(FileChangeStatus::New),
            Some(file_state) => {
                // Quick check: size changed
                if metadata.len() != file_state.size {
                    return Ok(FileChangeStatus::ContentChanged);
                }

                // Quick check: modified time changed
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());

                if modified != file_state.modified_time {
                    // Metadata changed, check content if enabled
                    if self.use_content_hash {
                        let content = fs::read(path)?;
                        let hash = Self::hash_content(&content);
                        if hash != file_state.content_hash {
                            return Ok(FileChangeStatus::ContentChanged);
                        }
                        // Content same despite metadata change
                        return Ok(FileChangeStatus::MetadataChanged);
                    }
                    return Ok(FileChangeStatus::ContentChanged);
                }

                Ok(FileChangeStatus::Unchanged)
            }
        }
    }

    /// Update file state after checking
    pub fn update_file_state(
        &self,
        path: &Path,
        had_errors: bool,
        diagnostic_count: usize,
    ) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        let content = fs::read(path)?;
        let hash = Self::hash_content(&content);

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());

        let file_state = FileState {
            content_hash: hash,
            modified_time: modified,
            size: metadata.len(),
            had_errors,
            diagnostic_count,
        };

        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut state = self.state.write();
        state.files.insert(canonical_path, file_state);

        Ok(())
    }

    /// Remove a file from state (e.g., when deleted)
    pub fn remove_file(&self, path: &Path) {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut state = self.state.write();
        state.files.remove(&canonical_path);
    }

    /// Get files that need checking from a list of paths
    pub fn filter_changed_files(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        paths
            .iter()
            .filter(|path| {
                matches!(
                    self.check_file(path),
                    Ok(FileChangeStatus::New
                        | FileChangeStatus::ContentChanged
                        | FileChangeStatus::MetadataChanged)
                )
            })
            .cloned()
            .collect()
    }

    /// Get statistics about the incremental state
    pub fn stats(&self) -> IncrementalStats {
        let state = self.state.read();
        let total_files = state.files.len();
        let files_with_errors = state.files.values().filter(|f| f.had_errors).count();
        let total_diagnostics: usize = state.files.values().map(|f| f.diagnostic_count).sum();

        IncrementalStats {
            total_files,
            files_with_errors,
            total_diagnostics,
            last_check: state.last_check,
        }
    }

    /// Update the last check timestamp
    pub fn mark_check_complete(&self) {
        let mut state = self.state.write();
        state.last_check = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Set the configuration hash (invalidates state if changed)
    pub fn set_config_hash(&self, hash: &str) {
        let mut state = self.state.write();
        if state.config_hash.as_deref() != Some(hash) {
            // Config changed, invalidate all file states
            state.files.clear();
            state.config_hash = Some(hash.to_string());
        }
    }

    /// Clear all state
    pub fn clear(&self) {
        let mut state = self.state.write();
        state.files.clear();
        state.last_check = None;
    }

    /// Hash file content using blake3
    fn hash_content(content: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(content);
        hasher.finalize().to_hex().to_string()
    }
}

/// Statistics about incremental checking state
#[derive(Debug, Clone)]
pub struct IncrementalStats {
    /// Total files tracked
    pub total_files: usize,
    /// Files that had errors on last check
    pub files_with_errors: usize,
    /// Total diagnostics across all files
    pub total_diagnostics: usize,
    /// Timestamp of last check
    pub last_check: Option<u64>,
}

impl Drop for IncrementalChecker {
    fn drop(&mut self) {
        // Save state on drop
        if let Err(e) = self.save_state() {
            tracing::warn!("Failed to save incremental state: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_file_detection() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::New);
    }

    #[test]
    fn test_unchanged_file_detection() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        // First check - new file
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::New);

        // Update state
        checker.update_file_state(&file_path, false, 0).unwrap();

        // Second check - unchanged
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::Unchanged);
    }

    #[test]
    fn test_content_change_detection() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        // Update state
        checker.update_file_state(&file_path, false, 0).unwrap();

        // Modify file
        fs::write(&file_path, "const x = 2;").unwrap();

        // Check - should detect change
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::ContentChanged);
    }

    #[test]
    fn test_deleted_file_detection() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        // Update state
        checker.update_file_state(&file_path, false, 0).unwrap();

        // Delete file
        fs::remove_file(&file_path).unwrap();

        // Check - should detect deletion
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::Deleted);
    }

    #[test]
    fn test_filter_changed_files() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file1 = temp_dir.path().join("file1.js");
        let file2 = temp_dir.path().join("file2.js");
        let file3 = temp_dir.path().join("file3.js");

        fs::write(&file1, "const a = 1;").unwrap();
        fs::write(&file2, "const b = 2;").unwrap();
        fs::write(&file3, "const c = 3;").unwrap();

        // Mark file1 and file2 as checked
        checker.update_file_state(&file1, false, 0).unwrap();
        checker.update_file_state(&file2, false, 0).unwrap();

        // Modify file2
        fs::write(&file2, "const b = 22;").unwrap();

        let paths = vec![file1.clone(), file2.clone(), file3.clone()];
        let changed = checker.filter_changed_files(&paths);

        // file1 unchanged, file2 changed, file3 new
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&file2));
        assert!(changed.contains(&file3));
    }

    #[test]
    fn test_state_persistence() {
        let temp_dir = tempdir().unwrap();
        let state_path = temp_dir.path().join(".dx-check-state.json");

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        // Create checker and update state
        {
            let checker = IncrementalChecker::with_state_path(state_path.clone());
            checker.update_file_state(&file_path, false, 0).unwrap();
            checker.save_state().unwrap();
        }

        // Create new checker and verify state loaded
        {
            let checker = IncrementalChecker::with_state_path(state_path);
            let status = checker.check_file(&file_path).unwrap();
            assert_eq!(status, FileChangeStatus::Unchanged);
        }
    }

    #[test]
    fn test_config_hash_invalidation() {
        let temp_dir = tempdir().unwrap();
        let checker = IncrementalChecker::new(temp_dir.path());

        let file_path = temp_dir.path().join("test.js");
        fs::write(&file_path, "const x = 1;").unwrap();

        // Set initial config and update state
        checker.set_config_hash("config_v1");
        checker.update_file_state(&file_path, false, 0).unwrap();

        // File should be unchanged
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::Unchanged);

        // Change config - should invalidate state
        checker.set_config_hash("config_v2");

        // File should now be new (state was cleared)
        let status = checker.check_file(&file_path).unwrap();
        assert_eq!(status, FileChangeStatus::New);
    }
}
