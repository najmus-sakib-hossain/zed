//! DX Directory Management for Forge
//!
//! Shared module for managing the `.dx` folder structure.

use std::path::{Path, PathBuf};

/// All DX cache subdirectories
pub const DX_SUBDIRS: &[&str] = &[
    "www",
    "extension",
    "cli",
    "cache",
    "runtime",
    "package-manager",
    "workspace",
    "test-runner",
    "compatibility",
    "serializer",
    "forge",
    "style",
    "ui",
    "font",
    "media",
    "icon",
    "i18n",
    "auth",
    "test",
    "driven",
    "generator",
];

/// DX directory paths for a project
#[derive(Debug, Clone)]
pub struct DxPaths {
    /// Project root directory
    pub project_root: PathBuf,
    /// .dx directory
    pub dx_root: PathBuf,
}

impl DxPaths {
    /// Create paths for a project
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let dx_root = project_root.join(".dx");
        Self {
            project_root,
            dx_root,
        }
    }

    /// Get the dx config file path (no extension)
    pub fn config_file(&self) -> PathBuf {
        self.project_root.join("dx")
    }

    /// Get path to a subdirectory
    pub fn subdir(&self, name: &str) -> PathBuf {
        self.dx_root.join(name)
    }

    // Tool-specific directories
    pub fn www(&self) -> PathBuf {
        self.subdir("www")
    }
    pub fn extension(&self) -> PathBuf {
        self.subdir("extension")
    }
    pub fn cli(&self) -> PathBuf {
        self.subdir("cli")
    }
    pub fn cache(&self) -> PathBuf {
        self.subdir("cache")
    }
    pub fn runtime(&self) -> PathBuf {
        self.subdir("runtime")
    }
    pub fn package_manager(&self) -> PathBuf {
        self.subdir("package-manager")
    }
    pub fn workspace(&self) -> PathBuf {
        self.subdir("workspace")
    }
    pub fn test_runner(&self) -> PathBuf {
        self.subdir("test-runner")
    }
    pub fn compatibility(&self) -> PathBuf {
        self.subdir("compatibility")
    }
    pub fn serializer(&self) -> PathBuf {
        self.subdir("serializer")
    }
    pub fn forge(&self) -> PathBuf {
        self.subdir("forge")
    }
    pub fn style(&self) -> PathBuf {
        self.subdir("style")
    }
    pub fn ui(&self) -> PathBuf {
        self.subdir("ui")
    }
    pub fn font(&self) -> PathBuf {
        self.subdir("font")
    }
    pub fn media(&self) -> PathBuf {
        self.subdir("media")
    }
    pub fn icon(&self) -> PathBuf {
        self.subdir("icon")
    }
    pub fn i18n(&self) -> PathBuf {
        self.subdir("i18n")
    }
    pub fn auth(&self) -> PathBuf {
        self.subdir("auth")
    }
    pub fn test(&self) -> PathBuf {
        self.subdir("test")
    }
    pub fn driven(&self) -> PathBuf {
        self.subdir("driven")
    }
    pub fn generator(&self) -> PathBuf {
        self.subdir("generator")
    }

    /// Ensure all directories exist
    pub fn ensure_all(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dx_root)?;
        for subdir in DX_SUBDIRS {
            std::fs::create_dir_all(self.subdir(subdir))?;
        }
        Ok(())
    }

    /// Check if .dx directory exists
    pub fn exists(&self) -> bool {
        self.dx_root.exists()
    }

    /// Check if dx config file exists
    pub fn config_exists(&self) -> bool {
        self.config_file().exists()
    }
}

/// Get DxPaths for current directory
pub fn current_project() -> DxPaths {
    DxPaths::new(std::env::current_dir().unwrap_or_default())
}
