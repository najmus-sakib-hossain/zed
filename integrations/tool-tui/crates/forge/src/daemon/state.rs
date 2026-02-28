//! Daemon State Manager
//!
//! Persistent state management for the Forge Daemon with:
//! - Project state tracking
//! - Tool execution history
//! - Cache statistics
//! - R2 sync state

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::dx_cache::DxToolId;

// ============================================================================
// TOOL STATE
// ============================================================================

/// Individual tool state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    /// Tool identifier
    pub id: String,
    /// Current status
    pub status: ToolStatus,
    /// Last execution time
    pub last_run: Option<u64>,
    /// Last run duration (ms)
    pub last_duration_ms: Option<u64>,
    /// Last run success
    pub last_success: bool,
    /// Total executions
    pub total_runs: u64,
    /// Total successes
    pub total_successes: u64,
    /// Total failures
    pub total_failures: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average duration (ms)
    pub avg_duration_ms: f64,
}

/// Tool status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    /// Not yet run
    Idle,
    /// Currently running
    Running,
    /// Completed successfully
    Success,
    /// Failed
    Failed,
    /// Disabled
    Disabled,
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            id: String::new(),
            status: ToolStatus::Idle,
            last_run: None,
            last_duration_ms: None,
            last_success: false,
            total_runs: 0,
            total_successes: 0,
            total_failures: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_duration_ms: 0.0,
        }
    }
}

impl ToolState {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ..Default::default()
        }
    }

    /// Record an execution
    pub fn record_execution(&mut self, success: bool, duration_ms: u64, hits: u64, misses: u64) {
        // SystemTime::now() is always after UNIX_EPOCH on any reasonable system,
        // but we use unwrap_or(0) for safety in edge cases
        self.last_run = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
        self.last_duration_ms = Some(duration_ms);
        self.last_success = success;
        self.total_runs += 1;
        self.cache_hits += hits;
        self.cache_misses += misses;

        if success {
            self.total_successes += 1;
            self.status = ToolStatus::Success;
        } else {
            self.total_failures += 1;
            self.status = ToolStatus::Failed;
        }

        // Update average duration
        let total_duration =
            self.avg_duration_ms * (self.total_runs - 1) as f64 + duration_ms as f64;
        self.avg_duration_ms = total_duration / self.total_runs as f64;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_runs == 0 {
            0.0
        } else {
            self.total_successes as f64 / self.total_runs as f64
        }
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

// ============================================================================
// PROJECT STATE
// ============================================================================

/// Project state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Project root path
    pub root: PathBuf,
    /// Project name
    pub name: String,
    /// DX config file hash (for change detection)
    pub config_hash: Option<String>,
    /// Package.json hash
    pub package_hash: Option<String>,
    /// Last scanned timestamp
    pub last_scan: Option<u64>,
    /// Total files tracked
    pub files_tracked: u64,
    /// Tools state
    pub tools: HashMap<String, ToolState>,
    /// R2 sync state
    pub r2_sync: R2SyncState,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ProjectState {
    pub fn new(root: &Path, name: &str) -> Self {
        Self {
            root: root.to_path_buf(),
            name: name.to_string(),
            config_hash: None,
            package_hash: None,
            last_scan: None,
            files_tracked: 0,
            tools: HashMap::new(),
            r2_sync: R2SyncState::default(),
            metadata: HashMap::new(),
        }
    }

    /// Get or create tool state
    pub fn tool(&mut self, id: &str) -> &mut ToolState {
        self.tools.entry(id.to_string()).or_insert_with(|| ToolState::new(id))
    }

    /// Get tool state (read-only)
    pub fn get_tool(&self, id: &str) -> Option<&ToolState> {
        self.tools.get(id)
    }
}

/// R2 sync state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct R2SyncState {
    /// Enabled
    pub enabled: bool,
    /// Bucket name
    pub bucket: Option<String>,
    /// Last sync timestamp
    pub last_sync: Option<u64>,
    /// Files synced
    pub files_synced: u64,
    /// Bytes uploaded
    pub bytes_uploaded: u64,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Sync errors
    pub errors: u64,
}

// ============================================================================
// DAEMON STATE MANAGER
// ============================================================================

/// Daemon state manager
///
/// Manages persistent state for the Forge Daemon
pub struct DaemonStateManager {
    /// State file path
    state_file: PathBuf,
    /// Project states (keyed by root path)
    projects: Arc<RwLock<HashMap<PathBuf, ProjectState>>>,
    /// Auto-save enabled
    auto_save: bool,
}

