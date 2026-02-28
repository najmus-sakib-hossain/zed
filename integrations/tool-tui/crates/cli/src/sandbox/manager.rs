//! Sandbox manager for creating and managing sandboxes

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::Sandbox;
use super::backend::{SandboxBackend, SandboxBackendType, detect_backend};
use super::config::SandboxConfig;
use super::docker::DockerSandbox;
use super::native::NativeSandbox;
use super::wasm::WasmSandbox;

/// Sandbox status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxStatus {
    Created,
    Running,
    Stopped,
    Failed,
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxStatus::Created => write!(f, "created"),
            SandboxStatus::Running => write!(f, "running"),
            SandboxStatus::Stopped => write!(f, "stopped"),
            SandboxStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Sandbox manager for lifecycle management
pub struct SandboxManager {
    sandboxes: Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn SandboxBackend>>>>>>,
    metadata: Arc<RwLock<HashMap<String, Sandbox>>>,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new sandbox
    pub async fn create(
        &self,
        name: String,
        backend_type: SandboxBackendType,
        config: SandboxConfig,
    ) -> Result<String> {
        let backend_type = if backend_type == SandboxBackendType::Auto {
            detect_backend()
        } else {
            backend_type
        };

        let sandbox = Sandbox::new(name.clone(), backend_type, config.clone())?;
        let id = sandbox.id.clone();

        // Create backend instance
        let mut backend: Box<dyn SandboxBackend> = match backend_type {
            SandboxBackendType::Docker => {
                Box::new(DockerSandbox::new().context("Failed to create Docker sandbox")?)
            }
            SandboxBackendType::Podman => {
                // Podman uses same API as Docker
                Box::new(DockerSandbox::new().context("Failed to create Podman sandbox")?)
            }
            SandboxBackendType::Native => {
                let root_dir = sandbox.root_dir.clone();
                Box::new(NativeSandbox::new(root_dir).context("Failed to create native sandbox")?)
            }
            SandboxBackendType::Wasm => {
                Box::new(WasmSandbox::new().context("Failed to create WASM sandbox")?)
            }
            SandboxBackendType::Auto => unreachable!(),
        };

        // Initialize backend
        backend.create(&config).await?;

        // Store sandbox
        self.sandboxes.write().await.insert(id.clone(), Arc::new(RwLock::new(backend)));
        self.metadata.write().await.insert(id.clone(), sandbox);

        Ok(id)
    }

    /// Get sandbox by ID
    pub async fn get(&self, id: &str) -> Result<Sandbox> {
        self.metadata
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", id))
    }

    /// Get sandbox by name
    pub async fn get_by_name(&self, name: &str) -> Result<Sandbox> {
        self.metadata
            .read()
            .await
            .values()
            .find(|s| s.name == name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", name))
    }

    /// List all sandboxes
    pub async fn list(&self) -> Vec<Sandbox> {
        self.metadata.read().await.values().cloned().collect()
    }

    /// Execute command in sandbox
    pub async fn execute(
        &self,
        id: &str,
        command: &[String],
    ) -> Result<super::backend::SandboxResult> {
        let backend = self
            .sandboxes
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", id))?;

        backend.read().await.execute(command).await
    }

    /// Copy file into sandbox
    pub async fn copy_in(
        &self,
        id: &str,
        host_path: &PathBuf,
        sandbox_path: &PathBuf,
    ) -> Result<()> {
        let backend = self
            .sandboxes
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", id))?;

        backend.read().await.copy_in(host_path, sandbox_path).await
    }

    /// Copy file out of sandbox
    pub async fn copy_out(
        &self,
        id: &str,
        sandbox_path: &PathBuf,
        host_path: &PathBuf,
    ) -> Result<()> {
        let backend = self
            .sandboxes
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", id))?;

        backend.read().await.copy_out(sandbox_path, host_path).await
    }

    /// Destroy sandbox
    pub async fn destroy(&self, id: &str) -> Result<()> {
        let backend = self
            .sandboxes
            .write()
            .await
            .remove(id)
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", id))?;

        backend.write().await.destroy().await?;
        self.metadata.write().await.remove(id);

        Ok(())
    }

    /// Destroy all sandboxes
    pub async fn destroy_all(&self) -> Result<()> {
        let ids: Vec<String> = self.metadata.read().await.keys().cloned().collect();

        for id in ids {
            let _ = self.destroy(&id).await; // Best effort
        }

        Ok(())
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}
