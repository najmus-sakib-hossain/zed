//! Version control snapshots - commit-like snapshots for tool state
//!
//! Provides Git-like version control features for DX tool states, including:
//! - Creating snapshots of tool state
//! - Branching and merging
//! - Version history
//! - Diff computation

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::Version;
use crate::storage::Database;

/// Unique identifier for a snapshot
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(String);

impl SnapshotId {
    /// Create a new snapshot ID from content hash
    pub fn from_hash(hash: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(hash)))
    }

    /// Create from string (not to be confused with std::str::FromStr trait)
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the hash string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..8]) // Show first 8 chars like Git
    }
}

/// A snapshot of tool state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique identifier
    pub id: SnapshotId,

    /// Parent snapshot(s) - multiple parents for merges
    pub parents: Vec<SnapshotId>,

    /// Snapshot message
    pub message: String,

    /// Author
    pub author: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Tool state at this snapshot
    pub tool_states: HashMap<String, ToolState>,

    /// Files tracked in this snapshot
    pub files: HashMap<PathBuf, FileSnapshot>,

    /// Snapshot metadata
    pub metadata: HashMap<String, String>,
}

/// State of a tool at snapshot time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    pub tool_name: String,
    pub version: Version,
    pub config: HashMap<String, serde_json::Value>,
    pub output_files: Vec<PathBuf>,
}

/// Snapshot of a file's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

impl FileSnapshot {
    /// Create a file snapshot from a path
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read(path)?;
        let hash = format!("{:x}", Sha256::digest(&content));
        let metadata = std::fs::metadata(path)?;
        let modified =
            metadata.modified().map(DateTime::<Utc>::from).unwrap_or_else(|_| Utc::now());

        Ok(Self {
            path: path.to_path_buf(),
            hash,
            size: metadata.len(),
            modified,
        })
    }

    /// Check if file has changed since snapshot
    pub fn has_changed(&self) -> Result<bool> {
        if !self.path.exists() {
            return Ok(true);
        }

        let content = std::fs::read(&self.path)?;
        let current_hash = format!("{:x}", Sha256::digest(&content));
        Ok(current_hash != self.hash)
    }
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub head: SnapshotId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Snapshot manager for version control
pub struct SnapshotManager {
    _db: Database,
    snapshots_path: PathBuf,
    branches_path: PathBuf,
    current_branch: String,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(forge_dir: &Path) -> Result<Self> {
        // Ensure forge directory exists and open the shared forge database
        std::fs::create_dir_all(forge_dir)?;
        let db = Database::new(forge_dir)?;

        let snapshots_path = forge_dir.join("snapshots");
        let branches_path = forge_dir.join("branches.json");

        std::fs::create_dir_all(&snapshots_path)?;

        // Initialize with main branch if needed
        let current_branch = if branches_path.exists() {
            let content = std::fs::read_to_string(&branches_path)?;
            let branches: HashMap<String, Branch> = serde_json::from_str(&content)?;
            branches.keys().next().cloned().unwrap_or_else(|| "main".to_string())
        } else {
            "main".to_string()
        };

        Ok(Self {
            _db: db,
            snapshots_path,
            branches_path,
            current_branch,
        })
    }

    /// Create a new snapshot
    pub fn create_snapshot(
        &mut self,
        message: impl Into<String>,
        tool_states: HashMap<String, ToolState>,
        files: Vec<PathBuf>,
    ) -> Result<SnapshotId> {
        let author = whoami::username();
        let timestamp = Utc::now();
        let message: String = message.into();

        // Create file snapshots
        let mut file_snapshots = HashMap::new();
        for file in files {
            if file.exists() {
                let snapshot = FileSnapshot::from_path(&file)?;
                file_snapshots.insert(file, snapshot);
            }
        }

        // Get parent snapshot from current branch
        let parents = self
            .get_branch_head(&self.current_branch)?
            .map(|head| vec![head])
            .unwrap_or_default();

        // Compute snapshot ID from full commit-like content so that
        // even identical tool states taken at different times produce
        // distinct snapshot IDs. This prevents cycles in history
        // traversal.
        let id_content = serde_json::to_vec(&(
            &tool_states,
            &file_snapshots,
            &parents,
            &message,
            &author,
            timestamp,
        ))?;
        let id = SnapshotId::from_hash(&id_content);

        let snapshot = Snapshot {
            id: id.clone(),
            parents,
            message,
            author,
            timestamp,
            tool_states,
            files: file_snapshots,
            metadata: HashMap::new(),
        };

        // Save snapshot
        self.save_snapshot(&snapshot)?;

        // Update branch head (clone current_branch to avoid borrow conflict)
        let current_branch = self.current_branch.clone();
        self.update_branch_head(&current_branch, id.clone())?;

        tracing::info!("Created snapshot {} on branch {}", id, self.current_branch);
        Ok(id)
    }

