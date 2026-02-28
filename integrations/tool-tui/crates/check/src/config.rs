//! Dx Check Configuration
//!
//! Zero-config by default with full customization support.
//! Supports dx.toml, biome.json, and environment variable substitution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("Invalid glob pattern: {pattern} - {message}")]
    InvalidGlobPattern { pattern: String, message: String },

    #[error("Unknown rule: {0}")]
    UnknownRule(String),

    #[error("Invalid severity: {0}")]
    InvalidSeverity(String),

    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
}

/// Validation result for configuration
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

/// Main configuration for dx-check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CheckerConfig {
    /// Whether checking is enabled
    pub enabled: bool,

    /// Root directory for checking
    pub root: PathBuf,

    /// Files/patterns to include
    pub include: Vec<String>,

    /// Files/patterns to exclude
    pub exclude: Vec<String>,

    /// Rule configurations
    pub rules: RuleConfigs,

    /// Formatter settings
    pub format: FormatConfig,

    /// Ignore configuration
    pub ignore: IgnoreConfig,

    /// Cache settings
    pub cache: CacheConfig,

    /// Parallel execution settings
    pub parallel: ParallelConfig,

    /// Score threshold configuration for CI/CD
    pub thresholds: Option<ThresholdConfig>,

    /// Architecture boundary rules
    pub architecture: Option<ArchitectureConfig>,

    /// Project-specific overrides
    pub overrides: Vec<OverrideConfig>,
}

impl Default for CheckerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            root: PathBuf::from("."),
            include: vec![
                "**/*.js".into(),
                "**/*.jsx".into(),
                "**/*.ts".into(),
                "**/*.tsx".into(),
                "**/*.mjs".into(),
                "**/*.cjs".into(),
            ],
            exclude: vec![
                "**/node_modules/**".into(),
                "**/dist/**".into(),
                "**/build/**".into(),
                "**/.git/**".into(),
                "**/coverage/**".into(),
            ],
            rules: RuleConfigs::default(),
            format: FormatConfig::default(),
            ignore: IgnoreConfig::default(),
            cache: CacheConfig::default(),
            parallel: ParallelConfig::default(),
            thresholds: None,
            architecture: None,
            overrides: Vec::new(),
        }
    }
}

impl CheckerConfig {
    /// Create config from dx.toml if present, otherwise use defaults
    #[must_use]
    pub fn auto_detect(root: &Path) -> Self {
        Self::from_dx_toml(root).unwrap_or_else(|_| {
            Self::from_biome(root).unwrap_or_else(|_| {
                let mut config = Self::default();
                config.root = root.to_path_buf();
                config
            })
        })
    }

