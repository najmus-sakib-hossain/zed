//! Hash-based snapshot testing
//!
//! This crate implements O(1) snapshot verification via hash comparison.
//! Snapshots are stored with their Blake3 hashes for fast verification,
//! and diffs are generated only when hashes mismatch.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use blake3::Hash;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

pub use dx_py_core::{SnapshotError, TestId};

/// Result of snapshot verification
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotResult {
    /// Snapshot matches expected
    Match,
    /// Snapshot differs from expected
    Mismatch {
        /// Unified diff showing differences
        diff: String,
    },
    /// No existing snapshot (new test)
    New {
        /// The actual content
        content: Vec<u8>,
    },
}

/// Snapshot entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotEntry {
    /// Blake3 hash of the content
    hash: [u8; 32],
    /// Path to the snapshot file
    path: String,
}

/// Binary snapshot index format
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotIndexData {
    /// Magic bytes for validation
    magic: [u8; 4],
    /// Version number
    version: u16,
    /// Snapshot entries
    entries: HashMap<u64, SnapshotEntry>,
}

impl Default for SnapshotIndexData {
    fn default() -> Self {
        Self {
            magic: *b"DXSI",
            version: 1,
            entries: HashMap::new(),
        }
    }
}

/// Hash-based snapshot index
///
/// Provides O(1) snapshot verification by comparing Blake3 hashes
/// before loading content. Diffs are only generated when hashes mismatch.
pub struct SnapshotIndex {
    /// Directory containing snapshots
    snapshot_dir: PathBuf,
    /// Index data
    data: SnapshotIndexData,
    /// Path to index file
    index_path: PathBuf,
    /// Dirty flag for saving
    dirty: bool,
}

impl SnapshotIndex {
    /// Create or load a snapshot index
    pub fn new(snapshot_dir: impl Into<PathBuf>) -> Result<Self, SnapshotError> {
        let snapshot_dir = snapshot_dir.into();
        fs::create_dir_all(&snapshot_dir)?;

        let index_path = snapshot_dir.join("index.dxsi");

        let data = if index_path.exists() {
            Self::load_index(&index_path)?
        } else {
            SnapshotIndexData::default()
        };

        Ok(Self {
            snapshot_dir,
            data,
            index_path,
            dirty: false,
        })
    }

    /// Load index from disk
    fn load_index(path: &Path) -> Result<SnapshotIndexData, SnapshotError> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let data: SnapshotIndexData = bincode::deserialize(&bytes)
            .map_err(|e| SnapshotError::IndexCorrupted(e.to_string()))?;

        if &data.magic != b"DXSI" {
            return Err(SnapshotError::IndexCorrupted("Invalid magic bytes".into()));
        }

