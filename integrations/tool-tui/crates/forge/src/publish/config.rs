//! Publish configuration types and builder.
//!
//! Provides configuration options for the publish workflow, including
//! plugin metadata, versioning, and submission settings.

use std::path::PathBuf;

/// Configuration for publishing a plugin.
#[derive(Debug, Clone)]
pub struct PublishConfig {
    /// Plugin name (kebab-case, e.g., "my-awesome-plugin")
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Author email for attribution
    pub author_email: String,
    /// GitHub username for CONTRIBUTORS.md
    pub github_username: Option<String>,
    /// Path to plugin source directory
    pub source_path: PathBuf,
    /// Path to output package
    pub output_path: PathBuf,
    /// Whether to run in dry-run mode
    pub dry_run: bool,
    /// Skip validation steps
    pub skip_validation: bool,
    /// Auto-merge if CI passes
    pub auto_merge: bool,
    /// Ed25519 private key path for signing
    pub signing_key_path: Option<PathBuf>,
    /// Target repository for submission
    pub target_repo: String,
}

impl PublishConfig {
    /// Create a new PublishConfig builder.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name (kebab-case)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = PublishConfig::builder("my-plugin")
    ///     .with_version("1.0.0")
    ///     .build()?;
    /// ```
    pub fn builder(name: impl Into<String>) -> PublishConfigBuilder {
        PublishConfigBuilder::new(name)
    }
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::from("0.1.0"),
            description: String::new(),
            author_email: String::new(),
            github_username: None,
            source_path: PathBuf::from("."),
            output_path: PathBuf::from("./dist"),
            dry_run: false,
            skip_validation: false,
            auto_merge: true,
            signing_key_path: None,
            target_repo: String::from("dx-ecosystem/dx-plugins"),
        }
    }
}

/// Builder for `PublishConfig`.
#[derive(Debug)]
pub struct PublishConfigBuilder {
    config: PublishConfig,
}

impl PublishConfigBuilder {
    /// Create a new builder with the plugin name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: PublishConfig {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Set the plugin version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.config.version = version.into();
        self
    }

    /// Set the plugin description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.config.description = description.into();
        self
    }

    /// Set the author email.
    pub fn with_author(mut self, email: impl Into<String>) -> Self {
        self.config.author_email = email.into();
        self
    }

    /// Set the GitHub username.
    pub fn with_github_username(mut self, username: impl Into<String>) -> Self {
        self.config.github_username = Some(username.into());
        self
    }

    /// Set the source path.
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.source_path = path.into();
        self
    }

    /// Set the output path.
    pub fn with_output_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.output_path = path.into();
        self
    }

    /// Enable dry-run mode (no actual submission).
    pub fn dry_run(mut self) -> Self {
        self.config.dry_run = true;
        self
    }

    /// Skip validation steps.
    pub fn skip_validation(mut self) -> Self {
        self.config.skip_validation = true;
        self
    }

    /// Disable auto-merge.
    pub fn no_auto_merge(mut self) -> Self {
        self.config.auto_merge = false;
        self
    }

    /// Set the signing key path.
    pub fn with_signing_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.signing_key_path = Some(path.into());
        self
    }

    /// Set the target repository.
    pub fn with_target_repo(mut self, repo: impl Into<String>) -> Self {
        self.config.target_repo = repo.into();
        self
    }

    /// Build the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn build(self) -> Result<PublishConfig, PublishConfigError> {
        if self.config.name.is_empty() {
            return Err(PublishConfigError::MissingField("name"));
        }
        if self.config.version.is_empty() {
            return Err(PublishConfigError::MissingField("version"));
        }
        if !is_valid_semver(&self.config.version) {
            return Err(PublishConfigError::InvalidVersion(self.config.version));
        }
        if !is_valid_kebab_case(&self.config.name) {
            return Err(PublishConfigError::InvalidName(self.config.name));
        }
        Ok(self.config)
    }
}

/// Errors that can occur when building a `PublishConfig`.
#[derive(Debug, thiserror::Error)]
pub enum PublishConfigError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid version format: {0} (expected semver)")]
    InvalidVersion(String),

    #[error("Invalid plugin name: {0} (expected kebab-case)")]
    InvalidName(String),
}

/// Validate semver format (simplified).
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

/// Validate kebab-case name.
fn is_valid_kebab_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("10.20.30"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver("1.a.0"));
    }

    #[test]
    fn test_valid_kebab_case() {
        assert!(is_valid_kebab_case("my-plugin"));
        assert!(is_valid_kebab_case("plugin"));
        assert!(is_valid_kebab_case("plugin-v2"));
        assert!(!is_valid_kebab_case("-plugin"));
        assert!(!is_valid_kebab_case("plugin-"));
        assert!(!is_valid_kebab_case("my--plugin"));
        assert!(!is_valid_kebab_case("MyPlugin"));
    }

    #[test]
    fn test_builder() {
        let config = PublishConfig::builder("test-plugin")
            .with_version("1.0.0")
            .with_description("A test plugin")
            .with_author("test@example.com")
            .build()
            .unwrap();

        assert_eq!(config.name, "test-plugin");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.description, "A test plugin");
        assert_eq!(config.author_email, "test@example.com");
    }
}
