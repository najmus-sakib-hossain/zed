//! Rule difference calculator

use crate::parser::UnifiedRule;

/// Calculates differences between rule sets
#[derive(Debug, Default)]
pub struct RuleDiffer;

impl RuleDiffer {
    /// Create a new differ
    pub fn new() -> Self {
        Self
    }

    /// Calculate differences between two rule sets
    pub fn diff(&self, old: &[UnifiedRule], new: &[UnifiedRule]) -> RuleDiff {
        let mut diff = RuleDiff::default();

        // Simple comparison based on type and content
        for new_rule in new {
            if !self.contains_similar(old, new_rule) {
                diff.added.push(new_rule.clone());
            }
        }

        for old_rule in old {
            if !self.contains_similar(new, old_rule) {
                diff.removed.push(old_rule.clone());
            }
        }

        diff
    }

    fn contains_similar(&self, rules: &[UnifiedRule], target: &UnifiedRule) -> bool {
        rules.iter().any(|r| self.rules_similar(r, target))
    }

    fn rules_similar(&self, a: &UnifiedRule, b: &UnifiedRule) -> bool {
        match (a, b) {
            (
                UnifiedRule::Standard {
                    description: desc_a,
                    ..
                },
                UnifiedRule::Standard {
                    description: desc_b,
                    ..
                },
            ) => desc_a == desc_b,
            (
                UnifiedRule::Persona { name: name_a, .. },
                UnifiedRule::Persona { name: name_b, .. },
            ) => name_a == name_b,
            (UnifiedRule::Raw { content: a }, UnifiedRule::Raw { content: b }) => a == b,
            _ => false,
        }
    }
}

/// Difference between two rule sets
#[derive(Debug, Default, Clone)]
pub struct RuleDiff {
    /// Rules added in new set
    pub added: Vec<UnifiedRule>,
    /// Rules removed from old set
    pub removed: Vec<UnifiedRule>,
}

impl RuleDiff {
    /// Check if there are any differences
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty()
    }

    /// Get total number of changes
    pub fn change_count(&self) -> usize {
        self.added.len() + self.removed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::RuleCategory;

    #[test]
    fn test_diff_identical() {
        let differ = RuleDiffer::new();
        let rules = vec![UnifiedRule::Standard {
            category: RuleCategory::Style,
            priority: 0,
            description: "Test rule".to_string(),
            pattern: None,
        }];

        let diff = differ.diff(&rules, &rules);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_diff_added() {
        let differ = RuleDiffer::new();
        let old = vec![];
        let new = vec![UnifiedRule::Standard {
            category: RuleCategory::Style,
            priority: 0,
            description: "New rule".to_string(),
            pattern: None,
        }];

        let diff = differ.diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 0);
    }

    #[test]
    fn test_diff_removed() {
        let differ = RuleDiffer::new();
        let old = vec![UnifiedRule::Standard {
            category: RuleCategory::Style,
            priority: 0,
            description: "Old rule".to_string(),
            pattern: None,
        }];
        let new = vec![];

        let diff = differ.diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 1);
    }
}
