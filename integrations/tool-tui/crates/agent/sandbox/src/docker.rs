//! Docker sandbox implementation using bollard

use anyhow::Result;
use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, LogOutput, RemoveContainerOptions,
    StartContainerOptions,
};
use futures_util::StreamExt;
use std::time::Instant;
use tracing::{debug, info, warn};

use super::ExecutionResult;
use super::config::SandboxConfig;

/// Docker-based sandbox
pub struct DockerSandbox {
    docker: Docker,
    container_id: Option<String>,
    config: SandboxConfig,
}

impl DockerSandbox {
    /// Create a new Docker sandbox
    pub async fn new(config: SandboxConfig) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        // Verify Docker is running
        docker
            .ping()
            .await
            .map_err(|e| anyhow::anyhow!("Docker not available: {}. Is Docker running?", e))?;

        Ok(Self {
            docker,
            container_id: None,
            config,
        })
    }

    /// Create the container
    pub async fn create(&mut self) -> Result<String> {
        let mut binds = Vec::new();
        for (host, container) in &self.config.volumes {
            binds.push(format!("{}:{}:rw", host, container));
        }

        let memory = (self.config.limits.memory_mb * 1024 * 1024) as i64;
        let nano_cpus = (self.config.limits.cpu_cores * 1e9) as i64;

        let host_config = bollard::models::HostConfig {
            binds: Some(binds),
            memory: Some(memory),
            nano_cpus: Some(nano_cpus),
            pids_limit: Some(self.config.limits.max_pids as i64),
            network_mode: if self.config.network.enabled {
                None
            } else {
                Some("none".into())
            },
            ..Default::default()
        };

        let env: Vec<String> =
            self.config.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        let container_config = ContainerConfig {
            image: Some(self.config.image.clone()),
            working_dir: Some(self.config.workdir.clone()),
            env: Some(env),
            host_config: Some(host_config),
            tty: Some(true),
            cmd: Some(vec!["sleep".into(), "infinity".into()]),
            ..Default::default()
        };

        let name = format!("dx-sandbox-{}", uuid::Uuid::new_v4().as_simple());
        let create_opts = CreateContainerOptions {
            name: name.as_str(),
            platform: None,
        };

        let response = self.docker.create_container(Some(create_opts), container_config).await?;

        self.container_id = Some(response.id.clone());
        info!("Created sandbox container: {}", &response.id[..12]);

        Ok(response.id)
    }

    /// Start the container
    pub async fn start(&self) -> Result<()> {
        let id = self
            .container_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Container not created"))?;

        self.docker.start_container(id, None::<StartContainerOptions<String>>).await?;

        debug!("Started sandbox container: {}", &id[..12]);
        Ok(())
    }

    /// Execute a command inside the sandbox
    pub async fn execute(&self, command: &[&str]) -> Result<ExecutionResult> {
        let id = self
            .container_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Container not created"))?;

        let start = Instant::now();

        // Create exec instance
        let exec_config = bollard::exec::CreateExecOptions {
            cmd: Some(command.iter().map(|s| s.to_string()).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some(self.config.workdir.clone()),
            ..Default::default()
        };

        let exec = self.docker.create_exec(id, exec_config).await?;

        // Start exec
        let output = self.docker.start_exec(&exec.id, None).await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let bollard::exec::StartExecResults::Attached {
            output: mut stream, ..
        } = output
        {
            while let Some(msg) = stream.next().await {
                match msg? {
                    LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(-1) as i32;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            duration_ms,
            killed: false,
        })
    }

    /// Destroy the container
    pub async fn destroy(&mut self) -> Result<()> {
        if let Some(id) = self.container_id.take() {
            let opts = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            self.docker.remove_container(&id, Some(opts)).await?;
            info!("Destroyed sandbox container: {}", &id[..12]);
        }
        Ok(())
    }

    /// Check if Docker is available
    pub fn is_available() -> bool {
        // Try to connect synchronously check
        std::process::Command::new("docker")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        if self.config.auto_cleanup && self.container_id.is_some() {
            warn!("Sandbox container not properly cleaned up. Call destroy() explicitly.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_available_check() {
        // Just verify the function runs
        let _ = DockerSandbox::is_available();
    }

    #[test]
    fn test_default_sandbox_config() {
        let config = SandboxConfig::default();
        assert_eq!(config.image, "ubuntu:24.04");
        assert_eq!(config.limits.memory_mb, 512);
    }
}
