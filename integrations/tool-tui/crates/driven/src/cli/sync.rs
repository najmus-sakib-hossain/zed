//! Sync command - synchronize rules across editors

use crate::{DrivenConfig, Result, sync::SyncEngine};
use std::path::Path;

/// Sync command handler
#[derive(Debug)]
pub struct SyncCommand;

impl SyncCommand {
    /// Run one-time sync
    pub fn run(project_root: &Path) -> Result<()> {
        let config = Self::load_config(project_root)?;
        let engine = SyncEngine::new(&config.sync.source_of_truth, config.editors);

        let spinner = super::create_spinner("Syncing rules to all editors...");
        let report = engine.sync(project_root)?;
        spinner.finish_and_clear();

        // Report results
        if report.is_success() {
            super::print_success(&format!("Synced rules to {} editors", report.synced_count()));
            for synced in &report.synced {
                println!("  {} → {}", synced.editor, synced.path.display());
            }
        } else {
            super::print_warning(&format!(
                "Synced {} editors, {} errors",
                report.synced_count(),
                report.errors.len()
            ));
            for error in &report.errors {
                super::print_error(&format!("  {} - {}", error.editor, error.error));
            }
        }

        Ok(())
    }

    /// Run watch mode (continuous sync)
    pub fn watch(project_root: &Path) -> Result<()> {
        let config = Self::load_config(project_root)?;
        let mut engine = SyncEngine::new(&config.sync.source_of_truth, config.editors.clone());

        super::print_info("Starting file watcher...");
        engine.start_watching(project_root)?;

        super::print_success("Watching for changes. Press Ctrl+C to stop.");
        println!();

        // Initial sync
        Self::run(project_root)?;

        // Watch loop would go here - for now just wait
        // In a real implementation, this would use tokio or similar
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            // Check for changes and re-sync
        }
    }

    fn load_config(project_root: &Path) -> Result<DrivenConfig> {
        let config_path = project_root.join(".driven/config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| crate::DrivenError::Config(format!("Failed to read config: {}", e)))?;
            toml::from_str(&content)
                .map_err(|e| crate::DrivenError::Config(format!("Failed to parse config: {}", e)))
        } else {
            Ok(DrivenConfig::default())
        }
    }
}
