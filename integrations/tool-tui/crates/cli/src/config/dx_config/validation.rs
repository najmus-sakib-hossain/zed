//! Configuration validation logic

use crate::utils::error::DxError;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::DxConfig;

impl DxConfig {
    /// Load configuration with field validation
    pub fn load_validated(path: &Path) -> Result<(Self, Vec<String>), DxError> {
        let config = Self::load(path)?;
        config.validate()?;

        let content = fs::read_to_string(path).map_err(|e| DxError::Io {
            message: e.to_string(),
        })?;
        let unknown_fields = Self::check_unknown_fields(&content);

        Ok((config, unknown_fields))
    }

    /// Validate configuration fields
    pub fn validate(&self) -> Result<(), DxError> {
        if self.project.name.trim().is_empty() {
            return Err(DxError::ConfigInvalid {
                path: PathBuf::from("dx.toml"),
                line: 0,
                message: "project.name cannot be empty".to_string(),
            });
        }

        if self.dev.port == 0 {
            return Err(DxError::ConfigInvalid {
                path: PathBuf::from("dx.toml"),
                line: 0,
                message: "dev.port cannot be 0".to_string(),
            });
        }

        if let Some(ref media) = self.tools.media
            && (media.quality == 0 || media.quality > 100)
        {
            return Err(DxError::ConfigInvalid {
                path: PathBuf::from("dx.toml"),
                line: 0,
                message: format!(
                    "tools.media.quality must be between 1 and 100, got {}",
                    media.quality
                ),
            });
        }

        Ok(())
    }

    /// Check for unknown fields in the configuration
    pub fn check_unknown_fields(content: &str) -> Vec<String> {
        let known_fields: HashSet<&str> = [
            "project",
            "project.name",
            "project.version",
            "project.description",
            "build",
            "build.target",
            "build.minify",
            "build.sourcemap",
            "build.out_dir",
            "dev",
            "dev.port",
            "dev.open",
            "dev.https",
            "runtime",
            "runtime.jsx",
            "runtime.typescript",
            "tools",
            "tools.style",
            "tools.media",
            "tools.font",
            "tools.icon",
            "tools.style.preprocessor",
            "tools.style.modules",
            "tools.style.postcss_plugins",
            "tools.media.quality",
            "tools.media.formats",
            "tools.font.subset",
            "tools.font.ranges",
            "tools.icon.sprite",
            "tools.icon.sizes",
        ]
        .into_iter()
        .collect();

        let mut unknown = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1];
                if !known_fields.contains(section) {
                    unknown.push(format!("Unknown section: [{}]", section));
                }
            }
        }

        unknown
    }
}
