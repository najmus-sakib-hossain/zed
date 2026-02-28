//! no-empty rule
//!
//! Disallow empty block statements

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;

/// Rule: no-empty
/// Disallows empty block statements
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct NoEmpty {
    /// Allow empty catch blocks
    allow_empty_catch: bool,
}

impl NoEmpty {
    #[must_use]
    pub fn new(allow_empty_catch: bool) -> Self {
        Self { allow_empty_catch }
    }

    const META: RuleMeta = RuleMeta {
        id: RuleId::new(10),
        name: "no-empty",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow empty block statements",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-empty"),
    };
}

impl Rule for NoEmpty {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::BlockStatement(block) = node {
            // Check if block is empty (no statements)
            if block.body.is_empty() {
                // Check if this is a catch block and we allow empty catch
                // For now, we report all empty blocks
                // A full implementation would check the parent node

                let diagnostic = Diagnostic::error(
                    ctx.file_path.to_path_buf(),
                    Span::from(block.span),
                    "no-empty",
                    "Empty block statement",
                )
                .with_suggestion("Add a comment or code to the block, or remove it");

                ctx.report(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoEmpty::default();
        assert_eq!(rule.meta().name, "no-empty");
        assert!(!rule.meta().fixable);
    }
}
