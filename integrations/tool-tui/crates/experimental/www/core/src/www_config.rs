//! DX-WWW Project Configuration Schema
//!
//! This module defines the comprehensive configuration schema for dx-www projects,
//! supporting multiple formats (.dx, .json, .toml) via dx-serializer.
//!
//! ## Configuration File Locations
//! - `dx.config` - Primary configuration file (DX format)
//! - `dx.config.json` - JSON format
//! - `dx.config.toml` - TOML format
//!
//! ## Example Configuration (DX format)
//! ```dx
//! app: {
//!   name: "my-app"
//!   version: "1.0.0"
//! }
//!
//! build: {
//!   entry: "pages"
//!   output: "dist"
//!   target: "wasm"
//! }
//!
//! features: {
//!   forms: true
//!   query: true
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Root configuration for a dx-www project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DxWwwConfig {
    /// Application metadata
    #[serde(default)]
    pub app: AppConfig,

    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,

    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Enabled features
    #[serde(default)]
    pub features: FeaturesConfig,

    /// Asset integrations
    #[serde(default)]
    pub assets: AssetsConfig,

    /// Environment overrides
    #[serde(default)]
    pub env: HashMap<String, EnvOverride>,
}

/// Application metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// Application name
    pub name: String,

    /// Application version
    #[serde(default = "default_version")]
    pub version: String,

    /// Application description
    #[serde(default)]
    pub description: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "dx-app".to_string(),
            version: default_version(),
            description: None,
        }
    }
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildConfig {
    /// Entry point directory
    #[serde(default = "default_entry")]
    pub entry: PathBuf,

    /// Output directory
    #[serde(default = "default_output")]
    pub output: PathBuf,

    /// Build target (wasm, node, edge)
    #[serde(default = "default_target")]
    pub target: String,

    /// Enable source maps
    #[serde(default)]
    pub sourcemap: bool,

    /// Minification level (0-3)
    #[serde(default = "default_minify")]
    pub minify: u8,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            output: default_output(),
            target: default_target(),
            sourcemap: false,
            minify: default_minify(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    /// Development port
    #[serde(default = "default_dev_port")]
    pub dev_port: u16,

    /// Production port
    #[serde(default = "default_prod_port")]
    pub prod_port: u16,

    /// Enable HTTPS
    #[serde(default)]
    pub https: bool,

    /// Host binding
    #[serde(default = "default_host")]
    pub host: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            dev_port: default_dev_port(),
            prod_port: default_prod_port(),
            https: false,
            host: default_host(),
        }
    }
}

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FeaturesConfig {
    /// Form handling and validation
    #[serde(default)]
    pub forms: bool,

    /// Data fetching and caching
    #[serde(default)]
    pub query: bool,

    /// Authentication and authorization
    #[serde(default)]
    pub auth: bool,

    /// Real-time synchronization
    #[serde(default)]
    pub sync: bool,

    /// Offline support and PWA
    #[serde(default)]
    pub offline: bool,

    /// Accessibility enhancements
    #[serde(default)]
    pub a11y: bool,

    /// Internationalization
    #[serde(default)]
    pub i18n: bool,
}

/// All valid feature names
pub const VALID_FEATURE_NAMES: &[&str] =
    &["forms", "query", "auth", "sync", "offline", "a11y", "i18n"];

/// Feature dependency graph - maps feature to its required dependencies
pub const FEATURE_DEPENDENCIES: &[(&str, &[&str])] = &[
    ("sync", &["query"]),   // sync requires query for data fetching
    ("offline", &["sync"]), // offline requires sync for data synchronization
    ("auth", &[]),          // auth has no dependencies
    ("forms", &[]),         // forms has no dependencies
    ("query", &[]),         // query has no dependencies
    ("a11y", &[]),          // a11y has no dependencies
    ("i18n", &[]),          // i18n has no dependencies
];

