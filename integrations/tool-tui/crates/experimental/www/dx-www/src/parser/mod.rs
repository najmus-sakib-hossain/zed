//! # Component Parser
//!
//! This module implements the parser for `.pg` (page) and `.cp` (component) files.
//! It extracts `<script>`, `<template>`, and `<style>` sections from component files.
//!
//! ## File Format
//!
//! ```html
//! <script lang="rust">
//! // Component logic
//! </script>
//!
//! <template>
//! <!-- HTML template -->
//! </template>
//!
//! <style>
//! /* CSS styles */
//! </style>
//! ```

pub mod script;
pub mod style;
pub mod template;

pub use script::{ParsedScript, ScriptParser};
pub use style::{AtRule, CssDeclaration, CssRule, ParsedStyle, StyleParser};
pub use template::{
    Attribute, AttributeValue, ComponentRef, DirectiveNode, ElementNode, ParsedTemplate,
    TemplateDirective, TemplateNode, TemplateParser,
};

use regex::Regex;
use std::path::{Path, PathBuf};

use crate::config::ScriptLanguage;
use crate::error::{DxError, DxResult};

// =============================================================================
// Parsed Component
// =============================================================================

/// A fully parsed component file.
#[derive(Debug, Clone)]
pub struct ParsedComponent {
    /// Original file path
    pub file_path: PathBuf,

    /// Component type
    pub component_type: ComponentType,

    /// Component name (derived from filename)
    pub name: String,

    /// Parsed script section
    pub script: Option<ParsedScript>,

    /// Parsed template section
    pub template: ParsedTemplate,

    /// Parsed style section
    pub style: Option<ParsedStyle>,

    /// Whether this component has a data loader
    pub has_data_loader: bool,

    /// Exported functions/types
    pub exports: Vec<Export>,
}

/// Type of component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// A page component (.pg)
    Page,
    /// A reusable component (.cp)
    Component,
    /// A layout component (_layout.pg)
    Layout,
}

impl ComponentType {
    /// Determine component type from file path.
    pub fn from_path(path: &Path) -> Self {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let file_name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");

        if file_name == "_layout" {
            Self::Layout
        } else if extension == "pg" {
            Self::Page
        } else {
            Self::Component
        }
    }
}

/// An exported item from a component.
#[derive(Debug, Clone)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
}

/// Kind of export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// A function
    Function,
    /// A type/struct
    Type,
    /// A constant
    Const,
    /// The Props type
    Props,
    /// The data loader function
    DataLoader,
}

// =============================================================================
// Component Parser
// =============================================================================

/// Parser for `.pg` and `.cp` component files.
#[derive(Debug)]
pub struct ComponentParser {
    /// Script parser
    script_parser: ScriptParser,
    /// Template parser
    template_parser: TemplateParser,
    /// Style parser
    style_parser: StyleParser,
    /// Regex for extracting sections
    section_regex: SectionRegex,
}

/// Pre-compiled regexes for section extraction.
#[derive(Debug)]
struct SectionRegex {
    script: Regex,
    template: Regex,
    style: Regex,
}

