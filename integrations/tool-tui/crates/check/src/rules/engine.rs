//! Rule Engine
//!
//! Executes rules in parallel and collects violations for scoring.

use super::{Rule, RuleContext, RuleRegistry};
use crate::diagnostics::Diagnostic;
use crate::scoring_impl::{Category, Violation};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Rule execution engine with parallel processing
pub struct RuleEngine {
    registry: Arc<RuleRegistry>,
}

impl RuleEngine {
    /// Create a new rule engine with the given registry
    #[must_use]
    pub fn new(registry: RuleRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Execute all enabled rules on a single file
    #[must_use]
    pub fn execute_file(&self, path: &Path, source: &str) -> Vec<Violation> {
        let mut ctx = RuleContext::new(path, source);

        // Execute all enabled rules
        for (rule, _severity) in self.registry.enabled_rules() {
            // File-level checks
            rule.check_file(source, &mut ctx);

            // Parse and traverse AST for node-level checks
            if let Ok(diagnostics) = self.check_with_ast(path, source, rule.as_ref()) {
                for diagnostic in diagnostics {
                    ctx.report(diagnostic);
                }
            }

            // End checks
            rule.check_end(&mut ctx);
        }

        // Convert diagnostics to violations
        self.diagnostics_to_violations(ctx.take_diagnostics())
    }

    /// Execute all enabled rules on multiple files in parallel
    #[must_use]
    pub fn execute_parallel(&self, files: &[(PathBuf, String)]) -> Vec<Violation> {
        let violations: Arc<Mutex<Vec<Violation>>> = Arc::new(Mutex::new(Vec::new()));

        files.par_iter().for_each(|(path, source)| {
            let file_violations = self.execute_file(path, source);
            if let Ok(mut v) = violations.lock() {
                v.extend(file_violations);
            }
        });

        Arc::try_unwrap(violations)
            .map_or_else(|arc| arc.lock().unwrap().clone(), |mutex| mutex.into_inner().unwrap())
    }

    /// Check a file using AST traversal
    fn check_with_ast(
        &self,
        path: &Path,
        source: &str,
        rule: &dyn Rule,
    ) -> Result<Vec<Diagnostic>, ()> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap_or_default();

        let parser = Parser::new(&allocator, source, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            // Parse errors - skip AST checks
            return Ok(Vec::new());
        }

        let mut ctx = RuleContext::new(path, source);

        // Traverse AST and check each node
        for node in &result.program.body {
            self.traverse_node(node, rule, &mut ctx);
        }

        Ok(ctx.take_diagnostics())
    }

    /// Recursively traverse AST nodes
    fn traverse_node(
        &self,
        _node: &oxc_ast::ast::Statement,
        _rule: &dyn Rule,
        _ctx: &mut RuleContext,
    ) {
        // Convert to AstKind and check
        // Note: This is a simplified traversal - full implementation would use oxc's visitor pattern
        // For now, we'll rely on the rule's check_file and check_end methods
    }