impl FeaturesConfig {
    /// Get list of enabled features
    pub fn enabled_features(&self) -> Vec<&'static str> {
        let mut features = Vec::new();
        if self.forms {
            features.push("forms");
        }
        if self.query {
            features.push("query");
        }
        if self.auth {
            features.push("auth");
        }
        if self.sync {
            features.push("sync");
        }
        if self.offline {
            features.push("offline");
        }
        if self.a11y {
            features.push("a11y");
        }
        if self.i18n {
            features.push("i18n");
        }
        features
    }

    /// Check if a feature is enabled by name
    pub fn is_enabled(&self, name: &str) -> bool {
        match name {
            "forms" => self.forms,
            "query" => self.query,
            "auth" => self.auth,
            "sync" => self.sync,
            "offline" => self.offline,
            "a11y" => self.a11y,
            "i18n" => self.i18n,
            _ => false,
        }
    }

    /// Enable a feature by name
    pub fn enable(&mut self, name: &str) -> bool {
        match name {
            "forms" => {
                self.forms = true;
                true
            }
            "query" => {
                self.query = true;
                true
            }
            "auth" => {
                self.auth = true;
                true
            }
            "sync" => {
                self.sync = true;
                true
            }
            "offline" => {
                self.offline = true;
                true
            }
            "a11y" => {
                self.a11y = true;
                true
            }
            "i18n" => {
                self.i18n = true;
                true
            }
            _ => false,
        }
    }

    /// Disable a feature by name
    pub fn disable(&mut self, name: &str) -> bool {
        match name {
            "forms" => {
                self.forms = false;
                true
            }
            "query" => {
                self.query = false;
                true
            }
            "auth" => {
                self.auth = false;
                true
            }
            "sync" => {
                self.sync = false;
                true
            }
            "offline" => {
                self.offline = false;
                true
            }
            "a11y" => {
                self.a11y = false;
                true
            }
            "i18n" => {
                self.i18n = false;
                true
            }
            _ => false,
        }
    }

    /// Validate a feature name
    pub fn is_valid_feature_name(name: &str) -> bool {
        VALID_FEATURE_NAMES.contains(&name)
    }

    /// Get all valid feature names
    pub fn valid_feature_names() -> &'static [&'static str] {
        VALID_FEATURE_NAMES
    }

    /// Get dependencies for a feature
    pub fn get_dependencies(feature: &str) -> &'static [&'static str] {
        for (name, deps) in FEATURE_DEPENDENCIES {
            if *name == feature {
                return deps;
            }
        }
        &[]
    }

    /// Validate feature dependencies and return warnings for missing dependencies
    pub fn validate_dependencies(&self) -> Vec<FeatureDependencyWarning> {
        let mut warnings = Vec::new();
        let enabled = self.enabled_features();

        for feature in &enabled {
            let deps = Self::get_dependencies(feature);
            for dep in deps {
                if !self.is_enabled(dep) {
                    warnings.push(FeatureDependencyWarning {
                        feature: feature.to_string(),
                        missing_dependency: dep.to_string(),
                        message: format!("Feature '{}' requires '{}' to be enabled", feature, dep),
                    });
                }
            }
        }

        warnings
    }

    /// Enable a feature and all its dependencies (including transitive dependencies)
    pub fn enable_with_dependencies(&mut self, name: &str) -> Result<Vec<&'static str>, String> {
        if !Self::is_valid_feature_name(name) {
            return Err(format!(
                "Unknown feature: '{}'. Valid features: {}",
                name,
                VALID_FEATURE_NAMES.join(", ")
            ));
        }

        let mut enabled = Vec::new();

        // Recursively enable dependencies
        fn enable_deps(
            features: &mut FeaturesConfig,
            feature: &str,
            enabled: &mut Vec<&'static str>,
        ) {
            let deps = FeaturesConfig::get_dependencies(feature);

            // Enable dependencies first (recursively)
            for dep in deps {
                if !features.is_enabled(dep) {
                    // Recursively enable this dependency's dependencies
                    enable_deps(features, dep, enabled);

                    // Then enable this dependency
                    features.enable(dep);
                    for valid_name in VALID_FEATURE_NAMES {
                        if *valid_name == *dep {
                            enabled.push(*valid_name);
                            break;
                        }
                    }
                }
            }
        }

        // Enable all dependencies recursively
        enable_deps(self, name, &mut enabled);

        // Enable the feature itself
        if !self.is_enabled(name) {
            self.enable(name);
            // Find the static str for the feature name
            for valid_name in VALID_FEATURE_NAMES {
                if *valid_name == name {
                    enabled.push(*valid_name);
                    break;
                }
            }
        }

        Ok(enabled)
    }
}

/// Warning for missing feature dependencies
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureDependencyWarning {
    /// The feature that has missing dependencies
    pub feature: String,
    /// The missing dependency
    pub missing_dependency: String,
    /// Human-readable warning message
    pub message: String,
}

impl std::fmt::Display for FeatureDependencyWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Asset configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AssetsConfig {
    /// Icon configuration
    #[serde(default)]
    pub icons: Option<IconsConfig>,

    /// Font configuration
    #[serde(default)]
    pub fonts: Option<FontsConfig>,

    /// Media configuration
    #[serde(default)]
    pub media: Option<MediaConfig>,
}

/// Icon configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IconsConfig {
    /// Icon sets to include
    #[serde(default)]
    pub sets: Vec<String>,

    /// Custom icon directory
    #[serde(default)]
    pub custom_dir: Option<PathBuf>,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontsConfig {
    /// Fonts to include
    #[serde(default)]
    pub families: Vec<FontFamily>,

    /// Enable subsetting
    #[serde(default = "default_true")]
    pub subset: bool,
}

/// Font family configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontFamily {
    /// Font family name
    pub name: String,

    /// Font weights to include
    #[serde(default)]
    pub weights: Vec<u16>,

    /// Is this a variable font
    #[serde(default)]
    pub variable: bool,
}

/// Media configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaConfig {
    /// Image optimization settings
    #[serde(default)]
    pub images: Option<ImageOptConfig>,

    /// Video optimization settings
    #[serde(default)]
    pub videos: Option<VideoOptConfig>,
}

/// Image optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageOptConfig {
    /// Output formats
    #[serde(default = "default_image_formats")]
    pub formats: Vec<String>,

    /// Quality (1-100)
    #[serde(default = "default_quality")]
    pub quality: u8,

    /// Generate blur placeholders
    #[serde(default = "default_true")]
    pub blur_placeholder: bool,
}

impl Default for ImageOptConfig {
    fn default() -> Self {
        Self {
            formats: default_image_formats(),
            quality: default_quality(),
            blur_placeholder: true,
        }
    }
}

/// Video optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOptConfig {
    /// Output formats
    #[serde(default = "default_video_formats")]
    pub formats: Vec<String>,
}

impl Default for VideoOptConfig {
    fn default() -> Self {
        Self {
            formats: default_video_formats(),
        }
    }
}

/// Environment-specific configuration override
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvOverride {
    /// Override values (flattened)
    #[serde(flatten)]
    pub overrides: HashMap<String, serde_json::Value>,
}

// ============================================================================
// Default Value Functions
// ============================================================================

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_entry() -> PathBuf {
    PathBuf::from("pages")
}

fn default_output() -> PathBuf {
    PathBuf::from("dist")
}

fn default_target() -> String {
    "wasm".to_string()
}

fn default_minify() -> u8 {
    2
}

