//! Configuration type definitions

use serde::{Deserialize, Serialize};

/// Default config file name
pub const DEFAULT_CONFIG_FILE: &str = "dx.toml";

/// Cache file extension
pub(crate) const CACHE_EXTENSION: &str = ".cache";

/// Main DX configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DxConfig {
    pub project: ProjectConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub dev: DevConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
}

/// Project metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub description: Option<String>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildConfig {
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_true")]
    pub minify: bool,
    #[serde(default)]
    pub sourcemap: bool,
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
}

/// Development server configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DevConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub https: bool,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    #[serde(default = "default_jsx")]
    pub jsx: String,
    #[serde(default = "default_true")]
    pub typescript: bool,
}

/// Tool-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ToolsConfig {
    #[serde(default)]
    pub style: Option<StyleToolConfig>,
    #[serde(default)]
    pub media: Option<MediaToolConfig>,
    #[serde(default)]
    pub font: Option<FontToolConfig>,
    #[serde(default)]
    pub icon: Option<IconToolConfig>,
}

/// Style tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StyleToolConfig {
    pub preprocessor: Option<String>,
    #[serde(default)]
    pub modules: bool,
    #[serde(default)]
    pub postcss_plugins: Vec<String>,
}

/// Media tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MediaToolConfig {
    #[serde(default = "default_quality")]
    pub quality: u8,
    #[serde(default)]
    pub formats: Vec<String>,
}

/// Font tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FontToolConfig {
    #[serde(default)]
    pub subset: bool,
    #[serde(default)]
    pub ranges: Vec<String>,
}

/// Icon tool configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct IconToolConfig {
    #[serde(default)]
    pub sprite: bool,
    #[serde(default)]
    pub sizes: Vec<u32>,
}

/// Cached configuration with metadata
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CachedConfig {
    pub config: DxConfig,
    pub source_mtime: u64,
}

// Default value functions
pub(crate) fn default_version() -> String {
    "0.1.0".to_string()
}

pub(crate) fn default_target() -> String {
    "browser".to_string()
}

pub(crate) fn default_out_dir() -> String {
    "dist".to_string()
}

pub(crate) fn default_port() -> u16 {
    3000
}

pub(crate) fn default_jsx() -> String {
    "dx".to_string()
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_quality() -> u8 {
    85
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: default_target(),
            minify: true,
            sourcemap: false,
            out_dir: default_out_dir(),
        }
    }
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            open: false,
            https: false,
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            jsx: default_jsx(),
            typescript: true,
        }
    }
}
