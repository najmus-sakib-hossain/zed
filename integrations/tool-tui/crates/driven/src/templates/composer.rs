//! Template composition engine

use super::TemplateRegistry;
use crate::{Result, parser::UnifiedRule};

/// Composes multiple templates into a unified rule set
#[derive(Debug)]
pub struct TemplateComposer<'a> {
    registry: &'a TemplateRegistry,
}

impl<'a> TemplateComposer<'a> {
    /// Create a new composer with a template registry
    pub fn new(registry: &'a TemplateRegistry) -> Self {
        Self { registry }
    }

    /// Compose multiple templates by name into a unified rule set
    pub fn compose(&self, template_names: &[&str]) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();

        for name in template_names {
            if let Some(template) = self.registry.get(name) {
                rules.extend(template.expand()?);
            }
        }

        // Deduplicate rules
        self.deduplicate(&mut rules);

        Ok(rules)
    }

    /// Compose templates with conflict resolution
    pub fn compose_with_priority(&self, template_names: &[&str]) -> Result<Vec<UnifiedRule>> {
        let mut all_rules: Vec<(usize, UnifiedRule)> = Vec::new();

        // Later templates have higher priority
        for (priority, name) in template_names.iter().enumerate() {
            if let Some(template) = self.registry.get(name) {
                for rule in template.expand()? {
                    all_rules.push((priority, rule));
                }
            }
        }

        // Sort by priority (higher priority = later in list)
        all_rules.sort_by_key(|(p, _)| std::cmp::Reverse(*p));

        let rules: Vec<UnifiedRule> = all_rules.into_iter().map(|(_, r)| r).collect();
        Ok(rules)
    }

    /// Remove duplicate rules
    fn deduplicate(&self, rules: &mut Vec<UnifiedRule>) {
        let mut seen_descriptions = std::collections::HashSet::new();
        rules.retain(|rule| {
            if let UnifiedRule::Standard { description, .. } = rule {
                seen_descriptions.insert(description.clone())
            } else {
                true
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_empty() {
        let registry = TemplateRegistry::new();
        let composer = TemplateComposer::new(&registry);

        let rules = composer.compose(&[]).unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_compose_single() {
        let registry = TemplateRegistry::new();
        let composer = TemplateComposer::new(&registry);

        let rules = composer.compose(&["architect"]).unwrap();
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_compose_multiple() {
        let registry = TemplateRegistry::new();
        let composer = TemplateComposer::new(&registry);

        let rules = composer.compose(&["architect", "rust-idioms", "testing"]).unwrap();
        assert!(!rules.is_empty());
    }
}