    /// Load configuration from dx.toml
    pub fn from_dx_toml(root: &Path) -> Result<Self, ConfigError> {
        let config_path = root.join("dx.toml");
        if !config_path.exists() {
            return Err(ConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "dx.toml not found",
            )));
        }

        let content = std::fs::read_to_string(&config_path)?;
        let content = substitute_env_vars(&content)?;

        // Parse the full TOML and extract [check] section
        let full_config: toml::Value = toml::from_str(&content)?;

        if let Some(check_section) = full_config.get("check") {
            let config: CheckerConfig = check_section.clone().try_into()?;
            let mut config = config;
            config.root = root.to_path_buf();
            Ok(config)
        } else {
            // Try parsing as direct CheckerConfig (for standalone dx-check.toml)
            let mut config: CheckerConfig = toml::from_str(&content)?;
            config.root = root.to_path_buf();
            Ok(config)
        }
    }

    /// Load configuration from biome.json for migration compatibility
    pub fn from_biome(root: &Path) -> Result<Self, ConfigError> {
        let biome_path = root.join("biome.json");
        if !biome_path.exists() {
            return Err(ConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "biome.json not found",
            )));
        }

        let content = std::fs::read_to_string(&biome_path)?;
        Self::from_biome_json(&content, root)
    }

    /// Parse Biome configuration JSON
    pub fn from_biome_json(content: &str, root: &Path) -> Result<Self, ConfigError> {
        let biome: serde_json::Value = serde_json::from_str(content)?;
        let mut config = Self::default();
        config.root = root.to_path_buf();

        // Convert Biome linter rules
        if let Some(linter) = biome.get("linter") {
            if let Some(enabled) = linter.get("enabled").and_then(serde_json::Value::as_bool) {
                config.enabled = enabled;
            }
            if let Some(rules) = linter.get("rules") {
                config.rules = RuleConfigs::from_biome(rules);
            }
            if let Some(ignore) = linter.get("ignore").and_then(|v| v.as_array()) {
                config.ignore.patterns =
                    ignore.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
        }

        // Convert Biome formatter settings
        if let Some(formatter) = biome.get("formatter") {
            if let Some(indent_style) = formatter.get("indentStyle").and_then(|v| v.as_str()) {
                config.format.use_tabs = indent_style == "tab";
            }
            if let Some(indent_width) =
                formatter.get("indentWidth").and_then(serde_json::Value::as_u64)
            {
                config.format.indent_width = indent_width as u8;
            }
            if let Some(line_width) = formatter.get("lineWidth").and_then(serde_json::Value::as_u64)
            {
                config.format.line_width = line_width as u16;
            }
        }

        // Convert JavaScript-specific settings
        if let Some(js) = biome.get("javascript")
            && let Some(formatter) = js.get("formatter")
        {
            if let Some(quote_style) = formatter.get("quoteStyle").and_then(|v| v.as_str()) {
                config.format.quote_style = match quote_style {
                    "single" => QuoteStyle::Single,
                    _ => QuoteStyle::Double,
                };
            }
            if let Some(semicolons) = formatter.get("semicolons").and_then(|v| v.as_str()) {
                config.format.semicolons = match semicolons {
                    "asNeeded" => Semicolons::AsNeeded,
                    _ => Semicolons::Always,
                };
            }
        }

        // Convert files configuration
        if let Some(files) = biome.get("files") {
            if let Some(ignore) = files.get("ignore").and_then(|v| v.as_array()) {
                let patterns: Vec<String> =
                    ignore.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                config.ignore.patterns.extend(patterns);
            }
            if let Some(include) = files.get("include").and_then(|v| v.as_array()) {
                config.include =
                    include.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
        }

        Ok(config)
    }

    /// Validate configuration against schema
    pub fn validate(&self) -> Result<ValidationResult, ConfigError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate include patterns
        for (i, pattern) in self.include.iter().enumerate() {
            if let Err(e) = glob::Pattern::new(pattern) {
                errors.push(ValidationError {
                    field: format!("include[{i}]"),
                    message: format!("Invalid glob pattern: {e}"),
                });
            }
        }

        // Validate exclude patterns
        for (i, pattern) in self.exclude.iter().enumerate() {
            if let Err(e) = glob::Pattern::new(pattern) {
                errors.push(ValidationError {
                    field: format!("exclude[{i}]"),
                    message: format!("Invalid glob pattern: {e}"),
                });
            }
        }

        // Validate ignore patterns
        for (i, pattern) in self.ignore.patterns.iter().enumerate() {
            if let Err(e) = glob::Pattern::new(pattern) {
                errors.push(ValidationError {
                    field: format!("ignore.patterns[{i}]"),
                    message: format!("Invalid glob pattern: {e}"),
                });
            }
        }

        // Validate format settings
        if self.format.indent_width == 0 || self.format.indent_width > 16 {
            errors.push(ValidationError {
                field: "format.indent_width".into(),
                message: "indent_width must be between 1 and 16".into(),
            });
        }

        if self.format.line_width < 40 || self.format.line_width > 400 {
            warnings.push(ValidationWarning {
                field: "format.line_width".into(),
                message: "line_width outside recommended range (40-400)".into(),
            });
        }

        // Validate override patterns
        for (i, override_config) in self.overrides.iter().enumerate() {
            for (j, pattern) in override_config.files.iter().enumerate() {
                if let Err(e) = glob::Pattern::new(pattern) {
                    errors.push(ValidationError {
                        field: format!("overrides[{i}].files[{j}]"),
                        message: format!("Invalid glob pattern: {e}"),
                    });
                }
            }
        }

        // Validate cache settings
        if self.cache.max_size == 0 {
            warnings.push(ValidationWarning {
                field: "cache.max_size".into(),
                message: "Cache max_size is 0, caching will be ineffective".into(),
            });
        }

        Ok(ValidationResult { errors, warnings })
    }

    /// Merge with CLI overrides
    #[must_use]
    pub fn with_cli_overrides(mut self, cli: &CliOverrides) -> Self {
        if let Some(ref fix) = cli.fix {
            self.rules.auto_fix = *fix;
        }
        if let Some(threads) = cli.threads {
            self.parallel.threads = threads;
        }
        self
    }

    /// Get effective rule config for a file path, applying overrides
    #[must_use]
    pub fn get_rule_config_for_file(&self, file_path: &Path) -> RuleConfigs {
        let mut config = self.rules.clone();

        for override_config in &self.overrides {
            if override_config.matches(file_path) {
                config.merge(&override_config.rules);
            }
        }

        config
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

/// Ignore configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct IgnoreConfig {
    /// Glob patterns for files to ignore
    pub patterns: Vec<String>,

    /// Whether to use .gitignore
    pub use_gitignore: bool,

    /// Whether to use .dxignore
    pub use_dxignore: bool,
}

/// Rule severity and configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RuleConfigs {
    /// Enable recommended rules
    pub recommended: bool,

    /// Enable auto-fix
    pub auto_fix: bool,

    /// Individual rule settings: `rule_id` -> severity/options
    #[serde(flatten)]
    pub rules: HashMap<String, RuleConfig>,
}

