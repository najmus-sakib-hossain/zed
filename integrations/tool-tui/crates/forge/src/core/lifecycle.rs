//! Tool lifecycle management
//!
//! Manages the lifecycle of DX tools including starting, stopping, and monitoring their status.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::orchestrator::DxTool;

/// Unique identifier for a tool instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolId(Uuid);

impl ToolId {
    /// Create a new random tool ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ToolId {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is registered but not running
    Stopped,

    /// Tool is in the process of starting
    Starting,

    /// Tool is running
    Running,

    /// Tool is in the process of stopping
    Stopping,

    /// Tool failed with an error
    Failed(String),

    /// Tool completed successfully
    Completed,
}

/// Lifecycle event emitted by the manager
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    /// Tool is starting
    ToolStarting { id: ToolId, name: String },

    /// Tool has started successfully
    ToolStarted { id: ToolId, name: String },

    /// Tool is stopping
    ToolStopping { id: ToolId, name: String },

    /// Tool has stopped
    ToolStopped { id: ToolId, name: String },

    /// Tool failed with an error
    ToolFailed {
        id: ToolId,
        name: String,
        error: String,
    },

    /// Tool completed successfully
    ToolCompleted { id: ToolId, name: String },
}

/// State of a managed tool
pub struct ToolState {
    pub id: ToolId,
    pub status: ToolStatus,
    pub tool: Box<dyn DxTool>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub handle: Option<JoinHandle<()>>,
}

impl ToolState {
    fn new(id: ToolId, tool: Box<dyn DxTool>) -> Self {
        Self {
            id,
            status: ToolStatus::Stopped,
            tool,
            started_at: None,
            stopped_at: None,
            handle: None,
        }
    }
}

/// Manages the lifecycle of DX tools
pub struct LifecycleManager {
    tools: HashMap<ToolId, ToolState>,
    event_bus: broadcast::Sender<LifecycleEvent>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        let (event_bus, _) = broadcast::channel(1000);

        Self {
            tools: HashMap::new(),
            event_bus,
        }
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: Box<dyn DxTool>) -> Result<ToolId> {
        let id = ToolId::new();
        let state = ToolState::new(id, tool);

        self.tools.insert(id, state);

        tracing::debug!("Registered tool with id: {:?}", id);
        Ok(id)
    }

    /// Start a tool
    pub async fn start_tool(&mut self, id: ToolId) -> Result<()> {
        let state = self.tools.get_mut(&id).ok_or_else(|| anyhow!("Tool not found: {:?}", id))?;

        if state.status == ToolStatus::Running {
            return Err(anyhow!("Tool is already running: {:?}", id));
        }

        let tool_name = state.tool.name().to_string();

        // Emit starting event
        state.status = ToolStatus::Starting;
        let _ = self.event_bus.send(LifecycleEvent::ToolStarting {
            id,
            name: tool_name.clone(),
        });

        // Update state
        state.status = ToolStatus::Running;
        state.started_at = Some(Utc::now());

        // Emit started event
        let _ = self.event_bus.send(LifecycleEvent::ToolStarted {
            id,
            name: tool_name,
        });

        tracing::info!("Started tool: {:?}", id);
        Ok(())
    }

    /// Stop a tool
    pub async fn stop_tool(&mut self, id: ToolId) -> Result<()> {
        let state = self.tools.get_mut(&id).ok_or_else(|| anyhow!("Tool not found: {:?}", id))?;

        if state.status == ToolStatus::Stopped {
            return Ok(());
        }

        let tool_name = state.tool.name().to_string();

        // Emit stopping event
        state.status = ToolStatus::Stopping;
        let _ = self.event_bus.send(LifecycleEvent::ToolStopping {
            id,
            name: tool_name.clone(),
        });

        // Cancel background task if running
        if let Some(handle) = state.handle.take() {
            handle.abort();
        }

        // Update state
        state.status = ToolStatus::Stopped;
        state.stopped_at = Some(Utc::now());

        // Emit stopped event
        let _ = self.event_bus.send(LifecycleEvent::ToolStopped {
            id,
            name: tool_name,
        });

        tracing::info!("Stopped tool: {:?}", id);
        Ok(())
    }

    /// Get tool status
    pub fn get_status(&self, id: ToolId) -> Option<ToolStatus> {
        self.tools.get(&id).map(|state| state.status.clone())
    }

    /// Get all tool IDs
    pub fn list_tool_ids(&self) -> Vec<ToolId> {
        self.tools.keys().copied().collect()
    }

    /// Stop all running tools
    pub fn stop_all(&mut self) -> Result<()> {
        let tool_ids: Vec<ToolId> = self.tools.keys().copied().collect();

        for id in tool_ids {
            if let Some(state) = self.tools.get(&id) {
                if state.status == ToolStatus::Running {
                    // Use blocking since we can't be async in Drop
                    let rt = tokio::runtime::Handle::try_current();
                    if let Ok(handle) = rt {
                        handle.block_on(async {
                            let _ = self.stop_tool(id).await;
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Subscribe to lifecycle events
    pub fn subscribe(&self) -> broadcast::Receiver<LifecycleEvent> {
        self.event_bus.subscribe()
    }

    /// Get count of running tools
    pub fn running_count(&self) -> usize {
        self.tools.values().filter(|state| state.status == ToolStatus::Running).count()
    }

    /// Get count of total registered tools
    pub fn total_count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::{ExecutionContext, ToolOutput};

    struct TestTool {
        name: String,
    }

    impl DxTool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn priority(&self) -> u32 {
            50
        }

        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }
    }

    #[tokio::test]
    async fn test_register_tool() {
        let mut manager = LifecycleManager::new();
        let tool = Box::new(TestTool {
            name: "test-tool".to_string(),
        });

        let id = manager.register_tool(tool).unwrap();
        assert_eq!(manager.total_count(), 1);
        assert_eq!(manager.get_status(id), Some(ToolStatus::Stopped));
    }

    #[tokio::test]
    async fn test_start_stop_tool() {
        let mut manager = LifecycleManager::new();
        let tool = Box::new(TestTool {
            name: "test-tool".to_string(),
        });

        let id = manager.register_tool(tool).unwrap();

        manager.start_tool(id).await.unwrap();
        assert_eq!(manager.get_status(id), Some(ToolStatus::Running));
        assert_eq!(manager.running_count(), 1);

        manager.stop_tool(id).await.unwrap();
        assert_eq!(manager.get_status(id), Some(ToolStatus::Stopped));
        assert_eq!(manager.running_count(), 0);
    }

    #[tokio::test]
    async fn test_lifecycle_events() {
        let mut manager = LifecycleManager::new();
        let mut rx = manager.subscribe();

        let tool = Box::new(TestTool {
            name: "test-tool".to_string(),
        });

        let id = manager.register_tool(tool).unwrap();

        // Start tool and check events
        manager.start_tool(id).await.unwrap();

        if let Ok(event) = rx.try_recv() {
            match event {
                LifecycleEvent::ToolStarting { id: _, name } => {
                    assert_eq!(name, "test-tool");
                }
                _ => panic!("Expected ToolStarting event"),
            }
        }
    }
}
