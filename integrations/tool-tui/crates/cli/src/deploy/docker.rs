//! Docker Build
//!
//! Docker image building and management.

use super::{DeployConfig, DeployError};
use std::process::Command;

/// Docker builder
pub struct DockerBuilder {
    /// Configuration
    config: DeployConfig,
}

impl DockerBuilder {
    /// Create a new Docker builder
    pub fn new(config: DeployConfig) -> Self {
        Self { config }
    }

    /// Build Docker image
    pub async fn build(&self, tag: &str) -> Result<BuildResult, DeployError> {
        let status = Command::new("docker")
            .args(["build", "-t", tag, "-f", "Dockerfile", "."])
            .current_dir(&self.config.project_root)
            .status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Docker build failed".to_string()));
        }

        // Get image info
        let info = self.get_image_info(tag).await?;

        Ok(BuildResult {
            tag: tag.to_string(),
            size: info.size,
            layers: info.layers,
        })
    }

    /// Build with cache optimization
    pub async fn build_optimized(&self, tag: &str) -> Result<BuildResult, DeployError> {
        let status = Command::new("docker")
            .args([
                "build",
                "--build-arg",
                "BUILDKIT_INLINE_CACHE=1",
                "--cache-from",
                tag,
                "-t",
                tag,
                ".",
            ])
            .current_dir(&self.config.project_root)
            .env("DOCKER_BUILDKIT", "1")
            .status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Docker optimized build failed".to_string()));
        }

        let info = self.get_image_info(tag).await?;

        Ok(BuildResult {
            tag: tag.to_string(),
            size: info.size,
            layers: info.layers,
        })
    }

    /// Get image info
    async fn get_image_info(&self, tag: &str) -> Result<ImageInfo, DeployError> {
        let output = Command::new("docker")
            .args(["inspect", "--format", "{{.Size}}", tag])
            .output()?;

        let size: u64 = String::from_utf8_lossy(&output.stdout).trim().parse().unwrap_or(0);

        let layers_output =
            Command::new("docker").args(["history", "--no-trunc", "-q", tag]).output()?;

        let layers = String::from_utf8_lossy(&layers_output.stdout).lines().count();

        Ok(ImageInfo { size, layers })
    }

    /// Push image to registry
    pub async fn push(&self, tag: &str) -> Result<(), DeployError> {
        let status = Command::new("docker").args(["push", tag]).status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Docker push failed".to_string()));
        }

        Ok(())
    }

    /// Tag image
    pub async fn tag(&self, source: &str, target: &str) -> Result<(), DeployError> {
        let status = Command::new("docker").args(["tag", source, target]).status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Docker tag failed".to_string()));
        }

        Ok(())
    }

    /// List local images
    pub async fn list_images(&self) -> Result<Vec<ImageInfo>, DeployError> {
        let output = Command::new("docker")
            .args([
                "images",
                "--format",
                "{{.Repository}}:{{.Tag}}|{{.Size}}|{{.ID}}",
            ])
            .output()?;

        let images = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    Some(ImageInfo {
                        size: parse_size(parts[1]),
                        layers: 0, // Not available from this format
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(images)
    }

    /// Clean up old images
    pub async fn cleanup(&self) -> Result<CleanupResult, DeployError> {
        // Remove dangling images
        let output = Command::new("docker").args(["image", "prune", "-f"]).output()?;

        let cleaned = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| l.starts_with("deleted:"))
            .count();

        Ok(CleanupResult {
            images_removed: cleaned,
        })
    }

    /// Run container locally
    pub async fn run(&self, tag: &str, port: u16) -> Result<String, DeployError> {
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "-p",
                &format!("{}:8080", port),
                "--name",
                &self.config.binary_name,
                tag,
            ])
            .output()?;

        if !output.status.success() {
            return Err(DeployError::DockerError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(container_id)
    }

    /// Stop container
    pub async fn stop(&self, container: &str) -> Result<(), DeployError> {
        let status = Command::new("docker").args(["stop", container]).status()?;

        if !status.success() {
            return Err(DeployError::DockerError("Failed to stop container".to_string()));
        }

        Ok(())
    }

    /// Check if Docker is available
    pub fn is_available() -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Build result
#[derive(Debug)]
pub struct BuildResult {
    /// Image tag
    pub tag: String,
    /// Image size in bytes
    pub size: u64,
    /// Number of layers
    pub layers: usize,
}

/// Image info
#[derive(Debug)]
pub struct ImageInfo {
    /// Image size in bytes
    pub size: u64,
    /// Number of layers
    pub layers: usize,
}

/// Cleanup result
#[derive(Debug)]
pub struct CleanupResult {
    /// Images removed
    pub images_removed: usize,
}

/// Parse Docker size string (e.g., "50MB") to bytes
fn parse_size(size_str: &str) -> u64 {
    let size_str = size_str.trim();

    if size_str.ends_with("GB") {
        let num: f64 = size_str.trim_end_matches("GB").parse().unwrap_or(0.0);
        (num * 1024.0 * 1024.0 * 1024.0) as u64
    } else if size_str.ends_with("MB") {
        let num: f64 = size_str.trim_end_matches("MB").parse().unwrap_or(0.0);
        (num * 1024.0 * 1024.0) as u64
    } else if size_str.ends_with("KB") {
        let num: f64 = size_str.trim_end_matches("KB").parse().unwrap_or(0.0);
        (num * 1024.0) as u64
    } else if size_str.ends_with('B') {
        size_str.trim_end_matches('B').parse().unwrap_or(0)
    } else {
        size_str.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100MB"), 104857600);
        assert_eq!(parse_size("1GB"), 1073741824);
        assert_eq!(parse_size("50KB"), 51200);
        assert_eq!(parse_size("1000B"), 1000);
    }

    #[test]
    fn test_docker_available() {
        // This test just checks the function doesn't panic
        let _ = DockerBuilder::is_available();
    }
}
