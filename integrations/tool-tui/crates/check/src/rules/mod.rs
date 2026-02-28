//! Rule System
//!
//! Binary-first rule engine with fused execution.
//! All rules compile to binary opcodes and execute in a single AST traversal.

pub mod binary;
pub mod builtin;
pub mod compiler;
pub mod dxs_generator;
pub mod dxs_parser;
pub mod engine;
pub mod extractor;
pub mod registry;
pub mod schema;
pub mod scoring_rule;
pub mod sr_loader;

#[cfg(test)]
mod binary_tests;

use crate::diagnostics::{Diagnostic, Span};
use oxc_ast::AstKind;
use std::path::Path;

pub use engine::RuleEngine;
pub use registry::RuleRegistry;
pub use scoring_rule::{
    Fix, FixSafety, Rule as ScoringRule, RuleContext as ScoringRuleContext, RuleId as ScoringRuleId,
};
pub use sr_loader::{SrRuleLoader, compile_sr_rules, load_compiled_rules};

/// Unique rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(pub u16);

impl RuleId {
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }
}

/// Rule severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Off - rule is disabled
    Off,
    /// Warning - potential issue
    Warn,
    /// Error - definite problem
    Error,
}

/// Rule category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Possible errors
    Correctness,
    /// Suspicious code
    Suspicious,
    /// Style preferences
    Style,
    /// Performance issues
    Performance,
    /// Security vulnerabilities
    Security,
    /// Complexity issues
    Complexity,
    /// Accessibility (for JSX)
    A11y,
    /// Import/export issues
    Imports,
}

impl Category {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Style => "style",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::Complexity => "complexity",
            Self::A11y => "a11y",
            Self::Imports => "imports",
        }
    }
}

/// Metadata about a rule
#[derive(Debug, Clone)]
pub struct RuleMeta {
    /// Unique identifier
    pub id: RuleId,
    /// Rule name (e.g., "no-console")
    pub name: &'static str,
    /// Category
    pub category: Category,
    /// Default severity
    pub default_severity: Severity,
    /// Short description
    pub description: &'static str,
    /// Whether the rule has an auto-fix
    pub fixable: bool,
    /// Whether this is a recommended rule
    pub recommended: bool,
    /// Documentation URL
    pub docs_url: Option<&'static str>,
}

/// Context provided to rules during execution
pub struct RuleContext<'a> {
    /// Source file path
    pub file_path: &'a Path,
    /// Source code
    pub source: &'a str,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
    // Scopes (for semantic analysis)
    // pub scopes: &'a ScopeTree,
}

impl<'a> RuleContext<'a> {
    #[must_use]
    pub fn new(file_path: &'a Path, source: &'a str) -> Self {
        Self {
            file_path,
            source,
            diagnostics: Vec::new(),
        }
    }

    /// Report a diagnostic
    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Report an error
    pub fn error(&mut self, span: Span, rule_id: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(
            self.file_path.to_path_buf(),
            span,
            rule_id,
            message,
        ));
    }

    /// Report a warning
    pub fn warn(&mut self, span: Span, rule_id: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::warn(
            self.file_path.to_path_buf(),
            span,
            rule_id,
            message,
        ));
    }

    /// Get source text for a span
    #[must_use]
    pub fn source_text(&self, span: Span) -> &str {
        &self.source[span.start as usize..span.end as usize]
    }

    /// Take collected diagnostics
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

/// Helper trait for cloning boxed rules
pub trait RuleClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn Rule>;
}

impl<T: Rule + Clone + 'static> RuleClone for T {
    fn clone_box(&self) -> Box<dyn Rule> {
        Box::new(self.clone())
    }
}

/// Trait for lint rules
pub trait Rule: RuleClone + Send + Sync {
    /// Get rule metadata
    fn meta(&self) -> &RuleMeta;

    /// Check a node - called for each AST node during traversal
    fn check(&self, node: &AstKind<'_>, ctx: &mut RuleContext<'_>);

