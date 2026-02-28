//! Snapshot Testing - Compare actual vs expected output
//!
//! Store expected output snapshots and compare against actual results.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Snapshot file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// Version for format compatibility
    pub version: u32,
    /// Snapshots keyed by test name
    pub snapshots: HashMap<String, Snapshot>,
}

impl Default for SnapshotFile {
    fn default() -> Self {
        Self {
            version: 1,
            snapshots: HashMap::new(),
        }
    }
}

/// A single snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The expected value (serialized)
    pub value: String,
    /// When this snapshot was last updated
    pub updated_at: u64,
    /// Number of times this snapshot has been verified
    pub verified_count: u32,
}

impl Snapshot {
    pub fn new(value: String) -> Self {
        Self {
            value,
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            verified_count: 0,
        }
    }
}

/// Snapshot comparison result
#[derive(Debug, Clone)]
pub enum SnapshotResult {
    /// Snapshot matches
    Match,
    /// Snapshot doesn't match (expected, actual)
    Mismatch { expected: String, actual: String },
    /// No snapshot exists yet
    New { actual: String },
}

/// Snapshot manager for a test file
pub struct SnapshotManager {
    /// Path to the snapshot file
    snapshot_path: PathBuf,
    /// Loaded snapshots
    snapshots: SnapshotFile,
    /// Whether to update snapshots
    update_mode: bool,
    /// Pending updates
    pending_updates: HashMap<String, String>,
}

impl SnapshotManager {
    /// Create a new snapshot manager for a test file
    pub fn new(test_file: &Path, update_mode: bool) -> Self {
        let snapshot_path = Self::snapshot_path_for(test_file);
        let snapshots = Self::load_snapshots(&snapshot_path).unwrap_or_default();

        Self {
            snapshot_path,
            snapshots,
            update_mode,
            pending_updates: HashMap::new(),
        }
    }

    /// Get the snapshot file path for a test file
    fn snapshot_path_for(test_file: &Path) -> PathBuf {
        let parent = test_file.parent().unwrap_or(Path::new("."));
        let snapshots_dir = parent.join("__snapshots__");
        let file_name = test_file.file_name().and_then(|s| s.to_str()).unwrap_or("test");
        snapshots_dir.join(format!("{}.snap", file_name))
    }

    /// Load snapshots from file
    fn load_snapshots(path: &Path) -> Option<SnapshotFile> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }

    /// Compare a value against its snapshot
    pub fn compare(&mut self, test_name: &str, actual: &str) -> SnapshotResult {
        if let Some(snapshot) = self.snapshots.snapshots.get_mut(test_name) {
            if snapshot.value == actual {
                snapshot.verified_count += 1;
                SnapshotResult::Match
            } else if self.update_mode {
                // Update the snapshot
                self.pending_updates.insert(test_name.to_string(), actual.to_string());
                SnapshotResult::Match
            } else {
                SnapshotResult::Mismatch {
                    expected: snapshot.value.clone(),
                    actual: actual.to_string(),
                }
            }
        } else if self.update_mode {
            // Create new snapshot
            self.pending_updates.insert(test_name.to_string(), actual.to_string());
            SnapshotResult::Match
        } else {
            SnapshotResult::New {
                actual: actual.to_string(),
            }
        }
    }

    /// Save pending updates
    pub fn save(&mut self) -> std::io::Result<()> {
        if self.pending_updates.is_empty() {
            return Ok(());
        }

        // Apply pending updates
        for (name, value) in self.pending_updates.drain() {
            self.snapshots.snapshots.insert(name, Snapshot::new(value));
        }

        // Create snapshots directory
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write snapshot file
        let file = File::create(&self.snapshot_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.snapshots)?;

        Ok(())
    }

    /// Get number of snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.snapshots.len()
    }

    /// Get number of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }
}

/// Serialize a value for snapshot comparison
pub fn serialize_for_snapshot<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", "serialization error"))
}

/// Generate a diff between expected and actual
pub fn generate_diff(expected: &str, actual: &str) -> String {
    let mut diff = String::new();

    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp = expected_lines.get(i).copied().unwrap_or("");
        let act = actual_lines.get(i).copied().unwrap_or("");

        if exp == act {
            diff.push_str(&format!("  {}\n", exp));
        } else {
            if !exp.is_empty() {
                diff.push_str(&format!("- {}\n", exp));
            }
            if !act.is_empty() {
                diff.push_str(&format!("+ {}\n", act));
            }
        }
    }

    diff
}

/// Inline snapshot support (for toMatchInlineSnapshot)
pub struct InlineSnapshot {
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Expected value (from source)
    pub expected: Option<String>,
    /// Actual value
    pub actual: String,
}

impl InlineSnapshot {
    /// Check if inline snapshot matches
    pub fn matches(&self) -> bool {
        match &self.expected {
            Some(expected) => expected == &self.actual,
            None => false, // New snapshot
        }
    }

    /// Generate source code update for inline snapshot
    pub fn generate_update(&self) -> String {
        // Escape the actual value for embedding in source
        let escaped = self.actual.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");

        format!("`{}`", escaped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_comparison() {
        let temp_dir = std::env::temp_dir().join("dx-test-snapshots");
        let test_file = temp_dir.join("test.ts");

        let mut manager = SnapshotManager::new(&test_file, true);

        // First comparison creates new snapshot
        let result = manager.compare("test1", "hello world");
        assert!(matches!(result, SnapshotResult::Match));

        // Save and reload
        manager.save().unwrap();

        let mut manager2 = SnapshotManager::new(&test_file, false);

        // Same value should match
        let result = manager2.compare("test1", "hello world");
        assert!(matches!(result, SnapshotResult::Match));

        // Different value should mismatch
        let result = manager2.compare("test1", "goodbye world");
        assert!(matches!(result, SnapshotResult::Mismatch { .. }));

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_diff_generation() {
        let expected = "line1\nline2\nline3";
        let actual = "line1\nmodified\nline3";

        let diff = generate_diff(expected, actual);
        assert!(diff.contains("- line2"));
        assert!(diff.contains("+ modified"));
    }
}
