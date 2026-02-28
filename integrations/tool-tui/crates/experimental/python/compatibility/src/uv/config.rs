//! uv configuration parsing
//!
//! Parses uv.toml and [tool.uv] sections from pyproject.toml.

use crate::config::ConfigError;
use crate::runtime::PythonVersion;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Python selection preference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PythonPreference {
    /// Only use managed Python installations
    OnlyManaged,
    /// Prefer managed Python installations
    Managed,
    /// Prefer system Python installations
    #[default]
    System,
    /// Only use system Python installations
    OnlySystem,
}

/// uv configuration settings
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UvConfig {
    /// Primary package index URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_url: Option<String>,

    /// Additional index URLs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_index_url: Vec<String>,

    /// Local package directories
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub find_links: Vec<PathBuf>,

    /// Packages to never install as binary
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub no_binary: Vec<String>,

    /// Packages to only install as binary
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub only_binary: Vec<String>,

    /// Target Python version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,

    /// Python selection preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_preference: Option<PythonPreference>,

    /// Cache directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    /// Whether to compile Python files to bytecode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_bytecode: Option<bool>,
}

impl UvConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the Python version as a PythonVersion
    pub fn get_python_version(&self) -> Option<PythonVersion> {
        self.python_version.as_ref().and_then(|v| PythonVersion::parse(v).ok())
    }

    /// Check if this config has any settings
    pub fn is_empty(&self) -> bool {
        self.index_url.is_none()
            && self.extra_index_url.is_empty()
            && self.find_links.is_empty()
            && self.no_binary.is_empty()
            && self.only_binary.is_empty()
            && self.python_version.is_none()
            && self.python_preference.is_none()
    }
}

/// Merged configuration with warnings
#[derive(Debug, Clone)]
pub struct MergedConfig {
    /// The merged configuration
    pub config: UvConfig,
    /// Warnings about overridden settings
    pub warnings: Vec<String>,
}

/// pyproject.toml structure for [tool.uv] parsing
#[derive(Debug, Deserialize)]
struct PyProjectToml {
    tool: Option<ToolSection>,
}

#[derive(Debug, Deserialize)]
struct ToolSection {
    uv: Option<UvConfig>,
}

/// Loads uv configuration from files
pub struct UvConfigLoader;

impl UvConfigLoader {
    /// Load from uv.toml file
    pub fn load_uv_toml(path: &Path) -> Result<UvConfig, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: UvConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from pyproject.toml [tool.uv] section
    pub fn load_pyproject_uv(path: &Path) -> Result<Option<UvConfig>, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let pyproject: PyProjectToml = toml::from_str(&content)?;

        Ok(pyproject.tool.and_then(|t| t.uv))
    }

    /// Find and load uv configuration from a directory
    pub fn find_and_load(dir: &Path) -> Result<Option<UvConfig>, ConfigError> {
        // First try uv.toml
        let uv_toml = dir.join("uv.toml");
        if uv_toml.exists() {
            return Ok(Some(Self::load_uv_toml(&uv_toml)?));
        }

        // Then try pyproject.toml
        let pyproject = dir.join("pyproject.toml");
        if pyproject.exists() {
            return Self::load_pyproject_uv(&pyproject);
        }

        Ok(None)
    }

    /// Merge uv config with dx-py config (dx-py takes precedence)
    pub fn merge_with_dxpy(uv: UvConfig, dxpy: &crate::config::DxPyConfig) -> MergedConfig {
        let mut warnings = Vec::new();
        let mut config = uv.clone();

        // dx-py settings take precedence
        if let Some(ref index) = dxpy.index_url {
            if uv.index_url.is_some() && uv.index_url.as_ref() != Some(index) {
                warnings.push(format!(
                    "uv index-url '{}' overridden by dx-py setting '{}'",
                    uv.index_url.as_ref().unwrap(),
                    index
                ));
            }
            config.index_url = Some(index.clone());
        }

        if !dxpy.extra_index_urls.is_empty() {
            if !uv.extra_index_url.is_empty() {
                warnings.push("uv extra-index-url overridden by dx-py settings".to_string());
            }
            config.extra_index_url = dxpy.extra_index_urls.clone();
        }

        if let Some(ref version) = dxpy.python_version {
            let version_str = version.to_string();
            if uv.python_version.is_some() && uv.python_version.as_ref() != Some(&version_str) {
                warnings.push(format!(
                    "uv python-version '{}' overridden by dx-py setting '{}'",
                    uv.python_version.as_ref().unwrap(),
                    version_str
                ));
            }
            config.python_version = Some(version_str);
        }

        if let Some(ref cache) = dxpy.cache_dir {
            if uv.cache_dir.is_some() && uv.cache_dir.as_ref() != Some(cache) {
                warnings.push("uv cache-dir overridden by dx-py setting".to_string());
            }
            config.cache_dir = Some(cache.clone());
        }

        MergedConfig { config, warnings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_uv_toml() {
        let dir = TempDir::new().unwrap();
        let uv_toml = dir.path().join("uv.toml");

        fs::write(
            &uv_toml,
            r#"
index-url = "https://pypi.org/simple"
extra-index-url = ["https://test.pypi.org/simple"]
python-version = "3.12"
python-preference = "system"
"#,
        )
        .unwrap();

        let config = UvConfigLoader::load_uv_toml(&uv_toml).unwrap();

        assert_eq!(config.index_url, Some("https://pypi.org/simple".to_string()));
        assert_eq!(config.extra_index_url, vec!["https://test.pypi.org/simple"]);
        assert_eq!(config.python_version, Some("3.12".to_string()));
        assert_eq!(config.python_preference, Some(PythonPreference::System));
    }

    #[test]
    fn test_parse_pyproject_uv() {
        let dir = TempDir::new().unwrap();
        let pyproject = dir.path().join("pyproject.toml");

        fs::write(
            &pyproject,
            r#"
[project]
name = "test"

[tool.uv]
index-url = "https://pypi.org/simple"
python-version = "3.11"
"#,
        )
        .unwrap();

        let config = UvConfigLoader::load_pyproject_uv(&pyproject).unwrap().unwrap();

        assert_eq!(config.index_url, Some("https://pypi.org/simple".to_string()));
        assert_eq!(config.python_version, Some("3.11".to_string()));
    }

    #[test]
    fn test_merge_configs() {
        let uv = UvConfig {
            index_url: Some("https://uv.pypi.org".to_string()),
            python_version: Some("3.11".to_string()),
            ..Default::default()
        };

        let dxpy = crate::config::DxPyConfig {
            index_url: Some("https://dxpy.pypi.org".to_string()),
            python_version: Some(PythonVersion::new(3, 12, 0)),
            ..Default::default()
        };

        let merged = UvConfigLoader::merge_with_dxpy(uv, &dxpy);

        assert_eq!(merged.config.index_url, Some("https://dxpy.pypi.org".to_string()));
        assert_eq!(merged.config.python_version, Some("3.12.0".to_string()));
        assert_eq!(merged.warnings.len(), 2); // Both settings were overridden
    }
}
