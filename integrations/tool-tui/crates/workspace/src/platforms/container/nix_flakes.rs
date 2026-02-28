//! Nix Flakes configuration generator.
//!
//! Generates:
//! - flake.nix
//! - .envrc (for direnv integration)

use super::{ContainerGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Nix Flakes configuration generator.
#[derive(Debug, Default)]
pub struct NixFlakesGenerator {
    /// Generate .envrc for direnv integration.
    pub with_direnv: bool,
}

impl NixFlakesGenerator {
    /// Create a new Nix Flakes generator.
    pub fn new() -> Self {
        Self { with_direnv: true }
    }

    /// Disable direnv integration.
    pub fn without_direnv(mut self) -> Self {
        self.with_direnv = false;
        self
    }

    /// Generate flake.nix content.
    fn generate_flake(&self, config: &WorkspaceConfig) -> String {
        let mut lines = vec![
            "# dx-workspace generated Nix Flake".to_string(),
            "{".to_string(),
            "  description = \"dx development environment\";".to_string(),
            String::new(),
            // Inputs
            "  inputs = {".to_string(),
            "    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";".to_string(),
            "    flake-utils.url = \"github:numtide/flake-utils\";".to_string(),
        ];

        if config.detected_features.is_cargo_project {
            lines.push("    rust-overlay = {".to_string());
            lines.push("      url = \"github:oxalica/rust-overlay\";".to_string());
            lines.push("      inputs.nixpkgs.follows = \"nixpkgs\";".to_string());
            lines.push("    };".to_string());
        }

        lines.push("  };".to_string());
        lines.push(String::new());

        // Outputs
        lines.push("  outputs = { self, nixpkgs, flake-utils, ... }@inputs:".to_string());
        lines.push("    flake-utils.lib.eachDefaultSystem (system:".to_string());
        lines.push("      let".to_string());

        if config.detected_features.is_cargo_project {
            lines.push("        overlays = [ inputs.rust-overlay.overlays.default ];".to_string());
            lines.push("        pkgs = import nixpkgs { inherit system overlays; };".to_string());
            lines.push(String::new());
            lines.push(
                "        rustToolchain = pkgs.rust-bin.stable.latest.default.override {"
                    .to_string(),
            );
            lines.push(
                "          extensions = [ \"rust-src\" \"rust-analyzer\" \"clippy\" ];".to_string(),
            );

            if config.detected_features.has_dx_client {
                lines.push("          targets = [ \"wasm32-unknown-unknown\" ];".to_string());
            }

            lines.push("        };".to_string());
        } else {
            lines.push("        pkgs = import nixpkgs { inherit system; };".to_string());
        }

        lines.push("      in".to_string());
        lines.push("      {".to_string());

        // Dev shell
        lines.push("        devShells.default = pkgs.mkShell {".to_string());
        lines.push("          buildInputs = with pkgs; [".to_string());

        if config.detected_features.is_cargo_project {
            lines.push("            rustToolchain".to_string());
            lines.push("            cargo-watch".to_string());
            lines.push("            cargo-edit".to_string());
        }

        if config.detected_features.has_dx_client {
            lines.push("            wasm-pack".to_string());
            lines.push("            wasm-bindgen-cli".to_string());
            lines.push("            binaryen".to_string());
        }

        // Common dependencies
        lines.push("            pkg-config".to_string());
        lines.push("            openssl".to_string());
        lines.push("            git".to_string());

        lines.push("          ];".to_string());
        lines.push(String::new());

        // Shell hook
        lines.push("          shellHook = ''".to_string());
        lines.push("            echo \"dx development environment loaded\"".to_string());
        lines.push("            export RUST_BACKTRACE=1".to_string());

        if config.detected_features.is_cargo_project {
            lines.push("            export CARGO_HOME=\"$PWD/.cargo\"".to_string());
        }

        lines.push("          '';".to_string());
        lines.push("        };".to_string());

        // Packages output
        lines.push(String::new());
        lines.push("        packages.default = pkgs.stdenv.mkDerivation {".to_string());
        lines.push(format!("          pname = \"{}\";", config.name));
        lines.push("          version = \"0.1.0\";".to_string());
        lines.push("          src = ./.;".to_string());
        lines.push(String::new());

        if config.detected_features.is_cargo_project {
            lines.push("          nativeBuildInputs = with pkgs; [ rustToolchain ];".to_string());
            lines.push("          buildInputs = with pkgs; [ openssl pkg-config ];".to_string());
            lines.push(String::new());
            lines.push("          buildPhase = \"cargo build --release\";".to_string());
            lines.push("          installPhase = \"mkdir -p $out/bin && cp target/release/* $out/bin/ || true\";".to_string());
        }

        lines.push("        };".to_string());

        lines.push("      }".to_string());
        lines.push("    );".to_string());
        lines.push("}".to_string());

        lines.join("\n")
    }

    /// Generate .envrc content for direnv.
    fn generate_envrc(&self) -> String {
        [
            "# dx-workspace generated direnv configuration",
            "use flake",
            "",
            "# Watch additional files for changes",
            "watch_file flake.nix",
            "watch_file flake.lock",
        ]
        .join("\n")
    }
}

impl ContainerGenerator for NixFlakesGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Generate flake.nix
        let flake_content = self.generate_flake(config);
        files.push(GeneratedFile::new("flake.nix", flake_content.clone()));

        let flake_path = output_dir.join("flake.nix");
        fs::write(&flake_path, &flake_content).map_err(|e| crate::Error::io(&flake_path, e))?;

        // Generate .envrc if requested
        if self.with_direnv {
            let envrc_content = self.generate_envrc();
            files.push(GeneratedFile::new(".envrc", envrc_content.clone()));

            let envrc_path = output_dir.join(".envrc");
            fs::write(&envrc_path, &envrc_content).map_err(|e| crate::Error::io(&envrc_path, e))?;
        }

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join("flake.nix").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let flake_path = project_dir.join("flake.nix");
        if flake_path.exists() {
            fs::remove_file(&flake_path).map_err(|e| crate::Error::io(&flake_path, e))?;
        }

        let lock_path = project_dir.join("flake.lock");
        if lock_path.exists() {
            fs::remove_file(&lock_path).map_err(|e| crate::Error::io(&lock_path, e))?;
        }

        let envrc_path = project_dir.join(".envrc");
        if envrc_path.exists() {
            fs::remove_file(&envrc_path).map_err(|e| crate::Error::io(&envrc_path, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_flake() {
        let mut config = WorkspaceConfig::new("test-project");
        config.detected_features.is_cargo_project = true;
        config.detected_features.has_dx_client = true;

        let generator = NixFlakesGenerator::new();
        let content = generator.generate_flake(&config);

        assert!(content.contains("rust-overlay"));
        assert!(content.contains("wasm32-unknown-unknown"));
        assert!(content.contains("wasm-pack"));
    }

    #[test]
    fn test_generate_envrc() {
        let generator = NixFlakesGenerator::new();
        let content = generator.generate_envrc();

        assert!(content.contains("use flake"));
    }
}
