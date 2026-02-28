//! Configuration serialization and validation
//!
//! Custom serialization and validation for configuration.

use super::types::DxPyConfig;

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read config file
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParseError(#[from] toml::de::Error),

    /// Failed to serialize TOML
    #[error("Failed to serialize TOML: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),

    /// Invalid configuration value
    #[error("Invalid configuration value: {field} - {message}")]
    ValidationError { field: String, message: String },
}

impl ConfigError {
    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Validate a configuration value
pub fn validate_index_url(url: &str) -> Result<(), ConfigError> {
    if url.is_empty() {
        return Err(ConfigError::validation("index_url", "URL cannot be empty"));
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ConfigError::validation(
            "index_url",
            "URL must start with http:// or https://",
        ));
    }

    Ok(())
}

/// Validate max concurrent downloads
pub fn validate_max_concurrent_downloads(max: u32) -> Result<(), ConfigError> {
    if max == 0 {
        return Err(ConfigError::validation("max_concurrent_downloads", "Must be at least 1"));
    }

    if max > 100 {
        return Err(ConfigError::validation("max_concurrent_downloads", "Must be at most 100"));
    }

    Ok(())
}

/// Validate a DxPyConfig
pub fn validate_config(config: &DxPyConfig) -> Result<(), ConfigError> {
    // Validate index_url if present
    if let Some(ref url) = config.index_url {
        validate_index_url(url)?;
    }

    // Validate extra_index_urls
    for (i, url) in config.extra_index_urls.iter().enumerate() {
        if url.is_empty() {
            return Err(ConfigError::validation(
                format!("extra_index_urls[{}]", i),
                "URL cannot be empty",
            ));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ConfigError::validation(
                format!("extra_index_urls[{}]", i),
                "URL must start with http:// or https://",
            ));
        }
    }

    // Validate max_concurrent_downloads if present
    if let Some(max) = config.max_concurrent_downloads {
        validate_max_concurrent_downloads(max)?;
    }

    // Validate Python version if present
    if let Some(ref version) = config.python_version {
        if !version.is_supported() {
            return Err(ConfigError::validation(
                "python_version",
                format!("Python version {} is not supported (requires 3.8-3.13)", version),
            ));
        }
    }

    Ok(())
}

/// Parse and validate a TOML string into DxPyConfig
pub fn parse_and_validate(toml_str: &str) -> Result<DxPyConfig, ConfigError> {
    let config: DxPyConfig = toml::from_str(toml_str)?;
    validate_config(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::PythonVersion;

    #[test]
    fn test_validate_index_url() {
        assert!(validate_index_url("https://pypi.org/simple").is_ok());
        assert!(validate_index_url("http://localhost:8080").is_ok());
        assert!(validate_index_url("").is_err());
        assert!(validate_index_url("ftp://invalid.com").is_err());
    }

    #[test]
    fn test_validate_max_concurrent_downloads() {
        assert!(validate_max_concurrent_downloads(1).is_ok());
        assert!(validate_max_concurrent_downloads(50).is_ok());
        assert!(validate_max_concurrent_downloads(100).is_ok());
        assert!(validate_max_concurrent_downloads(0).is_err());
        assert!(validate_max_concurrent_downloads(101).is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, 12, 0)),
            index_url: Some("https://pypi.org/simple".to_string()),
            extra_index_urls: vec!["https://extra.pypi.org".to_string()],
            cache_dir: None,
            max_concurrent_downloads: Some(10),
            uv_compat: None,
        };
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_index_url() {
        let config = DxPyConfig {
            index_url: Some("ftp://invalid.com".to_string()),
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError { field, .. }) = result {
            assert_eq!(field, "index_url");
        }
    }

    #[test]
    fn test_validate_config_invalid_max_downloads() {
        let config = DxPyConfig {
            max_concurrent_downloads: Some(0),
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError { field, .. }) = result {
            assert_eq!(field, "max_concurrent_downloads");
        }
    }

    #[test]
    fn test_validate_config_unsupported_python() {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(2, 7, 0)),
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        if let Err(ConfigError::ValidationError { field, .. }) = result {
            assert_eq!(field, "python_version");
        }
    }

    #[test]
    fn test_parse_and_validate_valid() {
        let toml = r#"
            index-url = "https://pypi.org/simple"
            max-concurrent-downloads = 10
        "#;
        let result = parse_and_validate(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_and_validate_invalid() {
        let toml = r#"
            index-url = "ftp://invalid.com"
        "#;
        let result = parse_and_validate(toml);
        assert!(result.is_err());
    }
}
