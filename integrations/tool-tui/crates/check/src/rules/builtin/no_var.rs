//! no-var rule
//!
//! Require let or const instead of var

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::VariableDeclarationKind;

/// Rule: no-var
/// Disallows var declarations in favor of let/const
#[derive(Debug, Clone, Default)]
pub struct NoVar;

impl NoVar {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(6),
        name: "no-var",
        category: Category::Style,
        default_severity: Severity::Warn,
        description: "Require let or const instead of var",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-var"),
    };
}

impl Rule for NoVar {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::VariableDeclaration(decl) = node
            && decl.kind == VariableDeclarationKind::Var
        {
            let var_span = Span::new(decl.span.start, decl.span.start + 3);

            let diagnostic = Diagnostic::warn(
                ctx.file_path.to_path_buf(),
                var_span,
                "no-var",
                "Unexpected var, use let or const instead",
            )
            .with_suggestion("Replace 'var' with 'let' or 'const'")
            .with_fix(Fix::replace("Replace with 'let'", var_span, "let"));

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoVar;
        assert_eq!(rule.meta().name, "no-var");
        assert!(rule.meta().fixable);
    }
}
