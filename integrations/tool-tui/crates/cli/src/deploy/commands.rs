//! Deploy CLI Commands
//!
//! Provides the `dx deploy` subcommand with one-click deployment capabilities.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::deploy::cloud::CloudDeployer;
use crate::deploy::docker::DockerBuilder;
use crate::deploy::oneclick::{OneClickDeploy, Platform, ProjectType};
use crate::deploy::vps::deploy_to_vps;
use crate::deploy::{BuildMode, DeployConfig, DeployPlatform};

/// Deploy command configuration
pub struct DeployCommand {
    /// Project root path
    pub project_root: PathBuf,
    /// Target platform
    pub platform: Option<Platform>,
    /// Dry run mode
    pub dry_run: bool,
    /// Verbose output
    pub verbose: bool,
}

impl DeployCommand {
    /// Create new deploy command
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            platform: None,
            dry_run: false,
            verbose: false,
        }
    }

    /// Set target platform
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Enable dry run mode
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.verbose = enabled;
        self
    }

    /// Create a DeployConfig for this command
    fn create_deploy_config(&self, platform: DeployPlatform) -> DeployConfig {
        let project_name = self
            .project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();

        DeployConfig {
            project_root: self.project_root.clone(),
            platform,
            build_mode: BuildMode::Optimized,
            binary_name: project_name,
            target_size: 20 * 1024 * 1024,
            target_memory: 128 * 1024 * 1024,
        }
    }

    /// Initialize deployment configuration
    pub async fn init(&self) -> anyhow::Result<InitResult> {
        let project_name = self
            .project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();

        let mut deployer = OneClickDeploy::new(&project_name, self.project_root.to_str().unwrap());
        let project_type = deployer.detect_project_type().clone();

        if self.verbose {
            println!("📦 Detected project type: {:?}", project_type);
        }

        // Generate all configurations
        let configs = deployer.generate_all();
        let docker_compose = deployer.generate_docker_compose();

        // Collect files to create
        let mut files = HashMap::new();

        files.insert(self.project_root.join("Dockerfile"), deployer.generate_dockerfile());

        files.insert(self.project_root.join("docker-compose.yml"), docker_compose);

        files.insert(
            self.project_root.join("fly.toml"),
            configs.get(&Platform::FlyIo).cloned().unwrap_or_default(),
        );

        files.insert(
            self.project_root.join("railway.toml"),
            configs.get(&Platform::Railway).cloned().unwrap_or_default(),
        );

        files.insert(
            self.project_root.join("render.yaml"),
            configs.get(&Platform::Render).cloned().unwrap_or_default(),
        );

        files.insert(
            self.project_root.join(".do/app.yaml"),
            configs.get(&Platform::DigitalOcean).cloned().unwrap_or_default(),
        );

        // Create .github/workflows directory structure
        let workflows_dir = self.project_root.join(".github").join("workflows");

        files.insert(
            workflows_dir.join("ci.yml"),
            include_str!("../deploy/templates/ci.yml.template").to_string(),
        );

        files.insert(
            workflows_dir.join("release.yml"),
            include_str!("../deploy/templates/release.yml.template").to_string(),
        );

        files.insert(
            workflows_dir.join("security.yml"),
            include_str!("../deploy/templates/security.yml.template").to_string(),
        );

        if !self.dry_run {
            // Actually create the files
            for (path, content) in &files {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(path, content).await?;
                if self.verbose {
                    println!("✅ Created: {}", path.display());
                }
            }
        }

        Ok(InitResult {
            project_type,
            files_created: files.keys().map(|p| p.display().to_string()).collect(),
        })
    }

    /// Deploy to Docker
    pub async fn docker(&self, tag: Option<&str>) -> anyhow::Result<DockerResult> {
        let project_name = self
            .project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();

        let image_tag = tag.unwrap_or(&project_name);

        if self.dry_run {
            return Ok(DockerResult {
                image_id: "dry-run".into(),
                image_tag: image_tag.into(),
                size_bytes: 0,
            });
        }

        let config = self.create_deploy_config(DeployPlatform::Docker);
        let builder = DockerBuilder::new(config);
        let build_result = builder.build(image_tag).await?;

        Ok(DockerResult {
            image_id: build_result.tag,
            image_tag: image_tag.into(),
            size_bytes: build_result.size,
        })
    }

    /// Deploy to Railway
    pub async fn railway(&self, _token: &str) -> anyhow::Result<CloudResult> {
        let config = self.create_deploy_config(DeployPlatform::Railway);
        let deployer = CloudDeployer::new(config);

        if self.dry_run {
            return Ok(CloudResult {
                platform: "Railway".into(),
                url: "https://dry-run.railway.app".into(),
                status: "dry-run".into(),
            });
        }

        let result = deployer.deploy_railway().await?;

        Ok(CloudResult {
            platform: "Railway".into(),
            url: result.url.unwrap_or_default(),
            status: "deployed".into(),
        })
    }

    /// Deploy to Fly.io
    pub async fn fly(&self, _token: &str) -> anyhow::Result<CloudResult> {
        let config = self.create_deploy_config(DeployPlatform::Fly);
        let deployer = CloudDeployer::new(config);

        if self.dry_run {
            return Ok(CloudResult {
                platform: "Fly.io".into(),
                url: "https://dry-run.fly.dev".into(),
                status: "dry-run".into(),
            });
        }

        let result = deployer.deploy_fly().await?;

        Ok(CloudResult {
            platform: "Fly.io".into(),
            url: result.url.unwrap_or_default(),
            status: "deployed".into(),
        })
    }

    /// Deploy to VPS via SSH
    pub async fn vps(
        &self,
        host: &str,
        user: &str,
        _key_path: Option<&str>,
    ) -> anyhow::Result<VpsResult> {
        if self.dry_run {
            return Ok(VpsResult {
                host: host.into(),
                service_name: "app".into(),
                status: "dry-run".into(),
            });
        }

        let config = self.create_deploy_config(DeployPlatform::Vps);
        deploy_to_vps(&config, host, user).await?;

        Ok(VpsResult {
            host: host.into(),
            service_name: "app".into(),
            status: "deployed".into(),
        })
    }

    /// Check deployment status
    pub async fn status(&self, platform: Platform) -> anyhow::Result<StatusResult> {
        let deploy_platform = match platform {
            Platform::FlyIo => DeployPlatform::Fly,
            Platform::Railway => DeployPlatform::Railway,
            Platform::Render => DeployPlatform::Render,
            Platform::DigitalOcean => DeployPlatform::DigitalOcean,
            Platform::Docker => DeployPlatform::Docker,
            Platform::AWS => DeployPlatform::AwsEcs,
            Platform::GCP => DeployPlatform::GcpCloudRun,
            Platform::VPS => DeployPlatform::Vps,
            _ => DeployPlatform::Docker,
        };
        let config = self.create_deploy_config(deploy_platform);
        let deployer = CloudDeployer::new(config);

        let (status, url) = match platform {
            Platform::Railway => {
                let status_result = deployer.status_railway().await?;
                let status = if status_result.healthy {
                    "healthy"
                } else {
                    "unhealthy"
                };
                (status.to_string(), "https://railway.app".into())
            }
            Platform::FlyIo => {
                let status_result = deployer.status_fly().await?;
                let status = if status_result.healthy {
                    "healthy"
                } else {
                    "unhealthy"
                };
                let url = format!(
                    "https://{}.fly.dev",
                    self.project_root.file_name().and_then(|n| n.to_str()).unwrap_or("app")
                );
                (status.to_string(), url)
            }
            _ => ("unsupported".into(), String::new()),
        };

        Ok(StatusResult {
            platform: format!("{:?}", platform),
            status,
            url,
        })
    }

    /// Get deployment logs
    pub async fn logs(&self, platform: Platform, lines: usize) -> anyhow::Result<Vec<String>> {
        let deploy_platform = match platform {
            Platform::FlyIo => DeployPlatform::Fly,
            Platform::Railway => DeployPlatform::Railway,
            _ => DeployPlatform::Docker,
        };
        let config = self.create_deploy_config(deploy_platform);
        let deployer = CloudDeployer::new(config);

        match platform {
            Platform::FlyIo => {
                let logs = deployer.logs_fly(lines).await?;
                Ok(logs.lines().map(|s| s.to_string()).collect())
            }
            Platform::Railway => {
                let logs = deployer.logs_railway(lines).await?;
                Ok(logs.lines().map(|s| s.to_string()).collect())
            }
            _ => Ok(vec!["Logs not available for this platform".into()]),
        }
    }

    /// Scale deployment
    pub async fn scale(&self, platform: Platform, instances: u32) -> anyhow::Result<()> {
        let deploy_platform = match platform {
            Platform::FlyIo => DeployPlatform::Fly,
            Platform::Railway => DeployPlatform::Railway,
            _ => DeployPlatform::Docker,
        };
        let config = self.create_deploy_config(deploy_platform);
        let deployer = CloudDeployer::new(config);

        match platform {
            Platform::FlyIo => deployer.scale_fly(instances, 256).await.map_err(Into::into),
            _ => Err(anyhow::anyhow!("Scaling not supported for this platform")),
        }
    }
}

