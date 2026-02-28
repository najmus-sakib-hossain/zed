//! Desktop editor platform generators.

use crate::{Result, WorkspaceConfig};
use std::path::Path;

/// VS Code / VS Codium configuration generator.
pub mod vscode;

/// Zed editor configuration generator.
pub mod zed;

/// Neovim configuration generator.
pub mod neovim;

/// IntelliJ / Fleet configuration generator.
pub mod intellij;

/// Helix editor configuration generator.
pub mod helix;

/// Sublime Text configuration generator.
pub mod sublime;

/// Trait for desktop editor generators.
pub trait DesktopGenerator {
    /// Generate configuration files for this editor.
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>>;

    /// Check if this editor's configuration already exists.
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
