//! no-unreachable rule
//!
//! Disallow unreachable code after return, throw, continue, and break statements

use crate::diagnostics::{Diagnostic, Span};
use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;
use oxc_ast::ast::Statement;

/// Rule: no-unreachable
/// Disallows unreachable code after control flow statements
#[derive(Debug, Clone, Default)]
pub struct NoUnreachable;

impl NoUnreachable {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(12),
        name: "no-unreachable",
        category: Category::Correctness,
        default_severity: Severity::Error,
        description: "Disallow unreachable code after return, throw, continue, and break statements",
        fixable: true,
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-unreachable"),
    };
}

impl Rule for NoUnreachable {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>) {
        if let AstKind::BlockStatement(block) = node {
            check_block_for_unreachable(&block.body, ctx);
        }
    }
}

fn check_block_for_unreachable(statements: &[Statement], ctx: &mut RuleContext<'_>) {
    let mut found_terminator = false;

    for stmt in statements {
        if found_terminator {
            // This statement is unreachable
            let span = get_statement_span(stmt);
            let diagnostic = Diagnostic::error(
                ctx.file_path.to_path_buf(),
                Span::from(span),
                "no-unreachable",
                "Unreachable code",
            )
            .with_suggestion("Remove this unreachable code");

            ctx.report(diagnostic);
            continue;
        }

        // Check if this statement is a terminator
        if is_terminator(stmt) {
            found_terminator = true;
        }
    }
}

fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::ReturnStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
    )
}

fn get_statement_span(stmt: &Statement) -> oxc_span::Span {
    match stmt {
        Statement::BlockStatement(s) => s.span,
        Statement::BreakStatement(s) => s.span,
        Statement::ContinueStatement(s) => s.span,
        Statement::DebuggerStatement(s) => s.span,
        Statement::DoWhileStatement(s) => s.span,
        Statement::EmptyStatement(s) => s.span,
        Statement::ExpressionStatement(s) => s.span,
        Statement::ForInStatement(s) => s.span,
        Statement::ForOfStatement(s) => s.span,
        Statement::ForStatement(s) => s.span,
        Statement::IfStatement(s) => s.span,
        Statement::LabeledStatement(s) => s.span,
        Statement::ReturnStatement(s) => s.span,
        Statement::SwitchStatement(s) => s.span,
        Statement::ThrowStatement(s) => s.span,
        Statement::TryStatement(s) => s.span,
        Statement::WhileStatement(s) => s.span,
        Statement::WithStatement(s) => s.span,
        Statement::VariableDeclaration(s) => s.span,
        Statement::FunctionDeclaration(s) => s.span,
        Statement::ClassDeclaration(s) => s.span,
        Statement::ImportDeclaration(s) => s.span,
        Statement::ExportAllDeclaration(s) => s.span,
        Statement::ExportDefaultDeclaration(s) => s.span,
        Statement::ExportNamedDeclaration(s) => s.span,
        Statement::TSTypeAliasDeclaration(s) => s.span,
        Statement::TSInterfaceDeclaration(s) => s.span,
        Statement::TSEnumDeclaration(s) => s.span,
        Statement::TSModuleDeclaration(s) => s.span,
        Statement::TSImportEqualsDeclaration(s) => s.span,
        Statement::TSExportAssignment(s) => s.span,
        Statement::TSNamespaceExportDeclaration(s) => s.span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoUnreachable;
        assert_eq!(rule.meta().name, "no-unreachable");
        assert!(rule.meta().fixable);
    }
}