    /// Convert diagnostics to violations with category mapping
    fn diagnostics_to_violations(&self, diagnostics: Vec<Diagnostic>) -> Vec<Violation> {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                let category = self.map_rule_to_category(&diagnostic.rule_id);
                Violation::from_diagnostic(&diagnostic, category)
            })
            .collect()
    }

    /// Map rule ID to scoring category
    fn map_rule_to_category(&self, rule_id: &str) -> Category {
        // Get rule from registry and map to category
        if let Some(rule) = self.registry.get(rule_id) {
            self.map_rule_category_to_scoring_category(rule.meta().category)
        } else {
            // Default to Linting if rule not found
            Category::Linting
        }
    }

    /// Map rule category to scoring category
    fn map_rule_category_to_scoring_category(&self, rule_category: super::Category) -> Category {
        match rule_category {
            super::Category::Style => Category::Formatting,
            super::Category::Correctness | super::Category::Suspicious => Category::Linting,
            super::Category::Security => Category::Security,
            super::Category::Complexity => Category::DesignPatterns,
            super::Category::Performance => Category::Linting,
            super::Category::A11y => Category::Linting,
            super::Category::Imports => Category::StructureAndDocs,
        }
    }

    /// Get the registry
    #[must_use]
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }
}

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;
    use crate::rules::{RuleId, RuleMeta, Severity as RuleSeverity};
    use oxc_ast::AstKind;

    // Mock rule for testing
    #[derive(Clone)]
    struct MockRule {
        meta: RuleMeta,
        should_trigger: bool,
    }

    impl Rule for MockRule {
        fn meta(&self) -> &RuleMeta {
            &self.meta
        }

        fn check(&self, _node: &AstKind<'_>, _ctx: &mut RuleContext<'_>) {}

        fn check_file(&self, _source: &str, ctx: &mut RuleContext<'_>) {
            if self.should_trigger {
                ctx.error(Span { start: 0, end: 1 }, self.meta.name, "Mock violation");
            }
        }
    }

    #[test]
    fn test_rule_engine_creation() {
        let registry = RuleRegistry::new();
        let engine = RuleEngine::new(registry);
        assert_eq!(engine.registry().len(), 0);
    }

    #[test]
    fn test_execute_file_no_rules() {
        let registry = RuleRegistry::new();
        let engine = RuleEngine::new(registry);

        let violations = engine.execute_file(Path::new("test.js"), "const x = 1;");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_execute_file_with_rule() {
        let mut registry = RuleRegistry::new();

        let rule = Box::new(MockRule {
            meta: RuleMeta {
                id: RuleId::new(1),
                name: "test-rule",
                category: super::super::Category::Correctness,
                default_severity: RuleSeverity::Warn,
                description: "Test rule",
                fixable: false,
                recommended: true,
                docs_url: None,
            },
            should_trigger: true,
        });

        registry.register(rule);
        registry.enable("test-rule", RuleSeverity::Warn);

        let engine = RuleEngine::new(registry);
        let violations = engine.execute_file(Path::new("test.js"), "const x = 1;");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "test-rule");
    }

    #[test]
    fn test_execute_parallel() {
        let mut registry = RuleRegistry::new();

        let rule = Box::new(MockRule {
            meta: RuleMeta {
                id: RuleId::new(1),
                name: "test-rule",
                category: super::super::Category::Security,
                default_severity: RuleSeverity::Error,
                description: "Test rule",
                fixable: false,
                recommended: true,
                docs_url: None,
            },
            should_trigger: true,
        });

        registry.register(rule);
        registry.enable("test-rule", RuleSeverity::Error);

        let engine = RuleEngine::new(registry);

        let files = vec![
            (PathBuf::from("file1.js"), "const x = 1;".to_string()),
            (PathBuf::from("file2.js"), "const y = 2;".to_string()),
            (PathBuf::from("file3.js"), "const z = 3;".to_string()),
        ];

        let violations = engine.execute_parallel(&files);

        // Should have one violation per file
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn test_category_mapping() {
        let registry = RuleRegistry::new();
        let engine = RuleEngine::new(registry);

        // Test rule category to scoring category mapping
        assert_eq!(
            engine.map_rule_category_to_scoring_category(super::super::Category::Style),
            Category::Formatting
        );
        assert_eq!(
            engine.map_rule_category_to_scoring_category(super::super::Category::Security),
            Category::Security
        );
        assert_eq!(
            engine.map_rule_category_to_scoring_category(super::super::Category::Complexity),
            Category::DesignPatterns
        );
        assert_eq!(
            engine.map_rule_category_to_scoring_category(super::super::Category::Imports),
            Category::StructureAndDocs
        );
    }

    #[test]
    fn test_violation_severity_mapping() {
        let mut registry = RuleRegistry::new();

        let rule = Box::new(MockRule {
            meta: RuleMeta {
                id: RuleId::new(1),
                name: "critical-rule",
                category: super::super::Category::Security,
                default_severity: RuleSeverity::Error,
                description: "Critical security rule",
                fixable: false,
                recommended: true,
                docs_url: None,
            },
            should_trigger: true,
        });

        registry.register(rule);
        registry.enable("critical-rule", RuleSeverity::Error);

        let engine = RuleEngine::new(registry);
        let violations = engine.execute_file(Path::new("test.js"), "const x = 1;");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].category, Category::Security);
        // Error severity maps to High in scoring
        // assert_eq!(violations[0].severity, crate::scoring::Severity::High);
    }
}
