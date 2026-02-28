//! Scoring-integrated Rule System
//!
//! This module provides the Rule trait and data structures for the 500-point scoring system.
//! Rules defined here integrate with the scoring categories and support auto-fix functionality.

use crate::diagnostics::{Diagnostic, Span};
use crate::scoring_impl::{Category, Severity};
use std::path::Path;

/// Unique rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(pub u16);

impl RuleId {
    /// Create a new `RuleId`
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Get the numeric ID
    #[must_use]
    pub const fn as_u16(&self) -> u16 {
        self.0
    }
}

impl From<u16> for RuleId {
    fn from(id: u16) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context provided to rules during execution
pub struct RuleContext<'a> {
    /// Source file path
    pub file_path: &'a Path,
    /// Source code content
    pub source: &'a str,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> RuleContext<'a> {
    /// Create a new `RuleContext`
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
        let start = span.start as usize;
        let end = span.end as usize;
        if start <= end && end <= self.source.len() {
            &self.source[start..end]
        } else {
            ""
        }
    }

    /// Take collected diagnostics
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get all diagnostics (without taking ownership)
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Fix classification for safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixSafety {
    /// Always safe to apply automatically (e.g., formatting)
    Safe,
    /// May change behavior, requires user confirmation
    Unsafe,
    /// Cannot be automatically fixed
    None,
}

/// A fix that can be applied to source code
#[derive(Debug, Clone)]
pub struct Fix {
    /// The span to replace
    pub span: Span,
    /// The replacement text
    pub replacement: String,
    /// Safety classification
    pub safety: FixSafety,
    /// Description of what the fix does
    pub description: String,
}

impl Fix {
    /// Create a new safe fix
    pub fn safe(
        span: Span,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            safety: FixSafety::Safe,
            description: description.into(),
        }
    }

    /// Create a new unsafe fix
    pub fn unsafe_fix(
        span: Span,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            safety: FixSafety::Unsafe,
            description: description.into(),
        }
    }

    /// Check if this fix is safe to apply automatically
    #[must_use]
    pub fn is_safe(&self) -> bool {
        self.safety == FixSafety::Safe
    }

    /// Apply this fix to source code
    #[must_use]
    pub fn apply(&self, source: &str) -> String {
        let start = self.span.start as usize;
        let end = self.span.end as usize;

        if start > source.len() || end > source.len() || start > end {
            return source.to_string();
        }

        let mut result = String::with_capacity(source.len());
        result.push_str(&source[..start]);
        result.push_str(&self.replacement);
        result.push_str(&source[end..]);
        result
    }
}

/// Trait for scoring-integrated lint rules
///
/// This trait defines rules that integrate with the 500-point scoring system.
/// Each rule belongs to a single category and has a defined score impact.
pub trait Rule: Send + Sync {
    /// Get the unique rule identifier
    fn id(&self) -> RuleId;

    /// Get the category this rule belongs to
    fn category(&self) -> Category;

    /// Get the severity level of violations
    fn severity(&self) -> Severity;

    /// Get the score impact (points deducted per violation)
    fn score_impact(&self) -> u16;

    /// Check source code for violations
    ///
    /// This method should analyze the source code and report any violations
    /// through the `RuleContext`.
    fn check(&self, ctx: &mut RuleContext<'_>);

    /// Attempt to automatically fix violations
    ///
    /// Returns a list of fixes that can be applied to the source code.
    /// Returns None if the rule doesn't support auto-fix.
    fn auto_fix(&self, ctx: &RuleContext<'_>) -> Option<Vec<Fix>> {
        let _ = ctx;
        None
    }

    /// Get the rule name (for display and configuration)
    fn name(&self) -> &str;

    /// Get a description of what this rule checks
    fn description(&self) -> &'static str {
        ""
    }

    /// Check if this rule supports auto-fix
    fn is_fixable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rule_id_creation() {
        let id = RuleId::new(42);
        assert_eq!(id.as_u16(), 42);
        assert_eq!(id.to_string(), "42");
    }

    #[test]
    fn test_rule_id_from_u16() {
        let id: RuleId = 100.into();
        assert_eq!(id.as_u16(), 100);
    }

    #[test]
    fn test_rule_id_equality() {
        let id1 = RuleId::new(1);
        let id2 = RuleId::new(1);
        let id3 = RuleId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_rule_context_creation() {
        let path = Path::new("test.rs");
        let source = "fn main() {}";
        let ctx = RuleContext::new(path, source);

        assert_eq!(ctx.file_path, path);
        assert_eq!(ctx.source, source);
        assert_eq!(ctx.diagnostics().len(), 0);
    }

    #[test]
    fn test_rule_context_report_error() {
        let path = Path::new("test.rs");
        let source = "fn main() {}";
        let mut ctx = RuleContext::new(path, source);

        ctx.error(Span::new(0, 2), "test-rule", "Test error");

        let diagnostics = ctx.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "test-rule");
        assert_eq!(diagnostics[0].message, "Test error");
    }