fn default_dev_port() -> u16 {
    3000
}

fn default_prod_port() -> u16 {
    8080
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_true() -> bool {
    true
}

fn default_quality() -> u8 {
    85
}

fn default_image_formats() -> Vec<String> {
    vec!["webp".to_string(), "avif".to_string()]
}

fn default_video_formats() -> Vec<String> {
    vec!["mp4".to_string(), "webm".to_string()]
}

// ============================================================================
// Configuration Loading
// ============================================================================

/// Configuration file candidates in order of preference
const CONFIG_CANDIDATES: &[&str] = &[
    "dx.config",
    "dx.config.dx",
    "dx.config.json",
    "dx.config.toml",
];

/// Find the configuration file in the project root
pub fn find_config_file(root: &Path) -> Result<PathBuf> {
    for candidate in CONFIG_CANDIDATES {
        let path = root.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!(
        "No dx.config file found in {}. Expected one of: {}",
        root.display(),
        CONFIG_CANDIDATES.join(", ")
    ))
}

/// Load configuration from a file path
pub fn load_config(path: &Path) -> Result<DxWwwConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("dx");

    parse_config(&content, extension)
}

/// Load configuration from the project root
pub fn load_config_from_root(root: &Path) -> Result<DxWwwConfig> {
    let config_path = find_config_file(root)?;
    load_config(&config_path)
}

/// Parse configuration content based on format
pub fn parse_config(content: &str, format: &str) -> Result<DxWwwConfig> {
    match format.to_lowercase().as_str() {
        "json" => serde_json::from_str(content).context("Failed to parse JSON configuration"),
        "toml" => toml::from_str(content).context("Failed to parse TOML configuration"),
        "dx" | "" => parse_dx_config(content),
        _ => Err(anyhow::anyhow!("Unsupported config format: {}", format)),
    }
}

/// Parse DX format configuration
fn parse_dx_config(content: &str) -> Result<DxWwwConfig> {
    // For now, try to parse as TOML-like format
    // The DX format is similar to TOML but with some differences
    // A full implementation would use dx-serializer

    // Try TOML first as DX format is similar
    if let Ok(config) = toml::from_str::<DxWwwConfig>(content) {
        return Ok(config);
    }

    // Try JSON as fallback
    if let Ok(config) = serde_json::from_str::<DxWwwConfig>(content) {
        return Ok(config);
    }

    // Manual parsing for simple DX format
    parse_simple_dx_config(content)
}

/// Parse simple DX format configuration
fn parse_simple_dx_config(content: &str) -> Result<DxWwwConfig> {
    let mut config = DxWwwConfig::default();
    let mut current_section = String::new();
    let mut _in_block = false;
    let mut block_depth = 0;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Track block depth
        if line.contains('{') {
            block_depth += line.matches('{').count();
            _in_block = true;
        }
        if line.contains('}') {
            block_depth = block_depth.saturating_sub(line.matches('}').count());
            if block_depth == 0 {
                _in_block = false;
                current_section.clear();
            }
        }

        // Parse section headers
        if line.ends_with(':') && line.contains('{') {
            let section = line.trim_end_matches(':').trim_end_matches('{').trim();
            current_section = section.to_string();
            continue;
        }

        // Parse key-value pairs
        if let Some((key, value)) = parse_key_value(line) {
            apply_config_value(&mut config, &current_section, &key, &value)?;
        }
    }

    Ok(config)
}

/// Parse a key-value pair from a line
fn parse_key_value(line: &str) -> Option<(String, String)> {
    // Handle "key: value" format
    if let Some(colon_pos) = line.find(':') {
        let key = line[..colon_pos].trim().to_string();
        let value = line[colon_pos + 1..].trim().trim_matches('"').trim_matches('\'').to_string();

        // Skip if value contains block start
        if value.contains('{') || value.is_empty() {
            return None;
        }

        return Some((key, value));
    }
    None
}

