//! Tool Registry
//!
//! Manages registration and tracking of all DX tools.

use crate::dx_cache::DxToolId;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    Ready,
    Running,
    Disabled,
    Error,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolStatus::Ready => write!(f, "Ready"),
            ToolStatus::Running => write!(f, "Running"),
            ToolStatus::Disabled => write!(f, "Disabled"),
            ToolStatus::Error => write!(f, "Error"),
        }
    }
}

/// Tool information for external queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: ToolStatus,
    pub is_dummy: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub error_count: u64,
}

/// Internal tool entry
struct ToolEntry {
    id: DxToolId,
    name: String,
    status: ToolStatus,
    is_dummy: bool,
    last_run: Option<DateTime<Utc>>,
    run_count: u64,
    error_count: u64,
}

/// Tool registry that manages all DX tools
pub struct ToolRegistry {
    tools: RwLock<HashMap<DxToolId, ToolEntry>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Create registry with dummy tools pre-registered
    pub fn with_dummy_tools() -> Self {
        let registry = Self::new();

        // Register dummy tool entries
        let dummy_tools = [
            (DxToolId::Bundler, "dx-bundler"),
            (DxToolId::Style, "dx-style"),
            (DxToolId::Test, "dx-test-runner"),
            (DxToolId::NodeModules, "dx-package-manager"),
            (DxToolId::Serializer, "dx-serializer"),
            (DxToolId::Www, "dx-www"),
        ];

        for (id, name) in dummy_tools {
            registry.register_entry(id, name, true);
        }

        registry
    }

    /// Register a tool entry
    fn register_entry(&self, id: DxToolId, name: &str, is_dummy: bool) {
        let entry = ToolEntry {
            id,
            name: name.to_string(),
            status: ToolStatus::Ready,
            is_dummy,
            last_run: None,
            run_count: 0,
            error_count: 0,
        };
        self.tools.write().insert(id, entry);
    }

    /// Register a real tool (replaces dummy if exists)
    pub fn register(&self, id: DxToolId, name: &str) {
        self.register_entry(id, name, false);
    }

    /// Get tool info for all registered tools
    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .read()
            .values()
            .map(|entry| ToolInfo {
                id: format!("{:?}", entry.id),
                name: entry.name.clone(),
                version: "0.1.0".to_string(),
                status: entry.status,
                is_dummy: entry.is_dummy,
                last_run: entry.last_run,
                run_count: entry.run_count,
                error_count: entry.error_count,
            })
            .collect()
    }

    /// Get a specific tool's info
    pub fn get_info(&self, id: DxToolId) -> Option<ToolInfo> {
        self.tools.read().get(&id).map(|entry| ToolInfo {
            id: format!("{:?}", id),
            name: entry.name.clone(),
            version: "0.1.0".to_string(),
            status: entry.status,
            is_dummy: entry.is_dummy,
            last_run: entry.last_run,
            run_count: entry.run_count,
            error_count: entry.error_count,
        })
    }

    /// Set tool status
    pub fn set_status(&self, id: DxToolId, status: ToolStatus) {
        if let Some(entry) = self.tools.write().get_mut(&id) {
            entry.status = status;
        }
    }

    /// Enable a tool
    pub fn enable(&self, id: DxToolId) -> bool {
        if let Some(entry) = self.tools.write().get_mut(&id) {
            entry.status = ToolStatus::Ready;
            true
        } else {
            false
        }
    }

    /// Disable a tool
    pub fn disable(&self, id: DxToolId) -> bool {
        if let Some(entry) = self.tools.write().get_mut(&id) {
            entry.status = ToolStatus::Disabled;
            true
        } else {
            false
        }
    }

    /// Record a tool execution
    pub fn record_execution(&self, id: DxToolId, success: bool) {
        if let Some(entry) = self.tools.write().get_mut(&id) {
            entry.last_run = Some(Utc::now());
            entry.run_count += 1;
            if !success {
                entry.error_count += 1;
            }
        }
    }

    /// Check if a tool exists
    pub fn exists(&self, id: DxToolId) -> bool {
        self.tools.read().contains_key(&id)
    }

    /// Get tool count
    pub fn count(&self) -> usize {
        self.tools.read().len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
