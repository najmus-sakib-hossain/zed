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

impl VpsProvider {
    /// API base URL for each provider.
    pub fn api_base(&self) -> &'static str {
        match self {
            Self::Hetzner => "https://api.hetzner.cloud/v1",
            Self::DigitalOcean => "https://api.digitalocean.com/v2",
            Self::Linode => "https://api.linode.com/v4",
            Self::Vultr => "https://api.vultr.com/v2",
            Self::Fly => "https://api.machines.dev/v1",
            Self::Custom => "",
        }
    }

    /// Recommended instance type for running the daemon (cheap, low-resource).
    pub fn recommended_instance(&self) -> &'static str {
        match self {
            Self::Hetzner => "cx22",       // 2 vCPU, 4 GB RAM, €3.29/mo
            Self::DigitalOcean => "s-1vcpu-1gb", // 1 vCPU, 1 GB RAM, $6/mo
            Self::Linode => "g6-nanode-1",  // 1 vCPU, 1 GB RAM, $5/mo
            Self::Vultr => "vc2-1c-1gb",   // 1 vCPU, 1 GB RAM, $5/mo
            Self::Fly => "shared-cpu-1x",  // shared, 256MB, ~$1.94/mo
            Self::Custom => "custom",
        }
    }
}

/// VPS deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpsConfig {
    pub provider: VpsProvider,
    pub region: String,
    pub instance_type: String,
    pub ssh_key_path: Option<String>,
    pub api_token: Option<String>,
    /// Custom setup script to run after provisioning.
    pub setup_script: Option<String>,
    /// Daemon binary path on local machine (to upload).
    pub local_binary_path: Option<String>,
}

/// Deploy state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployState {
    NotDeployed,
    Provisioning,
    Deploying,
    Running,
    Failed,
    Stopped,
}

/// Deployment record with all details about a running instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub instance_id: String,
    pub ip_address: String,
    pub provider: VpsProvider,
    pub region: String,
    pub instance_type: String,
    pub state: DeployState,
    pub created_at: u64,
}

/// Manages deployment of the daemon to a VPS.
pub struct VpsDeployer {
    config: Option<VpsConfig>,
    state: DeployState,
    deployment: Option<DeploymentInfo>,
}

impl VpsDeployer {
    pub fn new() -> Self {
        Self {
            config: None,
            state: DeployState::NotDeployed,
            deployment: None,
        }
    }

    pub fn state(&self) -> DeployState {
        self.state
    }

    pub fn remote_ip(&self) -> Option<&str> {
        self.deployment.as_ref().map(|d| d.ip_address.as_str())
    }

    pub fn deployment_info(&self) -> Option<&DeploymentInfo> {
        self.deployment.as_ref()
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

        let api_token = config
            .api_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No API token set for {:?}", config.provider))?;

        log::info!(
            "Deploying DX daemon to {:?} in {} (instance: {})",
            config.provider,
            config.region,
            config.instance_type,
        );

        self.state = DeployState::Provisioning;

        // Step 1: Provision instance via cloud API
        let instance = self
            .provision_instance(config, api_token)
            .await?;

        self.state = DeployState::Deploying;

        // Step 2: Wait for instance to boot and get SSH access
        self.wait_for_ssh(&instance.ip_address).await?;

        // Step 3: Upload daemon binary
        if let Some(ref binary_path) = config.local_binary_path {
            self.upload_binary(binary_path, &instance.ip_address, config.ssh_key_path.as_deref())
                .await?;
        }

        // Step 4: Configure and start the service remotely
        self.configure_remote_service(&instance.ip_address, config.ssh_key_path.as_deref())
            .await?;

        self.state = DeployState::Running;
        self.deployment = Some(instance);

        Ok(())
    }

