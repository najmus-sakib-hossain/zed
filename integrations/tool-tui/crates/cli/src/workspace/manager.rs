//! Multi-Workspace Manager
//!
//! Enables running multiple workspaces simultaneously with isolated task execution.
//!
//! # Features
//!
//! - Concurrent workspace execution
//! - Task isolation and scheduling
//! - Resource management and limits
//! - Inter-workspace communication
//! - Progress monitoring across workspaces
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  Multi-Workspace Orchestrator                    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
//! │  │ Workspace A │  │ Workspace B │  │ Workspace C │             │
//! │  │ (Research)  │  │   (Code)    │  │  (Deploy)   │             │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
//! │         │                │                │                     │
//! │         └────────────────┼────────────────┘                     │
//! │                          ▼                                      │
//! │              ┌───────────────────────┐                         │
//! │              │    Task Scheduler     │                         │
//! │              │  (Priority + Limits)  │                         │
//! │              └───────────────────────┘                         │
//! │                          │                                      │
//! │              ┌───────────┴───────────┐                         │
//! │              ▼                       ▼                          │
//! │  ┌─────────────────┐    ┌─────────────────┐                   │
//! │  │ Resource Pool   │    │  Message Bus    │                   │
//! │  │ (CPU, Memory)   │    │ (IPC Channel)   │                   │
//! │  └─────────────────┘    └─────────────────┘                   │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::workspace::{WorkspaceManager, WorkspaceConfig, Task};
//!
//! let mut manager = WorkspaceManager::new()?;
//!
//! // Create workspaces
//! let research_ws = manager.create_workspace(WorkspaceConfig {
//!     name: "research".to_string(),
//!     task_type: TaskType::Research,
//!     ..Default::default()
//! }).await?;
//!
//! let code_ws = manager.create_workspace(WorkspaceConfig {
//!     name: "coding".to_string(),
//!     task_type: TaskType::Coding,
//!     ..Default::default()
//! }).await?;
//!
//! // Submit tasks
//! manager.submit_task(research_ws, Task::new("Research Rust async patterns")).await?;
//! manager.submit_task(code_ws, Task::new("Implement WebSocket handler")).await?;
//!
//! // Run all workspaces concurrently
//! manager.run_all().await?;
//! ```

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::sync::{RwLock, Semaphore, broadcast};
use tracing::{debug, info};
use uuid::Uuid;

/// Workspace identifier
pub type WorkspaceId = Uuid;

/// Task identifier
pub type TaskId = Uuid;

/// Workspace manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerConfig {
    /// Maximum concurrent workspaces
    pub max_workspaces: usize,
    /// Maximum concurrent tasks per workspace
    pub max_tasks_per_workspace: usize,
    /// Global memory limit (bytes)
    pub global_memory_limit: u64,
    /// Per-workspace memory limit (bytes)
    pub per_workspace_memory_limit: u64,
    /// Task timeout (seconds)
    pub task_timeout_secs: u64,
    /// Enable task isolation
    pub enable_isolation: bool,
    /// Workspace data directory
    pub data_dir: PathBuf,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            max_workspaces: 10,
            max_tasks_per_workspace: 5,
            global_memory_limit: 4 * 1024 * 1024 * 1024, // 4GB
            per_workspace_memory_limit: 512 * 1024 * 1024, // 512MB
            task_timeout_secs: 300,
            enable_isolation: true,
            data_dir: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dx")
                .join("workspaces"),
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace name
    pub name: String,
    /// Workspace description
    pub description: Option<String>,
    /// Task type this workspace handles
    pub task_type: TaskType,
    /// Priority (higher = more resources)
    pub priority: Priority,
    /// Memory limit override
    pub memory_limit: Option<u64>,
    /// Custom environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Auto-restart on failure
    pub auto_restart: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: None,
            task_type: TaskType::General,
            priority: Priority::Normal,
            memory_limit: None,
            env: HashMap::new(),
            working_dir: None,
            auto_restart: true,
            max_retries: 3,
        }
    }
}