/// Apply a configuration value to the config struct
fn apply_config_value(
    config: &mut DxWwwConfig,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    match section {
        "app" => match key {
            "name" => config.app.name = value.to_string(),
            "version" => config.app.version = value.to_string(),
            "description" => config.app.description = Some(value.to_string()),
            _ => {}
        },
        "build" => match key {
            "entry" => config.build.entry = PathBuf::from(value),
            "output" => config.build.output = PathBuf::from(value),
            "target" => config.build.target = value.to_string(),
            "sourcemap" => config.build.sourcemap = value == "true",
            "minify" => config.build.minify = value.parse().unwrap_or(2),
            _ => {}
        },
        "server" => match key {
            "dev_port" => config.server.dev_port = value.parse().unwrap_or(3000),
            "prod_port" => config.server.prod_port = value.parse().unwrap_or(8080),
            "https" => config.server.https = value == "true",
            "host" => config.server.host = value.to_string(),
            _ => {}
        },
        "features" => match key {
            "forms" => config.features.forms = value == "true",
            "query" => config.features.query = value == "true",
            "auth" => config.features.auth = value == "true",
            "sync" => config.features.sync = value == "true",
            "offline" => config.features.offline = value == "true",
            "a11y" => config.features.a11y = value == "true",
            "i18n" => config.features.i18n = value == "true",
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

// ============================================================================
// Configuration Validation
// ============================================================================

/// Configuration validation error
#[derive(Debug, Clone)]
pub struct ConfigValidationError {
    /// Field that failed validation
    pub field: String,
    /// Error message
    pub message: String,
    /// Line number (if available)
    pub line: Option<usize>,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "Line {}: {}: {}", line, self.field, self.message)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
    }
}

/// Validate a configuration
pub fn validate_config(config: &DxWwwConfig) -> Vec<ConfigValidationError> {
    let mut errors = Vec::new();

    // Validate app name
    if config.app.name.is_empty() {
        errors.push(ConfigValidationError {
            field: "app.name".to_string(),
            message: "Application name cannot be empty".to_string(),
            line: None,
        });
    }

    // Validate app name format (kebab-case)
    if !config
        .app
        .name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        errors.push(ConfigValidationError {
            field: "app.name".to_string(),
            message:
                "Application name must be kebab-case (lowercase letters, numbers, and hyphens)"
                    .to_string(),
            line: None,
        });
    }

    // Validate version format (semver-like)
    if !is_valid_version(&config.app.version) {
        errors.push(ConfigValidationError {
            field: "app.version".to_string(),
            message: "Version must be in semver format (e.g., 1.0.0)".to_string(),
            line: None,
        });
    }

    // Validate build target
    let valid_targets = ["wasm", "node", "edge"];
    if !valid_targets.contains(&config.build.target.as_str()) {
        errors.push(ConfigValidationError {
            field: "build.target".to_string(),
            message: format!(
                "Invalid target '{}'. Valid targets: {}",
                config.build.target,
                valid_targets.join(", ")
            ),
            line: None,
        });
    }

    // Validate minify level
    if config.build.minify > 3 {
        errors.push(ConfigValidationError {
            field: "build.minify".to_string(),
            message: "Minify level must be between 0 and 3".to_string(),
            line: None,
        });
    }

    // Validate ports
    if config.server.dev_port == 0 {
        errors.push(ConfigValidationError {
            field: "server.dev_port".to_string(),
            message: "Development port cannot be 0".to_string(),
            line: None,
        });
    }

    if config.server.prod_port == 0 {
        errors.push(ConfigValidationError {
            field: "server.prod_port".to_string(),
            message: "Production port cannot be 0".to_string(),
            line: None,
        });
    }

    // Validate image quality
    if let Some(ref assets) = config.assets.media {
        if let Some(ref images) = assets.images {
            if images.quality == 0 || images.quality > 100 {
                errors.push(ConfigValidationError {
                    field: "assets.media.images.quality".to_string(),
                    message: "Image quality must be between 1 and 100".to_string(),
                    line: None,
                });
            }
        }
    }

    errors
}

/// Validate a feature name
pub fn validate_feature_name(name: &str) -> Result<(), ConfigValidationError> {
    if FeaturesConfig::is_valid_feature_name(name) {
        Ok(())
    } else {
        Err(ConfigValidationError {
            field: "features".to_string(),
            message: format!(
                "Unknown feature: '{}'. Valid features: {}",
                name,
                VALID_FEATURE_NAMES.join(", ")
            ),
            line: None,
        })
    }
}

/// Validate feature dependencies and return warnings
pub fn validate_feature_dependencies(config: &DxWwwConfig) -> Vec<FeatureDependencyWarning> {
    config.features.validate_dependencies()
}

/// Check if a version string is valid (semver-like)
fn is_valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

// ============================================================================
// Configuration Serialization
// ============================================================================

/// Serialize configuration to DX format
pub fn serialize_to_dx(config: &DxWwwConfig) -> String {
    let mut output = String::new();

    output.push_str("# DX-WWW Project Configuration\n\n");

    // App section
    output.push_str("app: {\n");
    output.push_str(&format!("  name: \"{}\"\n", config.app.name));
    output.push_str(&format!("  version: \"{}\"\n", config.app.version));
    if let Some(ref desc) = config.app.description {
        output.push_str(&format!("  description: \"{}\"\n", desc));
    }
    output.push_str("}\n\n");

    // Build section
    output.push_str("build: {\n");
    output.push_str(&format!("  entry: \"{}\"\n", config.build.entry.display()));
    output.push_str(&format!("  output: \"{}\"\n", config.build.output.display()));
    output.push_str(&format!("  target: \"{}\"\n", config.build.target));
    output.push_str(&format!("  minify: {}\n", config.build.minify));
    if config.build.sourcemap {
        output.push_str("  sourcemap: true\n");
    }
    output.push_str("}\n\n");

    // Server section
    output.push_str("server: {\n");
    output.push_str(&format!("  dev_port: {}\n", config.server.dev_port));
    output.push_str(&format!("  prod_port: {}\n", config.server.prod_port));
    if config.server.https {
        output.push_str("  https: true\n");
    }
    output.push_str("}\n");

    // Features section (only if any are enabled)
    let enabled = config.features.enabled_features();
    if !enabled.is_empty() {
        output.push_str("\nfeatures: {\n");
        for feature in enabled {
            output.push_str(&format!("  {}: true\n", feature));
        }
        output.push_str("}\n");
    }

    output
}

/// Serialize configuration to JSON
pub fn serialize_to_json(config: &DxWwwConfig) -> Result<String> {
    serde_json::to_string_pretty(config).context("Failed to serialize config to JSON")
}

/// Serialize configuration to TOML
pub fn serialize_to_toml(config: &DxWwwConfig) -> Result<String> {
    toml::to_string_pretty(config).context("Failed to serialize config to TOML")
}

// ============================================================================
// Environment Configuration
// ============================================================================

/// Valid environment names
pub const VALID_ENVIRONMENTS: &[&str] = &[
    "dev",
    "development",
    "staging",
    "prod",
    "production",
    "test",
];