impl DaemonStateManager {
    /// Create a new state manager
    pub fn new(state_file: &Path) -> Result<Self> {
        let projects = if state_file.exists() {
            let content = std::fs::read_to_string(state_file)
                .with_context(|| format!("Failed to read daemon state file: {:?}", state_file))?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            state_file: state_file.to_path_buf(),
            projects: Arc::new(RwLock::new(projects)),
            auto_save: true,
        })
    }

    /// Create an in-memory state manager (no persistence)
    pub fn in_memory() -> Self {
        Self {
            state_file: PathBuf::new(),
            projects: Arc::new(RwLock::new(HashMap::new())),
            auto_save: false,
        }
    }

    /// Get or create project state
    pub fn project(&self, root: &Path, name: &str) -> ProjectState {
        let mut projects = self.projects.write();
        projects
            .entry(root.to_path_buf())
            .or_insert_with(|| ProjectState::new(root, name))
            .clone()
    }

    /// Update project state
    pub fn update_project(&self, state: ProjectState) -> Result<()> {
        self.projects.write().insert(state.root.clone(), state);

        if self.auto_save {
            self.save().context("Failed to auto-save daemon state after project update")?;
        }

        Ok(())
    }

    /// Record tool execution
    pub fn record_tool_execution(
        &self,
        project_root: &Path,
        tool_id: DxToolId,
        success: bool,
        duration_ms: u64,
        cache_hits: u64,
        cache_misses: u64,
    ) -> Result<()> {
        let mut projects = self.projects.write();

        if let Some(project) = projects.get_mut(project_root) {
            project.tool(tool_id.folder_name()).record_execution(
                success,
                duration_ms,
                cache_hits,
                cache_misses,
            );
        }

        drop(projects);

        if self.auto_save {
            self.save().context("Failed to auto-save daemon state after tool execution")?;
        }

        Ok(())
    }

    /// Get all projects
    pub fn all_projects(&self) -> Vec<ProjectState> {
        self.projects.read().values().cloned().collect()
    }

    /// Remove a project
    pub fn remove_project(&self, root: &Path) -> Result<()> {
        self.projects.write().remove(root);

        if self.auto_save {
            self.save().context("Failed to auto-save daemon state after project removal")?;
        }

        Ok(())
    }

    /// Save state to disk
    pub fn save(&self) -> Result<()> {
        if self.state_file.as_os_str().is_empty() {
            return Ok(()); // In-memory mode
        }

        // Create parent directory if needed
        if let Some(parent) = self.state_file.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory for state file: {:?}", parent)
            })?;
        }

        let content = serde_json::to_string_pretty(&*self.projects.read())
            .context("Failed to serialize daemon state")?;
        std::fs::write(&self.state_file, content)
            .with_context(|| format!("Failed to write daemon state file: {:?}", self.state_file))?;

        Ok(())
    }

    /// Load state from disk
    pub fn load(&self) -> Result<()> {
        if self.state_file.exists() {
            let content = std::fs::read_to_string(&self.state_file).with_context(|| {
                format!("Failed to read daemon state file: {:?}", self.state_file)
            })?;
            let projects: HashMap<PathBuf, ProjectState> =
                serde_json::from_str(&content).context("Failed to parse daemon state file")?;
            *self.projects.write() = projects;
        }
        Ok(())
    }

    /// Clear all state
    pub fn clear(&self) -> Result<()> {
        self.projects.write().clear();

        if self.auto_save {
            self.save().context("Failed to auto-save daemon state after clearing")?;
        }

        Ok(())
    }

    /// Get aggregated statistics
    pub fn stats(&self) -> DaemonStateStats {
        let projects = self.projects.read();

        let mut total_runs = 0u64;
        let mut total_successes = 0u64;
        let mut total_cache_hits = 0u64;
        let mut total_cache_misses = 0u64;
        let mut tool_stats: HashMap<String, ToolState> = HashMap::new();

        for project in projects.values() {
            for (tool_id, tool) in &project.tools {
                total_runs += tool.total_runs;
                total_successes += tool.total_successes;
                total_cache_hits += tool.cache_hits;
                total_cache_misses += tool.cache_misses;

                let entry =
                    tool_stats.entry(tool_id.clone()).or_insert_with(|| ToolState::new(tool_id));
                entry.total_runs += tool.total_runs;
                entry.total_successes += tool.total_successes;
                entry.total_failures += tool.total_failures;
                entry.cache_hits += tool.cache_hits;
                entry.cache_misses += tool.cache_misses;
            }
        }

        DaemonStateStats {
            projects: projects.len(),
            total_runs,
            total_successes,
            success_rate: if total_runs > 0 {
                total_successes as f64 / total_runs as f64
            } else {
                0.0
            },
            cache_hits: total_cache_hits,
            cache_misses: total_cache_misses,
            cache_hit_rate: if total_cache_hits + total_cache_misses > 0 {
                total_cache_hits as f64 / (total_cache_hits + total_cache_misses) as f64
            } else {
                0.0
            },
            tool_stats,
        }
    }
}

/// Aggregated daemon statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStateStats {
    pub projects: usize,
    pub total_runs: u64,
    pub total_successes: u64,
    pub success_rate: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub tool_stats: HashMap<String, ToolState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_state_record_execution() {
        let mut state = ToolState::new("bundler");

        state.record_execution(true, 100, 10, 5);
        assert_eq!(state.total_runs, 1);
        assert_eq!(state.total_successes, 1);
        assert_eq!(state.cache_hits, 10);
        assert_eq!(state.cache_misses, 5);

        state.record_execution(false, 200, 0, 10);
        assert_eq!(state.total_runs, 2);
        assert_eq!(state.total_successes, 1);
        assert_eq!(state.total_failures, 1);
    }

    #[test]
    fn test_project_state() {
        let mut project = ProjectState::new(Path::new("/test"), "test-project");

        project.tool("bundler").record_execution(true, 50, 5, 2);

        let tool = project.get_tool("bundler").unwrap();
        assert_eq!(tool.total_runs, 1);
    }

    #[test]
    fn test_state_manager_in_memory() {
        let manager = DaemonStateManager::in_memory();

        let project = manager.project(Path::new("/test"), "test");
        assert_eq!(project.name, "test");

        let stats = manager.stats();
        assert_eq!(stats.projects, 1);
    }
}
