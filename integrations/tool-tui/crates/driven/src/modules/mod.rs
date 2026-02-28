//! # Module System
//!
//! Extensible module system for Driven that allows custom agents, workflows,
//! templates, and resources to be packaged and shared.
//!
//! ## Features
//!
//! - Module installation from path or URL
//! - Dependency resolution and version management
//! - Resource isolation to prevent conflicts
//! - DX LLM format manifests
//!
//! ## Module Manifest Format
//!
//! Modules are defined using DX LLM format in a `module.dx` file:
//!
//! ```text
//! # My Custom Module
//! id|my-module
//! nm|My Custom Module
//! v|1.0.0
//! desc|A custom module for specialized workflows
//! author|Your Name
//! license|MIT
//!
//! # Dependencies
//! dep.base-module|^1.0.0
//!
//! # Resources
//! agent.0|custom-agent
//! workflow.0|custom-workflow
//! template.0|custom-template
//! ```
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use driven::modules::{ModuleManager, Module};
//!
//! // Create a module manager
//! let mut manager = ModuleManager::new(".driven/modules");
//!
//! // Load existing modules
//! manager.load()?;
//!
//! // Install a new module
//! manager.install(Path::new("./my-module"))?;
//!
//! // List installed modules
//! for module in manager.list_installed() {
//!     println!("{}: {} v{}", module.id, module.name, module.version);
//! }
//!
//! // Get all agents from installed modules (namespaced)
//! let agents = manager.get_all_agents();
//! // Returns: ["my-module:custom-agent", ...]
//! ```
//!
//! ## Resource Isolation
//!
//! When isolation is enabled (default), all module resources are namespaced
//! with the module ID to prevent conflicts:
//!
//! - Agent `my-agent` in module `my-module` becomes `my-module:my-agent`
//! - Workflow `my-flow` in module `my-module` becomes `my-module:my-flow`
//!
//! ## Dependency Resolution
//!
//! The module system supports:
//!
//! - Exact version matching: `1.0.0`
//! - Caret requirements: `^1.0.0` (compatible with major version)
//! - Tilde requirements: `~1.0.0` (compatible with minor version)
//! - Wildcard: `*` (any version)
//! - Optional dependencies: marked with `optional: true`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{DrivenError, Result};

#[cfg(test)]
mod property_tests;

/// A Driven module containing agents, workflows, templates, and resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Module {
    /// Unique identifier for the module
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Description of the module
    pub description: String,
    /// Author information
    pub author: Option<String>,
    /// License identifier
    pub license: Option<String>,
    /// Module dependencies
    pub dependencies: Vec<ModuleDependency>,
    /// Agent definitions provided by this module
    pub agents: Vec<String>,
    /// Workflow definitions provided by this module
    pub workflows: Vec<String>,
    /// Template definitions provided by this module
    pub templates: Vec<String>,
    /// Additional resources (files, configs, etc.)
    pub resources: Vec<String>,
    /// Installation path (set after installation)
    #[serde(skip)]
    pub install_path: Option<PathBuf>,
}

impl Module {
    /// Create a new module with required fields
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: None,
            license: None,
            dependencies: Vec::new(),
            agents: Vec::new(),
            workflows: Vec::new(),
            templates: Vec::new(),
            resources: Vec::new(),
            install_path: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a dependency
    pub fn with_dependency(mut self, dep: ModuleDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add an agent
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agents.push(agent.into());
        self
    }

    /// Add a workflow
    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflows.push(workflow.into());
        self
    }

    /// Add a template
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.templates.push(template.into());
        self
    }

    /// Add a resource
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resources.push(resource.into());
        self
    }

    /// Get the namespaced ID for an agent
    pub fn namespaced_agent(&self, agent: &str) -> String {
        format!("{}:{}", self.id, agent)
    }

    /// Get the namespaced ID for a workflow
    pub fn namespaced_workflow(&self, workflow: &str) -> String {
        format!("{}:{}", self.id, workflow)
    }

    /// Get the namespaced ID for a template
    pub fn namespaced_template(&self, template: &str) -> String {
        format!("{}:{}", self.id, template)
    }

    /// Check if this module satisfies a version requirement
    pub fn satisfies_version(&self, version_req: &str) -> bool {
        // Simple version matching - supports exact, ^, ~, and * patterns
        if version_req == "*" {
            return true;
        }
        if version_req.starts_with('^') {
            // Compatible with major version
            let req = version_req.trim_start_matches('^');
            return self.version.starts_with(&req.split('.').next().unwrap_or("0").to_string());
        }
        if version_req.starts_with('~') {
            // Compatible with minor version
            let req = version_req.trim_start_matches('~');
            let parts: Vec<&str> = req.split('.').collect();
            let self_parts: Vec<&str> = self.version.split('.').collect();
            return parts.first() == self_parts.first() && parts.get(1) == self_parts.get(1);
        }
        // Exact match
        self.version == version_req
    }
}

