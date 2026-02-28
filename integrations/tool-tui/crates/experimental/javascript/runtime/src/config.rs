//! Configuration file loading for DX runtime
//!
//! Supports loading configuration from:
//! - `dx.config.json` - JSON format configuration
//! - `dx.config.js` - JavaScript format configuration (exports object)
//!
//! Configuration files are searched in the project root directory.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Error type for configuration loading
#[derive(Debug)]
pub enum ConfigError {
    /// File could not be read
    IoError(std::io::Error),
    /// JSON parsing failed
    ParseError(String),
    /// Invalid configuration value
    ValidationError(String),
    /// JavaScript config file not supported yet
    JsConfigNotSupported,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ConfigError::JsConfigNotSupported => {
                write!(f, "JavaScript config files are not yet supported")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::IoError(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::ParseError(e.to_string())
    }
}

/// Project configuration loaded from dx.config.json or dx.config.js
///
/// This struct represents the full configuration that can be specified
/// in a project's configuration file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    /// Runtime configuration
    #[serde(default)]
    pub runtime: RuntimeConfigFile,

    /// Package manager configuration
    #[serde(default)]
    pub package_manager: PackageManagerConfigFile,

    /// Bundler configuration
    #[serde(default)]
    pub bundler: BundlerConfigFile,

    /// Test runner configuration
    #[serde(default)]
    pub test_runner: TestRunnerConfigFile,
}

/// Runtime-specific configuration from config file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigFile {
    /// Maximum heap size in MB
    #[serde(default)]
    pub max_heap_size: Option<usize>,

    /// Enable experimental features
    #[serde(default)]
    pub experimental: Vec<String>,

    /// Module resolution mode ("node" or "bundler")
    #[serde(default)]
    pub module_resolution: Option<String>,

    /// Enable TypeScript type checking
    #[serde(default)]
    pub type_check: Option<bool>,

    /// Number of worker threads
    #[serde(default)]
    pub workers: Option<usize>,
}

/// Package manager configuration from config file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageManagerConfigFile {
    /// npm registry URL
    #[serde(default)]
    pub registry: Option<String>,

    /// Cache directory
    #[serde(default)]
    pub cache_dir: Option<String>,

    /// Enable strict peer dependencies
    #[serde(default)]
    pub strict_peer_deps: Option<bool>,
}

/// Bundler configuration from config file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BundlerConfigFile {
    /// Target ECMAScript version
    #[serde(default)]
    pub target: Option<String>,

    /// Enable minification
    #[serde(default)]
    pub minify: Option<bool>,

    /// Generate source maps
    #[serde(default)]
    pub source_maps: Option<bool>,

    /// Entry points
    #[serde(default)]
    pub entry: Option<Vec<String>>,

    /// Output directory
    #[serde(default)]
    pub out_dir: Option<String>,
}

/// Test runner configuration from config file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TestRunnerConfigFile {
    /// Enable parallel test execution
    #[serde(default)]
    pub parallel: Option<bool>,

    /// Enable code coverage
    #[serde(default)]
    pub coverage: Option<bool>,

    /// Test timeout in milliseconds
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Test file patterns
    #[serde(default)]
    pub include: Option<Vec<String>>,

    /// Files to exclude
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

/// Result of loading a configuration file
#[derive(Debug)]
pub struct LoadedConfig {
    /// The loaded configuration
    pub config: ProjectConfig,
    /// Path to the config file that was loaded
    pub config_path: PathBuf,
}

/// Load configuration from a project root directory
///
/// Searches for configuration files in the following order:
/// 1. `dx.config.json`
/// 2. `dx.config.js` (not yet supported)
///
/// If no configuration file is found, returns default configuration.
///
/// # Arguments
///
/// * `project_root` - Path to the project root directory
///
/// # Returns
///
/// Returns `Ok(Some(LoadedConfig))` if a config file was found and loaded,
/// `Ok(None)` if no config file exists (defaults will be used),
/// or `Err(ConfigError)` if a config file exists but is invalid.
pub fn load_config(project_root: &Path) -> Result<Option<LoadedConfig>, ConfigError> {
    // Try dx.config.json first
    let json_path = project_root.join("dx.config.json");
    if json_path.exists() {
        let content = fs::read_to_string(&json_path)?;
        let config: ProjectConfig = serde_json::from_str(&content)?;
        validate_config(&config)?;
        return Ok(Some(LoadedConfig {
            config,
            config_path: json_path,
        }));
    }

    // Try dx.config.js (not yet supported)
    let js_path = project_root.join("dx.config.js");
    if js_path.exists() {
        // For now, return an error indicating JS config is not supported
        // In the future, we could evaluate the JS file to get the config
        return Err(ConfigError::JsConfigNotSupported);
    }

    // No config file found - use defaults
    Ok(None)
}

/// Load configuration from a specific file path
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
///
/// # Returns
///
/// Returns the loaded configuration or an error if loading fails.
pub fn load_config_from_file(config_path: &Path) -> Result<ProjectConfig, ConfigError> {
    let content = fs::read_to_string(config_path)?;

    let config: ProjectConfig = if config_path.extension().is_some_and(|ext| ext == "json") {
        serde_json::from_str(&content)?
    } else if config_path.extension().is_some_and(|ext| ext == "js") {
        return Err(ConfigError::JsConfigNotSupported);
    } else {
        // Try JSON by default
        serde_json::from_str(&content)?
    };

    validate_config(&config)?;
    Ok(config)
}

