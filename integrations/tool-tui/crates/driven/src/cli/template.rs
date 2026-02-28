//! Template command - manage rule templates

use crate::{Result, templates::TemplateRegistry};
use dialoguer::{Select, theme::ColorfulTheme};
use std::path::Path;

/// Template command handler
#[derive(Debug)]
pub struct TemplateCommand;

impl TemplateCommand {
    /// List available templates
    pub fn list() -> Result<()> {
        let registry = TemplateRegistry::new();
        let templates = registry.list();

        println!("📦 Available Templates:");
        println!();

        for template in templates {
            println!("  {} - {}", template.name, template.description);
        }

        println!();
        super::print_info("Use 'driven template apply <name>' to apply a template");

        Ok(())
    }

    /// Search templates
    pub fn search(query: &str) -> Result<()> {
        let registry = TemplateRegistry::new();
        let results = registry.search(query);

        if results.is_empty() {
            super::print_warning(&format!("No templates matching '{}'", query));
        } else {
            println!("🔍 Templates matching '{}':", query);
            println!();

            for template in results {
                println!("  {} - {}", template.name, template.description);
            }
        }

        Ok(())
    }

    /// Apply a template
    pub fn apply(project_root: &Path, template_name: &str) -> Result<()> {
        let registry = TemplateRegistry::new();

        let template = registry.get(template_name).ok_or_else(|| {
            crate::DrivenError::Template(format!("Template not found: {}", template_name))
        })?;

        let spinner = super::create_spinner(&format!("Applying template '{}'...", template_name));

        // Get the content by rendering the template
        let content = template.render(&Default::default())?;

        // Write to rules file
        let rules_path = project_root.join(".driven/rules.md");
        if let Some(parent) = rules_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::DrivenError::Template(format!("Failed to create directory: {}", e))
            })?;
        }

        std::fs::write(&rules_path, content).map_err(|e| {
            crate::DrivenError::Template(format!("Failed to write template: {}", e))
        })?;

        spinner.finish_and_clear();
        super::print_success(&format!(
            "Applied template '{}' to {}",
            template_name,
            rules_path.display()
        ));

        Ok(())
    }

    /// Interactive template selection
    pub fn interactive(project_root: &Path) -> Result<()> {
        let registry = TemplateRegistry::new();
        let templates = registry.list();
        let theme = ColorfulTheme::default();

        let items: Vec<_> =
            templates.iter().map(|t| format!("{} - {}", t.name, t.description)).collect();

        let selection = Select::with_theme(&theme)
            .with_prompt("Select a template")
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| crate::DrivenError::Cli(e.to_string()))?;

        Self::apply(project_root, &templates[selection].name)
    }
}
