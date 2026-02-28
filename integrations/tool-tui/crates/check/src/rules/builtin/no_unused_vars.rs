//! no-unused-vars rule
//!
//! Disallow unused variables

use crate::rules::{Category, Rule, RuleContext, RuleId, RuleMeta, Severity};
use oxc_ast::AstKind;

/// Rule: no-unused-vars
/// Disallows declared variables that are never used
#[derive(Debug, Clone)]
pub struct NoUnusedVars {
    /// Variables starting with _ are ignored
    ignore_pattern: Option<String>,
    /// Ignore rest siblings
    ignore_rest_siblings: bool,
    /// Check for unused function arguments
    args: ArgsOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgsOption {
    /// Check all arguments
    All,
    /// Only check arguments after the last used one
    AfterUsed,
    /// Don't check arguments
    None,
}

impl Default for NoUnusedVars {
    fn default() -> Self {
        Self {
            ignore_pattern: Some("^_".to_string()),
            ignore_rest_siblings: true,
            args: ArgsOption::AfterUsed,
        }
    }
}

impl NoUnusedVars {
    const META: RuleMeta = RuleMeta {
        id: RuleId::new(4),
        name: "no-unused-vars",
        category: Category::Correctness,
        default_severity: Severity::Warn,
        description: "Disallow unused variables",
        fixable: false, // Complex to auto-fix
        recommended: true,
        docs_url: Some("https://dx.dev/rules/no-unused-vars"),
    };

    fn is_ignored(&self, name: &str) -> bool {
        if let Some(ref pattern) = self.ignore_pattern {
            if pattern.starts_with('^') && name.starts_with(&pattern[1..]) {
                return true;
            }
            // Simple prefix check - full regex support could be added
            if name.starts_with('_') {
                return true;
            }
        }
        false
    }
}

impl Rule for NoUnusedVars {
    fn meta(&self) -> &RuleMeta {
        &Self::META
    }

    fn check(&self, _node: &AstKind<'_>, _ctx: &mut RuleContext<'_>) {
        // This rule needs full scope analysis, implemented in check_end
    }

    fn check_end(&self, _ctx: &mut RuleContext<'_>) {
        // In a full implementation, we'd analyze the scope tree here
        // For now, this is a placeholder for the semantic analysis

        // The actual implementation would:
        // 1. Build a map of all variable declarations
        // 2. Track all variable references
        // 3. Report variables that are declared but never referenced

        // This requires integration with oxc's semantic analysis
        // which provides scope and reference information
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoUnusedVars::default();
        assert_eq!(rule.meta().name, "no-unused-vars");
        assert!(!rule.meta().fixable);
    }

    #[test]
    fn test_ignored_patterns() {
        let rule = NoUnusedVars::default();
        assert!(rule.is_ignored("_unused"));
        assert!(!rule.is_ignored("used"));
    }
}
