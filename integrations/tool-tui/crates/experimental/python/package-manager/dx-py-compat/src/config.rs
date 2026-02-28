//! Configuration system for dx-py
//!
//! Provides layered configuration loading from:
//! - Environment variables (highest priority)
//! - Project pyproject.toml [tool.dx-py]
//! - Global config ~/.config/dx-py/config.toml
//! - Default values (lowest priority)

use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Default PyPI index URL
pub const DEFAULT_INDEX_URL: &str = "https://pypi.org/simple/";

/// Default cache directory name
pub const DEFAULT_CACHE_DIR_NAME: &str = "dx-py";

/// Environment variable prefix for dx-py configuration
pub const ENV_PREFIX: &str = "DX_PY_";

/// Main configuration struct with all dx-py settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Primary PyPI index URL
    #[serde(default = "default_index_url")]
    pub index_url: String,

    /// Additional index URLs to search
    #[serde(default)]
    pub extra_index_urls: Vec<String>,

    /// Hosts to trust for HTTPS (skip certificate verification)
    #[serde(default)]
    pub trusted_hosts: Vec<String>,

    /// Cache directory path
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    /// Whether to allow downloading Python interpreters
    #[serde(default = "default_python_downloads")]
    pub python_downloads: bool,

    /// Maximum concurrent downloads
    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: usize,

    /// Download timeout in seconds
    #[serde(default = "default_download_timeout")]
    pub download_timeout: u64,

    /// Number of retry attempts for failed downloads
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// Whether to use hard links when installing packages
    #[serde(default = "default_use_hard_links")]
    pub use_hard_links: bool,

    /// Verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Offline mode (no network requests)
    #[serde(default)]
    pub offline: bool,

    /// Custom Python path
    #[serde(default)]
    pub python_path: Option<PathBuf>,

    /// Directory for managed Python installations
    #[serde(default = "default_python_install_dir")]
    pub python_install_dir: PathBuf,

    /// Directory for global tool installations
    #[serde(default = "default_tools_dir")]
    pub tools_dir: PathBuf,
}

fn default_index_url() -> String {
    DEFAULT_INDEX_URL.to_string()
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join(DEFAULT_CACHE_DIR_NAME)
}

fn default_python_downloads() -> bool {
    true
}

fn default_max_concurrent_downloads() -> usize {
    8
}

fn default_download_timeout() -> u64 {
    300 // 5 minutes
}

fn default_retry_count() -> u32 {
    3
}

fn default_use_hard_links() -> bool {
    true
}

fn default_python_install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local"))
        .join(DEFAULT_CACHE_DIR_NAME)
        .join("python")
}

fn default_tools_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local"))
        .join(DEFAULT_CACHE_DIR_NAME)
        .join("tools")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            index_url: default_index_url(),
            extra_index_urls: Vec::new(),
            trusted_hosts: Vec::new(),
            cache_dir: default_cache_dir(),
            python_downloads: default_python_downloads(),
            max_concurrent_downloads: default_max_concurrent_downloads(),
            download_timeout: default_download_timeout(),
            retry_count: default_retry_count(),
            use_hard_links: default_use_hard_links(),
            verbose: false,
            offline: false,
            python_path: None,
            python_install_dir: default_python_install_dir(),
            tools_dir: default_tools_dir(),
        }
    }
}

impl Config {
    /// Create a new Config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration with full layering:
    /// env vars > project config > global config > defaults
    pub fn load() -> Result<Self> {
        Self::load_from_dir(&std::env::current_dir().unwrap_or_default())
    }

    /// Load configuration from a specific project directory
    pub fn load_from_dir(project_dir: &Path) -> Result<Self> {
        let mut config = Self::default();

        // Layer 1: Load global config (lowest priority after defaults)
        if let Some(global_config) = Self::load_global_config()? {
            config.merge(&global_config);
        }

        // Layer 2: Load project config from pyproject.toml
        if let Some(project_config) = Self::load_project_config(project_dir)? {
            config.merge(&project_config);
        }

        // Layer 3: Apply environment variables (highest priority)
        config.apply_env_vars();

        Ok(config)
    }

    /// Load global configuration from ~/.config/dx-py/config.toml
    fn load_global_config() -> Result<Option<ConfigFile>> {
        let config_path = Self::global_config_path();
        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: ConfigFile = toml::from_str(&content)
            .map_err(|e| Error::Cache(format!("Failed to parse global config: {}", e)))?;
        Ok(Some(config))
    }

    /// Load project configuration from pyproject.toml [tool.dx-py]
    fn load_project_config(project_dir: &Path) -> Result<Option<ConfigFile>> {
        let pyproject_path = project_dir.join("pyproject.toml");
        if !pyproject_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&pyproject_path)?;
        let pyproject: PyProjectConfig = toml::from_str(&content)
            .map_err(|e| Error::Cache(format!("Failed to parse pyproject.toml: {}", e)))?;

