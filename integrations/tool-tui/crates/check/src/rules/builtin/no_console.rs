//! no-console rule
//!
//! Disallow the use of console

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::{CallExpression, Expression};

/// Rule: no-console
/// Disallows console.* calls in production code
#[derive(Debug, Clone, Default)]
pub struct NoConsole {
    /// Methods to allow (e.g., ["warn", "error"])
    allow: Vec<String>,
}

impl NoConsole {
    #[must_use]
    pub fn new(allow: Vec<String>) -> Self {
        Self { allow }
    }

    const META: RuleMeta = RuleMeta {
        id: RuleId::new(1),
        name: "no-console",
        category: Category::Suspicious,
        default_severity: Severity::Warn,
        description: "Disallow the use of console",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-console"),
    };
}

impl Rule for NoConsole {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::CallExpression(call) = node
            && let Some((span, method)) = is_console_call(call)
        {
            // Check if method is in allow list
            if !self.allow.iter().any(|a| a == method) {
                let diagnostic = Diagnostic::warn(
                    ctx.file_path.to_path_buf(),
                    span.into(),
                    "no-console",
                    format!("Unexpected console.{method} statement"),
                )
                .with_suggestion("Remove this console statement before deploying to production")
                .with_fix(Fix::delete("Remove console statement", find_statement_span(call)));

                ctx.report(diagnostic);
            }
        }
    }
}

/// Check if a call expression is console.*
fn is_console_call<'a>(call: &'a CallExpression<'a>) -> Option<(oxc_span::Span, &'a str)> {
    if let Expression::StaticMemberExpression(member) = &call.callee
        && let Expression::Identifier(id) = &member.object
        && id.name == "console"
    {
        return Some((call.span, member.property.name.as_str()));
    }
    None
}

/// Find the span of the containing statement (for deletion)
fn find_statement_span(call: &CallExpression) -> Span {
    // For now, just use the call span
    // In full implementation, we'd walk up to the ExpressionStatement
    Span::from(call.span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoConsole::default();
        assert_eq!(rule.meta().name, "no-console");
        assert!(rule.meta().fixable);
    }
}