/// Apply environment-specific overrides to a configuration
pub fn apply_environment_overrides(config: &mut DxWwwConfig, environment: &str) -> Result<()> {
    // Clone the overrides to avoid borrow checker issues
    let overrides = config.env.get(environment).map(|e| e.overrides.clone());

    // Apply the environment overrides if they exist
    if let Some(env_overrides) = overrides {
        apply_overrides(config, &env_overrides)?;
    }

    // Also check for common aliases
    let alias = match environment {
        "dev" => Some("development"),
        "development" => Some("dev"),
        "prod" => Some("production"),
        "production" => Some("prod"),
        _ => None,
    };

    if let Some(alias_env) = alias {
        let alias_overrides = config.env.get(alias_env).map(|e| e.overrides.clone());
        if let Some(env_overrides) = alias_overrides {
            apply_overrides(config, &env_overrides)?;
        }
    }

    Ok(())
}

/// Apply override values to a configuration
fn apply_overrides(
    config: &mut DxWwwConfig,
    overrides: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    for (key, value) in overrides {
        apply_single_override(config, key, value)?;
    }
    Ok(())
}

/// Apply a single override value to the configuration
fn apply_single_override(
    config: &mut DxWwwConfig,
    key: &str,
    value: &serde_json::Value,
) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();

    match parts.as_slice() {
        // App overrides
        ["app", "name"] => {
            if let Some(s) = value.as_str() {
                config.app.name = s.to_string();
            }
        }
        ["app", "version"] => {
            if let Some(s) = value.as_str() {
                config.app.version = s.to_string();
            }
        }
        ["app", "description"] => {
            if let Some(s) = value.as_str() {
                config.app.description = Some(s.to_string());
            }
        }

        // Build overrides
        ["build", "entry"] => {
            if let Some(s) = value.as_str() {
                config.build.entry = PathBuf::from(s);
            }
        }
        ["build", "output"] => {
            if let Some(s) = value.as_str() {
                config.build.output = PathBuf::from(s);
            }
        }
        ["build", "target"] => {
            if let Some(s) = value.as_str() {
                config.build.target = s.to_string();
            }
        }
        ["build", "sourcemap"] => {
            if let Some(b) = value.as_bool() {
                config.build.sourcemap = b;
            }
        }
        ["build", "minify"] => {
            if let Some(n) = value.as_u64() {
                config.build.minify = n as u8;
            }
        }

        // Server overrides
        ["server", "dev_port"] => {
            if let Some(n) = value.as_u64() {
                config.server.dev_port = n as u16;
            }
        }
        ["server", "prod_port"] => {
            if let Some(n) = value.as_u64() {
                config.server.prod_port = n as u16;
            }
        }
        ["server", "https"] => {
            if let Some(b) = value.as_bool() {
                config.server.https = b;
            }
        }
        ["server", "host"] => {
            if let Some(s) = value.as_str() {
                config.server.host = s.to_string();
            }
        }

        // Feature overrides
        ["features", feature_name] => {
            if let Some(b) = value.as_bool() {
                if b {
                    config.features.enable(feature_name);
                } else {
                    config.features.disable(feature_name);
                }
            }
        }

        _ => {
            // Unknown key - ignore for now
        }
    }

    Ok(())
}

/// Load configuration with environment overrides applied
pub fn load_config_with_env(root: &Path, environment: &str) -> Result<DxWwwConfig> {
    let mut config = load_config_from_root(root)?;
    apply_environment_overrides(&mut config, environment)?;
    Ok(config)
}

/// Merge two configurations, with the second taking precedence
pub fn merge_configs(base: &DxWwwConfig, override_config: &DxWwwConfig) -> DxWwwConfig {
    let mut merged = base.clone();

    // Override app config if different from default
    if override_config.app.name != "dx-app" {
        merged.app.name = override_config.app.name.clone();
    }
    if override_config.app.version != "0.1.0" {
        merged.app.version = override_config.app.version.clone();
    }
    if override_config.app.description.is_some() {
        merged.app.description = override_config.app.description.clone();
    }

    // Override build config if different from default
    if override_config.build.entry.as_path() != Path::new("pages") {
        merged.build.entry = override_config.build.entry.clone();
    }
    if override_config.build.output.as_path() != Path::new("dist") {
        merged.build.output = override_config.build.output.clone();
    }
    if override_config.build.target != "wasm" {
        merged.build.target = override_config.build.target.clone();
    }
    if override_config.build.sourcemap {
        merged.build.sourcemap = true;
    }
    if override_config.build.minify != 2 {
        merged.build.minify = override_config.build.minify;
    }

    // Override server config if different from default
    if override_config.server.dev_port != 3000 {
        merged.server.dev_port = override_config.server.dev_port;
    }
    if override_config.server.prod_port != 8080 {
        merged.server.prod_port = override_config.server.prod_port;
    }
    if override_config.server.https {
        merged.server.https = true;
    }
    if override_config.server.host != "127.0.0.1" {
        merged.server.host = override_config.server.host.clone();
    }

    // Merge features (OR logic - if either has it enabled, it's enabled)
    merged.features.forms = base.features.forms || override_config.features.forms;
    merged.features.query = base.features.query || override_config.features.query;
    merged.features.auth = base.features.auth || override_config.features.auth;
    merged.features.sync = base.features.sync || override_config.features.sync;
    merged.features.offline = base.features.offline || override_config.features.offline;
    merged.features.a11y = base.features.a11y || override_config.features.a11y;
    merged.features.i18n = base.features.i18n || override_config.features.i18n;

    // Merge env overrides
    for (key, value) in &override_config.env {
        merged.env.insert(key.clone(), value.clone());
    }

    merged
}

