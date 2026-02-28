//! Generator Integration Module
//!
//! This module provides integration between Driven and the dx-generator crate,
//! enabling template-based rule generation and spec scaffolding.
//!
//! ## Features
//!
//! - Template-based rule generation using dx-generator
//! - Spec scaffolding with generator templates
//! - Driven templates available in generator registry
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::generator_integration::{GeneratorBridge, DrivenTemplateProvider};
//!
//! // Create a generator bridge
//! let bridge = GeneratorBridge::new()?;
//!
//! // Generate rules from a template
//! let rules = bridge.generate_rules("rust-workspace", &params)?;
//!
//! // Generate spec scaffolding
//! let files = bridge.generate_spec_scaffold("feature-001", &params)?;
//! ```

use crate::{DrivenError, Result, UnifiedRule};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Bridge between Driven and dx-generator
///
/// Provides high-level APIs for using generator templates within Driven workflows.
#[derive(Debug)]
pub struct GeneratorBridge {
    /// Template search paths
    template_paths: Vec<PathBuf>,
    /// Cached template metadata
    template_cache: HashMap<String, TemplateInfo>,
    /// Whether the bridge is initialized
    initialized: bool,
}

/// Information about a template
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template category
    pub category: TemplateCategory,
    /// Required parameters
    pub parameters: Vec<ParameterInfo>,
    /// Output file pattern
    pub output_pattern: String,
}

/// Template categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCategory {
    /// Rule templates for AI editors
    Rule,
    /// Spec templates for spec-driven development
    Spec,
    /// Scaffold templates for project structure
    Scaffold,
    /// Component templates
    Component,
    /// Other templates
    Other,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value if any
    pub default: Option<String>,
}

/// Generated file from a template
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Relative path for the file
    pub path: PathBuf,
    /// File content
    pub content: String,
    /// Whether this file should overwrite existing
    pub overwrite: bool,
}

/// Parameters for template generation
#[derive(Debug, Clone, Default)]
pub struct GenerateParams {
    /// Key-value parameters
    params: HashMap<String, ParamValue>,
}

/// Parameter value types
#[derive(Debug, Clone)]
pub enum ParamValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Array of strings
    Array(Vec<String>),
}

impl GenerateParams {
    /// Create new empty parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a string parameter
    pub fn set_string(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), ParamValue::String(value.into()));
        self
    }

    /// Set an integer parameter
    pub fn set_int(mut self, key: &str, value: i64) -> Self {
        self.params.insert(key.to_string(), ParamValue::Integer(value));
        self
    }

    /// Set a boolean parameter
    pub fn set_bool(mut self, key: &str, value: bool) -> Self {
        self.params.insert(key.to_string(), ParamValue::Boolean(value));
        self
    }

    /// Set an array parameter
    pub fn set_array(mut self, key: &str, value: Vec<String>) -> Self {
        self.params.insert(key.to_string(), ParamValue::Array(value));
        self
    }

    /// Get a parameter value
    pub fn get(&self, key: &str) -> Option<&ParamValue> {
        self.params.get(key)
    }

    /// Get a string parameter
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.params.get(key) {
            Some(ParamValue::String(s)) => Some(s),
            _ => None,
        }
    }

    /// Get an integer parameter
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.params.get(key) {
            Some(ParamValue::Integer(i)) => Some(*i),
            _ => None,
        }
    }

    /// Get a boolean parameter
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.params.get(key) {
            Some(ParamValue::Boolean(b)) => Some(*b),
            _ => None,
        }
    }

    /// Convert to a HashMap of strings for template rendering
    pub fn to_string_map(&self) -> HashMap<String, String> {
        self.params
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    ParamValue::String(s) => s.clone(),
                    ParamValue::Integer(i) => i.to_string(),
                    ParamValue::Boolean(b) => b.to_string(),
                    ParamValue::Array(a) => a.join(", "),
                };
                (k.clone(), value)
            })
            .collect()
    }
}

