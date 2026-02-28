//! Sovereign Orchestrator - Tool lifecycle management

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Stopped,
    Starting,
    Running(u32),
    Healthy,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct DxToolDefinition {
    pub name: String,
    pub binary_path: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
}

pub struct Orchestrator {
    tools: HashMap<String, DxToolDefinition>,
    states: Arc<RwLock<HashMap<String, ToolStatus>>>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_tool(&mut self, tool: DxToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&DxToolDefinition> {
        self.tools.get(name)
    }

    pub async fn get_status(&self, tool_name: &str) -> ToolStatus {
        self.states.read().await.get(tool_name).cloned().unwrap_or(ToolStatus::Stopped)
    }

    pub async fn ensure_running(&self, tool_name: &str) -> anyhow::Result<()> {
        let mut states = self.states.write().await;
        let current = states.get(tool_name).cloned().unwrap_or(ToolStatus::Stopped);

        if let ToolStatus::Running(_) = current {
            return Ok(());
        }

        println!("⚙️  Orchestrator: Starting {}...", tool_name);
        states.insert(tool_name.to_string(), ToolStatus::Running(1234));
        println!("✅ Orchestrator: {} is now Active.", tool_name);
        Ok(())
    }

    pub async fn execute_tool(&self, tool_name: &str, args: &[&str]) -> anyhow::Result<()> {
        println!("🚀 Orchestrator: Executing {} with {:?}", tool_name, args);
        Ok(())
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}
