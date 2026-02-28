//! DX Plugin System
//!
//! Provides extensibility through WASM and native plugins with capability-based security.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       PLUGIN REGISTRY                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
//! │  │ plugins.sr   │──│   Loader     │──│  DashMap     │          │
//! │  │ (Config)     │  │   (Parser)   │  │  (Registry)  │          │
//! │  └──────────────┘  └──────────────┘  └──────────────┘          │
//! │                            │                                    │
//! │  ┌─────────────────────────┴─────────────────────────┐         │
//! │  │                    DxPlugin Trait                  │         │
//! │  │  ┌──────────────┐  ┌──────────────┐               │         │
//! │  │  │ WASM Plugin  │  │ Native Plugin│               │         │
//! │  │  │ (wasmtime)   │  │ (libloading) │               │         │
//! │  │  └──────────────┘  └──────────────┘               │         │
//! │  └───────────────────────────────────────────────────┘         │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::plugin::{PluginRegistry, PluginMetadata, Capability};
//!
//! // Load plugins from directory
//! let mut registry = PluginRegistry::new()?;
//! registry.load_from_dir("~/.dx/plugins")?;
//!
//! // Execute a plugin
//! let result = registry.execute("my-plugin", &["arg1", "arg2"]).await?;
//! ```

pub mod native;
pub mod registry;
pub mod sandbox;
pub mod traits;
pub mod wasm;

// --- Sprint 1.5 additions ---
pub mod hooks;
pub mod host_functions;
pub mod manager;
pub mod manifest;
pub mod resource_limiter;
pub mod signature;
pub mod validation;

pub use host_functions::HostState;
pub use manager::{PluginManager, PluginManagerConfig};
pub use registry::PluginRegistry;
pub use resource_limiter::{ResourceLimits, ResourceTracker};
pub use signature::{SignatureVerifier, VerificationResult};
pub use traits::{Capability, PluginMetadata};

use std::path::Path;

use anyhow::Result;

/// Plugin type (WASM or Native)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginType {
    /// WebAssembly plugin (sandboxed, secure)
    Wasm,
    /// Native dynamic library plugin (requires signature)
    Native,
}

impl PluginType {
    /// Detect plugin type from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "wasm" => Some(Self::Wasm),
            "so" | "dll" | "dylib" => Some(Self::Native),
            _ => None,
        }
    }
}

/// Plugin loading error
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Invalid plugin format: {0}")]
    InvalidFormat(String),

    #[error("Plugin signature verification failed: {0}")]
    SignatureError(String),

    #[error("Plugin capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Plugin execution error: {0}")]
    ExecutionError(String),

    #[error("Plugin sandbox error: {0}")]
    SandboxError(String),

    #[error("Plugin timeout")]
    Timeout,

    #[error("Plugin IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Plugin serialization error: {0}")]
    SerializationError(String),
}

/// Initialize the plugin system
pub async fn init() -> Result<PluginRegistry> {
    let registry = PluginRegistry::new()?;
    Ok(registry)
}

/// Discover plugins in the default directory
pub async fn discover_plugins() -> Result<Vec<PluginMetadata>> {
    let plugin_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".dx")
        .join("plugins");

    if !plugin_dir.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    let mut entries = tokio::fs::read_dir(&plugin_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(plugin_type) = PluginType::from_path(&path) {
            if let Ok(metadata) = load_metadata(&path, plugin_type).await {
                plugins.push(metadata);
            }
        }
    }

    Ok(plugins)
}

/// Load plugin metadata from a file
async fn load_metadata(path: &Path, plugin_type: PluginType) -> Result<PluginMetadata> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid plugin filename"))?
        .to_string();

    // Try to load metadata from .sr file
    let metadata_path = path.with_extension("sr");
    if metadata_path.exists() {
        let content = tokio::fs::read_to_string(&metadata_path).await?;
        // Parse the .sr file for metadata
        return parse_metadata(&name, &content, plugin_type, path);
    }

    // Default metadata if no .sr file
    Ok(PluginMetadata {
        name,
        version: "1.0.0".to_string(),
        description: String::new(),
        author: String::new(),
        capabilities: vec![],
        plugin_type,
        path: path.to_path_buf(),
        signature: None,
    })
}

/// Parse metadata from .sr content
fn parse_metadata(
    name: &str,
    content: &str,
    plugin_type: PluginType,
    path: &Path,
) -> Result<PluginMetadata> {
    let mut metadata = PluginMetadata {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        author: String::new(),
        capabilities: vec![],
        plugin_type,
        path: path.to_path_buf(),
        signature: None,
    };

    for line in content.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "version" => metadata.version = value.to_string(),
                "description" => metadata.description = value.to_string(),
                "author" => metadata.author = value.to_string(),
                "capabilities" => {
                    metadata.capabilities =
                        value.split(',').map(|s| Capability::from_str(s.trim())).collect();
                }
                _ => {}
            }
        }
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_from_path() {
        assert_eq!(PluginType::from_path(Path::new("test.wasm")), Some(PluginType::Wasm));
        assert_eq!(PluginType::from_path(Path::new("test.so")), Some(PluginType::Native));
        assert_eq!(PluginType::from_path(Path::new("test.dll")), Some(PluginType::Native));
        assert_eq!(PluginType::from_path(Path::new("test.txt")), None);
    }
}
