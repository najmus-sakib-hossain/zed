//! Replit configuration generator.
//!
//! Generates:
//! - .replit
//! - replit.nix

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Replit configuration generator.
#[derive(Debug, Default)]
pub struct ReplitGenerator;

impl ReplitGenerator {
    /// Create a new Replit generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate .replit content.
    fn generate_replit(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("# dx-workspace generated Replit configuration".to_string());
        lines.push(String::new());

        // Language and entrypoint
        if config.detected_features.is_cargo_project {
            lines.push("run = \"cargo run\"".to_string());
            lines.push("language = \"rust\"".to_string());
            lines.push("entrypoint = \"src/main.rs\"".to_string());
        }

        lines.push(String::new());

        // Compile command
        if config.detected_features.is_cargo_project {
            lines.push("[compile]".to_string());
            lines.push("run = \"cargo build\"".to_string());
            lines.push(String::new());
        }

        // Nix channel
        lines.push("[nix]".to_string());
        lines.push("channel = \"stable-23_11\"".to_string());
        lines.push(String::new());

        // Deployment
        if config.detected_features.has_dx_server {
            lines.push("[deployment]".to_string());
            lines.push("run = [\"sh\", \"-c\", \"dx build && dx serve\"]".to_string());
            lines.push("deploymentTarget = \"cloudrun\"".to_string());
            lines.push(String::new());
        }

        // Unit tests
        lines.push("[unitTest]".to_string());
        lines.push("language = \"rust\"".to_string());
        lines.push(String::new());

        // Debug
        lines.push("[debugger]".to_string());
        lines.push("support = true".to_string());
        lines.push(String::new());

        // Languages
        lines.push("[languages]".to_string());
        lines.push(String::new());

        lines.push("[languages.rust]".to_string());
        lines.push("pattern = \"**/*.rs\"".to_string());
        lines.push(String::new());

        lines.push("[languages.rust.languageServer]".to_string());
        lines.push("start = \"rust-analyzer\"".to_string());

        lines.join("\n")
    }

    /// Generate replit.nix content.
    fn generate_nix(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("{ pkgs }: {".to_string());
        lines.push("  deps = [".to_string());

        if config.detected_features.is_cargo_project {
            lines.push("    pkgs.rustup".to_string());
            lines.push("    pkgs.rust-analyzer".to_string());
            lines.push("    pkgs.cargo".to_string());
            lines.push("    pkgs.rustc".to_string());
            lines.push("    pkgs.rustfmt".to_string());
            lines.push("    pkgs.clippy".to_string());
        }

        if config.detected_features.has_dx_client {
            lines.push("    pkgs.wasm-pack".to_string());
        }

        lines.push("    pkgs.pkg-config".to_string());
        lines.push("    pkgs.openssl".to_string());
        lines.push("  ];".to_string());
        lines.push("}".to_string());

        lines.join("\n")
    }
}

impl CloudGenerator for ReplitGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Generate .replit
        let replit_content = self.generate_replit(config);
        files.push(GeneratedFile::new(".replit", replit_content.clone()));

        let replit_path = output_dir.join(".replit");
        fs::write(&replit_path, &replit_content).map_err(|e| crate::Error::io(&replit_path, e))?;

        // Generate replit.nix
        let nix_content = self.generate_nix(config);
        files.push(GeneratedFile::new("replit.nix", nix_content.clone()));

        let nix_path = output_dir.join("replit.nix");
        fs::write(&nix_path, &nix_content).map_err(|e| crate::Error::io(&nix_path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".replit").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let replit_path = project_dir.join(".replit");
        if replit_path.exists() {
            fs::remove_file(&replit_path).map_err(|e| crate::Error::io(&replit_path, e))?;
        }

        let nix_path = project_dir.join("replit.nix");
        if nix_path.exists() {
            fs::remove_file(&nix_path).map_err(|e| crate::Error::io(&nix_path, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_replit() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;

        let generator = ReplitGenerator::new();
        let content = generator.generate_replit(&config);

        assert!(content.contains("cargo run"));
        assert!(content.contains("language = \"rust\""));
    }

    #[test]
    fn test_generate_nix() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.detected_features.has_dx_client = true;

        let generator = ReplitGenerator::new();
        let content = generator.generate_nix(&config);

        assert!(content.contains("pkgs.rustup"));
        assert!(content.contains("pkgs.wasm-pack"));
    }
}
