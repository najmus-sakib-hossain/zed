//! no-with rule
//!
//! Disallow with statements

use crate::diagnostics::Diagnostic;
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;

/// Rule: no-with
/// Disallows with statements (deprecated in strict mode)
#[derive(Debug, Clone, Default)]
pub struct NoWith;

impl NoWith {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(8),
        name: "no-with",
        category: Category::Suspicious,
        default_severity: Severity::Error,
        description: "Disallow with statements",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-with"),
    };
}

impl Rule for NoWith {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::WithStatement(stmt) = node {
            let diagnostic = Diagnostic::error(
                ctx.file_path.to_path_buf(),
                stmt.span.into(),
                "no-with",
                "'with' statements are not allowed in strict mode",
            )
            .with_suggestion("Use a local variable instead: const obj = expression; obj.property");

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoWith;
        assert_eq!(rule.meta().name, "no-with");
        assert!(!rule.meta().fixable);
    }
}
