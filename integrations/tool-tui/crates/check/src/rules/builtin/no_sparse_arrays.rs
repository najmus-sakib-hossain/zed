//! no-sparse-arrays rule
//!
//! Disallow sparse arrays (arrays with empty slots)

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;

/// Rule: no-sparse-arrays
/// Disallows sparse arrays (arrays with empty slots like [1,,3])
#[derive(Debug, Clone, Default)]
pub struct NoSparseArrays;

impl NoSparseArrays {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(15),
        name: "no-sparse-arrays",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow sparse arrays",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-sparse-arrays"),
    };
}

impl Rule for NoSparseArrays {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::ArrayExpression(arr) = node {
            // Check for elision (empty slots)
            let has_elision =
                arr.elements.iter().any(oxc_ast::ast::ArrayExpressionElement::is_elision);

            if has_elision {
                let diagnostic = Diagnostic::error(
                    ctx.file_path.to_path_buf(),
                    Span::from(arr.span),
                    "no-sparse-arrays",
                    "Unexpected comma in middle of array",
                )
                .with_suggestion("Use explicit undefined values instead of empty slots");

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
        let rule = NoSparseArrays;
        assert_eq!(rule.meta().name, "no-sparse-arrays");
        assert!(!rule.meta().fixable);
    }
}
