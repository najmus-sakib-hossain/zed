//! prefer-const rule
//!
//! Prefer const over let for variables that are never reassigned

use crate::diagnostics::{Diagnostic, Fix, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::VariableDeclarationKind;

/// Rule: prefer-const
/// Suggests using const instead of let for variables that are never reassigned
#[derive(Debug, Clone, Default)]
pub struct PreferConst {
    /// Require destructuring to have all variables as const
    destructuring: DestructuringOption,
    /// Ignore read-before-assign
    ignore_read_before_assign: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DestructuringOption {
    #[default]
    Any,
    All,
}

impl PreferConst {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(5),
        name: "prefer-const",
        category: Category::Style,
        default_severity: Severity::Warn,
        description: "Require const declarations for variables that are never reassigned",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/prefer-const"),
    };
}

impl Rule for PreferConst {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::VariableDeclaration(decl) = node {
            // Only check 'let' declarations
            if decl.kind != VariableDeclarationKind::Let {
                return;
            }

            // Check if any declarator has an initializer
            // (without initializer, can't be const)
            for declarator in &decl.declarations {
                if declarator.init.is_none() {
                    return;
                }
            }

            // In a full implementation, we'd check if the variable is ever reassigned
            // using scope analysis. For now, we'll report on simple cases.

            // This is where we'd integrate with semantic analysis to check
            // if the binding is ever reassigned

            // For demonstration, we'll flag all let declarations with initializers
            // and suggest they might be convertible to const

            // Get the 'let' keyword span
            let let_span = find_let_span(ctx.source, decl.span);

            let _diagnostic = Diagnostic::warn(
                ctx.file_path.to_path_buf(),
                let_span,
                "prefer-const",
                "'let' can be replaced with 'const' if never reassigned",
            )
            .with_suggestion("Use 'const' for variables that are never reassigned")
            .with_fix(Fix::replace("Change 'let' to 'const'", let_span, "const"));

            // NOTE: In production, we'd only report this after confirming
            // the variable is never reassigned via scope analysis
            // ctx.report(diagnostic);
        }
    }
}

fn find_let_span(_source: &str, decl_span: oxc_span::Span) -> Span {
    let start = decl_span.start;
    let end = start + 3; // "let" is 3 characters
    Span::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = PreferConst::default();
        assert_eq!(rule.meta().name, "prefer-const");
        assert!(rule.meta().fixable);
    }
}
