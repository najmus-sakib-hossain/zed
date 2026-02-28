//! Rebuild configuration for the dx-style pipeline
//!
//! This module provides a typed configuration struct for the rebuild pipeline,
//! replacing environment variable passing with explicit configuration.

/// Configuration for the rebuild pipeline
#[derive(Debug, Clone)]
pub struct RebuildConfig {
    /// Force full rebuild even if HTML unchanged
    pub force_full: bool,
    /// Force CSS formatting
    pub force_format: bool,
    /// Enable debug logging
    pub debug: bool,
    /// Suppress output logging
    pub silent: bool,
    /// Disable incremental parsing
    pub disable_incremental: bool,
    /// Group rename similarity threshold (0.0 - 1.0)
    pub group_rename_threshold: f64,
    /// Enable aggressive group rewriting
    pub aggressive_rewrite: bool,
    /// Utility overlap threshold for group rewriting
    pub utility_overlap_threshold: f64,
    /// Cache directory path (passed explicitly instead of via env var)
    pub cache_dir: String,
    /// Style binary path (passed explicitly instead of via env var)
    pub style_bin_path: String,
}

impl Default for RebuildConfig {
    fn default() -> Self {
        Self {
            force_full: false,
            force_format: false,
            debug: false,
            silent: false,
            disable_incremental: false,
            group_rename_threshold: 0.6,
            aggressive_rewrite: false,
            utility_overlap_threshold: 0.5,
            cache_dir: ".dx/cache".to_string(),
            style_bin_path: ".dx/style/style.bin".to_string(),
        }
    }
}

impl RebuildConfig {
    /// Create a new builder for RebuildConfig
    pub fn builder() -> RebuildConfigBuilder {
        RebuildConfigBuilder::default()
    }

    /// Create RebuildConfig from environment variables (CLI entry point only)
    ///
    /// This method should only be called at the CLI entry point to convert
    /// environment variables into typed configuration. Internal code should
    /// receive RebuildConfig as a parameter.
    pub fn from_env() -> Self {
        Self {
            force_full: std::env::var("DX_FORCE_FULL").ok().as_deref() == Some("1"),
            force_format: std::env::var("DX_FORCE_FORMAT").ok().as_deref() == Some("1"),
            debug: std::env::var("DX_DEBUG").ok().as_deref() == Some("1"),
            silent: std::env::var("DX_SILENT_FORMAT").ok().as_deref() == Some("1"),
            disable_incremental: std::env::var("DX_DISABLE_INCREMENTAL").ok().as_deref()
                == Some("1"),
            group_rename_threshold: std::env::var("DX_GROUP_RENAME_SIMILARITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.6),
            aggressive_rewrite: std::env::var("DX_GROUP_AGGRESSIVE_REWRITE").ok().as_deref()
                == Some("1"),
            utility_overlap_threshold: std::env::var("DX_GROUP_REWRITE_UTILITY_OVERLAP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.5),
            // These will be set by the caller after loading Config
            cache_dir: ".dx/cache".to_string(),
            style_bin_path: ".dx/style/style.bin".to_string(),
        }
    }

    /// Create RebuildConfig from environment variables with explicit paths
    ///
    /// This is the preferred method for CLI entry points as it allows
    /// passing paths explicitly rather than through environment variables.
    pub fn from_env_with_paths(cache_dir: String, style_bin_path: String) -> Self {
        let mut config = Self::from_env();
        config.cache_dir = cache_dir;
        config.style_bin_path = style_bin_path;
        config
    }
}

/// Builder for RebuildConfig with fluent API
#[derive(Debug, Default)]
pub struct RebuildConfigBuilder {
    config: RebuildConfig,
}

#[allow(dead_code)]
impl RebuildConfigBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set force full rebuild
    pub fn force_full(mut self, value: bool) -> Self {
        self.config.force_full = value;
        self
    }

    /// Set force CSS formatting
    pub fn force_format(mut self, value: bool) -> Self {
        self.config.force_format = value;
        self
    }

    /// Set debug logging
    pub fn debug(mut self, value: bool) -> Self {
        self.config.debug = value;
        self
    }

    /// Set silent mode (suppress output logging)
    pub fn silent(mut self, value: bool) -> Self {
        self.config.silent = value;
        self
    }

    /// Set disable incremental parsing
    pub fn disable_incremental(mut self, value: bool) -> Self {
        self.config.disable_incremental = value;
        self
    }

    /// Set group rename similarity threshold (0.0 - 1.0)
    pub fn group_rename_threshold(mut self, value: f64) -> Self {
        self.config.group_rename_threshold = value.clamp(0.0, 1.0);
        self
    }

    /// Set aggressive group rewriting
    pub fn aggressive_rewrite(mut self, value: bool) -> Self {
        self.config.aggressive_rewrite = value;
        self
    }

    /// Set utility overlap threshold for group rewriting (0.0 - 1.0)
    pub fn utility_overlap_threshold(mut self, value: f64) -> Self {
        self.config.utility_overlap_threshold = value.clamp(0.0, 1.0);
        self
    }

    /// Set cache directory path
    pub fn cache_dir(mut self, value: impl Into<String>) -> Self {
        self.config.cache_dir = value.into();
        self
    }

    /// Set style binary path
    pub fn style_bin_path(mut self, value: impl Into<String>) -> Self {
        self.config.style_bin_path = value.into();
        self
    }

    /// Build the RebuildConfig
    pub fn build(self) -> RebuildConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RebuildConfig::default();
        assert!(!config.force_full);
        assert!(!config.force_format);
        assert!(!config.debug);
        assert!(!config.silent);
        assert!(!config.disable_incremental);
        assert!((config.group_rename_threshold - 0.6).abs() < f64::EPSILON);
        assert!(!config.aggressive_rewrite);
        assert!((config.utility_overlap_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_builder_pattern() {
        let config = RebuildConfig::builder()
            .force_full(true)
            .debug(true)
            .group_rename_threshold(0.8)
            .build();

        assert!(config.force_full);
        assert!(config.debug);
        assert!((config.group_rename_threshold - 0.8).abs() < f64::EPSILON);
        // Other fields should be default
        assert!(!config.force_format);
        assert!(!config.silent);
    }

    #[test]
    fn test_threshold_clamping() {
        let config = RebuildConfig::builder()
            .group_rename_threshold(1.5) // Should be clamped to 1.0
            .utility_overlap_threshold(-0.5) // Should be clamped to 0.0
            .build();

        assert!((config.group_rename_threshold - 1.0).abs() < f64::EPSILON);
        assert!(config.utility_overlap_threshold.abs() < f64::EPSILON);
    }
}