impl Default for SectionRegex {
    fn default() -> Self {
        Self {
            script: Regex::new(r#"(?s)<script(?:\s+lang="([^"]*)")?\s*>(.*?)</script>"#)
                .expect("Invalid script regex"),
            template: Regex::new(r"(?s)<template\s*>(.*?)</template>")
                .expect("Invalid template regex"),
            style: Regex::new(r#"(?s)<style(?:\s+(scoped))?\s*>(.*?)</style>"#)
                .expect("Invalid style regex"),
        }
    }
}

impl Default for ComponentParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentParser {
    /// Create a new component parser.
    pub fn new() -> Self {
        Self {
            script_parser: ScriptParser::new(),
            template_parser: TemplateParser::new(),
            style_parser: StyleParser::new(),
            section_regex: SectionRegex::default(),
        }
    }

    /// Parse a component file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the component file
    ///
    /// # Returns
    ///
    /// The parsed component
    pub fn parse_file(&self, path: &Path) -> DxResult<ParsedComponent> {
        let content = std::fs::read_to_string(path).map_err(|e| DxError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        self.parse(path, &content)
    }

    /// Parse component source code.
    ///
    /// # Arguments
    ///
    /// * `path` - Original file path (for error reporting)
    /// * `content` - Component source code
    ///
    /// # Returns
    ///
    /// The parsed component
    pub fn parse(&self, path: &Path, content: &str) -> DxResult<ParsedComponent> {
        let component_type = ComponentType::from_path(path);
        let name = self.extract_name(path);

        // Extract sections
        let script = self.extract_script(content, path)?;
        let template = self.extract_template(content, path)?;
        let style = self.extract_style(content, path)?;

        // Check for data loader
        let has_data_loader = script.as_ref().map(|s| s.has_data_loader).unwrap_or(false);

        // Collect exports
        let exports = script.as_ref().map(|s| s.exports.clone()).unwrap_or_default();

        Ok(ParsedComponent {
            file_path: path.to_path_buf(),
            component_type,
            name,
            script,
            template,
            style,
            has_data_loader,
            exports,
        })
    }

    /// Extract component name from file path.
    fn extract_name(&self, path: &Path) -> String {
        path.file_stem().and_then(|n| n.to_str()).unwrap_or("Component").to_string()
    }

    /// Extract and parse the script section.
    fn extract_script(&self, content: &str, path: &Path) -> DxResult<Option<ParsedScript>> {
        if let Some(captures) = self.section_regex.script.captures(content) {
            let lang_str = captures.get(1).map(|m| m.as_str()).unwrap_or("rust");
            let language = ScriptLanguage::from_str(lang_str).ok_or_else(|| {
                DxError::InvalidScriptLanguage {
                    language: lang_str.to_string(),
                    file: path.to_path_buf(),
                }
            })?;

            let source = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let script = self.script_parser.parse(source, language, path)?;
            Ok(Some(script))
        } else {
            Ok(None)
        }
    }

    /// Extract and parse the template section.
    fn extract_template(&self, content: &str, path: &Path) -> DxResult<ParsedTemplate> {
        if let Some(captures) = self.section_regex.template.captures(content) {
            let source = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            self.template_parser.parse(source, path)
        } else {
            Err(DxError::MissingSection {
                section: "template".to_string(),
                file: path.to_path_buf(),
            })
        }
    }

    /// Extract and parse the style section.
    fn extract_style(&self, content: &str, path: &Path) -> DxResult<Option<ParsedStyle>> {
        if let Some(captures) = self.section_regex.style.captures(content) {
            let scoped = captures.get(1).is_some();
            let source = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let style = self.style_parser.parse(source, scoped, path)?;
            Ok(Some(style))
        } else {
            Ok(None)
        }
    }
}

// =============================================================================
// Validation
// =============================================================================

/// Validate component naming conventions.
pub fn validate_component_name(name: &str, component_type: ComponentType) -> DxResult<()> {
    match component_type {
        ComponentType::Component => {
            // Components should be PascalCase
            if !name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                return Err(DxError::ParseError {
                    message: format!("Component names should be PascalCase, got: {name}"),
                    file: PathBuf::new(),
                    line: None,
                    column: None,
                    src: None,
                    span: None,
                });
            }
        }
        ComponentType::Page => {
            // Pages should be kebab-case or lowercase
            if name.chars().any(|c| c.is_uppercase()) && !name.starts_with('_') {
                return Err(DxError::ParseError {
                    message: format!("Page names should be kebab-case or lowercase, got: {name}"),
                    file: PathBuf::new(),
                    line: None,
                    column: None,
                    src: None,
                    span: None,
                });
            }
        }
        ComponentType::Layout => {
            // Layouts must be named _layout
            if name != "_layout" {
                return Err(DxError::ParseError {
                    message: format!("Layout files must be named _layout, got: {name}"),
                    file: PathBuf::new(),
                    line: None,
                    column: None,
                    src: None,
                    span: None,
                });
            }
        }
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_COMPONENT: &str = r#"
<script lang="rust">
pub struct Props {
    title: String,
}

pub async fn load() -> Props {
    Props { title: "Hello".into() }
}
</script>

<template>
  <h1>{title}</h1>
</template>

<style>
h1 { color: blue; }
</style>
"#;

    #[test]
    fn test_parse_component() {
        let parser = ComponentParser::new();
        let result = parser.parse(Path::new("test.pg"), SAMPLE_COMPONENT);
        assert!(result.is_ok());

        let component = result.unwrap();
        assert!(component.script.is_some());
        assert!(component.style.is_some());
        assert_eq!(component.component_type, ComponentType::Page);
    }

    #[test]
    fn test_component_type_from_path() {
        assert_eq!(ComponentType::from_path(Path::new("about.pg")), ComponentType::Page);
        assert_eq!(ComponentType::from_path(Path::new("Button.cp")), ComponentType::Component);
        assert_eq!(ComponentType::from_path(Path::new("_layout.pg")), ComponentType::Layout);
    }

    #[test]
    fn test_missing_template_error() {
        let parser = ComponentParser::new();
        let content = r#"
<script lang="rust">
// code
</script>
"#;
        let result = parser.parse(Path::new("test.pg"), content);
        assert!(matches!(result, Err(DxError::MissingSection { .. })));
    }

    #[test]
    fn test_validate_component_name() {
        // Valid component name
        assert!(validate_component_name("Button", ComponentType::Component).is_ok());
        assert!(validate_component_name("MyComponent", ComponentType::Component).is_ok());

        // Invalid component name
        assert!(validate_component_name("button", ComponentType::Component).is_err());

        // Valid page name
        assert!(validate_component_name("about", ComponentType::Page).is_ok());
        assert!(validate_component_name("my-page", ComponentType::Page).is_ok());

        // Valid layout name
        assert!(validate_component_name("_layout", ComponentType::Layout).is_ok());

        // Invalid layout name
        assert!(validate_component_name("layout", ComponentType::Layout).is_err());
    }
}
