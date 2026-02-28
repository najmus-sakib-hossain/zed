//! Rule Snapshots
//!
//! Point-in-time captures of rule state for resumability.

use std::collections::HashMap;
use std::path::Path;

use crate::{DrivenError, Result};

use super::SharedRules;

/// Snapshot of rule state
#[derive(Debug, Clone)]
pub struct RuleSnapshot {
    /// Snapshot ID
    pub id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Version at snapshot time
    pub version: u64,
    /// Rule data
    pub rules: HashMap<u32, Vec<u8>>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl RuleSnapshot {
    /// Create a new snapshot
    pub fn new(id: u64) -> Self {
        Self {
            id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: 0,
            rules: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Capture from shared rules
    pub fn capture(shared: &SharedRules, id: u64) -> Self {
        let mut snapshot = Self::new(id);
        snapshot.version = shared.version();

        for rule_id in shared.ids() {
            if let Some(rule) = shared.get(rule_id) {
                snapshot.rules.insert(rule_id, rule.as_bytes().to_vec());
            }
        }

        snapshot
    }

    /// Restore to shared rules
    pub fn restore(&self, shared: &SharedRules) {
        shared.clear();
        for (id, content) in &self.rules {
            shared.insert(content.clone());
        }
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.rules.values().map(|v| v.len()).sum()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();

        // Header
        output.extend_from_slice(&self.id.to_le_bytes());
        output.extend_from_slice(&self.timestamp.to_le_bytes());
        output.extend_from_slice(&self.version.to_le_bytes());
        output.extend_from_slice(&(self.rules.len() as u32).to_le_bytes());

        // Rules
        for (id, content) in &self.rules {
            output.extend_from_slice(&id.to_le_bytes());
            output.extend_from_slice(&(content.len() as u32).to_le_bytes());
            output.extend_from_slice(content);
        }

        // Metadata count
        output.extend_from_slice(&(self.metadata.len() as u32).to_le_bytes());

        // Metadata
        for (key, value) in &self.metadata {
            output.extend_from_slice(&(key.len() as u16).to_le_bytes());
            output.extend_from_slice(key.as_bytes());
            output.extend_from_slice(&(value.len() as u16).to_le_bytes());
            output.extend_from_slice(value.as_bytes());
        }

        output
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 28 {
            return Err(DrivenError::InvalidBinary("Snapshot too small".into()));
        }

        let mut pos = 0;

        let id = u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        let timestamp = u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        let version = u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        let rule_count =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        let mut rules = HashMap::with_capacity(rule_count);

        for _ in 0..rule_count {
            if pos + 8 > data.len() {
                return Err(DrivenError::InvalidBinary("Truncated snapshot".into()));
            }

            let id = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;

            let len = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                as usize;
            pos += 4;

            if pos + len > data.len() {
                return Err(DrivenError::InvalidBinary("Truncated rule data".into()));
            }

            rules.insert(id, data[pos..pos + len].to_vec());
            pos += len;
        }

        // Read metadata
        let mut metadata = HashMap::new();
        if pos + 4 <= data.len() {
            let meta_count =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            for _ in 0..meta_count {
                if pos + 2 > data.len() {
                    break;
                }
                let key_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
                pos += 2;

                if pos + key_len > data.len() {
                    break;
                }
                let key = String::from_utf8_lossy(&data[pos..pos + key_len]).to_string();
                pos += key_len;

                if pos + 2 > data.len() {
                    break;
                }
                let val_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
                pos += 2;

                if pos + val_len > data.len() {
                    break;
                }
                let value = String::from_utf8_lossy(&data[pos..pos + val_len]).to_string();
                pos += val_len;

                metadata.insert(key, value);
            }
        }

        Ok(Self {
            id,
            timestamp,
            version,
            rules,
            metadata,
        })
    }
}

/// Snapshot manager for storing and retrieving snapshots
#[derive(Debug)]
pub struct SnapshotManager {
    /// Storage directory
    directory: std::path::PathBuf,
    /// Next snapshot ID
    next_id: u64,
    /// Maximum snapshots to keep
    max_snapshots: usize,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(directory: impl AsRef<Path>) -> Result<Self> {
        let directory = directory.as_ref().to_path_buf();
        std::fs::create_dir_all(&directory)?;

        // Find highest existing ID
        let next_id = Self::find_highest_id(&directory)? + 1;

        Ok(Self {
            directory,
            next_id,
            max_snapshots: 10,
        })
    }

    /// Set maximum snapshots to keep
    pub fn with_max_snapshots(mut self, max: usize) -> Self {
        self.max_snapshots = max;
        self
    }

    /// Create a snapshot
    pub fn create(&mut self, shared: &SharedRules) -> Result<RuleSnapshot> {
        let snapshot = RuleSnapshot::capture(shared, self.next_id);
        self.next_id += 1;

        self.save(&snapshot)?;
        self.cleanup()?;

        Ok(snapshot)
    }

    /// Save a snapshot to disk
    pub fn save(&self, snapshot: &RuleSnapshot) -> Result<()> {
        let path = self.snapshot_path(snapshot.id);
        let data = snapshot.to_bytes();
        std::fs::write(&path, &data)?;
        Ok(())
    }

    /// Load a snapshot from disk
    pub fn load(&self, id: u64) -> Result<RuleSnapshot> {
        let path = self.snapshot_path(id);
        let data = std::fs::read(&path)?;
        RuleSnapshot::from_bytes(&data)
    }

    /// List all snapshots
    pub fn list(&self) -> Result<Vec<u64>> {
        let mut ids = Vec::new();

        for entry in std::fs::read_dir(&self.directory)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if let Some(id_str) =
                    name.strip_prefix("snapshot_").and_then(|s| s.strip_suffix(".drv"))
                {
                    if let Ok(id) = id_str.parse() {
                        ids.push(id);
                    }
                }
            }
        }

        ids.sort();
        Ok(ids)
    }

    /// Load the latest snapshot
    pub fn load_latest(&self) -> Result<Option<RuleSnapshot>> {
        let ids = self.list()?;
        match ids.last() {
            Some(&id) => Ok(Some(self.load(id)?)),
            None => Ok(None),
        }
    }

    /// Delete a snapshot
    pub fn delete(&self, id: u64) -> Result<()> {
        let path = self.snapshot_path(id);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    /// Clean up old snapshots
    fn cleanup(&self) -> Result<()> {
        let mut ids = self.list()?;
        while ids.len() > self.max_snapshots {
            if let Some(oldest) = ids.first() {
                self.delete(*oldest)?;
                ids.remove(0);
            }
        }
        Ok(())
    }

    fn snapshot_path(&self, id: u64) -> std::path::PathBuf {
        self.directory.join(format!("snapshot_{:08}.drv", id))
    }

    fn find_highest_id(directory: &Path) -> Result<u64> {
        let mut highest = 0u64;

        if directory.exists() {
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(id_str) =
                        name.strip_prefix("snapshot_").and_then(|s| s.strip_suffix(".drv"))
                    {
                        if let Ok(id) = id_str.parse::<u64>() {
                            highest = highest.max(id);
                        }
                    }
                }
            }
        }

        Ok(highest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let mut snapshot = RuleSnapshot::new(1);
        snapshot.rules.insert(1, b"rule one".to_vec());
        snapshot.rules.insert(2, b"rule two".to_vec());
        snapshot.metadata.insert("key".to_string(), "value".to_string());

        let bytes = snapshot.to_bytes();
        let restored = RuleSnapshot::from_bytes(&bytes).unwrap();

        assert_eq!(restored.id, 1);
        assert_eq!(restored.rules.len(), 2);
        assert_eq!(restored.rules.get(&1).unwrap(), b"rule one");
    }

    #[test]
    fn test_capture_restore() {
        let shared = SharedRules::new();
        shared.insert(b"test rule".to_vec());

        let snapshot = RuleSnapshot::capture(&shared, 1);
        assert_eq!(snapshot.rule_count(), 1);

        let new_shared = SharedRules::new();
        snapshot.restore(&new_shared);
        assert_eq!(new_shared.len(), 1);
    }
}
