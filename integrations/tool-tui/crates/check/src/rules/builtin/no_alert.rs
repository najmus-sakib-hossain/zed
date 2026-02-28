//! no-alert rule
//!
//! Disallow the use of alert, confirm, and prompt

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::Expression;

/// Rule: no-alert
/// Disallows alert, confirm, and prompt calls
#[derive(Debug, Clone, Default)]
pub struct NoAlert;

impl NoAlert {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(9),
        name: "no-alert",
        category: Category::Suspicious,
        default_severity: Severity::Warn,
        description: "Disallow the use of alert, confirm, and prompt",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-alert"),
    };
}

impl Rule for NoAlert {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::CallExpression(call) = node
            && let Expression::Identifier(id) = &call.callee
        {
            let name = id.name.as_str();
            if matches!(name, "alert" | "confirm" | "prompt") {
                let diagnostic = Diagnostic::warn(
                    ctx.file_path.to_path_buf(),
                    Span::from(call.span),
                    "no-alert",
                    format!("Unexpected {name}"),
                )
                .with_suggestion("Use a custom modal or notification system instead")
                .with_fix(Fix::delete("Remove alert call", Span::from(call.span)));

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
        let rule = NoAlert;
        assert_eq!(rule.meta().name, "no-alert");
        assert!(rule.meta().fixable);
    }
}