/// Validate configuration values
fn validate_config(config: &ProjectConfig) -> Result<(), ConfigError> {
    // Validate max heap size
    if let Some(heap_size) = config.runtime.max_heap_size {
        if heap_size < 16 {
            return Err(ConfigError::ValidationError(
                "maxHeapSize must be at least 16 MB".to_string(),
            ));
        }
        if heap_size > 16384 {
            return Err(ConfigError::ValidationError(
                "maxHeapSize must be at most 16384 MB (16 GB)".to_string(),
            ));
        }
    }

    // Validate module resolution
    if let Some(ref resolution) = config.runtime.module_resolution {
        if resolution != "node" && resolution != "bundler" {
            return Err(ConfigError::ValidationError(format!(
                "moduleResolution must be 'node' or 'bundler', got '{}'",
                resolution
            )));
        }
    }

    // Validate target
    if let Some(ref target) = config.bundler.target {
        let valid_targets = [
            "es5", "es6", "es2015", "es2016", "es2017", "es2018", "es2019", "es2020", "es2021",
            "es2022", "es2023", "esnext",
        ];
        if !valid_targets.contains(&target.to_lowercase().as_str()) {
            return Err(ConfigError::ValidationError(format!(
                "Invalid target '{}'. Valid targets: {:?}",
                target, valid_targets
            )));
        }
    }

    // Validate test timeout
    if let Some(timeout) = config.test_runner.timeout {
        if timeout == 0 {
            return Err(ConfigError::ValidationError(
                "Test timeout must be greater than 0".to_string(),
            ));
        }
    }

    Ok(())
}

/// Merge loaded config with DxConfig defaults
///
/// Takes a loaded ProjectConfig and applies its values to a DxConfig,
/// using defaults for any unspecified values.
pub fn merge_with_defaults(
    project_config: &ProjectConfig,
    defaults: &crate::DxConfig,
) -> crate::DxConfig {
    crate::DxConfig {
        cache_dir: defaults.cache_dir.clone(),
        type_check: project_config.runtime.type_check.unwrap_or(defaults.type_check),
        speculation: defaults.speculation,
        workers: project_config.runtime.workers.unwrap_or(defaults.workers),
        arena_size: defaults.arena_size,
        max_heap_size_mb: project_config.runtime.max_heap_size.unwrap_or(defaults.max_heap_size_mb),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_config() {
        let json = "{}";
        let config: ProjectConfig = serde_json::from_str(json).unwrap();
        assert!(config.runtime.max_heap_size.is_none());
    }

    #[test]
    fn test_parse_runtime_config() {
        let json = r#"{
            "runtime": {
                "maxHeapSize": 1024,
                "typeCheck": false,
                "workers": 4
            }
        }"#;
        let config: ProjectConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.runtime.max_heap_size, Some(1024));
        assert_eq!(config.runtime.type_check, Some(false));
        assert_eq!(config.runtime.workers, Some(4));
    }

    #[test]
    fn test_parse_full_config() {
        let json = r#"{
            "runtime": {
                "maxHeapSize": 512,
                "experimental": ["decorators"],
                "moduleResolution": "node"
            },
            "packageManager": {
                "registry": "https://registry.npmjs.org",
                "cacheDir": ".dx-cache"
            },
            "bundler": {
                "target": "es2022",
                "minify": true,
                "sourceMaps": true
            },
            "testRunner": {
                "parallel": true,
                "coverage": false,
                "timeout": 5000
            }
        }"#;
        let config: ProjectConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.runtime.max_heap_size, Some(512));
        assert_eq!(config.runtime.experimental, vec!["decorators"]);
        assert_eq!(config.runtime.module_resolution, Some("node".to_string()));

        assert_eq!(config.package_manager.registry, Some("https://registry.npmjs.org".to_string()));
        assert_eq!(config.package_manager.cache_dir, Some(".dx-cache".to_string()));

        assert_eq!(config.bundler.target, Some("es2022".to_string()));
        assert_eq!(config.bundler.minify, Some(true));
        assert_eq!(config.bundler.source_maps, Some(true));

        assert_eq!(config.test_runner.parallel, Some(true));
        assert_eq!(config.test_runner.coverage, Some(false));
        assert_eq!(config.test_runner.timeout, Some(5000));
    }

    #[test]
    fn test_validate_heap_size_too_small() {
        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                max_heap_size: Some(8),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_heap_size_too_large() {
        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                max_heap_size: Some(32768),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_module_resolution() {
        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                module_resolution: Some("invalid".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_target() {
        let config = ProjectConfig {
            bundler: BundlerConfigFile {
                target: Some("es3".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_timeout() {
        let config = ProjectConfig {
            test_runner: TestRunnerConfigFile {
                timeout: Some(0),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                max_heap_size: Some(512),
                module_resolution: Some("node".to_string()),
                ..Default::default()
            },
            bundler: BundlerConfigFile {
                target: Some("es2022".to_string()),
                ..Default::default()
            },
            test_runner: TestRunnerConfigFile {
                timeout: Some(5000),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_ok());
    }
}
