//! Atomic configuration save logic

use crate::utils::error::DxError;
use std::fs;
use std::io::Write;
use std::path::Path;

use super::cache::invalidate_cache;
use super::types::DxConfig;

impl DxConfig {
    /// Save configuration atomically with backup
    pub fn save_atomic(&self, path: &Path) -> Result<(), DxError> {
        // Create backup if file exists
        if path.exists() {
            let backup_path = path.with_extension("toml.bak");
            fs::copy(path, &backup_path).map_err(|e| DxError::Io {
                message: format!("Failed to create backup: {}", e),
            })?;
        }

        // Serialize to TOML
        let content = toml::to_string_pretty(self).map_err(|e| DxError::Io {
            message: format!("Failed to serialize config: {}", e),
        })?;

        // Write to temp file first
        let temp_path = path.with_extension("toml.tmp");
        let mut file = fs::File::create(&temp_path).map_err(|e| DxError::Io {
            message: format!("Failed to create temp file: {}", e),
        })?;

        file.write_all(content.as_bytes()).map_err(|e| DxError::Io {
            message: format!("Failed to write temp file: {}", e),
        })?;

        file.sync_all().map_err(|e| DxError::Io {
            message: format!("Failed to sync temp file: {}", e),
        })?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(|e| DxError::Io {
            message: format!("Failed to rename temp file: {}", e),
        })?;

        // Invalidate cache since we've modified the file
        invalidate_cache(path);

        Ok(())
    }
}
