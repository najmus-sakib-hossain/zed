//! Coverage analysis

use crate::{Result, format::RuleCategory, parser::UnifiedRule};

/// Analyzes rule coverage
#[derive(Debug, Default)]
pub struct CoverageAnalyzer;

impl CoverageAnalyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze coverage gaps
    pub fn analyze(&self, rules: &[UnifiedRule]) -> Result<Vec<String>> {
        let mut gaps = Vec::new();

        // Check for coverage of important categories
        let covered_categories = self.get_covered_categories(rules);

        let recommended = [
            RuleCategory::Style,
            RuleCategory::Naming,
            RuleCategory::ErrorHandling,
            RuleCategory::Testing,
            RuleCategory::Documentation,
        ];

        for cat in recommended {
            if !covered_categories.contains(&cat) {
                gaps.push(format!("No rules for category: {:?}", cat));
            }
        }

        // Check for persona
        let has_persona = rules.iter().any(|r| matches!(r, UnifiedRule::Persona { .. }));
        if !has_persona {
            gaps.push("No AI persona defined".to_string());
        }

        // Check for context
        let has_context = rules.iter().any(|r| matches!(r, UnifiedRule::Context { .. }));
        if !has_context {
            gaps.push("No project context defined".to_string());
        }

        Ok(gaps)
    }

    fn get_covered_categories(
        &self,
        rules: &[UnifiedRule],
    ) -> std::collections::HashSet<RuleCategory> {
        rules
            .iter()
            .filter_map(|r| {
                if let UnifiedRule::Standard { category, .. } = r {
                    Some(*category)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_empty() {
        let analyzer = CoverageAnalyzer::new();
        let gaps = analyzer.analyze(&[]).unwrap();

        // Should report missing categories and no persona/context
        assert!(!gaps.is_empty());
    }

    #[test]
    fn test_analyzer_complete() {
        let analyzer = CoverageAnalyzer::new();
        let rules = vec![
            UnifiedRule::Persona {
                name: "Test".to_string(),
                role: "Tester".to_string(),
                identity: None,
                style: None,
                traits: vec![],
                principles: vec![],
            },
            UnifiedRule::Context {
                includes: vec![],
                excludes: vec![],
                focus: vec![],
            },
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 0,
                description: "Style rule".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::Naming,
                priority: 0,
                description: "Naming rule".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::ErrorHandling,
                priority: 0,
                description: "Error rule".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::Testing,
                priority: 0,
                description: "Testing rule".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::Documentation,
                priority: 0,
                description: "Doc rule".to_string(),
                pattern: None,
            },
        ];

        let gaps = analyzer.analyze(&rules).unwrap();
        assert!(gaps.is_empty());
    }
}
