//! Clean generated configurations.

use crate::{Generator, Platform, Result, WorkspaceConfig};
use console::{Emoji, style};
use std::path::PathBuf;

static CLEAN: Emoji<'_, '_> = Emoji("🧹 ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");

/// Options for cleaning.
#[derive(Debug, Default)]
pub struct CleanOptions {
    /// Project directory.
    pub path: Option<PathBuf>,
    /// Specific platforms to clean.
    pub platforms: Vec<Platform>,
    /// Clean all platforms.
    pub all: bool,
    /// Dry run - show what would be deleted.
    pub dry_run: bool,
}

/// Command to clean generated configurations.
pub struct CleanCommand;

impl CleanCommand {
    /// Execute the clean command.
    pub fn execute(options: CleanOptions) -> Result<Vec<Platform>> {
        let project_dir = options.path.unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        println!(
            "{} {}Cleaning generated configurations...",
            style("[dx-workspace]").bold().cyan(),
            CLEAN
        );

        // Load config or create minimal one
        let config_path = project_dir.join("dx-workspace.json");
        let config = if config_path.exists() {
            WorkspaceConfig::load(&config_path)?
        } else {
            let mut config = WorkspaceConfig::new("temp");
            config.root = project_dir.clone();
            config
        };

        let generator = Generator::with_output_dir(&config, &project_dir);

        // Determine platforms to clean
        let platforms = if options.all {
            Platform::all()
        } else if !options.platforms.is_empty() {
            options.platforms
        } else {
            // Default: clean all existing
            Platform::all().into_iter().filter(|p| generator.exists(*p)).collect()
        };

        let mut cleaned = Vec::new();

        for platform in &platforms {
            if !generator.exists(*platform) {
                continue;
            }

            if options.dry_run {
                println!("  {} {} (would remove)", style("→").dim(), platform.display_name());
                cleaned.push(*platform);
            } else {
                match generator.clean(*platform) {
                    Ok(()) => {
                        println!("  {} {} removed", style(CHECK).green(), platform.display_name());
                        cleaned.push(*platform);
                    }
                    Err(e) => {
                        println!("  {} {} - {}", style("✗").red(), platform.display_name(), e);
                    }
                }
            }
        }

        println!();
        if options.dry_run {
            println!(
                "{} Would clean {} platform configuration(s)",
                style("[dx-workspace]").bold().cyan(),
                cleaned.len()
            );
        } else {
            println!(
                "{} Cleaned {} platform configuration(s)",
                style("[dx-workspace]").bold().cyan(),
                cleaned.len()
            );
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_options_default() {
        let options = CleanOptions::default();
        assert!(options.platforms.is_empty());
        assert!(!options.all);
        assert!(!options.dry_run);
    }
}
