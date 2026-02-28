//! Tool registry for managing installed DX tools

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::{Version, VersionReq};

/// Tool information for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Version,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub source: ToolSource,
    pub dependencies: HashMap<String, VersionReq>,
}

/// Source of a tool installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    /// Local development
    Local(PathBuf),
    /// Published crate
    Crate { version: String },
    /// Git repository
    Git { url: String, rev: String },
    /// R2 storage
    R2 { bucket: String, key: String },
}

/// DX Tool Version Registry
///
/// Manages installed tool versions, dependencies, and compatibility
pub struct ToolRegistry {
    registry_path: PathBuf,
    tools: HashMap<String, ToolInfo>,
}

impl ToolRegistry {
    /// Create or load a tool registry
    pub fn new(forge_dir: &Path) -> Result<Self> {
        let registry_path = forge_dir.join("tool_registry.json");

        let tools = if registry_path.exists() {
            let content =
                std::fs::read_to_string(&registry_path).context("Failed to read tool registry")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            registry_path,
            tools,
        })
    }

    /// Register a new tool
    pub fn register(
        &mut self,
        name: String,
        version: Version,
        source: ToolSource,
        dependencies: HashMap<String, VersionReq>,
    ) -> Result<()> {
        let info = ToolInfo {
            name: name.clone(),
            version,
            installed_at: chrono::Utc::now(),
            source,
            dependencies,
        };

        self.tools.insert(name, info);
        self.save()?;

        Ok(())
    }

    /// Get tool information
    pub fn get(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.get(name)
    }

    /// Check if a tool is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get tool version
    pub fn version(&self, name: &str) -> Option<&Version> {
        self.tools.get(name).map(|info| &info.version)
    }

    /// Check if all dependencies are satisfied
    pub fn check_dependencies(&self, tool_name: &str) -> Result<Vec<String>> {
        let mut missing = Vec::new();

        if let Some(info) = self.tools.get(tool_name) {
            for (dep_name, req) in &info.dependencies {
                match self.tools.get(dep_name) {
                    Some(dep_info) => {
                        if !dep_info.version.satisfies(req) {
                            missing.push(format!(
                                "{} requires {} {}, but {} is installed",
                                tool_name, dep_name, req, dep_info.version
                            ));
                        }
                    }
                    None => {
                        missing.push(format!("{} requires {} {}", tool_name, dep_name, req));
                    }
                }
            }
        }

        Ok(missing)
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&ToolInfo> {
        self.tools.values().collect()
    }

    /// Unregister a tool
    pub fn unregister(&mut self, name: &str) -> Result<()> {
        self.tools.remove(name);
        self.save()?;
        Ok(())
    }

    /// Check if an update is available
    pub fn needs_update(&self, name: &str, latest: &Version) -> bool {
        if let Some(info) = self.tools.get(name) {
            &info.version < latest
        } else {
            false
        }
    }

    /// Save registry to disk
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.tools)?;
        std::fs::write(&self.registry_path, content)?;
        Ok(())
    }
}