impl Default for RuleConfigs {
    fn default() -> Self {
        Self {
            recommended: true,
            auto_fix: false,
            rules: HashMap::new(),
        }
    }
}

impl RuleConfigs {
    /// Parse from Biome rules format
    pub fn from_biome(value: &serde_json::Value) -> Self {
        let mut configs = Self::default();

        if let Some(obj) = value.as_object() {
            // Handle "recommended" at top level
            if let Some(recommended) = obj.get("recommended").and_then(serde_json::Value::as_bool) {
                configs.recommended = recommended;
            }

            // Parse rule categories (correctness, style, etc.)
            for (category, rules) in obj {
                if category == "recommended" {
                    continue;
                }

                if let Some(rules_obj) = rules.as_object() {
                    for (rule, config) in rules_obj {
                        let rule_id = format!("{category}/{rule}");

                        let rule_config = match config {
                            serde_json::Value::String(s) => {
                                RuleConfig::Severity(RuleSeverity::from_str(s))
                            }
                            serde_json::Value::Object(obj) => {
                                let severity = obj
                                    .get("level")
                                    .and_then(|v| v.as_str())
                                    .map_or(RuleSeverity::Warn, RuleSeverity::from_str);

                                let options = obj
                                    .get("options")
                                    .and_then(|v| v.as_object())
                                    .map(|o| {
                                        o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                    })
                                    .unwrap_or_default();

                                RuleConfig::Full { severity, options }
                            }
                            _ => continue,
                        };

                        configs.rules.insert(rule_id, rule_config);
                    }
                }
            }
        }

        configs
    }

    /// Merge another rule config into this one (for overrides)
    pub fn merge(&mut self, other: &HashMap<String, RuleConfig>) {
        for (rule_id, config) in other {
            self.rules.insert(rule_id.clone(), config.clone());
        }
    }

    /// Get severity for a specific rule
    #[must_use]
    pub fn get_severity(&self, rule_id: &str) -> RuleSeverity {
        self.rules.get(rule_id).map_or(RuleSeverity::Warn, RuleConfig::severity)
    }

    /// Check if a rule is enabled
    #[must_use]
    pub fn is_enabled(&self, rule_id: &str) -> bool {
        self.rules
            .get(rule_id)
            .map_or(self.recommended, |c| c.severity() != RuleSeverity::Off)
    }
}

/// Individual rule configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RuleConfig {
    /// Simple severity string
    Severity(RuleSeverity),
    /// Full configuration with options
    Full {
        severity: RuleSeverity,
        #[serde(default)]
        options: HashMap<String, serde_json::Value>,
    },
}

impl RuleConfig {
    #[must_use]
    pub fn severity(&self) -> RuleSeverity {
        match self {
            RuleConfig::Severity(s) => *s,
            RuleConfig::Full { severity, .. } => *severity,
        }
    }

