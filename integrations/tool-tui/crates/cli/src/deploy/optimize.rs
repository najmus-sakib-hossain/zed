//! Binary Optimization
//!
//! Optimizes binary size and runtime performance for deployment.

use super::{BuildMode, DeployConfig, DeployError};
use std::path::PathBuf;
use std::process::Command;

/// Binary optimizer
pub struct BinaryOptimizer {
    /// Configuration
    config: DeployConfig,
}

/// Optimization result
#[derive(Debug)]
pub struct OptimizeResult {
    /// Original binary size
    pub original_size: u64,
    /// Optimized binary size
    pub optimized_size: u64,
    /// Size reduction percentage
    pub reduction_percent: f32,
    /// Optimizations applied
    pub optimizations: Vec<String>,
}

impl BinaryOptimizer {
    /// Create a new binary optimizer
    pub fn new(config: DeployConfig) -> Self {
        Self { config }
    }

    /// Optimize the binary
    pub async fn optimize(&self) -> Result<OptimizeResult, DeployError> {
        let binary_path = self.get_binary_path()?;
        let original_size = std::fs::metadata(&binary_path)
            .map_err(|e| DeployError::BuildFailed(format!("Binary not found: {}", e)))?
            .len();

        let mut optimizations = Vec::new();

        // Run optimization steps
        if self.config.build_mode == BuildMode::Optimized {
            // Strip debug symbols
            if self.strip_binary(&binary_path)? {
                optimizations.push("Stripped debug symbols".to_string());
            }

            // Compress with UPX (if available)
            if self.upx_compress(&binary_path)? {
                optimizations.push("UPX compression".to_string());
            }
        }

        let optimized_size = std::fs::metadata(&binary_path)
            .map_err(|e| DeployError::BuildFailed(format!("Binary lost: {}", e)))?
            .len();

        let reduction = if original_size > 0 {
            ((original_size - optimized_size) as f32 / original_size as f32) * 100.0
        } else {
            0.0
        };

        Ok(OptimizeResult {
            original_size,
            optimized_size,
            reduction_percent: reduction,
            optimizations,
        })
    }

    /// Get path to the built binary
    fn get_binary_path(&self) -> Result<PathBuf, DeployError> {
        let target_dir = match self.config.build_mode {
            BuildMode::Debug => "debug",
            BuildMode::Release | BuildMode::Optimized => "release",
        };

        let path = self
            .config
            .project_root
            .join("target")
            .join(target_dir)
            .join(&self.config.binary_name);

        // Handle Windows .exe extension
        #[cfg(windows)]
        let path = path.with_extension("exe");

        Ok(path)
    }

    /// Strip debug symbols from binary
    fn strip_binary(&self, path: &PathBuf) -> Result<bool, DeployError> {
        // Check if strip command exists
        let strip_cmd = if cfg!(target_os = "macos") {
            "strip"
        } else if cfg!(target_os = "linux") {
            "strip"
        } else {
            return Ok(false); // Windows uses different tools
        };

        let status = Command::new(strip_cmd).args(["-s", path.to_str().unwrap_or("")]).status();

        match status {
            Ok(s) => Ok(s.success()),
            Err(_) => Ok(false), // strip not available
        }
    }

    /// Compress binary with UPX
    fn upx_compress(&self, path: &PathBuf) -> Result<bool, DeployError> {
        let status = Command::new("upx")
            .args(["--best", "--lzma", path.to_str().unwrap_or("")])
            .status();

        match status {
            Ok(s) => Ok(s.success()),
            Err(_) => Ok(false), // UPX not available
        }
    }

    /// Generate optimized Cargo.toml profile
    pub fn generate_cargo_profile() -> String {
        r#"
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
panic = "abort"     # Smaller panic handling
strip = true        # Strip symbols

[profile.release.package."*"]
opt-level = "z"     # Optimize dependencies for size too
"#
        .to_string()
    }

