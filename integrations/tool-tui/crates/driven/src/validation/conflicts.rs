//! Conflict detection

use super::Conflict;
use crate::{Result, parser::UnifiedRule};

/// Detects conflicts between rules
#[derive(Debug, Default)]
pub struct ConflictDetector;

impl ConflictDetector {
    /// Create a new conflict detector
    pub fn new() -> Self {
        Self
    }

    /// Detect conflicts in a set of rules
    pub fn detect(&self, rules: &[UnifiedRule]) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        // Check for conflicting naming conventions
        conflicts.extend(self.check_naming_conflicts(rules));

        // Check for contradictory standards
        conflicts.extend(self.check_contradictions(rules));

        Ok(conflicts)
    }

    fn check_naming_conflicts(&self, rules: &[UnifiedRule]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        let mut naming_rules: Vec<&str> = Vec::new();

        for rule in rules {
            if let UnifiedRule::Standard { description, .. } = rule {
                let lower = description.to_lowercase();
                if lower.contains("snake_case")
                    || lower.contains("camelcase")
                    || lower.contains("pascalcase")
                {
                    naming_rules.push(description);
                }
            }
        }

        // Check for conflicting naming rules
        if naming_rules.len() > 1 {
            let has_snake = naming_rules.iter().any(|r| r.to_lowercase().contains("snake_case"));
            let has_camel = naming_rules.iter().any(|r| r.to_lowercase().contains("camelcase"));

            if has_snake && has_camel {
                // Only conflict if they're for the same context
                let func_snake = naming_rules.iter().any(|r| {
                    r.to_lowercase().contains("function") && r.to_lowercase().contains("snake_case")
                });
                let func_camel = naming_rules.iter().any(|r| {
                    r.to_lowercase().contains("function") && r.to_lowercase().contains("camelcase")
                });

                if func_snake && func_camel {
                    conflicts.push(Conflict {
                        description: "Conflicting function naming conventions".to_string(),
                        rules: naming_rules.iter().map(|s| s.to_string()).collect(),
                        suggestion: Some("Choose one naming convention for functions".to_string()),
                    });
                }
            }
        }

        conflicts
    }

    fn check_contradictions(&self, rules: &[UnifiedRule]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // Look for obvious contradictions
        let contradiction_pairs = [
            ("always", "never"),
            ("must", "must not"),
            ("require", "avoid"),
        ];

        let standards: Vec<_> = rules
            .iter()
            .filter_map(|r| {
                if let UnifiedRule::Standard { description, .. } = r {
                    Some(description.as_str())
                } else {
                    None
                }
            })
            .collect();

        for (positive, negative) in contradiction_pairs {
            for &rule1 in &standards {
                for &rule2 in &standards {
                    if rule1 == rule2 {
                        continue;
                    }

                    let r1_lower = rule1.to_lowercase();
                    let r2_lower = rule2.to_lowercase();

                    // Check if same topic with contradicting verbs
                    if (r1_lower.contains(positive) && r2_lower.contains(negative))
                        || (r1_lower.contains(negative) && r2_lower.contains(positive))
                    {
                        // Check for similar topics (simple word overlap)
                        let r1_words: std::collections::HashSet<_> =
                            r1_lower.split_whitespace().filter(|w| w.len() > 4).collect();
                        let r2_words: std::collections::HashSet<_> =
                            r2_lower.split_whitespace().filter(|w| w.len() > 4).collect();

                        let overlap: Vec<_> = r1_words.intersection(&r2_words).collect();
                        if overlap.len() >= 2 {
                            conflicts.push(Conflict {
                                description: "Potentially contradicting rules".to_string(),
                                rules: vec![rule1.to_string(), rule2.to_string()],
                                suggestion: Some("Review these rules for consistency".to_string()),
                            });
                        }
                    }
                }
            }
        }

        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_new() {
        let detector = ConflictDetector::new();
        let conflicts = detector.detect(&[]).unwrap();
        assert!(conflicts.is_empty());
    }
}
