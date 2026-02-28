//! Configuration loading and parsing

use crate::utils::error::DxError;
use std::fs;
use std::path::Path;

use super::cache::{load_from_cache, save_to_cache};
use super::types::{DEFAULT_CONFIG_FILE, DxConfig};

impl DxConfig {
    /// Load configuration from the default path (dx.toml in current directory)
    pub fn load_default() -> Result<Self, DxError> {
        Self::load(Path::new(DEFAULT_CONFIG_FILE))
    }

    /// Load configuration from a specific path
    pub fn load(path: &Path) -> Result<Self, DxError> {
        if !path.exists() {
            return Err(DxError::ConfigNotFound {
                path: path.to_path_buf(),
            });
        }

        if let Some(cached) = load_from_cache(path) {
            return Ok(cached);
        }

        let config = Self::load_and_parse(path)?;
        let _ = save_to_cache(path, &config);

        Ok(config)
    }

    /// Load configuration, returning default if not found
    pub fn load_or_default() -> Self {
        Self::load_default().unwrap_or_default()
    }

    /// Load configuration with a custom path override
    pub fn load_with_override(custom_path: Option<&Path>) -> Result<Self, DxError> {
        match custom_path {
            Some(path) => Self::load(path),
            None => Self::load_default(),
        }
    }

    /// Parse TOML content and return config or detailed error
    pub(crate) fn load_and_parse(path: &Path) -> Result<Self, DxError> {
        let content = fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DxError::ConfigNotFound {
                    path: path.to_path_buf(),
                }
            } else {
                DxError::Io {
                    message: e.to_string(),
                }
            }
        })?;

        toml::from_str(&content).map_err(|e| {
            let line = e
                .span()
                .map(|s| content[..s.start].chars().filter(|&c| c == '\n').count() + 1)
                .unwrap_or(1);

            DxError::ConfigInvalid {
                path: path.to_path_buf(),
                line,
                message: e.message().to_string(),
            }
        })
    }

    /// Load global configuration from ~/.dx/config.toml
    pub(crate) fn load_global() -> Result<Self, DxError> {
        let home = home::home_dir().ok_or_else(|| DxError::Io {
            message: "Could not determine home directory".to_string(),
        })?;

        let global_path = home.join(".dx").join("config.toml");
        Self::load(&global_path)
    }
}