/// Result of initialization
#[derive(Debug)]
pub struct InitResult {
    pub project_type: ProjectType,
    pub files_created: Vec<String>,
}

/// Result of Docker deployment
#[derive(Debug)]
pub struct DockerResult {
    pub image_id: String,
    pub image_tag: String,
    pub size_bytes: u64,
}

/// Result of cloud deployment
#[derive(Debug)]
pub struct CloudResult {
    pub platform: String,
    pub url: String,
    pub status: String,
}

/// Result of VPS deployment
#[derive(Debug)]
pub struct VpsResult {
    pub host: String,
    pub service_name: String,
    pub status: String,
}

/// Result of status check
#[derive(Debug)]
pub struct StatusResult {
    pub platform: String,
    pub status: String,
    pub url: String,
}

/// Parse platform from string
pub fn parse_platform(s: &str) -> Option<Platform> {
    match s.to_lowercase().as_str() {
        "docker" => Some(Platform::Docker),
        "railway" => Some(Platform::Railway),
        "fly" | "flyio" | "fly.io" => Some(Platform::FlyIo),
        "render" => Some(Platform::Render),
        "digitalocean" | "do" => Some(Platform::DigitalOcean),
        "aws" => Some(Platform::AWS),
        "gcp" | "google" => Some(Platform::GCP),
        "vercel" => Some(Platform::Vercel),
        "netlify" => Some(Platform::Netlify),
        "vps" | "ssh" => Some(Platform::VPS),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_platform() {
        assert_eq!(parse_platform("docker"), Some(Platform::Docker));
        assert_eq!(parse_platform("Railway"), Some(Platform::Railway));
        assert_eq!(parse_platform("fly.io"), Some(Platform::FlyIo));
        assert_eq!(parse_platform("DO"), Some(Platform::DigitalOcean));
        assert_eq!(parse_platform("unknown"), None);
    }

    #[tokio::test]
    async fn test_deploy_command_dry_run() {
        let cmd = DeployCommand::new(PathBuf::from(".")).dry_run(true).verbose(true);

        let result = cmd.docker(Some("test:latest")).await.unwrap();
        assert_eq!(result.image_tag, "test:latest");
        assert_eq!(result.status(), "dry-run");
    }
}

impl DockerResult {
    pub fn status(&self) -> &str {
        if self.image_id == "dry-run" {
            "dry-run"
        } else {
            "built"
        }
    }
}
