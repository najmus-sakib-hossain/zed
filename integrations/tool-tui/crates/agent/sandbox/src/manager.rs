//! Sandbox manager - high-level interface for sandbox lifecycle

use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

use super::config::SandboxConfig;
use super::docker::DockerSandbox;
use super::native::NativeSandbox;
use super::{ExecutionResult, SandboxType};

/// High-level sandbox manager
pub struct Manager {
    sandboxes: HashMap<String, SandboxInstance>,
    default_config: SandboxConfig,
}

enum SandboxInstance {
    Docker(DockerSandbox),
    Native(NativeSandbox),
}

impl Manager {
    pub fn new(default_config: SandboxConfig) -> Self {
        Self {
            sandboxes: HashMap::new(),
            default_config,
        }
    }

    /// Create a new sandbox for a session
    pub async fn create_sandbox(
        &mut self,
        session_id: &str,
        config: Option<SandboxConfig>,
    ) -> Result<String> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let sandbox_type = super::detect_sandbox();
        let sandbox_id = format!("sandbox-{}", uuid::Uuid::new_v4().as_simple());

        let instance = match sandbox_type {
            SandboxType::Docker => {
                let mut docker = DockerSandbox::new(config).await?;
                docker.create().await?;
                docker.start().await?;
                SandboxInstance::Docker(docker)
            }
            SandboxType::Native | SandboxType::Auto => {
                SandboxInstance::Native(NativeSandbox::new(config))
            }
        };

        self.sandboxes.insert(sandbox_id.clone(), instance);
        info!(
            "Created {} sandbox: {} for session {}",
            match sandbox_type {
                SandboxType::Docker => "Docker",
                SandboxType::Native => "Native",
                SandboxType::Auto => "Auto",
            },
            sandbox_id,
            session_id
        );

        Ok(sandbox_id)
    }

    /// Execute command in a sandbox
    pub async fn execute(&self, sandbox_id: &str, command: &[&str]) -> Result<ExecutionResult> {
        let sandbox = self
            .sandboxes
            .get(sandbox_id)
            .ok_or_else(|| anyhow::anyhow!("Sandbox not found: {}", sandbox_id))?;

        match sandbox {
            SandboxInstance::Docker(docker) => docker.execute(command).await,
            SandboxInstance::Native(native) => native.execute(command).await,
        }
    }

    /// Destroy a sandbox
    pub async fn destroy_sandbox(&mut self, sandbox_id: &str) -> Result<()> {
        if let Some(mut instance) = self.sandboxes.remove(sandbox_id) {
            match &mut instance {
                SandboxInstance::Docker(docker) => docker.destroy().await?,
                SandboxInstance::Native(_) => {} // Nothing to clean up
            }
            info!("Destroyed sandbox: {}", sandbox_id);
        }
        Ok(())
    }

    /// Destroy all sandboxes
    pub async fn destroy_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.sandboxes.keys().cloned().collect();
        for id in ids {
            self.destroy_sandbox(&id).await?;
        }
        Ok(())
    }

    /// List active sandboxes
    pub fn list_active(&self) -> Vec<String> {
        self.sandboxes.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let mgr = Manager::new(SandboxConfig::default());
        assert!(mgr.list_active().is_empty());
    }
}