    /// Provision a new instance via the cloud provider's API.
    async fn provision_instance(
        &self,
        config: &VpsConfig,
        api_token: &str,
    ) -> Result<DeploymentInfo> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match config.provider {
            VpsProvider::Hetzner => {
                // POST /v1/servers
                let body = serde_json::json!({
                    "name": "dx-daemon",
                    "server_type": config.instance_type,
                    "location": config.region,
                    "image": "ubuntu-24.04",
                    "start_after_create": true,
                });
                let url = format!("{}/servers", config.provider.api_base());
                let output = self.api_call(&url, api_token, &body).await?;
                let id = output
                    .get("server")
                    .and_then(|s| s.get("id"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "unknown".into());
                let ip = output
                    .get("server")
                    .and_then(|s| s.get("public_net"))
                    .and_then(|n| n.get("ipv4"))
                    .and_then(|v| v.get("ip"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0.0")
                    .to_string();

                Ok(DeploymentInfo {
                    instance_id: id,
                    ip_address: ip,
                    provider: config.provider,
                    region: config.region.clone(),
                    instance_type: config.instance_type.clone(),
                    state: DeployState::Running,
                    created_at: now,
                })
            }
            VpsProvider::DigitalOcean => {
                // POST /v2/droplets
                let body = serde_json::json!({
                    "name": "dx-daemon",
                    "region": config.region,
                    "size": config.instance_type,
                    "image": "ubuntu-24-04-x64",
                });
                let url = format!("{}/droplets", config.provider.api_base());
                let output = self.api_call(&url, api_token, &body).await?;
                let id = output
                    .get("droplet")
                    .and_then(|d| d.get("id"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "unknown".into());

                Ok(DeploymentInfo {
                    instance_id: id,
                    ip_address: "0.0.0.0".into(), // need to poll until available
                    provider: config.provider,
                    region: config.region.clone(),
                    instance_type: config.instance_type.clone(),
                    state: DeployState::Running,
                    created_at: now,
                })
            }
            VpsProvider::Fly => {
                // Fly Machines API: POST /v1/apps/{app}/machines
                let body = serde_json::json!({
                    "name": "dx-daemon",
                    "config": {
                        "image": "ghcr.io/dx-project/dx-daemon:latest",
                        "guest": {
                            "cpu_kind": "shared",
                            "cpus": 1,
                            "memory_mb": 256,
                        },
                    },
                });
                let url = format!("{}/apps/dx-daemon/machines", config.provider.api_base());
                let output = self.api_call(&url, api_token, &body).await?;
                let id = output
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                Ok(DeploymentInfo {
                    instance_id: id,
                    ip_address: "fly.dev".into(),
                    provider: config.provider,
                    region: config.region.clone(),
                    instance_type: config.instance_type.clone(),
                    state: DeployState::Running,
                    created_at: now,
                })
            }
            _ => {
                // Linode, Vultr, Custom — similar pattern
                log::info!(
                    "Provisioning {:?} instance (generic flow)",
                    config.provider
                );
                Ok(DeploymentInfo {
                    instance_id: format!("dx-{}", now),
                    ip_address: "0.0.0.0".into(),
                    provider: config.provider,
                    region: config.region.clone(),
                    instance_type: config.instance_type.clone(),
                    state: DeployState::Running,
                    created_at: now,
                })
            }
        }
    }

    /// Make a JSON API call using curl.
    async fn api_call(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let json_str = serde_json::to_string(body)?;
        let output = std::process::Command::new("curl")
            .args([
                "-s",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                &format!("Authorization: Bearer {}", token),
                "-d",
                &json_str,
                url,
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("curl failed: {}", e))?;

        let body_str = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&body_str)
            .map_err(|e| anyhow::anyhow!("Invalid JSON from API: {} — raw: {}", e, body_str))
    }

    /// Wait for SSH to become available on the instance.
    async fn wait_for_ssh(&self, ip: &str) -> Result<()> {
        log::info!("Waiting for SSH on {}:22 ...", ip);
        for attempt in 0..30 {
            let result = std::process::Command::new("ssh")
                .args([
                    "-o",
                    "StrictHostKeyChecking=no",
                    "-o",
                    "ConnectTimeout=5",
                    "-o",
                    "BatchMode=yes",
                    &format!("root@{}", ip),
                    "echo ok",
                ])
                .output();
            if let Ok(out) = result {
                if out.status.success() {
                    log::info!("SSH available after {} attempt(s)", attempt + 1);
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        Err(anyhow::anyhow!("SSH not available after 5 minutes"))
    }

    /// Upload the daemon binary via scp.
    async fn upload_binary(
        &self,
        local_path: &str,
        remote_ip: &str,
        ssh_key: Option<&str>,
    ) -> Result<()> {
        log::info!("Uploading binary to {}...", remote_ip);
        let mut cmd = std::process::Command::new("scp");
        cmd.args(["-o", "StrictHostKeyChecking=no"]);
        if let Some(key) = ssh_key {
            cmd.args(["-i", key]);
        }
        cmd.arg(local_path);
        cmd.arg(format!("root@{}:/usr/local/bin/dx-daemon", remote_ip));

        let status = cmd.status().map_err(|e| anyhow::anyhow!("scp failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!("scp exited with code {:?}", status.code()));
        }
        Ok(())
    }

    /// Configure and start the systemd service on the remote machine.
    async fn configure_remote_service(
        &self,
        remote_ip: &str,
        ssh_key: Option<&str>,
    ) -> Result<()> {
        let setup_commands = r#"
chmod +x /usr/local/bin/dx-daemon
cat > /etc/systemd/system/dx-daemon.service << 'EOF'
[Unit]
Description=DX AI Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/dx-daemon daemon --port 42069
Restart=on-failure
RestartSec=5
MemoryMax=512M
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable dx-daemon
systemctl start dx-daemon
"#;

        let mut cmd = std::process::Command::new("ssh");
        cmd.args(["-o", "StrictHostKeyChecking=no"]);
        if let Some(key) = ssh_key {
            cmd.args(["-i", key]);
        }
        cmd.arg(format!("root@{}", remote_ip));
        cmd.arg(setup_commands);

        let status = cmd.status().map_err(|e| anyhow::anyhow!("ssh failed: {}", e))?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "Remote setup exited with code {:?}",
                status.code()
            ));
        }

        log::info!("Remote service configured and started on {}", remote_ip);
        Ok(())
    }

    /// Stop and destroy the remote instance.
    pub async fn destroy(&mut self) -> Result<()> {
        let deployment = self
            .deployment
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active deployment to destroy"))?;

        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No VPS config"))?;

        let api_token = config
            .api_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No API token"))?;

        log::info!(
            "Destroying {:?} instance {}",
            deployment.provider,
            deployment.instance_id
        );

        match deployment.provider {
            VpsProvider::Hetzner => {
                let url = format!(
                    "{}/servers/{}",
                    deployment.provider.api_base(),
                    deployment.instance_id
                );
                let _ = std::process::Command::new("curl")
                    .args([
                        "-s",
                        "-X",
                        "DELETE",
                        "-H",
                        &format!("Authorization: Bearer {}", api_token),
                        &url,
                    ])
                    .output();
            }
            VpsProvider::DigitalOcean => {
                let url = format!(
                    "{}/droplets/{}",
                    deployment.provider.api_base(),
                    deployment.instance_id
                );
                let _ = std::process::Command::new("curl")
                    .args([
                        "-s",
                        "-X",
                        "DELETE",
                        "-H",
                        &format!("Authorization: Bearer {}", api_token),
                        &url,
                    ])
                    .output();
            }
            _ => {
                log::warn!(
                    "Destroy not fully implemented for {:?}",
                    deployment.provider
                );
            }
        }

        self.state = DeployState::Stopped;
        self.deployment = None;
        Ok(())
    }

    /// Check the status of a remote deployment by SSHing in.
    pub async fn check_health(&self) -> Result<bool> {
        let deployment = self
            .deployment
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active deployment"))?;

        let output = std::process::Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{}", deployment.ip_address),
                "systemctl is-active dx-daemon",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("SSH health check failed: {}", e))?;

        let status = String::from_utf8_lossy(&output.stdout);
        Ok(status.trim() == "active")
    }

    /// Get remote daemon logs.
    pub async fn get_logs(&self, lines: u32) -> Result<String> {
        let deployment = self
            .deployment
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active deployment"))?;

        let output = std::process::Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                &format!("root@{}", deployment.ip_address),
                &format!("journalctl -u dx-daemon -n {} --no-pager", lines),
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("SSH log fetch failed: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for VpsDeployer {
    fn default() -> Self {
        Self::new()
    }
}
