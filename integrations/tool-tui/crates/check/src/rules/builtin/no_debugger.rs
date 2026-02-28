//! no-debugger rule
//!
//! Disallow the use of debugger statements

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;

/// Rule: no-debugger
/// Disallows debugger statements
#[derive(Debug, Clone, Default)]
pub struct NoDebugger;

impl NoDebugger {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(2),
        name: "no-debugger",
        category: Category::Suspicious,
        default_severity: Severity::Error,
        description: "Disallow the use of debugger",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-debugger"),
    };
}

impl Rule for NoDebugger {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::DebuggerStatement(stmt) = node {
            let span = Span::from(stmt.span);
            let diagnostic = Diagnostic::error(
                ctx.file_path.to_path_buf(),
                span,
                "no-debugger",
                "Unexpected debugger statement",
            )
            .with_suggestion("Remove this debugger statement before deploying")
            .with_fix(Fix::delete("Remove debugger statement", span));

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoDebugger;
        assert_eq!(rule.meta().name, "no-debugger");
        assert!(rule.meta().recommended);
    }
}
