//! Export workspace configuration.

use crate::{Generator, Platform, Result, WorkspaceConfig};
use console::{Emoji, style};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

static EXPORT: Emoji<'_, '_> = Emoji("📦 ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");

/// Export format.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ExportFormat {
    /// JSON format.
    #[default]
    Json,
    /// YAML format.
    Yaml,
    /// Archive (zip).
    Archive,
}

/// Options for export.
#[derive(Debug, Default)]
pub struct ExportOptions {
    /// Project directory.
    pub path: Option<PathBuf>,
    /// Output file path.
    pub output: Option<PathBuf>,
    /// Export format.
    pub format: ExportFormat,
    /// Include generated files.
    pub include_generated: bool,
    /// Platforms to include.
    pub platforms: Vec<Platform>,
}

/// Command to export workspace configuration.
pub struct ExportCommand;

impl ExportCommand {
    /// Execute the export command.
    pub fn execute(options: ExportOptions) -> Result<PathBuf> {
        let project_dir = options.path.clone().unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        println!(
            "{} {}Exporting workspace configuration...",
            style("[dx-workspace]").bold().cyan(),
            EXPORT
        );

        // Load workspace config
        let config_path = project_dir.join("dx-workspace.json");
        let config = if config_path.exists() {
            WorkspaceConfig::load(&config_path)?
        } else {
            return Err(crate::Error::ConfigNotFound { path: config_path });
        };

        // Determine output path
        let output_path = options.output.clone().unwrap_or_else(|| {
            let ext = match options.format {
                ExportFormat::Json => "json",
                ExportFormat::Yaml => "yml",
                ExportFormat::Archive => "zip",
            };
            project_dir.join(format!("dx-workspace-export.{}", ext))
        });

        // Export based on format
        match options.format {
            ExportFormat::Json => Self::export_json(&config, &output_path)?,
            ExportFormat::Yaml => Self::export_yaml(&config, &output_path)?,
            ExportFormat::Archive => {
                Self::export_archive(&config, &project_dir, &output_path, &options)?
            }
        }

        println!();
        println!(
            "{} {}Exported to {}",
            style("[dx-workspace]").bold().cyan(),
            CHECK,
            output_path.display()
        );

        Ok(output_path)
    }

    fn export_json(config: &WorkspaceConfig, output_path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| crate::Error::json_parse(output_path, e))?;

        let mut file =
            fs::File::create(output_path).map_err(|e| crate::Error::io(output_path, e))?;

        file.write_all(json.as_bytes()).map_err(|e| crate::Error::io(output_path, e))?;

        println!("  {} Exported as JSON", style(CHECK).green());

        Ok(())
    }

    fn export_yaml(config: &WorkspaceConfig, output_path: &Path) -> Result<()> {
        let yaml =
            serde_yaml::to_string(config).map_err(|e| crate::Error::yaml_parse(output_path, e))?;

        let mut file =
            fs::File::create(output_path).map_err(|e| crate::Error::io(output_path, e))?;

        file.write_all(yaml.as_bytes()).map_err(|e| crate::Error::io(output_path, e))?;

        println!("  {} Exported as YAML", style(CHECK).green());

        Ok(())
    }

    fn export_archive(
        config: &WorkspaceConfig,
        project_dir: &Path,
        output_path: &Path,
        options: &ExportOptions,
    ) -> Result<()> {
        // For now, just export JSON + list what would be included
        println!(
            "  {} Archive export not yet implemented, falling back to JSON",
            style("ℹ").blue()
        );

        let json_path = output_path.with_extension("json");
        Self::export_json(config, &json_path)?;

        if options.include_generated {
            let generator = Generator::with_output_dir(config, project_dir);
            let platforms = if options.platforms.is_empty() {
                Platform::all().into_iter().filter(|p| generator.exists(*p)).collect()
            } else {
                options.platforms.clone()
            };

            println!("  {} Would include {} platform configs", style("ℹ").blue(), platforms.len());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_default() {
        assert_eq!(ExportFormat::default(), ExportFormat::Json);
    }
}
