//! Cloud Platform Deployment
//!
//! Support for various cloud platform deployments.

use super::{DeployConfig, DeployError, DeployPlatform, DeployResult, DeployStatus};
use std::process::Command;

/// Cloud deployer
pub struct CloudDeployer {
    /// Configuration
    config: DeployConfig,
}

impl CloudDeployer {
    /// Create a new cloud deployer
    pub fn new(config: DeployConfig) -> Self {
        Self { config }
    }

    /// Deploy to Railway
    pub async fn deploy_railway(&self) -> Result<DeployResult, DeployError> {
        // Check if Railway CLI is available
        if !self.check_cli("railway") {
            return Err(DeployError::CloudError(
                "Railway CLI not installed. Install with: npm install -g @railway/cli".to_string(),
            ));
        }

        // Check login status
        let status = Command::new("railway").args(["whoami"]).output()?;

        if !status.status.success() {
            return Err(DeployError::CloudError(
                "Not logged in to Railway. Run: railway login".to_string(),
            ));
        }

        // Deploy
        let deploy_status = Command::new("railway")
            .args(["up", "--detach"])
            .current_dir(&self.config.project_root)
            .status()?;

        if !deploy_status.success() {
            return Err(DeployError::CloudError("Railway deployment failed".to_string()));
        }

        Ok(DeployResult {
            platform: DeployPlatform::Railway,
            url: Some("https://your-app.railway.app".to_string()),
            status: DeployStatus::Success,
            message: "Deployed to Railway successfully".to_string(),
        })
    }

    /// Deploy to Fly.io
    pub async fn deploy_fly(&self) -> Result<DeployResult, DeployError> {
        // Check if Fly CLI is available
        if !self.check_cli("fly") {
            return Err(DeployError::CloudError(
                "Fly CLI not installed. Install from: https://fly.io/docs/hands-on/install-flyctl/"
                    .to_string(),
            ));
        }

        // Check login status
        let status = Command::new("fly").args(["auth", "whoami"]).output()?;

        if !status.status.success() {
            return Err(DeployError::CloudError(
                "Not logged in to Fly.io. Run: fly auth login".to_string(),
            ));
        }

        // Deploy
        let deploy_status = Command::new("fly")
            .args(["deploy"])
            .current_dir(&self.config.project_root)
            .status()?;

        if !deploy_status.success() {
            return Err(DeployError::CloudError("Fly deployment failed".to_string()));
        }

        Ok(DeployResult {
            platform: DeployPlatform::Fly,
            url: Some(format!("https://{}.fly.dev", self.config.binary_name)),
            status: DeployStatus::Success,
            message: "Deployed to Fly.io successfully".to_string(),
        })
    }

    /// Check deployment status on Railway
    pub async fn status_railway(&self) -> Result<PlatformStatus, DeployError> {
        let output = Command::new("railway")
            .args(["status"])
            .current_dir(&self.config.project_root)
            .output()?;

        Ok(PlatformStatus {
            platform: DeployPlatform::Railway,
            healthy: output.status.success(),
            details: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }

    /// Check deployment status on Fly
    pub async fn status_fly(&self) -> Result<PlatformStatus, DeployError> {
        let output = Command::new("fly")
            .args(["status"])
            .current_dir(&self.config.project_root)
            .output()?;

        Ok(PlatformStatus {
            platform: DeployPlatform::Fly,
            healthy: output.status.success(),
            details: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }

    /// Get logs from Railway
    pub async fn logs_railway(&self, lines: usize) -> Result<String, DeployError> {
        let output = Command::new("railway")
            .args(["logs", "--tail", &lines.to_string()])
            .current_dir(&self.config.project_root)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get logs from Fly
    pub async fn logs_fly(&self, lines: usize) -> Result<String, DeployError> {
        let output = Command::new("fly")
            .args(["logs", "--no-tail", "-n", &lines.to_string()])
            .current_dir(&self.config.project_root)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Scale deployment on Fly
    pub async fn scale_fly(&self, count: u32, memory_mb: u32) -> Result<(), DeployError> {
        let status = Command::new("fly")
            .args([
                "scale",
                "count",
                &count.to_string(),
                "--memory",
                &memory_mb.to_string(),
            ])
            .current_dir(&self.config.project_root)
            .status()?;

        if !status.success() {
            return Err(DeployError::CloudError("Scale failed".to_string()));
        }

        Ok(())
    }

    /// Check if a CLI tool is available
    fn check_cli(&self, cmd: &str) -> bool {
        Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get supported platforms
    pub fn supported_platforms() -> Vec<PlatformInfo> {
        vec![
            PlatformInfo {
                platform: DeployPlatform::Railway,
                name: "Railway".to_string(),
                cli_command: "railway".to_string(),
                install_url: "https://railway.app/".to_string(),
                pricing: "Free tier: 500 hours/month, $5/month after".to_string(),
            },
            PlatformInfo {
                platform: DeployPlatform::Fly,
                name: "Fly.io".to_string(),
                cli_command: "fly".to_string(),
                install_url: "https://fly.io/docs/hands-on/install-flyctl/".to_string(),
                pricing: "Free tier: 3 shared CPUs, $1.94/mo/256MB".to_string(),
            },
            PlatformInfo {
                platform: DeployPlatform::Render,
                name: "Render".to_string(),
                cli_command: "render".to_string(),
                install_url: "https://render.com/".to_string(),
                pricing: "Free tier available, $7/mo for starter".to_string(),
            },
            PlatformInfo {
                platform: DeployPlatform::DigitalOcean,
                name: "DigitalOcean App Platform".to_string(),
                cli_command: "doctl".to_string(),
                install_url: "https://docs.digitalocean.com/reference/doctl/".to_string(),
                pricing: "From $5/mo".to_string(),
            },
        ]
    }
}

/// Platform status
#[derive(Debug)]
pub struct PlatformStatus {
    /// Platform
    pub platform: DeployPlatform,
    /// Is healthy
    pub healthy: bool,
    /// Status details
    pub details: String,
}

/// Platform information
#[derive(Debug)]
pub struct PlatformInfo {
    /// Platform
    pub platform: DeployPlatform,
    /// Display name
    pub name: String,
    /// CLI command
    pub cli_command: String,
    /// Install URL
    pub install_url: String,
    /// Pricing info
    pub pricing: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_platforms() {
        let platforms = CloudDeployer::supported_platforms();

        assert!(!platforms.is_empty());
        assert!(platforms.iter().any(|p| p.platform == DeployPlatform::Railway));
        assert!(platforms.iter().any(|p| p.platform == DeployPlatform::Fly));
    }
}