/// A module dependency with version requirement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleDependency {
    /// Module ID of the dependency
    pub module_id: String,
    /// Version requirement (semver-like)
    pub version_req: String,
    /// Whether this dependency is optional
    #[serde(default)]
    pub optional: bool,
}

impl ModuleDependency {
    /// Create a new required dependency
    pub fn new(module_id: impl Into<String>, version_req: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            version_req: version_req.into(),
            optional: false,
        }
    }

    /// Create an optional dependency
    pub fn optional(module_id: impl Into<String>, version_req: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            version_req: version_req.into(),
            optional: true,
        }
    }
}

/// Module installation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Module is installed and ready
    Installed,
    /// Module is being installed
    Installing,
    /// Module has pending updates
    UpdateAvailable,
    /// Module has unmet dependencies
    DependencyError,
    /// Module is disabled
    Disabled,
}

/// Module manager for installing, updating, and managing modules
pub struct ModuleManager {
    /// Installed modules indexed by ID
    installed: HashMap<String, Module>,
    /// Module status tracking
    status: HashMap<String, ModuleStatus>,
    /// Base path for module storage
    registry_path: PathBuf,
    /// Namespace isolation enabled
    isolation_enabled: bool,
}

impl ModuleManager {
    /// Create a new module manager
    pub fn new(registry_path: impl Into<PathBuf>) -> Self {
        Self {
            installed: HashMap::new(),
            status: HashMap::new(),
            registry_path: registry_path.into(),
            isolation_enabled: true,
        }
    }

