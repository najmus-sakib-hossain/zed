//! # Deployment Module
//!
//! Handles deployment optimization, Docker builds, and cloud platform deployment.

pub mod cloud;
pub mod commands;
pub mod docker;
pub mod oneclick;
pub mod optimize;
pub mod vps;

use std::path::PathBuf;
use thiserror::Error;

/// Deployment errors
#[derive(Debug, Error)]
pub enum DeployError {
    #[error("Build failed: {0}")]
    BuildFailed(String),
    #[error("Docker error: {0}")]
    DockerError(String),
    #[error("Cloud error: {0}")]
    CloudError(String),
    #[error("SSH error: {0}")]
    SshError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Deployment configuration
#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// Project root
    pub project_root: PathBuf,
    /// Target platform
    pub platform: DeployPlatform,
    /// Build mode
    pub build_mode: BuildMode,
    /// Binary name
    pub binary_name: String,
    /// Target binary size (bytes)
    pub target_size: u64,
    /// Target memory usage (bytes)
    pub target_memory: u64,
}

/// Deployment platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployPlatform {
    /// Local Docker
    Docker,
    /// Railway
    Railway,
    /// Fly.io
    Fly,
    /// Render
    Render,
    /// DigitalOcean
    DigitalOcean,
    /// AWS ECS
    AwsEcs,
    /// GCP Cloud Run
    GcpCloudRun,
    /// VPS via SSH
    Vps,
}

/// Build mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    /// Debug build
    Debug,
    /// Release build
    Release,
    /// Optimized release (LTO, stripping)
    Optimized,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            platform: DeployPlatform::Docker,
            build_mode: BuildMode::Optimized,
            binary_name: "dx".to_string(),
            target_size: 20 * 1024 * 1024,    // 20MB
            target_memory: 128 * 1024 * 1024, // 128MB
        }
    }
}

/// Deployment manager
pub struct DeployManager {
    /// Configuration
    config: DeployConfig,
}

impl DeployManager {
    /// Create a new deployment manager
    pub fn new(config: DeployConfig) -> Self {
        Self { config }
    }

    /// Initialize deployment configuration files
    pub async fn init(&self) -> Result<InitResult, DeployError> {
        let mut files_created = Vec::new();

        // Create Dockerfile
        let dockerfile = self.generate_dockerfile();
        let dockerfile_path = self.config.project_root.join("Dockerfile");
        std::fs::write(&dockerfile_path, &dockerfile)?;
        files_created.push(dockerfile_path);

        // Create docker-compose.yml
        let compose = self.generate_docker_compose();
        let compose_path = self.config.project_root.join("docker-compose.yml");
        std::fs::write(&compose_path, &compose)?;
        files_created.push(compose_path);

        // Create platform-specific files
        match self.config.platform {
            DeployPlatform::Railway => {
                let railway = self.generate_railway_config();
                let path = self.config.project_root.join("railway.json");
                std::fs::write(&path, &railway)?;
                files_created.push(path);
            }
            DeployPlatform::Fly => {
                let fly = self.generate_fly_config();
                let path = self.config.project_root.join("fly.toml");
                std::fs::write(&path, &fly)?;
                files_created.push(path);
            }
            DeployPlatform::Render => {
                let render = self.generate_render_config();
                let path = self.config.project_root.join("render.yaml");
                std::fs::write(&path, &render)?;
                files_created.push(path);
            }
            _ => {}
        }

        Ok(InitResult {
            files_created,
            platform: self.config.platform,
        })
    }

    /// Generate optimized Dockerfile
    fn generate_dockerfile(&self) -> String {
        format!(
            r#"# DX Optimized Dockerfile
# Multi-stage build for minimal image size

# Build stage
FROM rust:1.75-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin {binary} \
    && strip target/release/{binary}

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false dx

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/{binary} /usr/local/bin/

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["{binary}", "health"] || exit 1

# Switch to non-root user
USER dx

# Default port
EXPOSE 8080

# Run
CMD ["{binary}", "serve"]
"#,
            binary = self.config.binary_name
        )
    }