    #[must_use]
    pub fn options(&self) -> &HashMap<String, serde_json::Value> {
        static EMPTY: std::sync::LazyLock<HashMap<String, serde_json::Value>> =
            std::sync::LazyLock::new(HashMap::new);
        match self {
            RuleConfig::Severity(_) => &EMPTY,
            RuleConfig::Full { options, .. } => options,
        }
    }
}

/// Rule severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Off,
    #[default]
    Warn,
    Error,
}

impl RuleSeverity {
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "off" | "0" | "none" => Self::Off,
            "warn" | "warning" | "1" => Self::Warn,
            "error" | "deny" | "2" => Self::Error,
            _ => Self::Warn,
        }
    }
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleSeverity::Off => write!(f, "off"),
            RuleSeverity::Warn => write!(f, "warn"),
            RuleSeverity::Error => write!(f, "error"),
        }
    }
}

/// Formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FormatConfig {
    /// Use tabs instead of spaces
    pub use_tabs: bool,

    /// Indentation width
    pub indent_width: u8,

    /// Line width before wrapping
    pub line_width: u16,

    /// Quote style for strings
    pub quote_style: QuoteStyle,

    /// Semicolons at end of statements
    pub semicolons: Semicolons,

    /// Trailing commas in multi-line
    pub trailing_comma: TrailingComma,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            use_tabs: false,
            indent_width: 2,
            line_width: 80,
            quote_style: QuoteStyle::Double,
            semicolons: Semicolons::Always,
            trailing_comma: TrailingComma::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QuoteStyle {
    Single,
    #[default]
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Semicolons {
    #[default]
    Always,
    AsNeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrailingComma {
    #[default]
    All,
    Es5,
    None,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CacheConfig {
    /// Enable AST caching
    pub enabled: bool,

    /// Cache directory (default: .dx/check)
    pub directory: PathBuf,

    /// Maximum cache size in bytes
    pub max_size: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: PathBuf::from(".dx/check"),
            max_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Parallel execution configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ParallelConfig {
    /// Number of worker threads (0 = auto-detect)
    pub threads: usize,

    /// Enable work stealing
    pub work_stealing: bool,

    /// Batch size for file processing
    pub batch_size: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            threads: 0, // Auto-detect
            work_stealing: true,
            batch_size: 100,
        }
    }
}

/// Architecture boundary enforcement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchitectureConfig {
    /// Defined layers
    pub layers: Vec<String>,

    /// Layer rules
    pub rules: Vec<LayerRule>,

    /// Glob pattern to layer mapping
    pub mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayerRule {
    pub from: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

/// Score threshold configuration for CI/CD
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[derive(Default)]
pub struct ThresholdConfig {
    /// Minimum total score (0-500)
    pub min_total_score: Option<u16>,

    /// Minimum score per category (0-100)
    pub min_formatting: Option<u16>,
    pub min_linting: Option<u16>,
    pub min_security: Option<u16>,
    pub min_design_patterns: Option<u16>,
    pub min_structure_docs: Option<u16>,
}

impl ThresholdConfig {
    /// Create a `ThresholdChecker` from this configuration
    #[must_use]
    pub fn to_checker(&self) -> crate::scoring_impl::ThresholdChecker {
        use crate::scoring_impl::{Category, ThresholdChecker};

        let mut checker = ThresholdChecker::new();

        if let Some(min_total) = self.min_total_score {
            checker = checker.with_total_threshold(min_total);
        }

        if let Some(min) = self.min_formatting {
            checker = checker.with_category_threshold(Category::Formatting, min);
        }

        if let Some(min) = self.min_linting {
            checker = checker.with_category_threshold(Category::Linting, min);
        }

        if let Some(min) = self.min_security {
            checker = checker.with_category_threshold(Category::Security, min);
        }

        if let Some(min) = self.min_design_patterns {
            checker = checker.with_category_threshold(Category::DesignPatterns, min);
        }

        if let Some(min) = self.min_structure_docs {
            checker = checker.with_category_threshold(Category::StructureAndDocs, min);
        }

        checker
    }
}

/// Override configuration for specific paths
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverrideConfig {
    /// Glob patterns for files to override
    pub files: Vec<String>,

    /// Rule overrides for matched files
    pub rules: HashMap<String, RuleConfig>,
}

impl OverrideConfig {
    /// Check if a file path matches any of the override patterns
    #[must_use]
    pub fn matches(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        self.files.iter().any(|pattern| {
            glob::Pattern::new(pattern).map(|p| p.matches(&path_str)).unwrap_or(false)
        })
    }
}

/// CLI overrides
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub fix: Option<bool>,
    pub threads: Option<usize>,
    pub format: Option<bool>,
}

/// Substitute environment variables in a string
/// Supports both $VAR and ${VAR} syntax
pub fn substitute_env_vars(content: &str) -> Result<String, ConfigError> {
    let mut result = content.to_string();

    // Match ${VAR} pattern
    let re_braced =
        regex_automata::meta::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("Invalid regex");

    // Process ${VAR} patterns
    let mut offset = 0i64;
    for captures in re_braced.captures_iter(content) {
        let full_match = captures.get_group(0).unwrap();
        let var_name_match = captures.get_group(1).unwrap();
        let var_name = &content[var_name_match.start..var_name_match.end];

        let replacement = env::var(var_name).unwrap_or_default();

        let start = (full_match.start as i64 + offset) as usize;
        let end = (full_match.end as i64 + offset) as usize;

        result.replace_range(start..end, &replacement);
        offset += replacement.len() as i64 - (full_match.end - full_match.start) as i64;
    }

    // Match $VAR pattern (not followed by {)
    let re_simple =
        regex_automata::meta::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").expect("Invalid regex");

    let content_after_braced = result.clone();
    let mut offset = 0i64;

    for captures in re_simple.captures_iter(&content_after_braced) {
        let full_match = captures.get_group(0).unwrap();
        let var_name_match = captures.get_group(1).unwrap();

        // Skip if this is part of ${VAR} (already processed)
        if full_match.start > 0
            && content_after_braced.as_bytes().get(full_match.start - 1) == Some(&b'{')
        {
            continue;
        }

        let var_name = &content_after_braced[var_name_match.start..var_name_match.end];
        let replacement = env::var(var_name).unwrap_or_default();

        let start = (full_match.start as i64 + offset) as usize;
        let end = (full_match.end as i64 + offset) as usize;

        result.replace_range(start..end, &replacement);
        offset += replacement.len() as i64 - (full_match.end - full_match.start) as i64;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CheckerConfig::default();
        assert!(config.enabled);
        assert!(config.rules.recommended);
        assert!(!config.rules.auto_fix);
        assert_eq!(config.format.indent_width, 2);
    }

    #[test]
    fn test_rule_severity_from_str() {
        assert_eq!(RuleSeverity::from_str("off"), RuleSeverity::Off);
        assert_eq!(RuleSeverity::from_str("warn"), RuleSeverity::Warn);
        assert_eq!(RuleSeverity::from_str("error"), RuleSeverity::Error);
        assert_eq!(RuleSeverity::from_str("0"), RuleSeverity::Off);
        assert_eq!(RuleSeverity::from_str("1"), RuleSeverity::Warn);
        assert_eq!(RuleSeverity::from_str("2"), RuleSeverity::Error);
    }

    #[test]
    fn test_config_validation() {
        let config = CheckerConfig::default();
        let result = config.validate().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let mut config = CheckerConfig::default();
        config.include.push("[invalid".into());
        let result = config.validate().unwrap();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_env_var_substitution() {
        // SAFETY: Setting test environment variable in isolated test
        unsafe {
            env::set_var("TEST_VAR", "test_value");
        }
        let result = substitute_env_vars("path/$TEST_VAR/file").unwrap();
        assert_eq!(result, "path/test_value/file");

        let result = substitute_env_vars("path/${TEST_VAR}/file").unwrap();
        assert_eq!(result, "path/test_value/file");
        // SAFETY: Cleaning up test environment variable
        unsafe {
            env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_override_matches() {
        let override_config = OverrideConfig {
            files: vec!["**/*.test.ts".into()],
            rules: HashMap::new(),
        };

        assert!(override_config.matches(Path::new("src/foo.test.ts")));
        assert!(!override_config.matches(Path::new("src/foo.ts")));
    }

    #[test]
    fn test_config_round_trip() {
        let config = CheckerConfig::default();
        let toml_str = config.to_toml().unwrap();
        let parsed: CheckerConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.enabled, parsed.enabled);
        assert_eq!(config.format.indent_width, parsed.format.indent_width);
    }
}
