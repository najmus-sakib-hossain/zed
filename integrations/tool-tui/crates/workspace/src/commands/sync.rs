//! Synchronize workspace configuration.

use crate::{Generator, Platform, Result, WorkspaceConfig};
use console::{Emoji, style};
use std::path::{Path, PathBuf};

static SYNC: Emoji<'_, '_> = Emoji("🔄 ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");

/// Sync direction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SyncDirection {
    /// Pull changes from IDE configs into dx-workspace.
    Pull,
    /// Push dx-workspace config to IDE configs.
    #[default]
    Push,
    /// Bidirectional sync (detect changes and merge).
    Both,
}

/// Options for synchronization.
#[derive(Debug, Default)]
pub struct SyncOptions {
    /// Project directory.
    pub path: Option<PathBuf>,
    /// Sync direction.
    pub direction: SyncDirection,
    /// Specific platforms to sync.
    pub platforms: Vec<Platform>,
    /// Force sync, overwriting conflicts.
    pub force: bool,
    /// Dry run - show what would change.
    pub dry_run: bool,
}

/// Command to synchronize configurations.
pub struct SyncCommand;

impl SyncCommand {
    /// Execute the sync command.
    pub fn execute(options: SyncOptions) -> Result<()> {
        let project_dir = options.path.clone().unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        println!(
            "{} {}Synchronizing configurations...",
            style("[dx-workspace]").bold().cyan(),
            SYNC
        );

        let config_path = project_dir.join("dx-workspace.json");
        let config = if config_path.exists() {
            WorkspaceConfig::load(&config_path)?
        } else {
            return Err(crate::Error::ConfigNotFound { path: config_path });
        };

        match options.direction {
            SyncDirection::Push => Self::push_sync(&config, &project_dir, &options)?,
            SyncDirection::Pull => Self::pull_sync(&config, &project_dir, &options)?,
            SyncDirection::Both => {
                // For bidirectional, first pull then push
                Self::pull_sync(&config, &project_dir, &options)?;
                Self::push_sync(&config, &project_dir, &options)?;
            }
        }

        println!();
        println!("{} {}Synchronization complete!", style("[dx-workspace]").bold().cyan(), CHECK);

        Ok(())
    }

    fn push_sync(
        config: &WorkspaceConfig,
        project_dir: &Path,
        options: &SyncOptions,
    ) -> Result<()> {
        println!("  {} Pushing configuration to IDE files...", style("→").dim());

        let generator = Generator::with_output_dir(config, project_dir);

        let platforms = if options.platforms.is_empty() {
            // Sync to platforms that already exist
            Platform::all().into_iter().filter(|p| generator.exists(*p)).collect()
        } else {
            options.platforms.clone()
        };

        for platform in &platforms {
            if options.dry_run {
                println!("    {} {} (would update)", style("→").dim(), platform.display_name());
            } else {
                match generator.generate(*platform) {
                    Ok(result) => {
                        println!(
                            "    {} {} ({} files updated)",
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
        }

        Ok(())
    }

    fn pull_sync(
        _config: &WorkspaceConfig,
        project_dir: &Path,
        options: &SyncOptions,
    ) -> Result<()> {
        println!("  {} Pulling changes from IDE files...", style("→").dim());

        // Check for VS Code settings changes
        let vscode_settings = project_dir.join(".vscode/settings.json");
        if vscode_settings.exists() {
            if options.dry_run {
                println!("    {} VS Code settings (would import)", style("→").dim());
            } else {
                // TODO: Implement settings import
                println!("    {} VS Code settings (import not yet implemented)", style("ℹ").blue());
            }
        }

        // Check for Gitpod changes
        let gitpod_yml = project_dir.join(".gitpod.yml");
        if gitpod_yml.exists() {
            if options.dry_run {
                println!("    {} Gitpod config (would import)", style("→").dim());
            } else {
                // TODO: Implement Gitpod import
                println!("    {} Gitpod config (import not yet implemented)", style("ℹ").blue());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_direction_default() {
        assert_eq!(SyncDirection::default(), SyncDirection::Push);
    }
}