    #[test]
    fn test_rule_context_source_text() {
        let path = Path::new("test.rs");
        let source = "fn main() {}";
        let ctx = RuleContext::new(path, source);

        let text = ctx.source_text(Span::new(0, 2));
        assert_eq!(text, "fn");

        let text = ctx.source_text(Span::new(3, 7));
        assert_eq!(text, "main");
    }

    #[test]
    fn test_rule_context_source_text_invalid_span() {
        let path = Path::new("test.rs");
        let source = "fn main() {}";
        let ctx = RuleContext::new(path, source);

        // Out of bounds
        let text = ctx.source_text(Span::new(0, 1000));
        assert_eq!(text, "");

        // Invalid range
        let text = ctx.source_text(Span::new(10, 5));
        assert_eq!(text, "");
    }

    #[test]
    fn test_fix_creation() {
        let fix = Fix::safe(Span::new(0, 5), "hello", "Replace with hello");
        assert_eq!(fix.replacement, "hello");
        assert_eq!(fix.description, "Replace with hello");
        assert!(fix.is_safe());
    }

    #[test]
    fn test_fix_unsafe() {
        let fix = Fix::unsafe_fix(Span::new(0, 5), "world", "Replace with world");
        assert_eq!(fix.safety, FixSafety::Unsafe);
        assert!(!fix.is_safe());
    }

    #[test]
    fn test_fix_apply() {
        let source = "hello world";
        let fix = Fix::safe(Span::new(0, 5), "goodbye", "Replace hello");

        let result = fix.apply(source);
        assert_eq!(result, "goodbye world");
    }

    #[test]
    fn test_fix_apply_middle() {
        let source = "hello world test";
        let fix = Fix::safe(Span::new(6, 11), "universe", "Replace world");

        let result = fix.apply(source);
        assert_eq!(result, "hello universe test");
    }

    #[test]
    fn test_fix_apply_invalid_span() {
        let source = "hello";
        let fix = Fix::safe(Span::new(0, 100), "test", "Invalid span");

        let result = fix.apply(source);
        assert_eq!(result, source); // Should return original
    }

    #[test]
    fn test_fix_safety_levels() {
        assert_eq!(FixSafety::Safe, FixSafety::Safe);
        assert_ne!(FixSafety::Safe, FixSafety::Unsafe);
        assert_ne!(FixSafety::Safe, FixSafety::None);
    }

    // Example rule implementation for testing
    struct TestRule {
        id: RuleId,
        category: Category,
        severity: Severity,
    }

    impl Rule for TestRule {
        fn id(&self) -> RuleId {
            self.id
        }

        fn category(&self) -> Category {
            self.category
        }

        fn severity(&self) -> Severity {
            self.severity
        }

        fn score_impact(&self) -> u16 {
            self.severity.points()
        }

        fn check(&self, ctx: &mut RuleContext<'_>) {
            // Simple test: report error if source contains "bad"
            if ctx.source.contains("bad") {
                ctx.error(Span::new(0, 3), "test-rule", "Found 'bad' in source");
            }
        }

        fn name(&self) -> &str {
            "test-rule"
        }

        fn description(&self) -> &'static str {
            "A test rule"
        }
    }

    #[test]
    fn test_rule_implementation() {
        let rule = TestRule {
            id: RuleId::new(1),
            category: Category::Linting,
            severity: Severity::High,
        };

        assert_eq!(rule.id(), RuleId::new(1));
        assert_eq!(rule.category(), Category::Linting);
        assert_eq!(rule.severity(), Severity::High);
        assert_eq!(rule.score_impact(), 5); // High severity = 5 points
        assert_eq!(rule.name(), "test-rule");
        assert_eq!(rule.description(), "A test rule");
    }

    #[test]
    fn test_rule_check() {
        let rule = TestRule {
            id: RuleId::new(1),
            category: Category::Linting,
            severity: Severity::High,
        };

        let path = Path::new("test.rs");
        let source = "bad code here";
        let mut ctx = RuleContext::new(path, source);

        rule.check(&mut ctx);

        let diagnostics = ctx.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Found 'bad' in source");
    }

    #[test]
    fn test_rule_check_no_violation() {
        let rule = TestRule {
            id: RuleId::new(1),
            category: Category::Linting,
            severity: Severity::High,
        };

        let path = Path::new("test.rs");
        let source = "good code here";
        let mut ctx = RuleContext::new(path, source);

        rule.check(&mut ctx);

        let diagnostics = ctx.diagnostics();
        assert_eq!(diagnostics.len(), 0);
    }
}