    /// Check if binary meets size target
    pub fn check_size_target(&self, size: u64) -> SizeCheck {
        if size <= self.config.target_size {
            SizeCheck::Pass {
                size,
                target: self.config.target_size,
            }
        } else {
            SizeCheck::Fail {
                size,
                target: self.config.target_size,
                over_by: size - self.config.target_size,
            }
        }
    }

    /// Get recommendations for further size reduction
    pub fn get_size_recommendations(&self) -> Vec<Recommendation> {
        vec![
            Recommendation {
                title: "Enable LTO".to_string(),
                description: "Link-time optimization can reduce binary size significantly"
                    .to_string(),
                cargo_config: Some("lto = true".to_string()),
                estimated_savings: "10-20%".to_string(),
            },
            Recommendation {
                title: "Single codegen unit".to_string(),
                description: "Allows better optimization but slower builds".to_string(),
                cargo_config: Some("codegen-units = 1".to_string()),
                estimated_savings: "5-10%".to_string(),
            },
            Recommendation {
                title: "Optimize for size".to_string(),
                description: "Use opt-level = 'z' instead of '3'".to_string(),
                cargo_config: Some("opt-level = \"z\"".to_string()),
                estimated_savings: "10-30%".to_string(),
            },
            Recommendation {
                title: "Strip symbols".to_string(),
                description: "Remove debug symbols from release binary".to_string(),
                cargo_config: Some("strip = true".to_string()),
                estimated_savings: "30-50%".to_string(),
            },
            Recommendation {
                title: "Abort on panic".to_string(),
                description: "Smaller panic handling code".to_string(),
                cargo_config: Some("panic = \"abort\"".to_string()),
                estimated_savings: "5-10%".to_string(),
            },
            Recommendation {
                title: "Feature audit".to_string(),
                description: "Disable unused features in dependencies".to_string(),
                cargo_config: None,
                estimated_savings: "Variable".to_string(),
            },
            Recommendation {
                title: "UPX compression".to_string(),
                description: "Compress the final binary with UPX".to_string(),
                cargo_config: None,
                estimated_savings: "50-70%".to_string(),
            },
        ]
    }
}

/// Size check result
#[derive(Debug)]
pub enum SizeCheck {
    /// Binary is under target size
    Pass { size: u64, target: u64 },
    /// Binary exceeds target size
    Fail {
        size: u64,
        target: u64,
        over_by: u64,
    },
}

/// Size reduction recommendation
#[derive(Debug)]
pub struct Recommendation {
    /// Recommendation title
    pub title: String,
    /// Description
    pub description: String,
    /// Cargo.toml configuration
    pub cargo_config: Option<String>,
    /// Estimated savings
    pub estimated_savings: String,
}

/// Format bytes as human-readable
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_size_check_pass() {
        let config = DeployConfig {
            target_size: 20 * 1024 * 1024, // 20MB
            ..Default::default()
        };
        let optimizer = BinaryOptimizer::new(config);

        match optimizer.check_size_target(15 * 1024 * 1024) {
            SizeCheck::Pass { .. } => {}
            SizeCheck::Fail { .. } => panic!("Should pass"),
        }
    }

    #[test]
    fn test_size_check_fail() {
        let config = DeployConfig {
            target_size: 20 * 1024 * 1024, // 20MB
            ..Default::default()
        };
        let optimizer = BinaryOptimizer::new(config);

        match optimizer.check_size_target(25 * 1024 * 1024) {
            SizeCheck::Pass { .. } => panic!("Should fail"),
            SizeCheck::Fail { over_by, .. } => {
                assert_eq!(over_by, 5 * 1024 * 1024);
            }
        }
    }

    #[test]
    fn test_cargo_profile() {
        let profile = BinaryOptimizer::generate_cargo_profile();

        assert!(profile.contains("opt-level"));
        assert!(profile.contains("lto = true"));
        assert!(profile.contains("strip = true"));
    }

    #[test]
    fn test_recommendations() {
        let config = DeployConfig::default();
        let optimizer = BinaryOptimizer::new(config);

        let recommendations = optimizer.get_size_recommendations();

        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.title.contains("LTO")));
        assert!(recommendations.iter().any(|r| r.title.contains("Strip")));
    }
}
