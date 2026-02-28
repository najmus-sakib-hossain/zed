//! Configuration types
//!
//! Main configuration structures for dx-py.

use crate::runtime::PythonVersion;
use crate::uv::UvConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// dx-py configuration
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DxPyConfig {
    /// Target Python version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<PythonVersion>,

    /// Primary package index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_url: Option<String>,

    /// Additional indexes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_index_urls: Vec<String>,

    /// Cache directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    /// Maximum concurrent downloads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_downloads: Option<u32>,

    /// uv compatibility settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uv_compat: Option<UvConfig>,
}

impl DxPyConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Python version
    pub fn with_python_version(mut self, version: PythonVersion) -> Self {
        self.python_version = Some(version);
        self
    }

    /// Set the index URL
    pub fn with_index_url(mut self, url: impl Into<String>) -> Self {
        self.index_url = Some(url.into());
        self
    }

    /// Add an extra index URL
    pub fn with_extra_index_url(mut self, url: impl Into<String>) -> Self {
        self.extra_index_urls.push(url.into());
        self
    }

    /// Set the cache directory
    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    /// Set max concurrent downloads
    pub fn with_max_concurrent_downloads(mut self, max: u32) -> Self {
        self.max_concurrent_downloads = Some(max);
        self
    }

    /// Load configuration from a TOML file
    pub fn load(path: &std::path::Path) -> Result<Self, super::ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &std::path::Path) -> Result<(), super::ConfigError> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, super::ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Deserialize from TOML string
    pub fn from_toml(s: &str) -> Result<Self, super::ConfigError> {
        Ok(toml::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = DxPyConfig::new()
            .with_python_version(PythonVersion::new(3, 12, 0))
            .with_index_url("https://pypi.org/simple")
            .with_max_concurrent_downloads(10);

        let toml = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml).unwrap();

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_config_round_trip() {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, 11, 5)),
            index_url: Some("https://test.pypi.org/simple".to_string()),
            extra_index_urls: vec!["https://extra.pypi.org".to_string()],
            cache_dir: Some(PathBuf::from("/tmp/cache")),
            max_concurrent_downloads: Some(5),
            uv_compat: None,
        };

        let toml = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml).unwrap();

        assert_eq!(config, parsed);
    }
}