    /// Get a snapshot by ID
    pub fn get_snapshot(&self, id: &SnapshotId) -> Result<Option<Snapshot>> {
        let snapshot_file = self.snapshots_path.join(format!("{}.json", id.as_str()));

        if !snapshot_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&snapshot_file)?;
        let snapshot: Snapshot = serde_json::from_str(&content)?;
        Ok(Some(snapshot))
    }

    /// Create a new branch
    pub fn create_branch(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();

        let head = self
            .get_branch_head(&self.current_branch)?
            .ok_or_else(|| anyhow::anyhow!("Current branch has no commits"))?;

        let branch = Branch {
            name: name.clone(),
            head,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.save_branch(&branch)?;
        tracing::info!("Created branch {}", name);
        Ok(())
    }

    /// Switch to a different branch
    pub fn checkout_branch(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();

        if !self.branch_exists(&name) {
            anyhow::bail!("Branch {} does not exist", name);
        }

        self.current_branch = name.clone();
        tracing::info!("Switched to branch {}", name);
        Ok(())
    }

    /// Get current branch name
    pub fn current_branch(&self) -> &str {
        &self.current_branch
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<Branch>> {
        if !self.branches_path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(&self.branches_path)?;
        let branches: HashMap<String, Branch> = serde_json::from_str(&content)?;
        Ok(branches.into_values().collect())
    }

    /// Get commit history for current branch
    pub fn history(&self, limit: usize) -> Result<Vec<Snapshot>> {
        let head = self.get_branch_head(&self.current_branch)?;

        if head.is_none() {
            return Ok(vec![]);
        }

        let mut history = Vec::new();
        let mut current = head;

        while let Some(id) = current {
            if history.len() >= limit {
                break;
            }

            if let Some(snapshot) = self.get_snapshot(&id)? {
                current = snapshot.parents.first().cloned();
                history.push(snapshot);
            } else {
                break;
            }
        }

        Ok(history)
    }

    /// Merge a branch into current branch
    pub fn merge(
        &mut self,
        source_branch: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<SnapshotId> {
        let source_branch = source_branch.into();

        let source_head = self
            .get_branch_head(&source_branch)?
            .ok_or_else(|| anyhow::anyhow!("Source branch has no commits"))?;

        let target_head = self
            .get_branch_head(&self.current_branch)?
            .ok_or_else(|| anyhow::anyhow!("Current branch has no commits"))?;

        // Get snapshots
        let source_snap = self
            .get_snapshot(&source_head)?
            .ok_or_else(|| anyhow::anyhow!("Source snapshot not found"))?;

        let target_snap = self
            .get_snapshot(&target_head)?
            .ok_or_else(|| anyhow::anyhow!("Target snapshot not found"))?;

        // Merge tool states (target takes precedence for conflicts)
        let mut merged_states = target_snap.tool_states.clone();
        for (name, state) in source_snap.tool_states {
            merged_states.entry(name).or_insert(state);
        }

        // Merge files
        let mut merged_files = target_snap.files.clone();
        for (path, file) in source_snap.files {
            merged_files.entry(path).or_insert(file);
        }

        // Create merge snapshot with both parents
        let author = whoami::username();
        let timestamp = Utc::now();
        let message: String = message.into();

        let id_content = serde_json::to_vec(&(
            &merged_states,
            &merged_files,
            &source_head,
            &target_head,
            &message,
            &author,
            timestamp,
        ))?;
        let id = SnapshotId::from_hash(&id_content);

        let snapshot = Snapshot {
            id: id.clone(),
            parents: vec![target_head, source_head],
            message,
            author,
            timestamp,
            tool_states: merged_states,
            files: merged_files,
            metadata: HashMap::new(),
        };

        self.save_snapshot(&snapshot)?;

        // Update branch head (clone to avoid borrow conflict)
        let current_branch = self.current_branch.clone();
        self.update_branch_head(&current_branch, id.clone())?;

        tracing::info!("Merged {} into {} ({})", source_branch, self.current_branch, id);
        Ok(id)
    }

    /// Compute diff between two snapshots
    pub fn diff(&self, from: &SnapshotId, to: &SnapshotId) -> Result<SnapshotDiff> {
        let from_snap = self
            .get_snapshot(from)?
            .ok_or_else(|| anyhow::anyhow!("From snapshot not found"))?;

        let to_snap =
            self.get_snapshot(to)?.ok_or_else(|| anyhow::anyhow!("To snapshot not found"))?;

        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        // Find added and modified files
        for (path, to_file) in &to_snap.files {
            match from_snap.files.get(path) {
                Some(from_file) => {
                    if from_file.hash != to_file.hash {
                        modified_files.push(path.clone());
                    }
                }
                None => {
                    added_files.push(path.clone());
                }
            }
        }

        // Find deleted files
        for path in from_snap.files.keys() {
            if !to_snap.files.contains_key(path) {
                deleted_files.push(path.clone());
            }
        }

        Ok(SnapshotDiff {
            from: from.clone(),
            to: to.clone(),
            added_files,
            modified_files,
            deleted_files,
        })
    }

    // Private helper methods

    fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let snapshot_file = self.snapshots_path.join(format!("{}.json", snapshot.id.as_str()));
        let content = serde_json::to_string_pretty(snapshot)?;
        std::fs::write(snapshot_file, content)?;
        Ok(())
    }

    fn save_branch(&self, branch: &Branch) -> Result<()> {
        let mut branches = if self.branches_path.exists() {
            let content = std::fs::read_to_string(&self.branches_path)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };

        branches.insert(branch.name.clone(), branch.clone());

        let content = serde_json::to_string_pretty(&branches)?;
        std::fs::write(&self.branches_path, content)?;
        Ok(())
    }

    fn get_branch_head(&self, name: &str) -> Result<Option<SnapshotId>> {
        if !self.branches_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.branches_path)?;
        let branches: HashMap<String, Branch> = serde_json::from_str(&content)?;

        Ok(branches.get(name).map(|b| b.head.clone()))
    }

    fn update_branch_head(&mut self, name: &str, head: SnapshotId) -> Result<()> {
        let mut branches: HashMap<String, Branch> = if self.branches_path.exists() {
            let content = std::fs::read_to_string(&self.branches_path)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };

        if let Some(branch) = branches.get_mut(name) {
            branch.head = head;
            branch.updated_at = Utc::now();
        } else {
            branches.insert(
                name.to_string(),
                Branch {
                    name: name.to_string(),
                    head,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }

        let content = serde_json::to_string_pretty(&branches)?;
        std::fs::write(&self.branches_path, content)?;
        Ok(())
    }

    fn branch_exists(&self, name: &str) -> bool {
        if !self.branches_path.exists() {
            return false;
        }

        if let Ok(content) = std::fs::read_to_string(&self.branches_path) {
            if let Ok(branches) = serde_json::from_str::<HashMap<String, Branch>>(&content) {
                return branches.contains_key(name);
            }
        }

        false
    }
}

/// Diff between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub from: SnapshotId,
    pub to: SnapshotId,
    pub added_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
}

impl SnapshotDiff {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.added_files.is_empty()
            || !self.modified_files.is_empty()
            || !self.deleted_files.is_empty()
    }

    /// Count total changed files
    pub fn total_changes(&self) -> usize {
        self.added_files.len() + self.modified_files.len() + self.deleted_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path()).unwrap();

        let mut tool_states = HashMap::new();
        tool_states.insert(
            "test-tool".to_string(),
            ToolState {
                tool_name: "test-tool".to_string(),
                version: Version::new(1, 0, 0),
                config: HashMap::new(),
                output_files: vec![],
            },
        );

        let id = manager.create_snapshot("Initial commit", tool_states, vec![]).unwrap();

        let snapshot = manager.get_snapshot(&id).unwrap().unwrap();
        assert_eq!(snapshot.message, "Initial commit");
        assert_eq!(snapshot.tool_states.len(), 1);
    }

    #[test]
    fn test_branching() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path()).unwrap();

        // Create initial snapshot
        manager.create_snapshot("Initial", HashMap::new(), vec![]).unwrap();

        // Create a new branch
        manager.create_branch("feature").unwrap();

        // Switch to new branch
        manager.checkout_branch("feature").unwrap();
        assert_eq!(manager.current_branch(), "feature");

        // List branches
        let branches = manager.list_branches().unwrap();
        assert!(branches.iter().any(|b| b.name == "feature"));
    }
}