    /// Check at file level (before traversal)
    fn check_file(&self, _source: &str, _ctx: &mut RuleContext<'_>) {}

    /// Check after traversal (for rules that need full context)
    fn check_end(&self, _ctx: &mut RuleContext<'_>) {}
}

/// Node kinds that rules can match on
/// Used for the jump table in fused execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeKind {
    // Statements
    BlockStatement,
    BreakStatement,
    ContinueStatement,
    DebuggerStatement,
    DoWhileStatement,
    EmptyStatement,
    ExpressionStatement,
    ForInStatement,
    ForOfStatement,
    ForStatement,
    IfStatement,
    LabeledStatement,
    ReturnStatement,
    SwitchStatement,
    ThrowStatement,
    TryStatement,
    WhileStatement,
    WithStatement,

    // Declarations
    VariableDeclaration,
    FunctionDeclaration,
    ClassDeclaration,
    ImportDeclaration,
    ExportNamedDeclaration,
    ExportDefaultDeclaration,
    ExportAllDeclaration,

    // Expressions
    ArrayExpression,
    ArrowFunctionExpression,
    AssignmentExpression,
    AwaitExpression,
    BinaryExpression,
    CallExpression,
    ChainExpression,
    ClassExpression,
    ConditionalExpression,
    FunctionExpression,
    Identifier,
    ImportExpression,
    LogicalExpression,
    MemberExpression,
    MetaProperty,
    NewExpression,
    ObjectExpression,
    SequenceExpression,
    TaggedTemplateExpression,
    TemplateLiteral,
    ThisExpression,
    UnaryExpression,
    UpdateExpression,
    YieldExpression,

    // Literals
    BooleanLiteral,
    NullLiteral,
    NumericLiteral,
    BigIntLiteral,
    RegExpLiteral,
    StringLiteral,

    // JSX
    JSXElement,
    JSXFragment,
    JSXOpeningElement,
    JSXClosingElement,
    JSXAttribute,
    JSXSpreadAttribute,
    JSXText,
    JSXExpressionContainer,

    // TypeScript
    TSTypeAnnotation,
    TSAsExpression,
    TSNonNullExpression,
    TSTypeAssertion,
    TSInterfaceDeclaration,
    TSTypeAliasDeclaration,
    TSEnumDeclaration,
    TSModuleDeclaration,

    // Patterns
    ArrayPattern,
    ObjectPattern,
    AssignmentPattern,
    RestElement,

    // Other
    SpreadElement,
    Property,
    MethodDefinition,
    ClassBody,
    Program,
}

impl NodeKind {
    /// Convert from oxc `AstKind`
    #[must_use]
    pub fn from_ast_kind(kind: &AstKind<'_>) -> Option<Self> {
        use AstKind::{
            ArrayExpression, ArrayPattern, ArrowFunctionExpression, AssignmentExpression,
            AssignmentPattern, AwaitExpression, BigIntLiteral, BinaryExpression, BindingIdentifier,
            BindingRestElement, BlockStatement, BooleanLiteral, BreakStatement, CallExpression,
            ChainExpression, Class, ClassBody, ConditionalExpression, ContinueStatement,
            DebuggerStatement, DoWhileStatement, EmptyStatement, ExportAllDeclaration,
            ExportDefaultDeclaration, ExportNamedDeclaration, ExpressionStatement, ForInStatement,
            ForOfStatement, ForStatement, Function, IdentifierReference, IfStatement,
            ImportDeclaration, ImportExpression, JSXAttributeItem, JSXClosingElement, JSXElement,
            JSXExpressionContainer, JSXFragment, JSXOpeningElement, JSXSpreadAttribute, JSXText,
            LabeledStatement, LogicalExpression, MemberExpression, MetaProperty, MethodDefinition,
            NewExpression, NullLiteral, NumericLiteral, ObjectExpression, ObjectPattern,
            ObjectProperty, Program, RegExpLiteral, ReturnStatement, SequenceExpression,
            SpreadElement, StringLiteral, SwitchStatement, TaggedTemplateExpression,
            TemplateLiteral, ThisExpression, ThrowStatement, TryStatement, UnaryExpression,
            UpdateExpression, VariableDeclaration, WhileStatement, WithStatement, YieldExpression,
        };
        Some(match kind {
            // Statements
            BlockStatement(_) => NodeKind::BlockStatement,
            BreakStatement(_) => NodeKind::BreakStatement,
            ContinueStatement(_) => NodeKind::ContinueStatement,
            DebuggerStatement(_) => NodeKind::DebuggerStatement,
            DoWhileStatement(_) => NodeKind::DoWhileStatement,
            EmptyStatement(_) => NodeKind::EmptyStatement,
            ExpressionStatement(_) => NodeKind::ExpressionStatement,
            ForInStatement(_) => NodeKind::ForInStatement,
            ForOfStatement(_) => NodeKind::ForOfStatement,
            ForStatement(_) => NodeKind::ForStatement,
            IfStatement(_) => NodeKind::IfStatement,
            LabeledStatement(_) => NodeKind::LabeledStatement,
            ReturnStatement(_) => NodeKind::ReturnStatement,
            SwitchStatement(_) => NodeKind::SwitchStatement,
            ThrowStatement(_) => NodeKind::ThrowStatement,
            TryStatement(_) => NodeKind::TryStatement,
            WhileStatement(_) => NodeKind::WhileStatement,
            WithStatement(_) => NodeKind::WithStatement,

            // Declarations
            VariableDeclaration(_) => NodeKind::VariableDeclaration,
            Function(_) => NodeKind::FunctionDeclaration,
            Class(_) => NodeKind::ClassDeclaration,
            ImportDeclaration(_) => NodeKind::ImportDeclaration,
            ExportNamedDeclaration(_) => NodeKind::ExportNamedDeclaration,
            ExportDefaultDeclaration(_) => NodeKind::ExportDefaultDeclaration,
            ExportAllDeclaration(_) => NodeKind::ExportAllDeclaration,

            // Expressions
            ArrayExpression(_) => NodeKind::ArrayExpression,
            ArrowFunctionExpression(_) => NodeKind::ArrowFunctionExpression,
            AssignmentExpression(_) => NodeKind::AssignmentExpression,
            AwaitExpression(_) => NodeKind::AwaitExpression,
            BinaryExpression(_) => NodeKind::BinaryExpression,
            CallExpression(_) => NodeKind::CallExpression,
            ChainExpression(_) => NodeKind::ChainExpression,
            ConditionalExpression(_) => NodeKind::ConditionalExpression,
            IdentifierReference(_) => NodeKind::Identifier,
            BindingIdentifier(_) => NodeKind::Identifier,
            ImportExpression(_) => NodeKind::ImportExpression,
            LogicalExpression(_) => NodeKind::LogicalExpression,
            MemberExpression(_) => NodeKind::MemberExpression,
            MetaProperty(_) => NodeKind::MetaProperty,
            NewExpression(_) => NodeKind::NewExpression,
            ObjectExpression(_) => NodeKind::ObjectExpression,
            SequenceExpression(_) => NodeKind::SequenceExpression,
            TaggedTemplateExpression(_) => NodeKind::TaggedTemplateExpression,
            TemplateLiteral(_) => NodeKind::TemplateLiteral,
            ThisExpression(_) => NodeKind::ThisExpression,
            UnaryExpression(_) => NodeKind::UnaryExpression,
            UpdateExpression(_) => NodeKind::UpdateExpression,
            YieldExpression(_) => NodeKind::YieldExpression,

            // Literals
            BooleanLiteral(_) => NodeKind::BooleanLiteral,
            NullLiteral(_) => NodeKind::NullLiteral,
            NumericLiteral(_) => NodeKind::NumericLiteral,
            BigIntLiteral(_) => NodeKind::BigIntLiteral,
            RegExpLiteral(_) => NodeKind::RegExpLiteral,
            StringLiteral(_) => NodeKind::StringLiteral,

            // JSX
            JSXElement(_) => NodeKind::JSXElement,
            JSXFragment(_) => NodeKind::JSXFragment,
            JSXOpeningElement(_) => NodeKind::JSXOpeningElement,
            JSXClosingElement(_) => NodeKind::JSXClosingElement,
            JSXAttributeItem(_) => NodeKind::JSXAttribute,
            JSXSpreadAttribute(_) => NodeKind::JSXSpreadAttribute,
            JSXText(_) => NodeKind::JSXText,
            JSXExpressionContainer(_) => NodeKind::JSXExpressionContainer,

            // Patterns
            ArrayPattern(_) => NodeKind::ArrayPattern,
            ObjectPattern(_) => NodeKind::ObjectPattern,
            AssignmentPattern(_) => NodeKind::AssignmentPattern,
            BindingRestElement(_) => NodeKind::RestElement,

            // Other
            SpreadElement(_) => NodeKind::SpreadElement,
            ObjectProperty(_) => NodeKind::Property,
            MethodDefinition(_) => NodeKind::MethodDefinition,
            ClassBody(_) => NodeKind::ClassBody,
            Program(_) => NodeKind::Program,

            _ => return None,
        })
    }
}

/// Fused rule set - all rules compiled for single-pass execution
#[allow(dead_code)]
pub struct FusedRuleSet {
    /// Rules grouped by the node kinds they care about
    rules_by_node: Vec<Vec<Box<dyn Rule>>>,
    /// File-level rules
    file_rules: Vec<Box<dyn Rule>>,
    /// End rules
    end_rules: Vec<Box<dyn Rule>>,
}

impl FusedRuleSet {
    #[must_use]
    pub fn new() -> Self {
        // Pre-allocate empty vecs for all node kinds
        let mut rules_by_node = Vec::with_capacity(100);
        for _ in 0..100 {
            rules_by_node.push(Vec::new());
        }
        Self {
            rules_by_node,
            file_rules: Vec::new(),
            end_rules: Vec::new(),
        }
    }

    /// Add a rule that matches specific node kinds
    pub fn add_rule(&mut self, rule: Box<dyn Rule>, node_kinds: &[NodeKind]) {
        for kind in node_kinds {
            let idx = *kind as usize;
            if idx < self.rules_by_node.len() {
                self.rules_by_node[idx].push(rule.clone_box());
            }
        }
    }

    /// Get rules for a specific node kind - O(1) lookup
    #[must_use]
    pub fn rules_for_node(&self, kind: NodeKind) -> &[Box<dyn Rule>] {
        let idx = kind as usize;
        if idx < self.rules_by_node.len() {
            &self.rules_by_node[idx]
        } else {
            &[]
        }
    }
}

impl Default for FusedRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_id() {
        let id = RuleId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_category_as_str() {
        assert_eq!(Category::Correctness.as_str(), "correctness");
        assert_eq!(Category::Security.as_str(), "security");
    }
}
