//! no-duplicate-keys rule
//!
//! Disallow duplicate keys in object literals

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::{ObjectPropertyKind, PropertyKey};
use std::collections::HashSet;

/// Rule: no-duplicate-keys
/// Disallows duplicate keys in object literals
#[derive(Debug, Clone, Default)]
pub struct NoDuplicateKeys;

impl NoDuplicateKeys {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(11),
        name: "no-duplicate-keys",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow duplicate keys in object literals",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-duplicate-keys"),
    };
}

impl Rule for NoDuplicateKeys {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::ObjectExpression(obj) = node {
            let mut seen_keys: HashSet<String> = HashSet::new();

            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(prop) = prop
                    && let Some(key_name) = get_property_key_name(&prop.key)
                    && !seen_keys.insert(key_name.clone())
                {
                    let diagnostic = Diagnostic::error(
                        ctx.file_path.to_path_buf(),
                        Span::from(prop.span),
                        "no-duplicate-keys",
                        format!("Duplicate key '{key_name}'"),
                    )
                    .with_suggestion("Remove the duplicate key or rename it");

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Get the name of a property key
fn get_property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.to_string()),
        PropertyKey::NumericLiteral(lit) => Some(lit.value.to_string()),
        _ => None, // Computed keys are not checked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoDuplicateKeys;
        assert_eq!(rule.meta().name, "no-duplicate-keys");
        assert!(!rule.meta().fixable);
    }
}
