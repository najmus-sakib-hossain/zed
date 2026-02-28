//! Container environment generators.

use crate::{Result, WorkspaceConfig};
use std::path::Path;

/// Nix Flakes configuration generator.
pub mod nix_flakes;

/// Docker Compose configuration generator.
pub mod docker_compose;

/// Trait for container generators.
pub trait ContainerGenerator {
    /// Generate configuration files for this container environment.
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>>;

    /// Check if this container environment's configuration already exists.
    fn exists(&self, project_dir: &Path) -> bool;

    /// Clean up generated configuration files.
    fn clean(&self, project_dir: &Path) -> Result<()>;
}

/// A generated configuration file.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Relative path from project root.
    pub path: String,
    /// File contents.
    pub content: String,
    /// Whether this file was newly created or updated.
    pub is_new: bool,
}

impl GeneratedFile {
    /// Create a new generated file record.
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            is_new: true,
        }
    }
}
