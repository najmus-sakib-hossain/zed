//! # Configuration System
//!
//! This module provides the configuration system for the DX WWW Framework.
//! Configuration is loaded from `dx.config.toml` and supports all framework settings.
//!
//! ## Configuration Schema
//!
//! ```toml
//! [project]
//! name = "my-app"

#![allow(missing_docs)]
//! version = "0.1.0"
//!
//! [build]
//! output_dir = ".dx/build"
//! optimization_level = "release"
//!
//! [dev]
//! port = 3000
//! hot_reload = true
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{
    DEFAULT_API_DIR, DEFAULT_CACHE_DIR, DEFAULT_COMPONENTS_DIR, DEFAULT_DEV_PORT,
    DEFAULT_OUTPUT_DIR, DEFAULT_PAGES_DIR, DEFAULT_PUBLIC_DIR, DEFAULT_STYLES_DIR,
};

// =============================================================================
// Error Types
// =============================================================================

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read configuration file
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),

    /// Failed to parse configuration file
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Configuration validation error
    #[error("Configuration validation error: {0}")]
    ValidationError(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid value
    #[error("Invalid value for '{field}': {message}")]
    InvalidValue { field: String, message: String },
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

// =============================================================================
// Main Configuration Struct
// =============================================================================

/// Root configuration for the DX WWW Framework.
///
/// This struct represents the complete configuration loaded from `dx.config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DxConfig {
    /// Project metadata
    pub project: ProjectConfig,

    /// Build configuration
    pub build: BuildConfig,

    /// Routing configuration
    pub routing: RoutingConfig,

    /// Development server configuration
    pub dev: DevConfig,

    /// Language support configuration
    pub languages: LanguageConfig,

    /// CSS compilation configuration
    pub css: CssConfig,

    /// Asset handling configuration
    pub assets: AssetConfig,

    /// Server configuration
    pub server: ServerConfig,
}

impl Default for DxConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            build: BuildConfig::default(),
            routing: RoutingConfig::default(),
            dev: DevConfig::default(),
            languages: LanguageConfig::default(),
            css: CssConfig::default(),
            assets: AssetConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl DxConfig {
    /// Load configuration from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The loaded and validated configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or validation fails
    pub fn load(path: impl AsRef<Path>) -> ConfigResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - TOML content as a string
    ///
    /// # Returns
    ///
    /// The loaded and validated configuration
    pub fn from_str(content: &str) -> ConfigResult<Self> {
        let config: Self = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration or return defaults if file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The loaded configuration or defaults
    pub fn load_or_default(path: impl AsRef<Path>) -> ConfigResult<Self> {
        let path = path.as_ref();
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration values are invalid
    pub fn validate(&self) -> ConfigResult<()> {
        // Validate project name
        if self.project.name.is_empty() {
            return Err(ConfigError::ValidationError("Project name cannot be empty".to_string()));
        }

        // Validate port range
        if self.dev.port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "dev.port".to_string(),
                message: "Port must be greater than 0".to_string(),
            });
        }

        // Validate optimization level
        self.build.optimization_level.validate()?;

        // Validate target
        self.build.target.validate()?;

        Ok(())
    }

    /// Save configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn save(&self, path: impl AsRef<Path>) -> ConfigResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the absolute path for the output directory.
    pub fn output_path(&self, project_root: &Path) -> PathBuf {
        if self.build.output_dir.is_absolute() {
            self.build.output_dir.clone()
        } else {
            project_root.join(&self.build.output_dir)
        }
    }

    /// Get the absolute path for the cache directory.
    pub fn cache_path(&self, project_root: &Path) -> PathBuf {
        if self.build.cache_dir.is_absolute() {
            self.build.cache_dir.clone()
        } else {
            project_root.join(&self.build.cache_dir)
        }
    }

    /// Get the absolute path for the pages directory.
    pub fn pages_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.routing.pages_dir)
    }

    /// Get the absolute path for the API directory.
    pub fn api_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.routing.api_dir)
    }

    /// Get the absolute path for the public directory.
    pub fn public_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.assets.public_dir)
    }
}

// =============================================================================
// Project Configuration
// =============================================================================

/// Project metadata configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Project version
    pub version: String,

    /// Project description
    pub description: Option<String>,

    /// Project authors
    pub authors: Vec<String>,

    /// Project license
    pub license: Option<String>,

    /// Project repository URL
    pub repository: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "dx-www-app".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
        }
    }
}

// =============================================================================
// Build Configuration
// =============================================================================