/// Find and load workspace-level configuration
pub fn find_workspace_config(project_root: &Path) -> Option<PathBuf> {
    let mut current = project_root.parent();

    while let Some(dir) = current {
        // Check for workspace config files
        for candidate in CONFIG_CANDIDATES {
            let workspace_config = dir.join(candidate);
            if workspace_config.exists() {
                if let Ok(content) = std::fs::read_to_string(&workspace_config) {
                    if content.contains("workspace") || content.contains("projects") {
                        return Some(workspace_config);
                    }
                }
            }
        }
        current = dir.parent();
    }

    None
}

/// Load configuration with workspace inheritance
pub fn load_config_with_workspace(project_root: &Path) -> Result<DxWwwConfig> {
    let project_config = load_config_from_root(project_root)?;

    // Try to find and load workspace config
    if let Some(workspace_config_path) = find_workspace_config(project_root) {
        if let Ok(workspace_config) = load_config(&workspace_config_path) {
            // Merge workspace config as base, project config as override
            return Ok(merge_configs(&workspace_config, &project_config));
        }
    }

    Ok(project_config)
}

/// Load configuration with both workspace inheritance and environment overrides
pub fn load_full_config(project_root: &Path, environment: &str) -> Result<DxWwwConfig> {
    let mut config = load_config_with_workspace(project_root)?;
    apply_environment_overrides(&mut config, environment)?;
    Ok(config)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DxWwwConfig::default();
        assert_eq!(config.app.name, "dx-app");
        assert_eq!(config.app.version, "0.1.0");
        assert_eq!(config.build.entry, PathBuf::from("pages"));
        assert_eq!(config.build.output, PathBuf::from("dist"));
        assert_eq!(config.server.dev_port, 3000);
        assert_eq!(config.server.prod_port, 8080);
    }

    #[test]
    fn test_features_config() {
        let mut features = FeaturesConfig::default();
        assert!(!features.is_enabled("forms"));

        features.enable("forms");
        assert!(features.is_enabled("forms"));

        features.disable("forms");
        assert!(!features.is_enabled("forms"));
    }

    #[test]
    fn test_enabled_features() {
        let mut features = FeaturesConfig::default();
        features.forms = true;
        features.query = true;

        let enabled = features.enabled_features();
        assert!(enabled.contains(&"forms"));
        assert!(enabled.contains(&"query"));
        assert!(!enabled.contains(&"auth"));
    }

    #[test]
    fn test_parse_json_config() {
        let json = r#"{
            "app": {
                "name": "test-app",
                "version": "1.0.0"
            },
            "build": {
                "entry": "src",
                "output": "build"
            }
        }"#;

        let config = parse_config(json, "json").unwrap();
        assert_eq!(config.app.name, "test-app");
        assert_eq!(config.app.version, "1.0.0");
        assert_eq!(config.build.entry, PathBuf::from("src"));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml = r#"
[app]
name = "test-app"
version = "2.0.0"

