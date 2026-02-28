//! Integrity Guard
//!
//! Continuous integrity verification for rule files.

use crate::Result;
use crate::binary::checksum::{Blake3Checksum, compute_blake3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Integrity status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegrityStatus {
    /// File has not been verified
    Unknown,
    /// File integrity is verified
    Verified,
    /// File has been modified
    Modified,
    /// File signature is invalid
    InvalidSignature,
    /// File is missing
    Missing,
}

/// Integrity record for a file
#[derive(Debug, Clone)]
pub struct IntegrityRecord {
    /// File path
    pub path: PathBuf,
    /// Content checksum
    pub checksum: Blake3Checksum,
    /// File size
    pub size: u64,
    /// Last modified time
    pub modified: std::time::SystemTime,
    /// Status
    pub status: IntegrityStatus,
}

/// Integrity guard for monitoring file integrity
pub struct IntegrityGuard {
    /// Tracked files
    records: HashMap<PathBuf, IntegrityRecord>,
    /// Verification callback
    on_violation: Option<Box<dyn Fn(&IntegrityRecord) + Send + Sync>>,
}

impl std::fmt::Debug for IntegrityGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntegrityGuard")
            .field("records", &self.records)
            .field("on_violation", &self.on_violation.is_some())
            .finish()
    }
}

impl IntegrityGuard {
    /// Create a new integrity guard
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            on_violation: None,
        }
    }

    /// Set violation callback
    pub fn on_violation<F>(mut self, callback: F) -> Self
    where
        F: Fn(&IntegrityRecord) + Send + Sync + 'static,
    {
        self.on_violation = Some(Box::new(callback));
        self
    }

    /// Add a file to track
    pub fn track(&mut self, path: &Path) -> Result<IntegrityStatus> {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => {
                self.records.insert(
                    path.to_path_buf(),
                    IntegrityRecord {
                        path: path.to_path_buf(),
                        checksum: [0; 16],
                        size: 0,
                        modified: std::time::UNIX_EPOCH,
                        status: IntegrityStatus::Missing,
                    },
                );
                return Ok(IntegrityStatus::Missing);
            }
        };

        let content = std::fs::read(path)?;
        let checksum = compute_blake3(&content);

        let record = IntegrityRecord {
            path: path.to_path_buf(),
            checksum,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
            status: IntegrityStatus::Verified,
        };

        self.records.insert(path.to_path_buf(), record);
        Ok(IntegrityStatus::Verified)
    }

    /// Verify a file's integrity
    pub fn verify(&mut self, path: &Path) -> Result<IntegrityStatus> {
        let record = match self.records.get(path) {
            Some(r) => r.clone(),
            None => {
                // Not tracked, track it now
                return self.track(path);
            }
        };

        // Check if file exists
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => {
                let mut record = record;
                record.status = IntegrityStatus::Missing;
                if let Some(cb) = &self.on_violation {
                    cb(&record);
                }
                self.records.insert(path.to_path_buf(), record);
                return Ok(IntegrityStatus::Missing);
            }
        };

        // Quick check: size changed?
        if metadata.len() != record.size {
            let mut record = record;
            record.status = IntegrityStatus::Modified;
            if let Some(cb) = &self.on_violation {
                cb(&record);
            }
            self.records.insert(path.to_path_buf(), record);
            return Ok(IntegrityStatus::Modified);
        }

        // Full check: content changed?
        let content = std::fs::read(path)?;
        let current_checksum = compute_blake3(&content);

        if current_checksum != record.checksum {
            let mut record = record;
            record.status = IntegrityStatus::Modified;
            if let Some(cb) = &self.on_violation {
                cb(&record);
            }
            self.records.insert(path.to_path_buf(), record);
            return Ok(IntegrityStatus::Modified);
        }

        Ok(IntegrityStatus::Verified)
    }

    /// Verify all tracked files
    pub fn verify_all(&mut self) -> Result<Vec<(PathBuf, IntegrityStatus)>> {
        let paths: Vec<_> = self.records.keys().cloned().collect();
        let mut results = Vec::with_capacity(paths.len());

        for path in paths {
            let status = self.verify(&path)?;
            results.push((path, status));
        }

        Ok(results)
    }

    /// Get status of a file
    pub fn status(&self, path: &Path) -> IntegrityStatus {
        self.records.get(path).map(|r| r.status).unwrap_or(IntegrityStatus::Unknown)
    }

    /// Get all records
    pub fn records(&self) -> &HashMap<PathBuf, IntegrityRecord> {
        &self.records
    }

    /// Remove a file from tracking
    pub fn untrack(&mut self, path: &Path) {
        self.records.remove(path);
    }

    /// Clear all tracking
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Update checksum after authorized modification
    pub fn update(&mut self, path: &Path) -> Result<()> {
        self.track(path)?;
        Ok(())
    }

    /// Get number of tracked files
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if any files are tracked
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Count files by status
    pub fn count_by_status(&self) -> HashMap<IntegrityStatus, usize> {
        let mut counts = HashMap::new();
        for record in self.records.values() {
            *counts.entry(record.status).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for IntegrityGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_and_verify() {
        let temp_dir = std::env::temp_dir().join("integrity_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("test.txt");

        std::fs::write(&test_file, b"test content").unwrap();

        let mut guard = IntegrityGuard::new();
        let status = guard.track(&test_file).unwrap();
        assert_eq!(status, IntegrityStatus::Verified);

        // Verify unchanged
        let status = guard.verify(&test_file).unwrap();
        assert_eq!(status, IntegrityStatus::Verified);

        // Modify file
        std::fs::write(&test_file, b"modified content").unwrap();

        // Verify should detect modification
        let status = guard.verify(&test_file).unwrap();
        assert_eq!(status, IntegrityStatus::Modified);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_missing_file() {
        let mut guard = IntegrityGuard::new();
        let status = guard.track(Path::new("/nonexistent/file.txt")).unwrap();
        assert_eq!(status, IntegrityStatus::Missing);
    }
}
