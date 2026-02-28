//! Docker-based sandbox backend

use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{Config, RemoveContainerOptions, StartContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use futures::StreamExt;
use std::path::Path;
use std::time::Instant;

use super::backend::{SandboxBackend, SandboxBackendType, SandboxResult};
use super::config::SandboxConfig;

/// Docker-based sandbox implementation
pub struct DockerSandbox {
    docker: Docker,
    container_id: Option<String>,
    config: SandboxConfig,
}

impl DockerSandbox {
    /// Create a new Docker sandbox
    pub fn new() -> Result<Self> {
        let docker =
            Docker::connect_with_local_defaults().context("Failed to connect to Docker daemon")?;

        Ok(Self {
            docker,
            container_id: None,
            config: SandboxConfig::default(),
        })
    }

    /// Get the container ID
    fn container_id(&self) -> Result<&str> {
        self.container_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Container not created"))
    }
}

#[async_trait]
impl SandboxBackend for DockerSandbox {
    async fn create(&mut self, config: &SandboxConfig) -> Result<()> {
        self.config = config.clone();

        // Build mounts
        let mounts: Vec<Mount> = config
            .mounts
            .iter()
            .map(|(host, container)| Mount {
                target: Some(container.to_string_lossy().to_string()),
                source: Some(host.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            })
            .collect();

        // Build environment variables
        let env: Vec<String> = config.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        // Configure resource limits
        let mut host_config = HostConfig {
            mounts: Some(mounts),
            network_mode: Some(config.network.to_string()),
            ..Default::default()
        };

        if let Some(memory) = config.limits.memory_bytes {
            host_config.memory = Some(memory as i64);
        }

        if let Some(cpu_shares) = config.limits.cpu_shares {
            host_config.cpu_shares = Some(cpu_shares as i64);
        }

        if let Some(max_pids) = config.limits.max_pids {
            host_config.pids_limit = Some(max_pids as i64);
        }

        // Create container
        let container_config = Config {
            image: Some("alpine:latest".to_string()),
            working_dir: Some(config.workdir.to_string_lossy().to_string()),
            env: Some(env),
            host_config: Some(host_config),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container::<String, String>(None, container_config)
            .await
            .context("Failed to create Docker container")?;

        self.container_id = Some(container.id.clone());

        // Start container
        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start Docker container")?;

        Ok(())
    }

    async fn execute(&self, command: &[String]) -> Result<SandboxResult> {
        let container_id = self.container_id()?;
        let start_time = Instant::now();

        // Create exec instance
        let exec = self
            .docker
            .create_exec(
                container_id,
                CreateExecOptions {
                    cmd: Some(command.to_vec()),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await
            .context("Failed to create exec instance")?;

        // Start exec and collect output
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            self.docker.start_exec(&exec.id, None).await?
        {
            while let Some(msg) = output.next().await {
                match msg? {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(1) as i32;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SandboxResult {
            exit_code,
            stdout,
            stderr,
            duration_ms,
        })
    }

    async fn copy_in(&self, host_path: &Path, sandbox_path: &Path) -> Result<()> {
        let container_id = self.container_id()?;

        // Read file content
        let content = tokio::fs::read(host_path).await.context("Failed to read host file")?;

        // Create tar archive
        let mut ar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();

        let file_name = sandbox_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid sandbox path"))?;

        ar.append_data(&mut header, file_name, content.as_slice())?;
        let tar_data = ar.into_inner()?;

        // Upload to container
        self.docker
            .upload_to_container(
                container_id,
                Some(bollard::container::UploadToContainerOptions {
                    path: sandbox_path
                        .parent()
                        .unwrap_or(Path::new("/"))
                        .to_string_lossy()
                        .to_string(),
                    ..Default::default()
                }),
                tar_data.into(),
            )
            .await
            .context("Failed to upload file to container")?;

        Ok(())
    }

    async fn copy_out(&self, sandbox_path: &Path, host_path: &Path) -> Result<()> {
        let container_id = self.container_id()?;

        // Download from container
        let mut stream = self.docker.download_from_container(
            container_id,
            Some(bollard::container::DownloadFromContainerOptions {
                path: sandbox_path.to_string_lossy().to_string(),
            }),
        );

        let mut tar_data = Vec::new();
        while let Some(chunk) = stream.next().await {
            tar_data.extend_from_slice(&chunk?);
        }

        // Extract tar archive in blocking task to avoid Send issues
        let host_path = host_path.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut ar = tar::Archive::new(tar_data.as_slice());
            for entry in ar.entries()? {
                let mut entry = entry?;
                let mut content = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut content)?;

                std::fs::write(&host_path, content).context("Failed to write host file")?;
                break; // Only extract first file
            }
            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    async fn destroy(&mut self) -> Result<()> {
        if let Some(container_id) = &self.container_id {
            self.docker
                .remove_container(
                    container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
                .context("Failed to remove Docker container")?;

            self.container_id = None;
        }

        Ok(())
    }

    fn is_available() -> bool {
        which::which("docker").is_ok()
    }

    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Docker
    }
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        if self.config.auto_cleanup && self.container_id.is_some() {
            // Best effort cleanup
            let _ = futures::executor::block_on(self.destroy());
        }
    }
}