    /// Generate docker-compose.yml
    fn generate_docker_compose(&self) -> String {
        format!(
            r#"version: '3.8'

services:
  {binary}:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - DX_ENV=production
    volumes:
      - dx-data:/app/data
    healthcheck:
      test: ["CMD", "{binary}", "health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 128M
        reservations:
          memory: 64M

volumes:
  dx-data:
"#,
            binary = self.config.binary_name
        )
    }

    /// Generate Railway configuration
    fn generate_railway_config(&self) -> String {
        serde_json::json!({
            "$schema": "https://railway.app/railway.schema.json",
            "build": {
                "builder": "DOCKERFILE",
                "dockerfilePath": "Dockerfile"
            },
            "deploy": {
                "startCommand": format!("{} serve", self.config.binary_name),
                "healthcheckPath": "/health",
                "healthcheckTimeout": 30,
                "restartPolicyType": "ON_FAILURE",
                "restartPolicyMaxRetries": 10
            }
        })
        .to_string()
    }

    /// Generate Fly.io configuration
    fn generate_fly_config(&self) -> String {
        format!(
            r#"app = "{binary}"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[http_service.concurrency]
  type = "requests"
  hard_limit = 250
  soft_limit = 200

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 256

[checks]
  [checks.health]
    port = 8080
    type = "http"
    interval = "30s"
    timeout = "5s"
    path = "/health"
"#,
            binary = self.config.binary_name
        )
    }

    /// Generate Render configuration
    fn generate_render_config(&self) -> String {
        format!(
            r#"services:
  - type: web
    name: {binary}
    env: docker
    dockerfilePath: ./Dockerfile
    healthCheckPath: /health
    envVars:
      - key: RUST_LOG
        value: info
      - key: DX_ENV
        value: production
    scaling:
      minInstances: 0
      maxInstances: 3
      targetMemoryPercent: 80
      targetCPUPercent: 80
"#,
            binary = self.config.binary_name
        )
    }

    /// Deploy to the configured platform
    pub async fn deploy(&self) -> Result<DeployResult, DeployError> {
        match self.config.platform {
            DeployPlatform::Docker => self.deploy_docker().await,
            DeployPlatform::Railway => self.deploy_railway().await,
            DeployPlatform::Fly => self.deploy_fly().await,
            DeployPlatform::Vps => {
                Err(DeployError::ConfigError("VPS requires host parameter".to_string()))
            }
            _ => Err(DeployError::ConfigError("Platform not yet supported".to_string())),
        }
    }

    /// Deploy to local Docker
    async fn deploy_docker(&self) -> Result<DeployResult, DeployError> {
        // Build image
        let status = std::process::Command::new("docker")
            .args(["build", "-t", &self.config.binary_name, "."])
            .current_dir(&self.config.project_root)
            .status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Docker build failed".to_string()));
        }

        Ok(DeployResult {
            platform: DeployPlatform::Docker,
            url: None,
            status: DeployStatus::Success,
            message: format!("Image {} built successfully", self.config.binary_name),
        })
    }

    /// Deploy to Railway
    async fn deploy_railway(&self) -> Result<DeployResult, DeployError> {
        let status = std::process::Command::new("railway")
            .args(["up", "--detach"])
            .current_dir(&self.config.project_root)
            .status()?;

        if !status.success() {
            return Err(DeployError::CloudError("Railway deployment failed".to_string()));
        }

        Ok(DeployResult {
            platform: DeployPlatform::Railway,
            url: Some("https://your-app.railway.app".to_string()),
            status: DeployStatus::Success,
            message: "Deployed to Railway".to_string(),
        })
    }

    /// Deploy to Fly.io
    async fn deploy_fly(&self) -> Result<DeployResult, DeployError> {
        let status = std::process::Command::new("fly")
            .args(["deploy"])
            .current_dir(&self.config.project_root)
            .status()?;

        if !status.success() {
            return Err(DeployError::CloudError("Fly deployment failed".to_string()));
        }

        Ok(DeployResult {
            platform: DeployPlatform::Fly,
            url: Some(format!("https://{}.fly.dev", self.config.binary_name)),
            status: DeployStatus::Success,
            message: "Deployed to Fly.io".to_string(),
        })
    }

    /// Deploy to VPS via SSH
    pub async fn deploy_vps(&self, host: &str, user: &str) -> Result<DeployResult, DeployError> {
        vps::deploy_to_vps(&self.config, host, user).await
    }
}

/// Initialization result
#[derive(Debug)]
pub struct InitResult {
    /// Files created
    pub files_created: Vec<PathBuf>,
    /// Target platform
    pub platform: DeployPlatform,
}

/// Deployment result
#[derive(Debug)]
pub struct DeployResult {
    /// Platform deployed to
    pub platform: DeployPlatform,
    /// Deployment URL
    pub url: Option<String>,
    /// Status
    pub status: DeployStatus,
    /// Status message
    pub message: String,
}

/// Deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployStatus {
    /// Deployment succeeded
    Success,
    /// Deployment in progress
    InProgress,
    /// Deployment failed
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeployConfig::default();

        assert_eq!(config.platform, DeployPlatform::Docker);
        assert_eq!(config.build_mode, BuildMode::Optimized);
        assert_eq!(config.target_size, 20 * 1024 * 1024);
    }

    #[test]
    fn test_generate_dockerfile() {
        let config = DeployConfig {
            binary_name: "test-app".to_string(),
            ..Default::default()
        };
        let manager = DeployManager::new(config);

        let dockerfile = manager.generate_dockerfile();

        assert!(dockerfile.contains("test-app"));
        assert!(dockerfile.contains("FROM rust:"));
        assert!(dockerfile.contains("HEALTHCHECK"));
        assert!(dockerfile.contains("USER dx"));
    }

    #[test]
    fn test_generate_docker_compose() {
        let config = DeployConfig {
            binary_name: "test-app".to_string(),
            ..Default::default()
        };
        let manager = DeployManager::new(config);

        let compose = manager.generate_docker_compose();

        assert!(compose.contains("test-app"));
        assert!(compose.contains("8080:8080"));
        assert!(compose.contains("memory: 128M"));
    }

    #[test]
    fn test_generate_fly_config() {
        let config = DeployConfig {
            binary_name: "test-app".to_string(),
            ..Default::default()
        };
        let manager = DeployManager::new(config);

        let fly = manager.generate_fly_config();

        assert!(fly.contains("test-app"));
        assert!(fly.contains("internal_port = 8080"));
        assert!(fly.contains("/health"));
    }
}