[build]
entry = "pages"
output = "dist"
target = "node"
"#;

        let config = parse_config(toml, "toml").unwrap();
        assert_eq!(config.app.name, "test-app");
        assert_eq!(config.app.version, "2.0.0");
        assert_eq!(config.build.target, "node");
    }

    #[test]
    fn test_validate_config() {
        let mut config = DxWwwConfig::default();
        config.app.name = "".to_string();

        let errors = validate_config(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "app.name"));
    }

    #[test]
    fn test_validate_invalid_target() {
        let mut config = DxWwwConfig::default();
        config.build.target = "invalid".to_string();

        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.field == "build.target"));
    }

    #[test]
    fn test_serialize_to_dx() {
        let config = DxWwwConfig::default();
        let dx = serialize_to_dx(&config);

        assert!(dx.contains("app:"));
        assert!(dx.contains("build:"));
        assert!(dx.contains("server:"));
    }

    #[test]
    fn test_config_round_trip_json() {
        let config = DxWwwConfig::default();
        let json = serialize_to_json(&config).unwrap();
        let parsed = parse_config(&json, "json").unwrap();

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_config_round_trip_toml() {
        let config = DxWwwConfig::default();
        let toml = serialize_to_toml(&config).unwrap();
        let parsed = parse_config(&toml, "toml").unwrap();

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_is_valid_version() {
        assert!(is_valid_version("1.0.0"));
        assert!(is_valid_version("0.1.0"));
        assert!(is_valid_version("10.20.30"));
        assert!(is_valid_version("1.0"));
        assert!(!is_valid_version("1"));
        assert!(!is_valid_version("1.0.0.0"));
        assert!(!is_valid_version("a.b.c"));
    }

    #[test]
    fn test_valid_feature_names() {
        assert!(FeaturesConfig::is_valid_feature_name("forms"));
        assert!(FeaturesConfig::is_valid_feature_name("query"));
        assert!(FeaturesConfig::is_valid_feature_name("auth"));
        assert!(FeaturesConfig::is_valid_feature_name("sync"));
        assert!(FeaturesConfig::is_valid_feature_name("offline"));
        assert!(FeaturesConfig::is_valid_feature_name("a11y"));
        assert!(FeaturesConfig::is_valid_feature_name("i18n"));
        assert!(!FeaturesConfig::is_valid_feature_name("invalid"));
        assert!(!FeaturesConfig::is_valid_feature_name(""));
    }

    #[test]
    fn test_feature_dependencies() {
        // sync requires query
        let deps = FeaturesConfig::get_dependencies("sync");
        assert!(deps.contains(&"query"));

        // offline requires sync
        let deps = FeaturesConfig::get_dependencies("offline");
        assert!(deps.contains(&"sync"));

        // forms has no dependencies
        let deps = FeaturesConfig::get_dependencies("forms");
        assert!(deps.is_empty());
    }

    #[test]
    fn test_validate_feature_dependencies() {
        let mut features = FeaturesConfig::default();

        // Enable sync without query - should warn
        features.sync = true;
        let warnings = features.validate_dependencies();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.feature == "sync" && w.missing_dependency == "query"));

        // Enable query - warning should go away
        features.query = true;
        let warnings = features.validate_dependencies();
        assert!(warnings.iter().all(|w| w.feature != "sync"));
    }

    #[test]
    fn test_enable_with_dependencies() {
        let mut features = FeaturesConfig::default();

        // Enable offline - should also enable sync and query
        let enabled = features.enable_with_dependencies("offline").unwrap();
        assert!(features.is_enabled("offline"));
        assert!(features.is_enabled("sync"));
        assert!(features.is_enabled("query"));
        assert!(enabled.contains(&"query"));
        assert!(enabled.contains(&"sync"));
        assert!(enabled.contains(&"offline"));
    }

    #[test]
    fn test_enable_invalid_feature() {
        let mut features = FeaturesConfig::default();
        let result = features.enable_with_dependencies("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_feature_name() {
        assert!(validate_feature_name("forms").is_ok());
        assert!(validate_feature_name("invalid").is_err());
    }

    #[test]
    fn test_apply_environment_overrides() {
        let mut config = DxWwwConfig::default();

        // Add environment override
        let mut overrides = HashMap::new();
        overrides.insert("server.dev_port".to_string(), serde_json::json!(4000));
        overrides.insert("build.sourcemap".to_string(), serde_json::json!(true));
        overrides.insert("features.forms".to_string(), serde_json::json!(true));

        config.env.insert("dev".to_string(), EnvOverride { overrides });

        // Apply overrides
        apply_environment_overrides(&mut config, "dev").unwrap();

        assert_eq!(config.server.dev_port, 4000);
        assert!(config.build.sourcemap);
        assert!(config.features.forms);
    }

    #[test]
    fn test_merge_configs() {
        let base = DxWwwConfig::default();

        let mut override_config = DxWwwConfig::default();
        override_config.app.name = "my-app".to_string();
        override_config.server.dev_port = 5000;
        override_config.features.query = true;

        let merged = merge_configs(&base, &override_config);

        assert_eq!(merged.app.name, "my-app");
        assert_eq!(merged.server.dev_port, 5000);
        assert!(merged.features.query);
    }

    #[test]
    fn test_environment_alias() {
        let mut config = DxWwwConfig::default();

        // Add override for "development" alias
        let mut overrides = HashMap::new();
        overrides.insert("server.dev_port".to_string(), serde_json::json!(4000));
        config.env.insert("development".to_string(), EnvOverride { overrides });

        // Apply using "dev" alias
        apply_environment_overrides(&mut config, "dev").unwrap();

        // Should pick up the "development" override
        assert_eq!(config.server.dev_port, 4000);
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Arbitrary generators for configuration values
    fn arbitrary_app_name() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("my-app".to_string()),
            Just("test-project".to_string()),
            Just("dx-web".to_string()),
            "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string()),
        ]
    }

    fn arbitrary_version() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("0.1.0".to_string()),
            Just("1.0.0".to_string()),
            Just("2.3.4".to_string()),
            "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}".prop_map(|s| s.to_string()),
        ]
    }

    fn arbitrary_path() -> impl Strategy<Value = PathBuf> {
        prop_oneof![
            Just(PathBuf::from("pages")),
            Just(PathBuf::from("src")),
            Just(PathBuf::from("dist")),
            Just(PathBuf::from("build")),
            "[a-z][a-z0-9-]{0,10}".prop_map(PathBuf::from),
        ]
    }

    fn arbitrary_target() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("wasm".to_string()),
            Just("node".to_string()),
            Just("edge".to_string()),
        ]
    }

    fn arbitrary_port() -> impl Strategy<Value = u16> {
        prop_oneof![
            Just(3000u16),
            Just(8080u16),
            Just(5000u16),
            1024u16..65535u16,
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 4: Configuration Round-Trip (JSON)
        /// *For any* valid DxWwwConfig object, serializing to JSON and parsing back
        /// SHALL produce an equivalent configuration object.
        ///
        /// **Validates: Requirements 2.1, 2.4, 9.5**
        #[test]
        fn prop_config_round_trip_json(
            name in arbitrary_app_name(),
            version in arbitrary_version(),
            entry in arbitrary_path(),
            output in arbitrary_path(),
            target in arbitrary_target(),
            dev_port in arbitrary_port(),
            prod_port in arbitrary_port(),
            forms in any::<bool>(),
            query in any::<bool>(),
            auth in any::<bool>(),
        ) {
            let config = DxWwwConfig {
                app: AppConfig {
                    name,
                    version,
                    description: None,
                },
                build: BuildConfig {
                    entry,
                    output,
                    target,
                    sourcemap: false,
                    minify: 2,
                },
                server: ServerConfig {
                    dev_port,
                    prod_port,
                    https: false,
                    host: "127.0.0.1".to_string(),
                },
                features: FeaturesConfig {
                    forms,
                    query,
                    auth,
                    sync: false,
                    offline: false,
                    a11y: false,
                    i18n: false,
                },
                assets: AssetsConfig::default(),
                env: HashMap::new(),
            };

            // Serialize to JSON
            let json = serialize_to_json(&config).unwrap();

            // Parse back
            let parsed = parse_config(&json, "json").unwrap();

            // Verify equality
            prop_assert_eq!(config, parsed);
        }

        /// Property 4b: Configuration Round-Trip (TOML)
        /// *For any* valid DxWwwConfig object, serializing to TOML and parsing back
        /// SHALL produce an equivalent configuration object.
        ///
        /// **Validates: Requirements 2.1, 2.4, 9.5**
        #[test]
        fn prop_config_round_trip_toml(
            name in arbitrary_app_name(),
            version in arbitrary_version(),
            entry in arbitrary_path(),
            output in arbitrary_path(),
            target in arbitrary_target(),
            dev_port in arbitrary_port(),
            prod_port in arbitrary_port(),
        ) {
            let config = DxWwwConfig {
                app: AppConfig {
                    name,
                    version,
                    description: None,
                },
                build: BuildConfig {
                    entry,
                    output,
                    target,
                    sourcemap: false,
                    minify: 2,
                },
                server: ServerConfig {
                    dev_port,
                    prod_port,
                    https: false,
                    host: "127.0.0.1".to_string(),
                },
                features: FeaturesConfig::default(),
                assets: AssetsConfig::default(),
                env: HashMap::new(),
            };

            // Serialize to TOML
            let toml = serialize_to_toml(&config).unwrap();

            // Parse back
            let parsed = parse_config(&toml, "toml").unwrap();

            // Verify equality
            prop_assert_eq!(config, parsed);
        }

        /// Property 5: Configuration Schema Validation
        /// *For any* configuration input, THE DX_WWW SHALL either:
        /// - Accept it as valid and produce a DxWwwConfig, OR
        /// - Reject it with a specific error message containing the invalid field name
        ///
        /// **Validates: Requirements 2.5, 9.2**
        #[test]
        fn prop_config_validation_completeness(
            name in arbitrary_app_name(),
            version in arbitrary_version(),
            target in arbitrary_target(),
            minify in 0u8..5u8,
            dev_port in 0u16..65535u16,
        ) {
            let config = DxWwwConfig {
                app: AppConfig {
                    name: name.clone(),
                    version: version.clone(),
                    description: None,
                },
                build: BuildConfig {
                    entry: PathBuf::from("pages"),
                    output: PathBuf::from("dist"),
                    target: target.clone(),
                    sourcemap: false,
                    minify,
                },
                server: ServerConfig {
                    dev_port,
                    prod_port: 8080,
                    https: false,
                    host: "127.0.0.1".to_string(),
                },
                features: FeaturesConfig::default(),
                assets: AssetsConfig::default(),
                env: HashMap::new(),
            };

            let errors = validate_config(&config);

            // If name is empty, there should be an error for app.name
            if name.is_empty() {
                prop_assert!(errors.iter().any(|e| e.field == "app.name"));
            }

            // If target is invalid, there should be an error for build.target
            let valid_targets = ["wasm", "node", "edge"];
            if !valid_targets.contains(&target.as_str()) {
                prop_assert!(errors.iter().any(|e| e.field == "build.target"));
            }

            // If minify > 3, there should be an error
            if minify > 3 {
                prop_assert!(errors.iter().any(|e| e.field == "build.minify"));
            }

            // If dev_port is 0, there should be an error
            if dev_port == 0 {
                prop_assert!(errors.iter().any(|e| e.field == "server.dev_port"));
            }
        }

        /// Property: Feature Enable/Disable Round-Trip
        /// *For any* feature name, enabling then disabling should return to original state
        #[test]
        fn prop_feature_enable_disable_round_trip(
            feature in prop_oneof![
                Just("forms"),
                Just("query"),
                Just("auth"),
                Just("sync"),
                Just("offline"),
                Just("a11y"),
                Just("i18n"),
            ]
        ) {
            let mut features = FeaturesConfig::default();
            let original = features.is_enabled(feature);

            // Enable
            features.enable(feature);
            prop_assert!(features.is_enabled(feature));

            // Disable
            features.disable(feature);
            prop_assert!(!features.is_enabled(feature));

            // If originally enabled, re-enable
            if original {
                features.enable(feature);
            }
            prop_assert_eq!(features.is_enabled(feature), original);
        }

        /// Property 14: Environment Override Application
        /// *For any* environment-specific configuration, the overrides SHALL be correctly
        /// merged with the base configuration, with environment values taking precedence.
        ///
        /// **Validates: Requirements 9.3**
        #[test]
        fn prop_environment_override_application(
            base_port in arbitrary_port(),
            override_port in arbitrary_port(),
            base_sourcemap in any::<bool>(),
            override_sourcemap in any::<bool>(),
        ) {
            let mut config = DxWwwConfig::default();
            config.server.dev_port = base_port;
            config.build.sourcemap = base_sourcemap;

            // Add environment override
            let mut overrides = HashMap::new();
            overrides.insert("server.dev_port".to_string(), serde_json::json!(override_port));
            overrides.insert("build.sourcemap".to_string(), serde_json::json!(override_sourcemap));
            config.env.insert("dev".to_string(), EnvOverride { overrides });

            // Apply overrides
            apply_environment_overrides(&mut config, "dev").unwrap();

            // Environment values should take precedence
            prop_assert_eq!(config.server.dev_port, override_port);
            prop_assert_eq!(config.build.sourcemap, override_sourcemap);
        }

        /// Property: Config merge is associative for features
        /// *For any* two configs, merging should combine features with OR logic
        #[test]
        fn prop_config_merge_features_or(
            base_forms in any::<bool>(),
            base_query in any::<bool>(),
            override_forms in any::<bool>(),
            override_query in any::<bool>(),
        ) {
            let mut base = DxWwwConfig::default();
            base.features.forms = base_forms;
            base.features.query = base_query;

            let mut override_config = DxWwwConfig::default();
            override_config.features.forms = override_forms;
            override_config.features.query = override_query;

            let merged = merge_configs(&base, &override_config);

            // Features should be OR'd together
            prop_assert_eq!(merged.features.forms, base_forms || override_forms);
            prop_assert_eq!(merged.features.query, base_query || override_query);
        }
    }
}
