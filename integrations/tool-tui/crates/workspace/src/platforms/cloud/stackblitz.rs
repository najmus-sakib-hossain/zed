//! StackBlitz configuration generator.
//!
//! Generates:
//! - .stackblitzrc

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

/// StackBlitz configuration generator.
#[derive(Debug, Default)]
pub struct StackBlitzGenerator;

impl StackBlitzGenerator {
    /// Create a new StackBlitz generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate .stackblitzrc content.
    fn generate_config(&self, config: &WorkspaceConfig) -> Value {
        let start_command = if config.detected_features.is_cargo_project {
            "cargo run"
        } else {
            "npm start"
        };

        json!({
            "installDependencies": true,
            "startCommand": start_command,
            "env": {
                "RUST_BACKTRACE": "1"
            }
        })
    }
}

impl CloudGenerator for StackBlitzGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let rc_config = self.generate_config(config);
        let content = serde_json::to_string_pretty(&rc_config).unwrap_or_default();

        files.push(GeneratedFile::new(".stackblitzrc", content.clone()));

        let path = output_dir.join(".stackblitzrc");
        fs::write(&path, &content).map_err(|e| crate::Error::io(&path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".stackblitzrc").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let path = project_dir.join(".stackblitzrc");
        if path.exists() {
            fs::remove_file(&path).map_err(|e| crate::Error::io(&path, e))?;
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
        config.detected_features.is_cargo_project = true;

        let generator = StackBlitzGenerator::new();
        let rc = generator.generate_config(&config);

        assert_eq!(rc["startCommand"], "cargo run");
    }
}
