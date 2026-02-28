//! Firebase Studio (Project IDX) configuration generator.
//!
//! Generates:
//! - .idx/dev.nix

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Firebase Studio (IDX) configuration generator.
#[derive(Debug, Default)]
pub struct FirebaseStudioGenerator;

impl FirebaseStudioGenerator {
    /// Create a new Firebase Studio generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate .idx/dev.nix content.
    fn generate_nix(&self, config: &WorkspaceConfig) -> String {
        let mut lines = vec![
            "# dx-workspace generated Firebase Studio (IDX) configuration".to_string(),
            "{ pkgs, ... }: {".to_string(),
            String::new(),
            // Channel
            "  channel = \"stable-23.11\";".to_string(),
            String::new(),
            // Packages
            "  packages = [".to_string(),
        ];

        if config.detected_features.is_cargo_project {
            lines.push("    pkgs.rustup".to_string());
            lines.push("    pkgs.cargo".to_string());
            lines.push("    pkgs.rustc".to_string());
            lines.push("    pkgs.rust-analyzer".to_string());
            lines.push("    pkgs.clippy".to_string());
            lines.push("    pkgs.rustfmt".to_string());
        }

        if config.detected_features.has_dx_client {
            lines.push("    pkgs.wasm-pack".to_string());
            lines.push("    pkgs.binaryen".to_string());
        }

        // Common tools
        lines.push("    pkgs.git".to_string());
        lines.push("    pkgs.curl".to_string());
        lines.push("    pkgs.pkg-config".to_string());
        lines.push("    pkgs.openssl".to_string());

        lines.push("  ];".to_string());
        lines.push(String::new());

        // Environment variables
        lines.push("  env = {".to_string());
        lines.push("    RUST_BACKTRACE = \"1\";".to_string());
        if config.detected_features.is_cargo_project {
            lines.push("    CARGO_HOME = \"/home/user/.cargo\";".to_string());
        }
        lines.push("  };".to_string());
        lines.push(String::new());

        // IDX configuration
        lines.push("  idx = {".to_string());

        // Extensions
        lines.push("    extensions = [".to_string());
        for ext in &config.extensions.core {
            lines.push(format!("      \"{}\"", ext.id));
        }
        for ext in &config.extensions.recommended {
            lines.push(format!("      \"{}\"", ext.id));
        }
        lines.push("    ];".to_string());
        lines.push(String::new());

        // Workspace
        lines.push("    workspace = {".to_string());
        lines.push("      onCreate = {".to_string());

        if config.detected_features.is_cargo_project {
            lines.push("        cargo-fetch = \"cargo fetch\";".to_string());
            lines.push(
                "        install-wasm-target = \"rustup target add wasm32-unknown-unknown\";"
                    .to_string(),
            );
        }

        lines.push("      };".to_string());
        lines.push(String::new());

        lines.push("      onStart = {".to_string());
        lines.push("        watch = \"cargo watch -x check\";".to_string());
        lines.push("      };".to_string());

        lines.push("    };".to_string());
        lines.push(String::new());

        // Previews
        if config.detected_features.has_dx_server || config.detected_features.has_dx_www {
            lines.push("    previews = {".to_string());
            lines.push("      enable = true;".to_string());
            lines.push("      previews = {".to_string());
            lines.push("        web = {".to_string());
            lines.push("          command = [\"dx\" \"dev\"];".to_string());
            lines.push("          manager = \"web\";".to_string());
            lines.push("          env = { PORT = \"$PORT\"; };".to_string());
            lines.push("        };".to_string());
            lines.push("      };".to_string());
            lines.push("    };".to_string());
        }

        lines.push("  };".to_string());
        lines.push("}".to_string());

        lines.join("\n")
    }
}

impl CloudGenerator for FirebaseStudioGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Create .idx directory
        let idx_dir = output_dir.join(".idx");
        fs::create_dir_all(&idx_dir).map_err(|e| crate::Error::io(&idx_dir, e))?;

        // Generate dev.nix
        let nix_content = self.generate_nix(config);
        files.push(GeneratedFile::new(".idx/dev.nix", nix_content.clone()));

        let nix_path = idx_dir.join("dev.nix");
        fs::write(&nix_path, &nix_content).map_err(|e| crate::Error::io(&nix_path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".idx").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let idx_dir = project_dir.join(".idx");
        if idx_dir.exists() {
            fs::remove_dir_all(&idx_dir).map_err(|e| crate::Error::io(&idx_dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExtensionRecommendations;

    #[test]
    fn test_generate_nix() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.detected_features.has_dx_client = true;
        config.extensions = ExtensionRecommendations::dx_defaults();

        let generator = FirebaseStudioGenerator::new();
        let content = generator.generate_nix(&config);

        assert!(content.contains("pkgs.rustup"));
        assert!(content.contains("pkgs.wasm-pack"));
        assert!(content.contains("extensions = ["));
    }
}
