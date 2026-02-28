//! Module CLI Command
//!
//! Commands for managing Driven modules: install, uninstall, list, update.

use crate::modules::{Module, ModuleManager, ModuleStatus};
use crate::{DrivenError, Result};
use console::style;
use std::path::{Path, PathBuf};

/// Module command handler
pub struct ModuleCommand {
    /// Module manager instance
    manager: ModuleManager,
    /// Project root path
    #[allow(dead_code)]
    project_root: PathBuf,
}

impl ModuleCommand {
    /// Create a new module command handler
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let project_root = project_root.into();
        let registry_path = project_root.join(".driven/modules");
        Self {
            manager: ModuleManager::new(registry_path),
            project_root,
        }
    }

    /// Initialize and load existing modules
    pub fn init(&mut self) -> Result<()> {
        self.manager.load()
    }

    /// Install a module from a path
    pub fn install(&mut self, module_path: &Path) -> Result<InstallResult> {
        // Ensure the path exists
        if !module_path.exists() {
            return Err(DrivenError::Config(format!(
                "Module path does not exist: {:?}",
                module_path
            )));
        }

        // Install the module
        self.manager.install(module_path)?;

        // Get the installed module info
        let manifest_path = module_path.join("module.dx");
        let content = std::fs::read_to_string(&manifest_path)?;
        let module = self.parse_manifest_for_info(&content)?;

        Ok(InstallResult {
            module_id: module.id,
            module_name: module.name,
            version: module.version,
            agents_count: module.agents.len(),
            workflows_count: module.workflows.len(),
            templates_count: module.templates.len(),
        })
    }

    /// Uninstall a module
    pub fn uninstall(&mut self, module_id: &str) -> Result<UninstallResult> {
        let module = self.manager.get(module_id).ok_or_else(|| {
            DrivenError::Config(format!("Module '{}' is not installed", module_id))
        })?;

        let name = module.name.clone();
        let version = module.version.clone();

        self.manager.uninstall(module_id)?;

        Ok(UninstallResult {
            module_id: module_id.to_string(),
            module_name: name,
            version,
        })
    }

    /// List all installed modules
    pub fn list(&self) -> Vec<ModuleInfo> {
        self.manager
            .list_installed()
            .iter()
            .map(|m| ModuleInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                version: m.version.clone(),
                description: m.description.clone(),
                status: self.manager.get_status(&m.id).unwrap_or(ModuleStatus::Installed),
                agents_count: m.agents.len(),
                workflows_count: m.workflows.len(),
                templates_count: m.templates.len(),
                dependencies_count: m.dependencies.len(),
            })
            .collect()
    }

    /// Update a module
    pub fn update(&mut self, module_id: &str, new_path: &Path) -> Result<UpdateResult> {
        let old_module = self.manager.get(module_id).ok_or_else(|| {
            DrivenError::Config(format!("Module '{}' is not installed", module_id))
        })?;

        let old_version = old_module.version.clone();

        self.manager.update(module_id, new_path)?;

        let new_module = self
            .manager
            .get(module_id)
            .ok_or_else(|| DrivenError::Config("Module not found after update".to_string()))?;

        Ok(UpdateResult {
            module_id: module_id.to_string(),
            module_name: new_module.name.clone(),
            old_version,
            new_version: new_module.version.clone(),
        })
    }

    /// Get detailed info about a specific module
    pub fn info(&self, module_id: &str) -> Result<ModuleDetails> {
        let module = self.manager.get(module_id).ok_or_else(|| {
            DrivenError::Config(format!("Module '{}' is not installed", module_id))
        })?;

        Ok(ModuleDetails {
            id: module.id.clone(),
            name: module.name.clone(),
            version: module.version.clone(),
            description: module.description.clone(),
            author: module.author.clone(),
            license: module.license.clone(),
            status: self.manager.get_status(module_id).unwrap_or(ModuleStatus::Installed),
            agents: module.agents.clone(),
            workflows: module.workflows.clone(),
            templates: module.templates.clone(),
            resources: module.resources.clone(),
            dependencies: module
                .dependencies
                .iter()
                .map(|d| DependencyInfo {
                    module_id: d.module_id.clone(),
                    version_req: d.version_req.clone(),
                    optional: d.optional,
                    installed: self.manager.is_installed(&d.module_id),
                })
                .collect(),
            install_path: module.install_path.clone(),
        })
    }

    /// Search for modules (placeholder for future registry support)
    pub fn search(&self, _query: &str) -> Vec<ModuleInfo> {
        // TODO: Implement module registry search
        Vec::new()
    }

    fn parse_manifest_for_info(&self, content: &str) -> Result<Module> {
        let mut module = Module::new("", "", "");

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('|') {
                match key.trim() {
                    "id" => module.id = value.trim().to_string(),
                    "nm" | "name" => module.name = value.trim().to_string(),
                    "v" | "version" => module.version = value.trim().to_string(),
                    key if key.starts_with("agent.") => {
                        module.agents.push(value.trim().to_string());
                    }
                    key if key.starts_with("workflow.") => {
                        module.workflows.push(value.trim().to_string());
                    }
                    key if key.starts_with("template.") => {
                        module.templates.push(value.trim().to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(module)
    }
}

/// Result of module installation
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub module_id: String,
    pub module_name: String,
    pub version: String,
    pub agents_count: usize,
    pub workflows_count: usize,
    pub templates_count: usize,
}

/// Result of module uninstallation
#[derive(Debug, Clone)]
pub struct UninstallResult {
    pub module_id: String,
    pub module_name: String,
    pub version: String,
}

/// Result of module update
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub module_id: String,
    pub module_name: String,
    pub old_version: String,
    pub new_version: String,
}

/// Summary info about a module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub status: ModuleStatus,
    pub agents_count: usize,
    pub workflows_count: usize,
    pub templates_count: usize,
    pub dependencies_count: usize,
}

