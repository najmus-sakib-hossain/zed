//! Update application with backup and restore

use super::signature::verify_signature;
use crate::utils::error::DxError;
use std::fs;
use std::path::{Path, PathBuf};

/// Update applier for the DX CLI
pub struct UpdateApplier {
    /// Path to the current binary
    binary_path: PathBuf,
    /// Path to the backup file
    backup_path: PathBuf,
}

impl UpdateApplier {
    /// Create a new update applier for the given binary path
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        let binary_path = binary_path.into();
        let backup_path = binary_path.with_extension("bak");
        Self {
            binary_path,
            backup_path,
        }
    }

    /// Create an update applier for the current executable
    pub fn for_current_exe() -> Result<Self, DxError> {
        let binary_path = std::env::current_exe().map_err(|e| DxError::Io {
            message: format!("Failed to get current executable path: {}", e),
        })?;
        Ok(Self::new(binary_path))
    }

    /// Create a backup of the current binary
    pub fn create_backup(&self) -> Result<(), DxError> {
        if self.binary_path.exists() {
            fs::copy(&self.binary_path, &self.backup_path).map_err(|e| DxError::Io {
                message: format!(
                    "Failed to create backup at {}: {}",
                    self.backup_path.display(),
                    e
                ),
            })?;
        }
        Ok(())
    }

    /// Restore from backup
    pub fn restore_from_backup(&self) -> Result<(), DxError> {
        if self.backup_path.exists() {
            fs::copy(&self.backup_path, &self.binary_path).map_err(|e| DxError::Io {
                message: format!(
                    "Failed to restore from backup {}: {}",
                    self.backup_path.display(),
                    e
                ),
            })?;
        }
        Ok(())
    }

    /// Remove the backup file
    pub fn remove_backup(&self) -> Result<(), DxError> {
        if self.backup_path.exists() {
            fs::remove_file(&self.backup_path).map_err(|e| DxError::Io {
                message: format!("Failed to remove backup {}: {}", self.backup_path.display(), e),
            })?;
        }
        Ok(())
    }

    /// Apply update with atomic replacement
    pub fn apply_update(
        &self,
        new_binary: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<(), DxError> {
        verify_signature(new_binary, signature, public_key)?;
        self.create_backup()?;

        let temp_path = self.binary_path.with_extension("tmp");
        let result = self.write_and_replace(new_binary, &temp_path);

        if result.is_err() {
            let _ = self.restore_from_backup();
            let _ = fs::remove_file(&temp_path);
        }

        result
    }

    fn write_and_replace(&self, new_binary: &[u8], temp_path: &Path) -> Result<(), DxError> {
        fs::write(temp_path, new_binary).map_err(|e| DxError::Io {
            message: format!("Failed to write temp file {}: {}", temp_path.display(), e),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            fs::set_permissions(temp_path, perms).map_err(|e| DxError::Io {
                message: format!("Failed to set permissions on {}: {}", temp_path.display(), e),
            })?;
        }

        fs::rename(temp_path, &self.binary_path).map_err(|e| DxError::Io {
            message: format!(
                "Failed to replace binary {} with {}: {}",
                self.binary_path.display(),
                temp_path.display(),
                e
            ),
        })?;

        Ok(())
    }

    /// Get the backup path
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }

    /// Check if a backup exists
    pub fn has_backup(&self) -> bool {
        self.backup_path.exists()
    }
}
