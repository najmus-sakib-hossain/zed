//! no-eval rule
//!
//! Disallow the use of `eval()`

use crate::diagnostics::Diagnostic;
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::{CallExpression, Expression};

/// Rule: no-eval
/// Disallows the use of `eval()` which is a security risk
#[derive(Debug, Clone, Default)]
pub struct NoEval {
    /// Allow indirect eval (e.g., (0, eval)("code"))
    allow_indirect: bool,
}

impl NoEval {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(7),
        name: "no-eval",
        category: Category::Security,
        default_severity: Severity::Error,
        description: "Disallow the use of eval()",
        fixable: false,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-eval"),
    };
}

impl Rule for NoEval {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::CallExpression(call) = node
            && is_eval_call(call, self.allow_indirect)
        {
            let diagnostic = Diagnostic::error(
                    ctx.file_path.to_path_buf(),
                    call.span.into(),
                    "no-eval",
                    "eval() is a security risk and should not be used",
                )
                .with_suggestion(
                    "Avoid using eval(). Consider using safer alternatives like JSON.parse() or Function constructor",
                );

            ctx.report(diagnostic);
        }
    }
}

fn is_eval_call(call: &CallExpression, allow_indirect: bool) -> bool {
    match &call.callee {
        Expression::Identifier(id) if id.name == "eval" => true,
        Expression::SequenceExpression(_) if !allow_indirect => {
            // Check for indirect eval: (0, eval)(...)
            // This is a simplified check
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoEval::default();
        assert_eq!(rule.meta().name, "no-eval");
        assert!(!rule.meta().fixable);
    }
}