/// Detailed info about a module
#[derive(Debug, Clone)]
pub struct ModuleDetails {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub status: ModuleStatus,
    pub agents: Vec<String>,
    pub workflows: Vec<String>,
    pub templates: Vec<String>,
    pub resources: Vec<String>,
    pub dependencies: Vec<DependencyInfo>,
    pub install_path: Option<PathBuf>,
}

/// Info about a dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub module_id: String,
    pub version_req: String,
    pub optional: bool,
    pub installed: bool,
}

/// Print modules in a table format
pub fn print_modules_table(modules: &[ModuleInfo]) {
    if modules.is_empty() {
        println!("{}", style("No modules installed").dim());
        return;
    }

    println!(
        "{:<20} {:<25} {:<10} {:<10} {:<8} {:<8} {:<8}",
        style("ID").bold().underlined(),
        style("Name").bold().underlined(),
        style("Version").bold().underlined(),
        style("Status").bold().underlined(),
        style("Agents").bold().underlined(),
        style("Flows").bold().underlined(),
        style("Tmpls").bold().underlined(),
    );

    for module in modules {
        let status_str = match module.status {
            ModuleStatus::Installed => style("OK").green(),
            ModuleStatus::Installing => style("...").yellow(),
            ModuleStatus::UpdateAvailable => style("UPD").cyan(),
            ModuleStatus::DependencyError => style("ERR").red(),
            ModuleStatus::Disabled => style("OFF").dim(),
        };

        println!(
            "{:<20} {:<25} {:<10} {:<10} {:<8} {:<8} {:<8}",
            module.id,
            truncate(&module.name, 24),
            module.version,
            status_str,
            module.agents_count,
            module.workflows_count,
            module.templates_count,
        );
    }
}

/// Print detailed module info
pub fn print_module_details(details: &ModuleDetails) {
    println!("{}", style(&details.name).bold().cyan());
    println!("{}", style("─".repeat(50)).dim());

    println!("  {} {}", style("ID:").bold(), details.id);
    println!("  {} {}", style("Version:").bold(), details.version);

    if !details.description.is_empty() {
        println!("  {} {}", style("Description:").bold(), details.description);
    }

    if let Some(author) = &details.author {
        println!("  {} {}", style("Author:").bold(), author);
    }

    if let Some(license) = &details.license {
        println!("  {} {}", style("License:").bold(), license);
    }

    let status_str = match details.status {
        ModuleStatus::Installed => style("Installed").green(),
        ModuleStatus::Installing => style("Installing").yellow(),
        ModuleStatus::UpdateAvailable => style("Update Available").cyan(),
        ModuleStatus::DependencyError => style("Dependency Error").red(),
        ModuleStatus::Disabled => style("Disabled").dim(),
    };
    println!("  {} {}", style("Status:").bold(), status_str);

    if let Some(path) = &details.install_path {
        println!("  {} {:?}", style("Path:").bold(), path);
    }

    if !details.agents.is_empty() {
        println!("\n  {} ({})", style("Agents").bold().underlined(), details.agents.len());
        for agent in &details.agents {
            println!("    • {}", agent);
        }
    }

    if !details.workflows.is_empty() {
        println!("\n  {} ({})", style("Workflows").bold().underlined(), details.workflows.len());
        for workflow in &details.workflows {
            println!("    • {}", workflow);
        }
    }

    if !details.templates.is_empty() {
        println!("\n  {} ({})", style("Templates").bold().underlined(), details.templates.len());
        for template in &details.templates {
            println!("    • {}", template);
        }
    }

    if !details.resources.is_empty() {
        println!("\n  {} ({})", style("Resources").bold().underlined(), details.resources.len());
        for resource in &details.resources {
            println!("    • {}", resource);
        }
    }

    if !details.dependencies.is_empty() {
        println!(
            "\n  {} ({})",
            style("Dependencies").bold().underlined(),
            details.dependencies.len()
        );
        for dep in &details.dependencies {
            let status = if dep.installed {
                style("✓").green()
            } else if dep.optional {
                style("○").dim()
            } else {
                style("✗").red()
            };
            let optional_str = if dep.optional { " (optional)" } else { "" };
            println!("    {} {} {}{}", status, dep.module_id, dep.version_req, optional_str);
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_module_command_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cmd = ModuleCommand::new(temp_dir.path());
        assert!(cmd.list().is_empty());
    }

    #[test]
    fn test_module_info_display() {
        let info = ModuleInfo {
            id: "test-module".to_string(),
            name: "Test Module".to_string(),
            version: "1.0.0".to_string(),
            description: "A test module".to_string(),
            status: ModuleStatus::Installed,
            agents_count: 2,
            workflows_count: 3,
            templates_count: 1,
            dependencies_count: 0,
        };

        // Just verify it doesn't panic
        print_modules_table(&[info]);
    }
}