impl GeneratorBridge {
    /// Create a new generator bridge with default template paths
    pub fn new() -> Result<Self> {
        let template_paths = vec![
            PathBuf::from(".driven/templates"),
            PathBuf::from(".dx/templates"),
        ];

        Ok(Self {
            template_paths,
            template_cache: HashMap::new(),
            initialized: false,
        })
    }

    /// Create a generator bridge with custom template paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Result<Self> {
        Ok(Self {
            template_paths: paths,
            template_cache: HashMap::new(),
            initialized: false,
        })
    }

    /// Add a template search path
    pub fn add_path(&mut self, path: impl AsRef<Path>) {
        self.template_paths.push(path.as_ref().to_path_buf());
    }

    /// Initialize the bridge by scanning for templates
    pub fn initialize(&mut self) -> Result<()> {
        self.template_cache.clear();

        // Clone paths to avoid borrow issues
        let paths: Vec<PathBuf> = self.template_paths.clone();

        for path in &paths {
            if path.exists() {
                self.scan_templates(path)?;
            }
        }

        self.initialized = true;
        Ok(())
    }

    /// Scan a directory for templates
    fn scan_templates(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir).map_err(DrivenError::Io)? {
            let entry = entry.map_err(DrivenError::Io)?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "dxt" || ext == "hbs" || ext == "template" {
                        if let Ok(info) = self.parse_template_info(&path) {
                            self.template_cache.insert(info.id.clone(), info);
                        }
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories
                self.scan_templates(&path)?;
            }
        }

        Ok(())
    }

    /// Parse template information from a file
    fn parse_template_info(&self, path: &Path) -> Result<TemplateInfo> {
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DrivenError::Template("Invalid template filename".to_string()))?
            .to_string();

        // For now, create basic info from filename
        // In a full implementation, this would parse template metadata
        Ok(TemplateInfo {
            id: id.clone(),
            name: id.replace(['-', '_'], " "),
            description: format!("Template from {}", path.display()),
            category: self.infer_category(&id),
            parameters: Vec::new(),
            output_pattern: String::new(),
        })
    }

    /// Infer template category from ID
    fn infer_category(&self, id: &str) -> TemplateCategory {
        if id.contains("rule") || id.contains("cursor") || id.contains("copilot") {
            TemplateCategory::Rule
        } else if id.contains("spec") || id.contains("requirement") || id.contains("design") {
            TemplateCategory::Spec
        } else if id.contains("scaffold") || id.contains("project") {
            TemplateCategory::Scaffold
        } else if id.contains("component") || id.contains("model") {
            TemplateCategory::Component
        } else {
            TemplateCategory::Other
        }
    }

    /// List available templates
    pub fn list_templates(&self) -> Vec<&TemplateInfo> {
        self.template_cache.values().collect()
    }

    /// List templates by category
    pub fn list_templates_by_category(&self, category: TemplateCategory) -> Vec<&TemplateInfo> {
        self.template_cache.values().filter(|t| t.category == category).collect()
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&TemplateInfo> {
        self.template_cache.get(id)
    }

    /// Generate rules from a template
    ///
    /// Uses the generator to render a rule template and parse the result
    /// into unified rules.
    pub fn generate_rules(
        &self,
        template_id: &str,
        params: &GenerateParams,
    ) -> Result<Vec<UnifiedRule>> {
        let _template = self
            .get_template(template_id)
            .ok_or_else(|| DrivenError::TemplateNotFound(template_id.to_string()))?;

        // In a full implementation, this would:
        // 1. Load the template using dx-generator
        // 2. Render with the provided parameters
        // 3. Parse the output as rules

        // For now, return a placeholder
        let _params_map = params.to_string_map();

        Ok(Vec::new())
    }

    /// Generate spec scaffolding
    ///
    /// Creates the directory structure and initial files for a new spec.
    pub fn generate_spec_scaffold(
        &self,
        spec_id: &str,
        params: &GenerateParams,
    ) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Get spec name from params or use ID
        let spec_name = params.get_string("name").unwrap_or(spec_id);

        // Generate requirements.md
        files.push(GeneratedFile {
            path: PathBuf::from(format!(".driven/specs/{}/requirements.md", spec_id)),
            content: self.generate_requirements_template(spec_name, params),
            overwrite: false,
        });

        // Generate design.md
        files.push(GeneratedFile {
            path: PathBuf::from(format!(".driven/specs/{}/design.md", spec_id)),
            content: self.generate_design_template(spec_name, params),
            overwrite: false,
        });

        // Generate tasks.md
        files.push(GeneratedFile {
            path: PathBuf::from(format!(".driven/specs/{}/tasks.md", spec_id)),
            content: self.generate_tasks_template(spec_name, params),
            overwrite: false,
        });

        Ok(files)
    }

    /// Generate requirements template content
    fn generate_requirements_template(&self, name: &str, _params: &GenerateParams) -> String {
        format!(
            r#"# Requirements Document

## Introduction

{name}

## Glossary

- **System**: [Definition]

## Requirements

### Requirement 1

**User Story:** As a [role], I want [feature], so that [benefit]

#### Acceptance Criteria

1. WHEN [event], THE [System] SHALL [response]
"#,
            name = name
        )
    }

    /// Generate design template content
    fn generate_design_template(&self, name: &str, _params: &GenerateParams) -> String {
        format!(
            r#"# Design Document: {name}

## Overview

[Summary of the design]

## Architecture

[Architecture description]

## Components and Interfaces

[Component descriptions]

## Data Models

[Data model definitions]

## Correctness Properties

[Properties to validate]

## Error Handling

[Error handling strategy]

## Testing Strategy

[Testing approach]
"#,
            name = name
        )
    }

    /// Generate tasks template content
    fn generate_tasks_template(&self, name: &str, _params: &GenerateParams) -> String {
        format!(
            r#"# Implementation Plan: {name}

## Overview

[Implementation approach]

## Tasks

- [ ] 1. Set up project structure
  - [ ] 1.1 Create directory structure
  - [ ] 1.2 Define core interfaces
  - _Requirements: 1.1_

- [ ] 2. Implement core functionality
  - [ ] 2.1 Implement main logic
  - _Requirements: 1.2_

- [ ] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional
- Each task references specific requirements for traceability
"#,
            name = name
        )
    }

    /// Write generated files to disk
    pub fn write_files(&self, files: &[GeneratedFile]) -> Result<Vec<PathBuf>> {
        let mut written = Vec::new();

        for file in files {
            // Create parent directories
            if let Some(parent) = file.path.parent() {
                std::fs::create_dir_all(parent).map_err(DrivenError::Io)?;
            }

            // Check if file exists and overwrite flag
            if file.path.exists() && !file.overwrite {
                continue;
            }

            // Write the file
            std::fs::write(&file.path, &file.content).map_err(DrivenError::Io)?;

            written.push(file.path.clone());
        }

        Ok(written)
    }
}

