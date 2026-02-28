//! no-unsafe-finally rule
//!
//! Disallow control flow statements in finally blocks

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::Statement;

/// Rule: no-unsafe-finally
/// Disallows control flow statements in finally blocks
#[derive(Debug, Clone, Default)]
pub struct NoUnsafeFinally;

impl NoUnsafeFinally {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(14),
        name: "no-unsafe-finally",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow control flow statements in finally blocks",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-unsafe-finally"),
    };
}

impl Rule for NoUnsafeFinally {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::TryStatement(try_stmt) = node
            && let Some(ref finalizer) = try_stmt.finalizer
        {
            check_finally_block(&finalizer.body, ctx);
        }
    }
}

fn check_finally_block(statements: &[Statement], ctx: &mut RuleContext<'_>) {
    for stmt in statements {
        match stmt {
            Statement::ReturnStatement(s) => {
                report_unsafe_finally(ctx, s.span, "return");
            }
            Statement::ThrowStatement(s) => {
                report_unsafe_finally(ctx, s.span, "throw");
            }
            Statement::BreakStatement(s) => {
                report_unsafe_finally(ctx, s.span, "break");
            }
            Statement::ContinueStatement(s) => {
                report_unsafe_finally(ctx, s.span, "continue");
            }
            Statement::BlockStatement(block) => {
                check_finally_block(&block.body, ctx);
            }
            Statement::IfStatement(if_stmt) => {
                if let Statement::BlockStatement(block) = &if_stmt.consequent {
                    check_finally_block(&block.body, ctx);
                }
                if let Some(ref alternate) = if_stmt.alternate
                    && let Statement::BlockStatement(block) = alternate
                {
                    check_finally_block(&block.body, ctx);
                }
            }
            _ => {}
        }
    }
}

fn report_unsafe_finally(ctx: &mut RuleContext<'_>, span: oxc_span::Span, keyword: &str) {
    let diagnostic = Diagnostic::error(
        ctx.file_path.to_path_buf(),
        Span::from(span),
        "no-unsafe-finally",
        format!("Unsafe use of {keyword} in finally block"),
    )
    .with_suggestion("Control flow in finally blocks can override try/catch behavior");

    ctx.report(diagnostic);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoUnsafeFinally;
        assert_eq!(rule.meta().name, "no-unsafe-finally");
        assert!(!rule.meta().fixable);
    }
}
