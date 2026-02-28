//! Cloud IDE platform generators.

use crate::{Result, WorkspaceConfig};
use std::path::Path;

/// Gitpod configuration generator.
pub mod gitpod;

/// GitHub Codespaces configuration generator.
pub mod codespaces;

/// CodeSandbox configuration generator.
pub mod codesandbox;

/// Firebase Studio (Project IDX) configuration generator.
pub mod firebase_studio;

/// StackBlitz configuration generator.
pub mod stackblitz;

/// Replit configuration generator.
pub mod replit;

/// Trait for cloud IDE generators.
pub trait CloudGenerator {
    /// Generate configuration files for this cloud IDE.
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>>;

    /// Check if this cloud IDE's configuration already exists.
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