impl Default for GeneratorBridge {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            template_paths: Vec::new(),
            template_cache: HashMap::new(),
            initialized: false,
        })
    }
}

/// Driven template provider for dx-generator
///
/// Makes Driven templates available in the generator registry.
#[derive(Debug, Default)]
pub struct DrivenTemplateProvider {
    /// Registered templates
    templates: HashMap<String, DrivenTemplate>,
}

/// A Driven-specific template
#[derive(Debug, Clone)]
pub struct DrivenTemplate {
    /// Template ID
    pub id: String,
    /// Template content
    pub content: String,
    /// Template type
    pub template_type: DrivenTemplateType,
}

/// Types of Driven templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrivenTemplateType {
    /// Rule template for AI editors
    Rule,
    /// Spec template
    Spec,
    /// Hook template
    Hook,
    /// Steering template
    Steering,
}

impl DrivenTemplateProvider {
    /// Create a new template provider
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a template
    pub fn register(&mut self, template: DrivenTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Get a template by ID
    pub fn get(&self, id: &str) -> Option<&DrivenTemplate> {
        self.templates.get(id)
    }

    /// List all templates
    pub fn list(&self) -> Vec<&DrivenTemplate> {
        self.templates.values().collect()
    }

    /// Register built-in templates
    pub fn register_builtins(&mut self) {
        // Register rule templates
        self.register(DrivenTemplate {
            id: "driven-rule-basic".to_string(),
            content: include_str!("templates/rule-basic.md").to_string(),
            template_type: DrivenTemplateType::Rule,
        });

        // Register spec templates
        self.register(DrivenTemplate {
            id: "driven-spec-requirements".to_string(),
            content: include_str!("templates/spec-requirements.md").to_string(),
            template_type: DrivenTemplateType::Spec,
        });

        // Register hook templates
        self.register(DrivenTemplate {
            id: "driven-hook-file-save".to_string(),
            content: include_str!("templates/hook-file-save.toml").to_string(),
            template_type: DrivenTemplateType::Hook,
        });

        // Register steering templates
        self.register(DrivenTemplate {
            id: "driven-steering-always".to_string(),
            content: include_str!("templates/steering-always.md").to_string(),
            template_type: DrivenTemplateType::Steering,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_params() {
        let params = GenerateParams::new()
            .set_string("name", "TestFeature")
            .set_int("version", 1)
            .set_bool("enabled", true);

        assert_eq!(params.get_string("name"), Some("TestFeature"));
        assert_eq!(params.get_int("version"), Some(1));
        assert_eq!(params.get_bool("enabled"), Some(true));
    }

    #[test]
    fn test_params_to_string_map() {
        let params = GenerateParams::new()
            .set_string("name", "Test")
            .set_int("count", 42)
            .set_bool("flag", true);

        let map = params.to_string_map();
        assert_eq!(map.get("name"), Some(&"Test".to_string()));
        assert_eq!(map.get("count"), Some(&"42".to_string()));
        assert_eq!(map.get("flag"), Some(&"true".to_string()));
    }

    #[test]
    fn test_generator_bridge_creation() {
        let bridge = GeneratorBridge::new();
        assert!(bridge.is_ok());
    }

    #[test]
    fn test_infer_category() {
        let bridge = GeneratorBridge::new().unwrap();

        assert_eq!(bridge.infer_category("cursor-rules"), TemplateCategory::Rule);
        assert_eq!(bridge.infer_category("spec-requirements"), TemplateCategory::Spec);
        assert_eq!(bridge.infer_category("project-scaffold"), TemplateCategory::Scaffold);
        assert_eq!(bridge.infer_category("react-component"), TemplateCategory::Component);
        assert_eq!(bridge.infer_category("other-template"), TemplateCategory::Other);
    }

    #[test]
    fn test_spec_scaffold_generation() {
        let bridge = GeneratorBridge::new().unwrap();
        let params = GenerateParams::new().set_string("name", "Test Feature");

        let files = bridge.generate_spec_scaffold("001", &params).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.path.to_string_lossy().contains("requirements.md")));
        assert!(files.iter().any(|f| f.path.to_string_lossy().contains("design.md")));
        assert!(files.iter().any(|f| f.path.to_string_lossy().contains("tasks.md")));
    }

    #[test]
    fn test_driven_template_provider() {
        let mut provider = DrivenTemplateProvider::new();

        provider.register(DrivenTemplate {
            id: "test-template".to_string(),
            content: "Test content".to_string(),
            template_type: DrivenTemplateType::Rule,
        });

        assert!(provider.get("test-template").is_some());
        assert_eq!(provider.list().len(), 1);
    }
}