        Ok(data)
    }

    /// Save index to disk
    pub fn save(&mut self) -> Result<(), SnapshotError> {
        if !self.dirty {
            return Ok(());
        }

        let bytes = bincode::serialize(&self.data)
            .map_err(|e| SnapshotError::IndexCorrupted(e.to_string()))?;

        let mut file = File::create(&self.index_path)?;
        file.write_all(&bytes)?;

        self.dirty = false;
        Ok(())
    }

    /// Get snapshot file path for a test
    #[allow(dead_code)]
    fn snapshot_path(&self, test_id: TestId) -> PathBuf {
        self.snapshot_dir.join(format!("{:016x}.snap", test_id.0))
    }

    /// Compute Blake3 hash of content
    pub fn hash_content(content: &[u8]) -> Hash {
        blake3::hash(content)
    }

    /// Verify snapshot matches expected
    ///
    /// Uses hash-first verification for O(1) matching.
    /// Only loads content and generates diff on mismatch.
    pub fn verify(&self, test_id: TestId, actual: &[u8]) -> SnapshotResult {
        let actual_hash = Self::hash_content(actual);

        // Check if we have an entry for this test
        if let Some(entry) = self.data.entries.get(&test_id.0) {
            let stored_hash = Hash::from_bytes(entry.hash);

            // Hash-first comparison (O(1))
            if actual_hash == stored_hash {
                return SnapshotResult::Match;
            }

            // Hashes differ - load content and generate diff
            let snapshot_path = self.snapshot_dir.join(&entry.path);
            if let Ok(expected) = fs::read(&snapshot_path) {
                let diff = self.generate_diff(&expected, actual);
                return SnapshotResult::Mismatch { diff };
            }
        }

        // No existing snapshot
        SnapshotResult::New {
            content: actual.to_vec(),
        }
    }

    /// Generate unified diff between expected and actual content
    fn generate_diff(&self, expected: &[u8], actual: &[u8]) -> String {
        let expected_str = String::from_utf8_lossy(expected);
        let actual_str = String::from_utf8_lossy(actual);

        let diff = TextDiff::from_lines(&*expected_str, &*actual_str);

        let mut output = String::new();
        output.push_str("--- expected\n");
        output.push_str("+++ actual\n");

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                output.push_str("...\n");
            }

            for op in group {
                for change in diff.iter_changes(op) {
                    let sign = match change.tag() {
                        ChangeTag::Delete => "-",
                        ChangeTag::Insert => "+",
                        ChangeTag::Equal => " ",
                    };

                    output.push_str(sign);
                    output.push_str(change.value());
                    if change.missing_newline() {
                        output.push('\n');
                    }
                }
            }
        }

        output
    }

    /// Update snapshot content
    ///
    /// Atomically updates both hash and content.
    pub fn update(&mut self, test_id: TestId, content: &[u8]) -> Result<(), SnapshotError> {
        let hash = Self::hash_content(content);
        let filename = format!("{:016x}.snap", test_id.0);
        let snapshot_path = self.snapshot_dir.join(&filename);

        // Write content to file
        let mut file = File::create(&snapshot_path)?;
        file.write_all(content)?;

        // Update index entry
        self.data.entries.insert(
            test_id.0,
            SnapshotEntry {
                hash: *hash.as_bytes(),
                path: filename,
            },
        );

        self.dirty = true;
        Ok(())
    }

    /// Delete a snapshot
    pub fn delete(&mut self, test_id: TestId) -> Result<(), SnapshotError> {
        if let Some(entry) = self.data.entries.remove(&test_id.0) {
            let snapshot_path = self.snapshot_dir.join(&entry.path);
            if snapshot_path.exists() {
                fs::remove_file(&snapshot_path)?;
            }
            self.dirty = true;
        }
        Ok(())
    }

    /// Get the stored hash for a test
    pub fn get_hash(&self, test_id: TestId) -> Option<Hash> {
        self.data.entries.get(&test_id.0).map(|e| Hash::from_bytes(e.hash))
    }

    /// Check if a snapshot exists for a test
    pub fn contains(&self, test_id: TestId) -> bool {
        self.data.entries.contains_key(&test_id.0)
    }

    /// Get the number of snapshots
    pub fn len(&self) -> usize {
        self.data.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
    }

    /// Load snapshot content using memory mapping
    pub fn load_content(&self, test_id: TestId) -> Result<Vec<u8>, SnapshotError> {
        let entry = self
            .data
            .entries
            .get(&test_id.0)
            .ok_or_else(|| SnapshotError::NotFound(format!("Test {:?}", test_id)))?;

        let snapshot_path = self.snapshot_dir.join(&entry.path);
        let file = File::open(&snapshot_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(mmap.to_vec())
    }

    /// Clear all snapshots
    pub fn clear(&mut self) -> Result<(), SnapshotError> {
        for entry in self.data.entries.values() {
            let path = self.snapshot_dir.join(&entry.path);
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        self.data.entries.clear();
        self.dirty = true;
        Ok(())
    }
}

impl Drop for SnapshotIndex {
    fn drop(&mut self) {
        // Try to save on drop, ignore errors
        let _ = self.save();
    }
}

#[cfg(test)]
mod tests;
