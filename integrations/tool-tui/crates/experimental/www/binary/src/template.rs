//! # Template Dictionary
//!
//! Template definitions with binding metadata.

use crate::opcodes::{Binding, TemplateDef};
use std::collections::HashMap;

/// Template dictionary
#[derive(Debug, Clone)]
pub struct TemplateDictionary {
    /// Template ID -> Definition
    templates: HashMap<u16, TemplateDef>,
}

impl TemplateDictionary {
    /// Create new dictionary
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add template
    pub fn add(&mut self, template: TemplateDef) {
        self.templates.insert(template.id, template);
    }

    /// Get template by ID
    pub fn get(&self, id: u16) -> Option<&TemplateDef> {
        self.templates.get(&id)
    }

    /// Get all templates
    pub fn templates(&self) -> Vec<&TemplateDef> {
        self.templates.values().collect()
    }

    /// Number of templates
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Clear dictionary
    pub fn clear(&mut self) {
        self.templates.clear();
    }
}

impl Default for TemplateDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Template builder helper
pub struct TemplateBuilder {
    id: u16,
    html_string_id: u32,
    bindings: Vec<Binding>,
}

impl TemplateBuilder {
    /// Create new template builder
    pub fn new(id: u16, html_string_id: u32) -> Self {
        Self {
            id,
            html_string_id,
            bindings: Vec::new(),
        }
    }

    /// Add binding
    pub fn add_binding(mut self, binding: Binding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Build template definition
    pub fn build(self) -> TemplateDef {
        TemplateDef {
            id: self.id,
            html_string_id: self.html_string_id,
            bindings: self.bindings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_dictionary() {
        let mut dict = TemplateDictionary::new();

        let template = TemplateBuilder::new(0, 123).build();

        dict.add(template);

        assert_eq!(dict.len(), 1);
        assert!(dict.get(0).is_some());
        assert!(dict.get(999).is_none());
    }

    #[test]
    fn test_template_builder() {
        let template = TemplateBuilder::new(42, 100).build();

        assert_eq!(template.id, 42);
        assert_eq!(template.html_string_id, 100);
        assert_eq!(template.bindings.len(), 0);
    }
}
