//! eqeqeq rule
//!
//! Require the use of === and !==

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::{BinaryExpression, BinaryOperator, Expression};

/// Rule: eqeqeq
/// Requires the use of === and !== instead of == and !=
#[derive(Debug, Clone, Default)]
pub struct Eqeqeq {
    /// Allow == null
    allow_null: bool,
}

impl Eqeqeq {
    #[must_use]
    pub fn new(allow_null: bool) -> Self {
        Self { allow_null }
    }

    const META: RuleMeta = RuleMeta {
        id: RuleId::new(3),
        name: "eqeqeq",
        category: Category::Suspicious,
        default_severity: Severity::Warn,
        description: "Require the use of === and !==",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/eqeqeq"),
    };
}

impl Rule for Eqeqeq {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::BinaryExpression(expr) = node {
            let (is_loose, strict_op) = match expr.operator {
                BinaryOperator::Equality => (true, "==="),
                BinaryOperator::Inequality => (true, "!=="),
                _ => return,
            };

            if !is_loose {
                return;
            }

            // Allow == null / != null if configured
            if self.allow_null && is_null_comparison(expr) {
                return;
            }

            let loose_op = if strict_op == "===" { "==" } else { "!=" };

            // Find the operator span within the expression
            let op_span = find_operator_span(ctx.source, expr.span, loose_op);

            let diagnostic = Diagnostic::warn(
                ctx.file_path.to_path_buf(),
                Span::from(expr.span),
                "eqeqeq",
                format!("Expected '{strict_op}' and instead saw '{loose_op}'"),
            )
            .with_suggestion(format!("Use '{strict_op}' for strict equality comparison"))
            .with_fix(Fix::replace(
                format!("Replace '{loose_op}' with '{strict_op}'"),
                op_span,
                strict_op,
            ));

            ctx.report(diagnostic);
        }
    }
}

/// Check if the comparison is against null
fn is_null_comparison(expr: &BinaryExpression) -> bool {
    matches!(&expr.left, Expression::NullLiteral(_))
        || matches!(&expr.right, Expression::NullLiteral(_))
}

/// Find the span of the operator within the expression
fn find_operator_span(source: &str, expr_span: oxc_span::Span, op: &str) -> Span {
    let expr_text = &source[expr_span.start as usize..expr_span.end as usize];
    if let Some(pos) = expr_text.find(op) {
        let start = expr_span.start + pos as u32;
        let end = start + op.len() as u32;
        Span::new(start, end)
    } else {
        // Fallback to expression span
        Span::from(expr_span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = Eqeqeq::default();
        assert_eq!(rule.meta().name, "eqeqeq");
        assert!(rule.meta().fixable);
    }
}