/// Task types for workspaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// General purpose
    General,
    /// Research and information gathering
    Research,
    /// Code writing and editing
    Coding,
    /// File operations
    FileOps,
    /// Browser automation
    Browser,
    /// Shell commands
    Shell,
    /// API calls
    Api,
    /// Data processing
    DataProcessing,
    /// Deployment tasks
    Deploy,
    /// Background/long-running
    Background,
}

impl Default for TaskType {
    fn default() -> Self {
        Self::General
    }
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Lowest priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Workspace state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceState {
    /// Created but not started
    Created,
    /// Initializing resources
    Initializing,
    /// Running and processing tasks
    Running,
    /// Paused
    Paused,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Error state
    Error,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Queued for execution
    Queued,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Timed out
    TimedOut,
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    /// Task description
    pub description: String,
    /// Task type
    pub task_type: TaskType,
    /// Priority
    pub priority: Priority,
    /// Input data
    pub input: serde_json::Value,
    /// Timeout override (seconds)
    pub timeout: Option<u64>,
    /// Dependencies (task IDs that must complete first)
    pub dependencies: Vec<TaskId>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Task {
    /// Create a new task
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            task_type: TaskType::General,
            priority: Priority::Normal,
            input: serde_json::Value::Null,
            timeout: None,
            dependencies: Vec::new(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set task type
    pub fn with_type(mut self, task_type: TaskType) -> Self {
        self.task_type = task_type;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set input data
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    /// Add dependency
    pub fn depends_on(mut self, task_id: TaskId) -> Self {
        self.dependencies.push(task_id);
        self
    }
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: TaskId,
    /// Final state
    pub state: TaskState,
    /// Output data
    pub output: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration
    pub duration_ms: u64,
    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp
    pub completed_at: DateTime<Utc>,
}

/// Workspace instance
pub struct Workspace {
    /// Workspace ID
    pub id: WorkspaceId,
    /// Configuration
    pub config: WorkspaceConfig,
    /// Current state
    state: Arc<RwLock<WorkspaceState>>,
    /// Task queue
    task_queue: Arc<RwLock<VecDeque<Task>>>,
    /// Running tasks
    running_tasks: Arc<DashMap<TaskId, Task>>,
    /// Completed results
    results: Arc<RwLock<Vec<TaskResult>>>,
    /// Task semaphore (limits concurrent tasks)
    task_semaphore: Arc<Semaphore>,
    /// Memory usage (bytes)
    memory_usage: Arc<AtomicU64>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Shutdown signal
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(config: WorkspaceConfig, max_concurrent_tasks: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            state: Arc::new(RwLock::new(WorkspaceState::Created)),
            task_queue: Arc::new(RwLock::new(VecDeque::new())),
            running_tasks: Arc::new(DashMap::new()),
            results: Arc::new(RwLock::new(Vec::new())),
            task_semaphore: Arc::new(Semaphore::new(max_concurrent_tasks)),
            memory_usage: Arc::new(AtomicU64::new(0)),
            created_at: Utc::now(),
            shutdown_tx: None,
        }
    }

    /// Get current state
    pub async fn state(&self) -> WorkspaceState {
        *self.state.read().await
    }

    /// Set state
    async fn set_state(&self, state: WorkspaceState) {
        *self.state.write().await = state;
    }

    /// Queue a task
    pub async fn queue_task(&self, task: Task) -> Result<TaskId> {
        let task_id = task.id;
        self.task_queue.write().await.push_back(task);
        debug!("Queued task {} in workspace {}", task_id, self.id);
        Ok(task_id)
    }

    /// Get queue length
    pub async fn queue_len(&self) -> usize {
        self.task_queue.read().await.len()
    }

    /// Get running task count
    pub fn running_count(&self) -> usize {
        self.running_tasks.len()
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Get results
    pub async fn results(&self) -> Vec<TaskResult> {
        self.results.read().await.clone()
    }

    /// Start the workspace
    pub async fn start(&mut self) -> Result<()> {
        self.set_state(WorkspaceState::Initializing).await;

        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Create workspace directory
        let ws_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("workspaces")
            .join(self.id.to_string());

        tokio::fs::create_dir_all(&ws_dir).await.ok();

        self.set_state(WorkspaceState::Running).await;
        info!("Workspace {} started: {}", self.id, self.config.name);

        Ok(())
    }

    /// Stop the workspace
    pub async fn stop(&mut self) -> Result<()> {
        self.set_state(WorkspaceState::Stopping).await;

        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Wait for running tasks to complete (with timeout)
        let timeout = tokio::time::Duration::from_secs(30);
        let start = std::time::Instant::now();

        while self.running_tasks.len() > 0 && start.elapsed() < timeout {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        self.set_state(WorkspaceState::Stopped).await;
        info!("Workspace {} stopped: {}", self.id, self.config.name);

        Ok(())
    }

    /// Process next task from queue
    async fn process_next(&self) -> Option<TaskResult> {
        let task = {
            let mut queue = self.task_queue.write().await;
            queue.pop_front()
        }?;

        let task_id = task.id;
        self.running_tasks.insert(task_id, task.clone());

        let started_at = Utc::now();
        let start_time = std::time::Instant::now();

        // Execute task (placeholder - would integrate with agent)
        let result = self.execute_task(&task).await;

        let completed_at = Utc::now();
        let duration_ms = start_time.elapsed().as_millis() as u64;

        self.running_tasks.remove(&task_id);

        let (output, error) = match result {
            Ok(val) => (Some(val), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let task_result = TaskResult {
            task_id,
            state: if error.is_none() {
                TaskState::Completed
            } else {
                TaskState::Failed
            },
            output,
            error,
            duration_ms,
            started_at: Some(started_at),
            completed_at,
        };

        self.results.write().await.push(task_result.clone());

        Some(task_result)
    }

    /// Execute a task
    async fn execute_task(&self, task: &Task) -> Result<serde_json::Value> {
        debug!("Executing task {}: {}", task.id, task.description);

        // This would integrate with the actual agent/executor
        // For now, simulate task execution
        match task.task_type {
            TaskType::Research => {
                // Simulate research task
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok(serde_json::json!({
                    "status": "completed",
                    "findings": []
                }))
            }
            TaskType::Coding => {
                // Simulate coding task
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                Ok(serde_json::json!({
                    "status": "completed",
                    "files_modified": []
                }))
            }
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                Ok(serde_json::json!({
                    "status": "completed"
                }))
            }
        }
    }
}

/// Inter-workspace message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMessage {
    /// Source workspace
    pub from: WorkspaceId,
    /// Target workspace (None = broadcast)
    pub to: Option<WorkspaceId>,
    /// Message type
    pub msg_type: MessageType,
    /// Payload
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Task completed notification
    TaskCompleted {
        /// ID of the completed task
        task_id: TaskId,
    },
    /// Data sharing between workspaces
    DataShare {
        /// Key identifying the shared data
        key: String,
    },
    /// Status update notification
    StatusUpdate,
    /// Resource request from another workspace
    ResourceRequest {
        /// Name of the requested resource
        resource: String,
    },
    /// Custom message type
    Custom(String),
}

/// Multi-workspace manager
pub struct WorkspaceManager {
    /// Configuration
    config: ManagerConfig,
    /// Active workspaces
    workspaces: Arc<DashMap<WorkspaceId, Arc<RwLock<Workspace>>>>,
    /// Message bus
    message_tx: broadcast::Sender<WorkspaceMessage>,
    /// Global task semaphore
    global_semaphore: Arc<Semaphore>,
    /// Total memory usage
    total_memory: Arc<AtomicU64>,
    /// Workspace count
    workspace_count: Arc<AtomicUsize>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new() -> Result<Self> {
        Self::with_config(ManagerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ManagerConfig) -> Result<Self> {
        let (message_tx, _) = broadcast::channel(1000);

        // Create data directory
        std::fs::create_dir_all(&config.data_dir)
            .context("Failed to create workspaces directory")?;

        Ok(Self {
            config: config.clone(),
            workspaces: Arc::new(DashMap::new()),
            message_tx,
            global_semaphore: Arc::new(Semaphore::new(
                config.max_workspaces * config.max_tasks_per_workspace,
            )),
            total_memory: Arc::new(AtomicU64::new(0)),
            workspace_count: Arc::new(AtomicUsize::new(0)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Create a new workspace
    pub async fn create_workspace(&self, config: WorkspaceConfig) -> Result<WorkspaceId> {
        let current_count = self.workspace_count.load(Ordering::Relaxed);
        if current_count >= self.config.max_workspaces {
            bail!(
                "Maximum workspace limit reached ({}/{})",
                current_count,
                self.config.max_workspaces
            );
        }

        let workspace = Workspace::new(config.clone(), self.config.max_tasks_per_workspace);
        let ws_id = workspace.id;

        self.workspaces.insert(ws_id, Arc::new(RwLock::new(workspace)));
        self.workspace_count.fetch_add(1, Ordering::Relaxed);

        info!("Created workspace {}: {}", ws_id, config.name);
        Ok(ws_id)
    }

    /// Get a workspace
    pub fn get_workspace(&self, id: WorkspaceId) -> Option<Arc<RwLock<Workspace>>> {
        self.workspaces.get(&id).map(|w| w.clone())
    }

    /// List all workspaces
    pub fn list_workspaces(&self) -> Vec<WorkspaceId> {
        self.workspaces.iter().map(|e| *e.key()).collect()
    }

    /// Submit a task to a workspace
    pub async fn submit_task(&self, workspace_id: WorkspaceId, task: Task) -> Result<TaskId> {
        let workspace = self.workspaces.get(&workspace_id).context("Workspace not found")?;

        let task_id = workspace.read().await.queue_task(task).await?;
        Ok(task_id)
    }

    /// Start all workspaces
    pub async fn start_all(&self) -> Result<()> {
        *self.running.write().await = true;

        for ws_ref in self.workspaces.iter() {
            let mut workspace = ws_ref.write().await;
            workspace.start().await?;
        }

        info!("Started {} workspaces", self.workspaces.len());
        Ok(())
    }

    /// Stop all workspaces
    pub async fn stop_all(&self) -> Result<()> {
        *self.running.write().await = false;

        for ws_ref in self.workspaces.iter() {
            let mut workspace = ws_ref.write().await;
            workspace.stop().await?;
        }

        info!("Stopped all workspaces");
        Ok(())
    }

    /// Run all workspaces concurrently
    pub async fn run_all(&self) -> Result<()> {
        self.start_all().await?;

        let workspaces: Vec<_> = self.workspaces.iter().map(|w| w.value().clone()).collect();

        let handles: Vec<_> = workspaces
            .into_iter()
            .map(|ws| {
                let running = self.running.clone();

                tokio::spawn(async move {
                    while *running.read().await {
                        let workspace = ws.read().await;
                        if workspace.state().await != WorkspaceState::Running {
                            break;
                        }
                        drop(workspace);

                        // Process tasks
                        let ws_ref = ws.read().await;
                        if let Some(result) = ws_ref.process_next().await {
                            debug!("Task {} completed: {:?}", result.task_id, result.state);
                        } else {
                            // No tasks, wait a bit
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                })
            })
            .collect();

        // Wait for all workspaces
        for handle in handles {
            handle.await.ok();
        }

        Ok(())
    }

    /// Send message between workspaces
    pub fn send_message(&self, message: WorkspaceMessage) -> Result<()> {
        self.message_tx
            .send(message)
            .map_err(|_| anyhow::anyhow!("Failed to send message"))?;
        Ok(())
    }

    /// Subscribe to messages
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceMessage> {
        self.message_tx.subscribe()
    }

    /// Get workspace status
    pub async fn status(&self) -> ManagerStatus {
        let mut workspace_status = Vec::new();

        for ws_ref in self.workspaces.iter() {
            let ws = ws_ref.read().await;
            workspace_status.push(WorkspaceStatus {
                id: ws.id,
                name: ws.config.name.clone(),
                state: ws.state().await,
                queue_len: ws.queue_len().await,
                running_tasks: ws.running_count(),
                memory_usage: ws.memory_usage(),
            });
        }

        ManagerStatus {
            total_workspaces: self.workspaces.len(),
            running: *self.running.read().await,
            total_memory: self.total_memory.load(Ordering::Relaxed),
            workspaces: workspace_status,
        }
    }

    /// Destroy a workspace
    pub async fn destroy_workspace(&self, id: WorkspaceId) -> Result<()> {
        if let Some((_, ws)) = self.workspaces.remove(&id) {
            let mut workspace = ws.write().await;
            workspace.stop().await?;
            self.workspace_count.fetch_sub(1, Ordering::Relaxed);
            info!("Destroyed workspace {}", id);
        }
        Ok(())
    }
}

/// Manager status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    /// Total workspace count
    pub total_workspaces: usize,
    /// Is running
    pub running: bool,
    /// Total memory usage
    pub total_memory: u64,
    /// Individual workspace status
    pub workspaces: Vec<WorkspaceStatus>,
}

/// Individual workspace status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// Workspace ID
    pub id: WorkspaceId,
    /// Workspace name
    pub name: String,
    /// Current state
    pub state: WorkspaceState,
    /// Tasks in queue
    pub queue_len: usize,
    /// Currently running tasks
    pub running_tasks: usize,
    /// Memory usage
    pub memory_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workspace_creation() {
        let manager = WorkspaceManager::new().unwrap();

        let ws_id = manager
            .create_workspace(WorkspaceConfig {
                name: "test".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(manager.get_workspace(ws_id).is_some());
    }

    #[tokio::test]
    async fn test_task_submission() {
        let manager = WorkspaceManager::new().unwrap();

        let ws_id = manager.create_workspace(WorkspaceConfig::default()).await.unwrap();

        let task = Task::new("Test task")
            .with_type(TaskType::Research)
            .with_priority(Priority::High);

        let task_id = manager.submit_task(ws_id, task).await.unwrap();
        assert!(!task_id.is_nil());
    }

    #[tokio::test]
    async fn test_multiple_workspaces() {
        let manager = WorkspaceManager::new().unwrap();

        let ws1 = manager
            .create_workspace(WorkspaceConfig {
                name: "research".to_string(),
                task_type: TaskType::Research,
                ..Default::default()
            })
            .await
            .unwrap();

        let ws2 = manager
            .create_workspace(WorkspaceConfig {
                name: "coding".to_string(),
                task_type: TaskType::Coding,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_ne!(ws1, ws2);
        assert_eq!(manager.list_workspaces().len(), 2);
    }

    #[tokio::test]
    async fn test_workspace_state() {
        let manager = WorkspaceManager::new().unwrap();

        let ws_id = manager.create_workspace(WorkspaceConfig::default()).await.unwrap();

        let ws = manager.get_workspace(ws_id).unwrap();
        let workspace = ws.read().await;

        assert_eq!(workspace.state().await, WorkspaceState::Created);
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("Research async patterns")
            .with_type(TaskType::Research)
            .with_priority(Priority::High)
            .with_input(serde_json::json!({"query": "rust async"}));

        assert_eq!(task.task_type, TaskType::Research);
        assert_eq!(task.priority, Priority::High);
        assert!(!task.input.is_null());
    }

    #[tokio::test]
    async fn test_manager_status() {
        let manager = WorkspaceManager::new().unwrap();

        manager
            .create_workspace(WorkspaceConfig {
                name: "ws1".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        manager
            .create_workspace(WorkspaceConfig {
                name: "ws2".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        let status = manager.status().await;
        assert_eq!(status.total_workspaces, 2);
        assert_eq!(status.workspaces.len(), 2);
    }
}
