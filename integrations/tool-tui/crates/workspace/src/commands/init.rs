//! Initialize workspace configuration.

use crate::{Generator, Platform, ProjectDetector, Result, WorkspaceConfig};
use console::{Emoji, style};
use std::path::PathBuf;

static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");
static FOLDER: Emoji<'_, '_> = Emoji("📁 ", "");

/// Options for workspace initialization.
#[derive(Debug, Default)]
pub struct InitOptions {
    /// Project directory (defaults to current directory).
    pub path: Option<PathBuf>,
    /// Platforms to generate (defaults to detected/common ones).
    pub platforms: Vec<Platform>,
    /// Skip confirmation prompts.
    pub yes: bool,
    /// Generate for all platforms.
    pub all: bool,
    /// Only detect, don't generate.
    pub detect_only: bool,
}

/// Command to initialize a dx-workspace configuration.
pub struct InitCommand;

impl InitCommand {
    /// Execute the init command.
    pub fn execute(options: InitOptions) -> Result<WorkspaceConfig> {
        let project_dir = options.path.unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        println!(
            "{} {}Initializing dx-workspace...",
            style("[dx-workspace]").bold().cyan(),
            SPARKLE
        );

        // Detect project
        println!("  {} {}Scanning project at {}", style("→").dim(), FOLDER, project_dir.display());

        let detector = ProjectDetector::new(&project_dir);
        let config = detector.detect()?;

        // Print detected features
        Self::print_detected_features(&config);

        if options.detect_only {
            return Ok(config);
        }

        // Determine which platforms to generate
        let platforms = if options.all {
            Platform::all()
        } else if !options.platforms.is_empty() {
            options.platforms
        } else {
            Self::default_platforms(&config)
        };

        // Generate configurations
        let generator = Generator::with_output_dir(&config, &project_dir);

        println!();
        println!(
            "  {} Generating configurations for {} platforms...",
            style("→").dim(),
            platforms.len()
        );

        for platform in &platforms {
            match generator.generate(*platform) {
                Ok(result) => {
                    println!(
                        "    {} {} ({} files)",
                        style(CHECK).green(),
                        platform.display_name(),
                        result.files.len()
                    );
                }
                Err(e) => {
                    println!("    {} {} - {}", style("✗").red(), platform.display_name(), e);
                }
            }
        }

        // Save workspace config
        let config_path = project_dir.join("dx-workspace.json");
        config.save(&config_path)?;
        println!();
        println!("  {} Saved configuration to {}", style(CHECK).green(), config_path.display());

        println!();
        println!(
            "{} {}Workspace initialized successfully!",
            style("[dx-workspace]").bold().cyan(),
            SPARKLE
        );

        Ok(config)
    }

    fn print_detected_features(config: &WorkspaceConfig) {
        let features = &config.detected_features;

        println!();
        println!("  {} Detected features:", style("→").dim());

        if features.is_cargo_project {
            println!("    {} Rust/Cargo project", style(CHECK).green());
        }
        if features.has_dx_www {
            println!("    {} dx-www components", style(CHECK).green());
        }
        if features.has_dx_style {
            println!("    {} dx-style styling", style(CHECK).green());
        }
        if features.has_dx_server {
            println!("    {} dx-server backend", style(CHECK).green());
        }
        if features.has_dx_client {
            println!("    {} dx-client WASM runtime", style(CHECK).green());
        }
        if features.has_dx_forge {
            println!("    {} dx-forge build pipeline", style(CHECK).green());
        }
        if features.uses_typescript {
            println!("    {} TypeScript", style(CHECK).green());
        }
    }

    fn default_platforms(config: &WorkspaceConfig) -> Vec<Platform> {
        let mut platforms = vec![Platform::VsCode];

        // Add cloud platforms if this looks like an open source project
        if config.detected_features.is_cargo_project {
            platforms.push(Platform::Codespaces);
            platforms.push(Platform::Gitpod);
        }

        platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_platforms() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;

        let platforms = InitCommand::default_platforms(&config);
        assert!(platforms.contains(&Platform::VsCode));
        assert!(platforms.contains(&Platform::Codespaces));
    }
}
