//! Neovim configuration generator.
//!
//! Generates Lua configuration for Neovim with LSP setup.

use super::{DesktopGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Neovim configuration generator.
#[derive(Debug, Default)]
pub struct NeovimGenerator;

impl NeovimGenerator {
    /// Create a new Neovim generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate .nvim.lua content for project-local config.
    fn generate_config(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("-- dx-workspace generated Neovim configuration".to_string());
        lines
            .push("-- This file is auto-generated. Manual changes may be overwritten.".to_string());
        lines.push(String::new());

        // Basic settings
        lines.push("-- Editor settings".to_string());
        lines.push(format!("vim.opt_local.tabstop = {}", config.editor.tab_size));
        lines.push(format!("vim.opt_local.shiftwidth = {}", config.editor.tab_size));
        lines.push(format!("vim.opt_local.expandtab = {}", config.editor.insert_spaces));

        lines.push(String::new());

        // Rust-analyzer configuration for dx projects
        if config.detected_features.is_cargo_project {
            lines.push("-- Rust-analyzer settings for dx project".to_string());
            lines.push("local lspconfig = require('lspconfig')".to_string());
            lines.push("if lspconfig.rust_analyzer then".to_string());
            lines.push("  lspconfig.rust_analyzer.setup({".to_string());
            lines.push("    settings = {".to_string());
            lines.push("      ['rust-analyzer'] = {".to_string());
            lines.push("        cargo = {".to_string());
            lines.push("          features = 'all',".to_string());
            lines.push("        },".to_string());
            lines.push("        checkOnSave = {".to_string());
            lines.push("          command = 'clippy',".to_string());
            lines.push("        },".to_string());
            lines.push("      },".to_string());
            lines.push("    },".to_string());
            lines.push("  })".to_string());
            lines.push("end".to_string());
            lines.push(String::new());
        }

        // Key mappings for dx commands
        lines.push("-- dx command keybindings".to_string());
        lines.push(
            "vim.keymap.set('n', '<leader>db', ':!dx build<CR>', { desc = 'dx build' })"
                .to_string(),
        );
        lines.push(
            "vim.keymap.set('n', '<leader>dd', ':!dx dev<CR>', { desc = 'dx dev' })".to_string(),
        );
        lines.push(
            "vim.keymap.set('n', '<leader>dc', ':!dx check<CR>', { desc = 'dx check' })"
                .to_string(),
        );
        lines.push(
            "vim.keymap.set('n', '<leader>df', ':!dx forge<CR>', { desc = 'dx forge' })"
                .to_string(),
        );

        lines.join("\n")
    }
}

impl DesktopGenerator for NeovimGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let content = self.generate_config(config);
        files.push(GeneratedFile::new(".nvim.lua", content.clone()));

        // Write the file
        let path = output_dir.join(".nvim.lua");
        fs::write(&path, &content).map_err(|e| crate::Error::io(&path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".nvim.lua").exists() || project_dir.join(".nvim").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let nvim_lua = project_dir.join(".nvim.lua");
        if nvim_lua.exists() {
            fs::remove_file(&nvim_lua).map_err(|e| crate::Error::io(&nvim_lua, e))?;
        }

        let nvim_dir = project_dir.join(".nvim");
        if nvim_dir.exists() {
            fs::remove_dir_all(&nvim_dir).map_err(|e| crate::Error::io(&nvim_dir, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config() {
        let mut config = WorkspaceConfig::new("test");
        config.editor.tab_size = 2;
        config.detected_features.is_cargo_project = true;

        let generator = NeovimGenerator::new();
        let content = generator.generate_config(&config);

        assert!(content.contains("tabstop = 2"));
        assert!(content.contains("rust_analyzer"));
    }
}
