//! Template registry for discovery and loading

use super::{Template, TemplateCategory, builtin};
use crate::{DrivenError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Information about a template
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category
    pub category: TemplateCategory,
}

/// Registry for discovering and loading templates
#[derive(Default)]
pub struct TemplateRegistry {
    /// Templates indexed by name
    templates: HashMap<String, Box<dyn Template>>,
    /// Custom template paths
    custom_paths: Vec<std::path::PathBuf>,
}

impl TemplateRegistry {
    /// Create a new registry with built-in templates
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.load_builtins();
        registry
    }

    /// Create an empty registry (no built-ins)
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load all built-in templates
    pub fn load_builtins(&mut self) {
        for template in builtin::all() {
            self.templates.insert(template.name().to_string(), template);
        }
    }

    /// Add a custom template path
    pub fn add_path(&mut self, path: impl AsRef<Path>) {
        self.custom_paths.push(path.as_ref().to_path_buf());
    }

    /// Register a template
    pub fn register(&mut self, template: Box<dyn Template>) {
        self.templates.insert(template.name().to_string(), template);
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&dyn Template> {
        self.templates.get(name).map(|t| t.as_ref())
    }

    /// List all templates with info
    pub fn list(&self) -> Vec<TemplateInfo> {
        self.templates
            .values()
            .map(|t| TemplateInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                category: t.category(),
            })
            .collect()
    }

    /// Search templates by name or description
    pub fn search(&self, query: &str) -> Vec<TemplateInfo> {
        let query_lower = query.to_lowercase();
        self.templates
            .values()
            .filter(|t| {
                t.name().to_lowercase().contains(&query_lower)
                    || t.description().to_lowercase().contains(&query_lower)
            })
            .map(|t| TemplateInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                category: t.category(),
            })
            .collect()
    }

    /// List templates by category
    pub fn list_by_category(&self, category: TemplateCategory) -> Vec<&dyn Template> {
        self.templates
            .values()
            .filter(|t| t.category() == category)
            .map(|t| t.as_ref())
            .collect()
    }

    /// Search templates by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<&dyn Template> {
        let tag_lower = tag.to_lowercase();
        self.templates
            .values()
            .filter(|t| t.tags().iter().any(|t| t.to_lowercase().contains(&tag_lower)))
            .map(|t| t.as_ref())
            .collect()
    }

    /// Load template from file
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        // For now, just check if it's a .drv file
        if path.extension().is_some_and(|ext| ext == "drv") {
            // TODO: Load binary template
            return Err(DrivenError::TemplateNotFound(format!(
                "Binary template loading not yet implemented: {}",
                path.display()
            )));
        }

        Err(DrivenError::TemplateNotFound(format!(
            "Unsupported template format: {}",
            path.display()
        )))
    }

    /// Get template count
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl std::fmt::Debug for TemplateRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateRegistry")
            .field("template_count", &self.templates.len())
            .field("custom_paths", &self.custom_paths)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry() {
        let registry = TemplateRegistry::new();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_empty_registry() {
        let registry = TemplateRegistry::empty();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_list_templates() {
        let registry = TemplateRegistry::new();
        let names = registry.list();
        assert!(!names.is_empty());
    }

    #[test]
    fn test_get_template() {
        let registry = TemplateRegistry::new();
        let template = registry.get("architect");
        assert!(template.is_some());
    }

    #[test]
    fn test_list_by_category() {
        let registry = TemplateRegistry::new();
        let personas = registry.list_by_category(TemplateCategory::Persona);
        assert!(!personas.is_empty());
    }
}