        Ok(pyproject.tool.and_then(|t| t.dx_py))
    }

    /// Get the path to the global config file
    pub fn global_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join(DEFAULT_CACHE_DIR_NAME)
            .join("config.toml")
    }

    /// Merge another config file into this config
    pub fn merge(&mut self, other: &ConfigFile) {
        if let Some(ref url) = other.index_url {
            self.index_url = url.clone();
        }
        if let Some(ref urls) = other.extra_index_urls {
            self.extra_index_urls = urls.clone();
        }
        if let Some(ref hosts) = other.trusted_hosts {
            self.trusted_hosts = hosts.clone();
        }
        if let Some(ref dir) = other.cache_dir {
            self.cache_dir = dir.clone();
        }
        if let Some(downloads) = other.python_downloads {
            self.python_downloads = downloads;
        }
        if let Some(max) = other.max_concurrent_downloads {
            self.max_concurrent_downloads = max;
        }
        if let Some(timeout) = other.download_timeout {
            self.download_timeout = timeout;
        }
        if let Some(count) = other.retry_count {
            self.retry_count = count;
        }
        if let Some(hard_links) = other.use_hard_links {
            self.use_hard_links = hard_links;
        }
        if let Some(verbose) = other.verbose {
            self.verbose = verbose;
        }
        if let Some(offline) = other.offline {
            self.offline = offline;
        }
        if let Some(ref path) = other.python_path {
            self.python_path = Some(path.clone());
        }
        if let Some(ref dir) = other.python_install_dir {
            self.python_install_dir = dir.clone();
        }
        if let Some(ref dir) = other.tools_dir {
            self.tools_dir = dir.clone();
        }
    }

    /// Apply environment variables to override config values
    pub fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("DX_PY_INDEX_URL") {
            self.index_url = val;
        }
        if let Ok(val) = env::var("DX_PY_EXTRA_INDEX_URLS") {
            self.extra_index_urls = val.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(val) = env::var("DX_PY_TRUSTED_HOSTS") {
            self.trusted_hosts = val.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(val) = env::var("DX_PY_CACHE_DIR") {
            self.cache_dir = PathBuf::from(val);
        }
        if let Ok(val) = env::var("DX_PY_PYTHON_DOWNLOADS") {
            self.python_downloads = val.parse().unwrap_or(self.python_downloads);
        }
        if let Ok(val) = env::var("DX_PY_MAX_CONCURRENT_DOWNLOADS") {
            self.max_concurrent_downloads = val.parse().unwrap_or(self.max_concurrent_downloads);
        }
        if let Ok(val) = env::var("DX_PY_DOWNLOAD_TIMEOUT") {
            self.download_timeout = val.parse().unwrap_or(self.download_timeout);
        }
        if let Ok(val) = env::var("DX_PY_RETRY_COUNT") {
            self.retry_count = val.parse().unwrap_or(self.retry_count);
        }
        if let Ok(val) = env::var("DX_PY_USE_HARD_LINKS") {
            self.use_hard_links = val.parse().unwrap_or(self.use_hard_links);
        }
        if let Ok(val) = env::var("DX_PY_VERBOSE") {
            self.verbose = val.parse().unwrap_or(self.verbose);
        }
        if let Ok(val) = env::var("DX_PY_OFFLINE") {
            self.offline = val.parse().unwrap_or(self.offline);
        }
        if let Ok(val) = env::var("DX_PY_PYTHON_PATH") {
            self.python_path = Some(PathBuf::from(val));
        }
        if let Ok(val) = env::var("DX_PY_PYTHON_INSTALL_DIR") {
            self.python_install_dir = PathBuf::from(val);
        }
        if let Ok(val) = env::var("DX_PY_TOOLS_DIR") {
            self.tools_dir = PathBuf::from(val);
        }
    }

    /// Save this config to the global config file
    pub fn save_global(&self) -> Result<()> {
        let config_path = Self::global_config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let config_file = ConfigFile::from(self);
        let content = toml::to_string_pretty(&config_file)
            .map_err(|e| Error::Cache(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }
}

/// Partial config file structure (all fields optional for merging)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    pub index_url: Option<String>,
    pub extra_index_urls: Option<Vec<String>>,
    pub trusted_hosts: Option<Vec<String>>,
    pub cache_dir: Option<PathBuf>,
    pub python_downloads: Option<bool>,
    pub max_concurrent_downloads: Option<usize>,
    pub download_timeout: Option<u64>,
    pub retry_count: Option<u32>,
    pub use_hard_links: Option<bool>,
    pub verbose: Option<bool>,
    pub offline: Option<bool>,
    pub python_path: Option<PathBuf>,
    pub python_install_dir: Option<PathBuf>,
    pub tools_dir: Option<PathBuf>,
}

impl From<&Config> for ConfigFile {
    fn from(config: &Config) -> Self {
        Self {
            index_url: Some(config.index_url.clone()),
            extra_index_urls: Some(config.extra_index_urls.clone()),
            trusted_hosts: Some(config.trusted_hosts.clone()),
            cache_dir: Some(config.cache_dir.clone()),
            python_downloads: Some(config.python_downloads),
            max_concurrent_downloads: Some(config.max_concurrent_downloads),
            download_timeout: Some(config.download_timeout),
            retry_count: Some(config.retry_count),
            use_hard_links: Some(config.use_hard_links),
            verbose: Some(config.verbose),
            offline: Some(config.offline),
            python_path: config.python_path.clone(),
            python_install_dir: Some(config.python_install_dir.clone()),
            tools_dir: Some(config.tools_dir.clone()),
        }
    }
}

/// Helper struct for parsing pyproject.toml
#[derive(Debug, Deserialize)]
struct PyProjectConfig {
    tool: Option<ToolConfig>,
}

#[derive(Debug, Deserialize)]
struct ToolConfig {
    #[serde(rename = "dx-py")]
    dx_py: Option<ConfigFile>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.index_url, DEFAULT_INDEX_URL);
        assert!(config.extra_index_urls.is_empty());
        assert!(config.trusted_hosts.is_empty());
        assert_eq!(config.max_concurrent_downloads, 8);
        assert_eq!(config.download_timeout, 300);
        assert_eq!(config.retry_count, 3);
        assert!(config.python_downloads);
        assert!(config.use_hard_links);
        assert!(!config.verbose);
        assert!(!config.offline);
    }

    #[test]
    fn test_config_merge() {
        let mut config = Config::default();
        let override_config = ConfigFile {
            index_url: Some("https://custom.pypi.org/simple/".to_string()),
            max_concurrent_downloads: Some(16),
            verbose: Some(true),
            ..Default::default()
        };

        config.merge(&override_config);

        assert_eq!(config.index_url, "https://custom.pypi.org/simple/");
        assert_eq!(config.max_concurrent_downloads, 16);
        assert!(config.verbose);
        // Unchanged values
        assert_eq!(config.retry_count, 3);
    }

    #[test]
    fn test_env_var_override() {
        // Save original values
        let orig_index = env::var("DX_PY_INDEX_URL").ok();
        let orig_verbose = env::var("DX_PY_VERBOSE").ok();

        // Set test env vars
        env::set_var("DX_PY_INDEX_URL", "https://test.pypi.org/simple/");
        env::set_var("DX_PY_VERBOSE", "true");

        let mut config = Config::default();
        config.apply_env_vars();

        assert_eq!(config.index_url, "https://test.pypi.org/simple/");
        assert!(config.verbose);

        // Restore original values
        match orig_index {
            Some(v) => env::set_var("DX_PY_INDEX_URL", v),
            None => env::remove_var("DX_PY_INDEX_URL"),
        }
        match orig_verbose {
            Some(v) => env::set_var("DX_PY_VERBOSE", v),
            None => env::remove_var("DX_PY_VERBOSE"),
        }
    }

    #[test]
    fn test_extra_index_urls_parsing() {
        let orig = env::var("DX_PY_EXTRA_INDEX_URLS").ok();

        env::set_var("DX_PY_EXTRA_INDEX_URLS", "https://a.com, https://b.com, https://c.com");

        let mut config = Config::default();
        config.apply_env_vars();

        assert_eq!(config.extra_index_urls.len(), 3);
        assert_eq!(config.extra_index_urls[0], "https://a.com");
        assert_eq!(config.extra_index_urls[1], "https://b.com");
        assert_eq!(config.extra_index_urls[2], "https://c.com");

        match orig {
            Some(v) => env::set_var("DX_PY_EXTRA_INDEX_URLS", v),
            None => env::remove_var("DX_PY_EXTRA_INDEX_URLS"),
        }
    }

    #[test]
    fn test_config_file_serialization() {
        let config = Config::default();
        let config_file = ConfigFile::from(&config);

        let toml_str = toml::to_string_pretty(&config_file).unwrap();
        assert!(toml_str.contains("index_url"));
        assert!(toml_str.contains("pypi.org"));
    }

    #[test]
    fn test_parse_project_config_toml() {
        let toml_content = r#"
[project]
name = "test-project"

[tool.dx-py]
index_url = "https://private.pypi.org/simple/"
extra_index_urls = ["https://extra1.org", "https://extra2.org"]
max_concurrent_downloads = 4
verbose = true
"#;

        let pyproject: PyProjectConfig = toml::from_str(toml_content).unwrap();
        let dx_py_config = pyproject.tool.unwrap().dx_py.unwrap();

        assert_eq!(dx_py_config.index_url, Some("https://private.pypi.org/simple/".to_string()));
        assert_eq!(
            dx_py_config.extra_index_urls,
            Some(vec![
                "https://extra1.org".to_string(),
                "https://extra2.org".to_string()
            ])
        );
        assert_eq!(dx_py_config.max_concurrent_downloads, Some(4));
        assert_eq!(dx_py_config.verbose, Some(true));
    }
}