/// Build system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildConfig {
    /// Output directory for compiled files
    pub output_dir: PathBuf,

    /// Cache directory for incremental builds
    pub cache_dir: PathBuf,

    /// Optimization level for builds
    pub optimization_level: OptimizationLevel,

    /// Build target
    pub target: BuildTarget,

    /// Enable source maps
    pub source_maps: bool,

    /// Enable minification
    pub minify: bool,

    /// Enable tree shaking
    pub tree_shake: bool,

    /// Maximum number of parallel compilation jobs
    pub parallel_jobs: Option<usize>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(DEFAULT_OUTPUT_DIR),
            cache_dir: PathBuf::from(DEFAULT_CACHE_DIR),
            optimization_level: OptimizationLevel::Release,
            target: BuildTarget::Web,
            source_maps: true,
            minify: true,
            tree_shake: true,
            parallel_jobs: None,
        }
    }
}

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    /// Debug build - no optimizations, fast compilation
    Debug,
    /// Release build - full optimizations
    Release,
    /// Size-optimized build - optimize for smaller binary size
    Size,
}

impl OptimizationLevel {
    /// Validate the optimization level.
    pub fn validate(&self) -> ConfigResult<()> {
        // All variants are valid
        Ok(())
    }

    /// Check if this is a debug build.
    pub fn is_debug(&self) -> bool {
        matches!(self, Self::Debug)
    }

    /// Check if this is a release build.
    pub fn is_release(&self) -> bool {
        matches!(self, Self::Release | Self::Size)
    }
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Release
    }
}

/// Build target for deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildTarget {
    /// Web browser target (WASM)
    Web,
    /// Server-side rendering target
    Server,
    /// Edge computing target (Cloudflare Workers, etc.)
    Edge,
    /// Static site generation
    Static,
}

impl BuildTarget {
    /// Validate the build target.
    pub fn validate(&self) -> ConfigResult<()> {
        // All variants are valid
        Ok(())
    }

    /// Check if this target requires SSR.
    pub fn requires_ssr(&self) -> bool {
        matches!(self, Self::Server | Self::Edge)
    }

    /// Check if this target is static.
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    /// Check if this target uses WASM.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Web | Self::Edge)
    }
}

impl Default for BuildTarget {
    fn default() -> Self {
        Self::Web
    }
}

// =============================================================================
// Routing Configuration
// =============================================================================

/// Routing system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingConfig {
    /// Pages directory
    pub pages_dir: String,

    /// API routes directory
    pub api_dir: String,

    /// Components directory
    pub components_dir: String,

    /// Layouts directory
    pub layouts_dir: String,

    /// Styles directory
    pub styles_dir: String,

    /// Lib (utilities) directory
    pub lib_dir: String,

    /// Add trailing slash to routes
    pub trailing_slash: bool,

    /// Case-sensitive route matching
    pub case_sensitive: bool,

    /// Enable automatic index routes
    pub auto_index: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            pages_dir: DEFAULT_PAGES_DIR.to_string(),
            api_dir: DEFAULT_API_DIR.to_string(),
            components_dir: DEFAULT_COMPONENTS_DIR.to_string(),
            layouts_dir: "layouts".to_string(),
            styles_dir: DEFAULT_STYLES_DIR.to_string(),
            lib_dir: "lib".to_string(),
            trailing_slash: false,
            case_sensitive: false,
            auto_index: true,
        }
    }
}

// =============================================================================
// Development Configuration
// =============================================================================

/// Development server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DevConfig {
    /// Server port
    pub port: u16,

    /// Server host
    pub host: String,

    /// Enable hot reload
    pub hot_reload: bool,

    /// Open browser on start
    pub open_browser: bool,

    /// Watch additional directories
    pub watch_dirs: Vec<PathBuf>,

    /// Ignore patterns for file watching
    pub ignore_patterns: Vec<String>,

    /// WebSocket port for hot reload (defaults to port + 1)
    pub ws_port: Option<u16>,

    /// Enable HTTPS in development
    pub https: bool,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_DEV_PORT,
            host: "localhost".to_string(),
            hot_reload: true,
            open_browser: true,
            watch_dirs: Vec::new(),
            ignore_patterns: vec![
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                ".dx/**".to_string(),
                "target/**".to_string(),
            ],
            ws_port: None,
            https: false,
        }
    }
}

impl DevConfig {
    /// Get the WebSocket port for hot reload.
    pub fn websocket_port(&self) -> u16 {
        self.ws_port.unwrap_or(self.port + 1)
    }

    /// Get the full server URL.
    pub fn server_url(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };
        format!("{}://{}:{}", protocol, self.host, self.port)
    }
}

// =============================================================================
// Language Configuration
// =============================================================================

/// Multi-language support configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageConfig {
    /// Default language for scripts without explicit lang attribute
    pub default: ScriptLanguage,

    /// Enabled languages
    pub enabled: Vec<ScriptLanguage>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            default: ScriptLanguage::Rust,
            enabled: vec![
                ScriptLanguage::Rust,
                ScriptLanguage::JavaScript,
                ScriptLanguage::TypeScript,
            ],
        }
    }
}

