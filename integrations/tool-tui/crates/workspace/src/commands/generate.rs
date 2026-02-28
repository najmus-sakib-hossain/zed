//! Generate platform configurations.

use crate::{Generator, Platform, Result, WorkspaceConfig};
use console::{Emoji, style};
use std::path::PathBuf;

static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");

/// Options for configuration generation.
#[derive(Debug, Default)]
pub struct GenerateOptions {
    /// Project directory.
    pub path: Option<PathBuf>,
    /// Specific platforms to generate.
    pub platforms: Vec<Platform>,
    /// Generate for all platforms.
    pub all: bool,
    /// Generate only for desktop editors.
    pub desktop: bool,
    /// Generate only for cloud IDEs.
    pub cloud: bool,
    /// Force regeneration even if config exists.
    pub force: bool,
}

/// Command to generate platform configurations.
pub struct GenerateCommand;

impl GenerateCommand {
    /// Execute the generate command.
    pub fn execute(options: GenerateOptions) -> Result<Vec<Platform>> {
        let project_dir = options.path.unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        // Load workspace config
        let config_path = project_dir.join("dx-workspace.json");
        let config = if config_path.exists() {
            WorkspaceConfig::load(&config_path)?
        } else {
            // Auto-detect if no config exists
            crate::ProjectDetector::new(&project_dir).detect()?
        };

        println!("{} {}Generating configurations...", style("[dx-workspace]").bold().cyan(), GEAR);

        // Determine platforms
        let platforms = if options.all {
            Platform::all()
        } else if options.desktop {
            Platform::desktop_editors().to_vec()
        } else if options.cloud {
            Platform::cloud_ides().to_vec()
        } else if !options.platforms.is_empty() {
            options.platforms
        } else {
            // Default: just VS Code
            vec![Platform::VsCode]
        };

        let generator = Generator::with_output_dir(&config, &project_dir);
        let mut generated = Vec::new();

        for platform in &platforms {
            // Skip if exists and not forcing
            if !options.force && generator.exists(*platform) {
                println!(
                    "  {} {} (skipped, already exists)",
                    style("→").dim(),
                    platform.display_name()
                );
                continue;
            }

            match generator.generate(*platform) {
                Ok(result) => {
                    println!(
                        "  {} {} ({} files)",
                        style(CHECK).green(),
                        platform.display_name(),
                        result.files.len()
                    );
                    generated.push(*platform);
                }
                Err(e) => {
                    println!("  {} {} - {}", style("✗").red(), platform.display_name(), e);
                }
            }
        }

        println!();
        println!(
            "{} Generated {} platform configuration(s)",
            style("[dx-workspace]").bold().cyan(),
            generated.len()
        );

        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_options_default() {
        let options = GenerateOptions::default();
        assert!(options.platforms.is_empty());
        assert!(!options.all);
        assert!(!options.force);
    }
}
