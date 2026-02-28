//! VPS deployer — deploys the daemon to a remote VPS.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// VPS provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VpsProvider {
    Hetzner,
    DigitalOcean,
    Linode,
    Vultr,
    Fly,
    Custom,
}

/// VPS deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpsConfig {
    pub provider: VpsProvider,
    pub region: String,
    pub instance_type: String,
    pub ssh_key_path: Option<String>,
    pub api_token: Option<String>,
}

/// Deploy state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployState {
    NotDeployed,
    Deploying,
    Running,
    Failed,
    Stopped,
}

/// Manages deployment of the daemon to a VPS.
pub struct VpsDeployer {
    config: Option<VpsConfig>,
    state: DeployState,
    remote_ip: Option<String>,
}

impl VpsDeployer {
    pub fn new() -> Self {
        Self {
            config: None,
            state: DeployState::NotDeployed,
            remote_ip: None,
        }
    }

    pub fn state(&self) -> DeployState {
        self.state
    }

    pub fn remote_ip(&self) -> Option<&str> {
        self.remote_ip.as_deref()
    }

    /// Configure VPS deployment.
    pub fn configure(&mut self, config: VpsConfig) {
        self.config = Some(config);
    }

    /// Deploy the daemon to the configured VPS.
    pub async fn deploy(&mut self) -> Result<()> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("VPS not configured"))?;

        log::info!(
            "Deploying DX daemon to {:?} in {}",
            config.provider,
            config.region
        );

        self.state = DeployState::Deploying;

        // Placeholder — real implementation uses cloud APIs
        // 1. Provision instance
        // 2. Upload binary
        // 3. Configure systemd
        // 4. Start service

        self.state = DeployState::Running;
        self.remote_ip = Some("0.0.0.0".into()); // placeholder

        Ok(())
    }

    /// Stop and destroy the remote instance.
    pub async fn destroy(&mut self) -> Result<()> {
        log::info!("Destroying VPS deployment");
        self.state = DeployState::Stopped;
        self.remote_ip = None;
        Ok(())
    }
}

impl Default for VpsDeployer {
    fn default() -> Self {
        Self::new()
    }
}
