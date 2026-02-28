//! CSS generation phase of the rebuild pipeline
//!
//! This module generates CSS rules for the given classes using
//! the style engine and group registry.

use ahash::AHashSet;

use crate::core::engine::StyleEngine;
use crate::core::group::GroupRegistry;

use super::types::{CssRule, GeneratedCss};

/// Generate CSS for the given classes
///
/// This function generates CSS rules for all classes in the set,
/// using the style engine for utility classes and the group registry
/// for grouped classes.
///
/// # Arguments
///
/// * `classes` - The set of class names to generate CSS for
/// * `registry` - The group registry for grouped classes (mutable for CSS generation)
/// * `engine` - The style engine for utility classes
///
/// # Returns
///
/// A `GeneratedCss` struct containing all generated CSS rules
/// and the total byte count.
#[allow(dead_code)]
pub fn generate_css(
    classes: &AHashSet<String>,
    registry: &mut GroupRegistry,
    engine: &StyleEngine,
) -> GeneratedCss {
    let mut rules = Vec::with_capacity(classes.len());
    let mut total_bytes = 0;

    for class in classes {
        // Skip internal tokens
        if registry.is_internal_token(class) {
            continue;
        }

        // Try to get CSS from group registry first, then from engine
        let css = if let Some(alias_css) = registry.generate_css_for(class, engine) {
            alias_css.to_string()
        } else if let Some(css) = engine.css_for_class(class) {
            css
        } else {
            continue;
        };

        total_bytes += css.len();
        rules.push(CssRule {
            class_name: class.clone(),
            css,
        });
    }

    GeneratedCss { rules, total_bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_css_empty() {
        let classes = AHashSet::new();
        let mut registry = GroupRegistry::default();
        let engine = StyleEngine::empty();

        let result = generate_css(&classes, &mut registry, &engine);

        assert!(result.rules.is_empty());
        assert_eq!(result.total_bytes, 0);
    }

    #[test]
    fn test_generate_css_skips_internal_tokens() {
        let mut classes = AHashSet::new();
        classes.insert("__internal__".to_string());

        let mut registry = GroupRegistry::default();
        let engine = StyleEngine::empty();

        let result = generate_css(&classes, &mut registry, &engine);

        // Internal tokens should be skipped
        assert!(result.rules.is_empty());
    }
}