    /// Load installed modules from the registry path
    pub fn load(&mut self) -> Result<()> {
        let modules_dir = self.registry_path.join("installed");
        if !modules_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&modules_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("module.dx");
                if manifest_path.exists() {
                    match self.load_manifest(&manifest_path) {
                        Ok(mut module) => {
                            module.install_path = Some(path.clone());
                            self.status.insert(module.id.clone(), ModuleStatus::Installed);
                            self.installed.insert(module.id.clone(), module);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to load module at {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Install a module from a path
    pub fn install(&mut self, module_path: &Path) -> Result<()> {
        // Load the module manifest
        let manifest_path = module_path.join("module.dx");
        if !manifest_path.exists() {
            return Err(DrivenError::Config(format!(
                "Module manifest not found at {:?}",
                manifest_path
            )));
        }

        let module = self.load_manifest(&manifest_path)?;

        // Check if already installed
        if self.installed.contains_key(&module.id) {
            return Err(DrivenError::Config(format!(
                "Module '{}' is already installed",
                module.id
            )));
        }

        // Resolve dependencies first
        self.resolve_dependencies(&module)?;

        // Mark as installing
        self.status.insert(module.id.clone(), ModuleStatus::Installing);

        // Create installation directory
        let install_dir = self.registry_path.join("installed").join(&module.id);
        std::fs::create_dir_all(&install_dir)?;

        // Copy module files
        self.copy_module_files(module_path, &install_dir)?;

        // Update module with install path
        let mut installed_module = module.clone();
        installed_module.install_path = Some(install_dir);

        // Register the module
        self.status.insert(installed_module.id.clone(), ModuleStatus::Installed);
        self.installed.insert(installed_module.id.clone(), installed_module);

        Ok(())
    }

    /// Uninstall a module
    pub fn uninstall(&mut self, module_id: &str) -> Result<()> {
        let module = self.installed.get(module_id).ok_or_else(|| {
            DrivenError::Config(format!("Module '{}' is not installed", module_id))
        })?;

        // Check if other modules depend on this one
        for (id, other) in &self.installed {
            if id != module_id {
                for dep in &other.dependencies {
                    if dep.module_id == module_id && !dep.optional {
                        return Err(DrivenError::Config(format!(
                            "Cannot uninstall '{}': module '{}' depends on it",
                            module_id, id
                        )));
                    }
                }
            }
        }

        // Remove installation directory
        if let Some(install_path) = &module.install_path {
            if install_path.exists() {
                std::fs::remove_dir_all(install_path)?;
            }
        }

        // Remove from registry
        self.installed.remove(module_id);
        self.status.remove(module_id);

        Ok(())
    }

    /// Update a module to a new version
    pub fn update(&mut self, module_id: &str, new_module_path: &Path) -> Result<()> {
        // Verify the module is installed
        if !self.installed.contains_key(module_id) {
            return Err(DrivenError::Config(format!("Module '{}' is not installed", module_id)));
        }

        // Load new module manifest
        let manifest_path = new_module_path.join("module.dx");
        let new_module = self.load_manifest(&manifest_path)?;

        // Verify it's the same module
        if new_module.id != module_id {
            return Err(DrivenError::Config(format!(
                "Module ID mismatch: expected '{}', got '{}'",
                module_id, new_module.id
            )));
        }

        // Uninstall old version
        self.uninstall(module_id)?;

        // Install new version
        self.install(new_module_path)?;

        Ok(())
    }

    /// List all installed modules
    pub fn list_installed(&self) -> Vec<&Module> {
        self.installed.values().collect()
    }

    /// Get a specific installed module
    pub fn get(&self, module_id: &str) -> Option<&Module> {
        self.installed.get(module_id)
    }

    /// Get module status
    pub fn get_status(&self, module_id: &str) -> Option<ModuleStatus> {
        self.status.get(module_id).copied()
    }

    /// Resolve dependencies for a module
    pub fn resolve_dependencies(&self, module: &Module) -> Result<Vec<&Module>> {
        let mut resolved = Vec::new();
        let mut to_resolve: Vec<&ModuleDependency> = module.dependencies.iter().collect();
        let mut visited = std::collections::HashSet::new();

        while let Some(dep) = to_resolve.pop() {
            if visited.contains(&dep.module_id) {
                continue;
            }
            visited.insert(dep.module_id.clone());

            match self.installed.get(&dep.module_id) {
                Some(installed) => {
                    if !installed.satisfies_version(&dep.version_req) {
                        return Err(DrivenError::Config(format!(
                            "Dependency '{}' version {} does not satisfy requirement {}",
                            dep.module_id, installed.version, dep.version_req
                        )));
                    }
                    resolved.push(installed);
                    // Add transitive dependencies
                    for trans_dep in &installed.dependencies {
                        to_resolve.push(trans_dep);
                    }
                }
                None if !dep.optional => {
                    return Err(DrivenError::Config(format!(
                        "Required dependency '{}' ({}) is not installed",
                        dep.module_id, dep.version_req
                    )));
                }
                None => {
                    // Optional dependency not installed, skip
                }
            }
        }

        Ok(resolved)
    }

    /// Check if a module is installed
    pub fn is_installed(&self, module_id: &str) -> bool {
        self.installed.contains_key(module_id)
    }

    /// Enable namespace isolation
    pub fn enable_isolation(&mut self) {
        self.isolation_enabled = true;
    }

    /// Disable namespace isolation
    pub fn disable_isolation(&mut self) {
        self.isolation_enabled = false;
    }

    /// Check if isolation is enabled
    pub fn is_isolation_enabled(&self) -> bool {
        self.isolation_enabled
    }

    /// Get all agents from installed modules (with namespacing if enabled)
    pub fn get_all_agents(&self) -> Vec<String> {
        let mut agents = Vec::new();
        for module in self.installed.values() {
            for agent in &module.agents {
                if self.isolation_enabled {
                    agents.push(module.namespaced_agent(agent));
                } else {
                    agents.push(agent.clone());
                }
            }
        }
        agents
    }

    /// Get all workflows from installed modules (with namespacing if enabled)
    pub fn get_all_workflows(&self) -> Vec<String> {
        let mut workflows = Vec::new();
        for module in self.installed.values() {
            for workflow in &module.workflows {
                if self.isolation_enabled {
                    workflows.push(module.namespaced_workflow(workflow));
                } else {
                    workflows.push(workflow.clone());
                }
            }
        }
        workflows
    }

    /// Get all templates from installed modules (with namespacing if enabled)
    pub fn get_all_templates(&self) -> Vec<String> {
        let mut templates = Vec::new();
        for module in self.installed.values() {
            for template in &module.templates {
                if self.isolation_enabled {
                    templates.push(module.namespaced_template(template));
                } else {
                    templates.push(template.clone());
                }
            }
        }
        templates
    }

    // Private helper methods

    fn load_manifest(&self, path: &Path) -> Result<Module> {
        let content = std::fs::read_to_string(path)?;
        self.parse_dx_manifest(&content)
    }

    fn parse_dx_manifest(&self, content: &str) -> Result<Module> {
        // Parse DX LLM format manifest
        let mut module = Module::new("", "", "");

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('|') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "id" => module.id = value.to_string(),
                    "nm" | "name" => module.name = value.to_string(),
                    "v" | "version" => module.version = value.to_string(),
                    "desc" | "description" => module.description = value.to_string(),
                    "author" => module.author = Some(value.to_string()),
                    "license" => module.license = Some(value.to_string()),
                    key if key.starts_with("dep.") => {
                        let dep_id = key.strip_prefix("dep.").unwrap();
                        module.dependencies.push(ModuleDependency::new(dep_id, value));
                    }
                    key if key.starts_with("agent.") => {
                        module.agents.push(value.to_string());
                    }
                    key if key.starts_with("workflow.") => {
                        module.workflows.push(value.to_string());
                    }
                    key if key.starts_with("template.") => {
                        module.templates.push(value.to_string());
                    }
                    key if key.starts_with("resource.") => {
                        module.resources.push(value.to_string());
                    }
                    _ => {}
                }
            }
        }

        if module.id.is_empty() {
            return Err(DrivenError::Config("Module manifest missing 'id' field".to_string()));
        }
        if module.name.is_empty() {
            module.name = module.id.clone();
        }
        if module.version.is_empty() {
            module.version = "0.0.0".to_string();
        }

        Ok(module)
    }

    fn copy_module_files(&self, src: &Path, dest: &Path) -> Result<()> {
        // Copy all files from source to destination
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
                self.copy_module_files(&src_path, &dest_path)?;
            } else {
                std::fs::copy(&src_path, &dest_path)?;
            }
        }
        Ok(())
    }
}

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new(".driven/modules")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = Module::new("test-module", "Test Module", "1.0.0")
            .with_description("A test module")
            .with_agent("test-agent")
            .with_workflow("test-workflow");

        assert_eq!(module.id, "test-module");
        assert_eq!(module.name, "Test Module");
        assert_eq!(module.version, "1.0.0");
        assert_eq!(module.agents.len(), 1);
        assert_eq!(module.workflows.len(), 1);
    }

    #[test]
    fn test_version_satisfaction() {
        let module = Module::new("test", "Test", "1.2.3");

        assert!(module.satisfies_version("1.2.3"));
        assert!(module.satisfies_version("*"));
        assert!(module.satisfies_version("^1"));
        assert!(module.satisfies_version("~1.2"));
        assert!(!module.satisfies_version("2.0.0"));
        assert!(!module.satisfies_version("^2"));
    }

    #[test]
    fn test_namespacing() {
        let module = Module::new("my-module", "My Module", "1.0.0")
            .with_agent("agent1")
            .with_workflow("workflow1");

        assert_eq!(module.namespaced_agent("agent1"), "my-module:agent1");
        assert_eq!(module.namespaced_workflow("workflow1"), "my-module:workflow1");
    }

    #[test]
    fn test_dependency_creation() {
        let dep = ModuleDependency::new("other-module", "^1.0.0");
        assert_eq!(dep.module_id, "other-module");
        assert_eq!(dep.version_req, "^1.0.0");
        assert!(!dep.optional);

        let opt_dep = ModuleDependency::optional("optional-module", "*");
        assert!(opt_dep.optional);
    }

    #[test]
    fn test_module_manager_creation() {
        let manager = ModuleManager::new("/tmp/test-modules");
        assert!(manager.list_installed().is_empty());
        assert!(manager.is_isolation_enabled());
    }
}
