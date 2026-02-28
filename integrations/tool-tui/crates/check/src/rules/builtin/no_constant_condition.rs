//! no-constant-condition rule
//!
//! Disallow constant expressions in conditions

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;

/// Rule: no-constant-condition
/// Disallows constant expressions in conditions
#[derive(Debug, Clone, Default)]
pub struct NoConstantCondition;

impl NoConstantCondition {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(13),
        name: "no-constant-condition",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow constant expressions in conditions",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-constant-condition"),
    };
}

impl Rule for NoConstantCondition {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        match node {
            AstKind::IfStatement(stmt) => {
                if is_constant_expression(&stmt.test) {
                    report_constant_condition(ctx, stmt.test.span());
                }
            }
            AstKind::WhileStatement(stmt) => {
                if is_constant_expression(&stmt.test) {
                    report_constant_condition(ctx, stmt.test.span());
                }
            }
            AstKind::DoWhileStatement(stmt) => {
                if is_constant_expression(&stmt.test) {
                    report_constant_condition(ctx, stmt.test.span());
                }
            }
            AstKind::ForStatement(stmt) => {
                if let Some(ref test) = stmt.test
                    && is_constant_expression(test)
                {
                    report_constant_condition(ctx, test.span());
                }
            }
            AstKind::ConditionalExpression(expr) => {
                if is_constant_expression(&expr.test) {
                    report_constant_condition(ctx, expr.test.span());
                }
            }
            _ => {}
        }
    }
}

fn report_constant_condition(ctx: &mut RuleContext<'_>, span: oxc_span::Span) {
    let diagnostic = Diagnostic::error(
        ctx.file_path.to_path_buf(),
        Span::from(span),
        "no-constant-condition",
        "Unexpected constant condition",
    )
    .with_suggestion("Use a variable or expression that can change");

    ctx.report(diagnostic);
}

fn is_constant_expression(expr: &Expression) -> bool {
    match expr {
        // Literals are always constant
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_) => true,

        // Unary expressions on constants are constant
        Expression::UnaryExpression(unary) => is_constant_expression(&unary.argument),

        // Binary expressions with constant operands are constant
        Expression::BinaryExpression(binary) => {
            is_constant_expression(&binary.left) && is_constant_expression(&binary.right)
        }

        // Logical expressions with constant operands are constant
        Expression::LogicalExpression(logical) => {
            is_constant_expression(&logical.left) && is_constant_expression(&logical.right)
        }

        // Parenthesized expressions
        Expression::ParenthesizedExpression(paren) => is_constant_expression(&paren.expression),

        // Everything else is not constant
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoConstantCondition;
        assert_eq!(rule.meta().name, "no-constant-condition");
        assert!(!rule.meta().fixable);
    }
}