/// Supported script languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    /// Rust language
    Rust,
    /// Python language
    Python,
    /// JavaScript language
    JavaScript,
    /// TypeScript language
    TypeScript,
    /// Go language
    Go,
}

impl ScriptLanguage {
    /// Get the file extension for this language.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Rust => "rs",
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Go => "go",
        }
    }

    /// Parse language from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "go" | "golang" => Some(Self::Go),
            _ => None,
        }
    }
}

impl Default for ScriptLanguage {
    fn default() -> Self {
        Self::Rust
    }
}

// =============================================================================
// CSS Configuration
// =============================================================================

/// CSS compilation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CssConfig {
    /// CSS compiler to use
    pub compiler: CssCompiler,

    /// Enable atomic CSS classes
    pub atomic_classes: bool,

    /// Purge unused CSS
    pub purge_unused: bool,

    /// CSS modules support
    pub modules: bool,

    /// Autoprefixer support
    pub autoprefixer: bool,

    /// CSS nesting support
    pub nesting: bool,
}

impl Default for CssConfig {
    fn default() -> Self {
        Self {
            compiler: CssCompiler::DxStyle,
            atomic_classes: true,
            purge_unused: true,
            modules: true,
            autoprefixer: true,
            nesting: true,
        }
    }
}

/// CSS compiler options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CssCompiler {
    /// dx-style atomic CSS compiler
    DxStyle,
    /// LightningCSS
    Lightning,
    /// No CSS compilation
    None,
}

impl Default for CssCompiler {
    fn default() -> Self {
        Self::DxStyle
    }
}

// =============================================================================
// Asset Configuration
// =============================================================================

/// Static asset handling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetConfig {
    /// Public directory for static assets
    pub public_dir: String,

    /// Optimize images during build
    pub optimize_images: bool,

    /// Add content hash to filenames for cache busting
    pub content_hash: bool,

    /// Maximum image width for optimization
    pub max_image_width: u32,

    /// Image quality for optimization (1-100)
    pub image_quality: u8,

    /// Generate WebP versions of images
    pub webp: bool,

    /// Generate AVIF versions of images
    pub avif: bool,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            public_dir: DEFAULT_PUBLIC_DIR.to_string(),
            optimize_images: true,
            content_hash: true,
            max_image_width: 1920,
            image_quality: 85,
            webp: true,
            avif: false,
        }
    }
}

// =============================================================================
// Server Configuration
// =============================================================================

/// Server runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Enable server-side rendering
    pub ssr: bool,

    /// API route prefix
    pub api_prefix: String,

    /// Enable CORS
    pub cors_enabled: bool,

    /// CORS allowed origins
    pub cors_origins: Vec<String>,

    /// Enable compression
    pub compression: bool,

    /// Enable request logging
    pub request_logging: bool,

    /// Request timeout in seconds
    pub timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ssr: true,
            api_prefix: "/api".to_string(),
            cors_enabled: false,
            cors_origins: Vec::new(),
            compression: true,
            request_logging: true,
            timeout: 30,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DxConfig::default();
        assert_eq!(config.project.name, "dx-www-app");
        assert_eq!(config.dev.port, 3000);
        assert!(config.dev.hot_reload);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [project]
            name = "test-app"
            version = "1.0.0"

            [build]
            output_dir = "dist"
            optimization_level = "release"

            [dev]
            port = 4000
            hot_reload = false
        "#;

        let config = DxConfig::from_str(toml).expect("Failed to parse config");
        assert_eq!(config.project.name, "test-app");
        assert_eq!(config.project.version, "1.0.0");
        assert_eq!(config.build.output_dir, PathBuf::from("dist"));
        assert_eq!(config.dev.port, 4000);
        assert!(!config.dev.hot_reload);
    }

    #[test]
    fn test_validation_empty_name() {
        let mut config = DxConfig::default();
        config.project.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_port() {
        let mut config = DxConfig::default();
        config.dev.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_script_language_extension() {
        assert_eq!(ScriptLanguage::Rust.extension(), "rs");
        assert_eq!(ScriptLanguage::Python.extension(), "py");
        assert_eq!(ScriptLanguage::JavaScript.extension(), "js");
        assert_eq!(ScriptLanguage::TypeScript.extension(), "ts");
        assert_eq!(ScriptLanguage::Go.extension(), "go");
    }

    #[test]
    fn test_dev_config_urls() {
        let config = DevConfig::default();
        assert_eq!(config.server_url(), "http://localhost:3000");
        assert_eq!(config.websocket_port(), 3001);
    }
}
