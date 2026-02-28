//! Auto-grouping module for automatic CSS class pattern detection and grouping
//!
//! This module provides functionality to automatically detect frequently co-occurring
//! CSS class patterns and generate optimized group classnames for them.

use ahash::AHashSet;

use crate::core::group::{GroupDefinition, GroupRegistry};
use crate::similarity::SimilarityDetector;

use super::classname::ClassnameGenerator;

/// Configuration for auto-grouping behavior.
#[derive(Debug, Clone)]
pub struct AutoGroupConfig {
    /// Whether auto-grouping is enabled
    pub enabled: bool,
    /// Minimum occurrences of a pattern to consider for grouping
    pub min_occurrences: usize,
    /// Jaccard similarity threshold for clustering (0.0-1.0)
    pub similarity_threshold: f64,
    /// Patterns containing these substrings are excluded
    pub excluded_patterns: Vec<String>,
    /// Whether to automatically rewrite HTML with new classnames
    pub auto_rewrite: bool,
}

impl Default for AutoGroupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_occurrences: 3,
            similarity_threshold: 0.6,
            excluded_patterns: Vec::new(),
            auto_rewrite: true,
        }
    }
}

/// Information about an auto-generated group.
#[derive(Debug, Clone)]
pub struct AutoGroupInfo {
    /// The generated classname alias (e.g., "dxg-abc12")
    pub alias: String,
    /// The classes that make up this group
    pub classes: Vec<String>,
}

/// Result of auto-grouping processing.
#[derive(Debug, Clone)]
pub struct AutoGroupRewrite {
    /// The rewritten HTML (if auto_rewrite is enabled)
    pub html: Vec<u8>,
    /// Information about generated groups
    pub groups: Vec<AutoGroupInfo>,
}

/// Orchestrates pattern detection, classname generation, and HTML rewriting.
pub struct AutoGrouper {
    detector: SimilarityDetector,
    config: AutoGroupConfig,
    existing_classes: AHashSet<String>,
}

impl AutoGrouper {
    /// Create a new AutoGrouper with the given configuration.
    pub fn new(config: AutoGroupConfig, existing_classes: AHashSet<String>) -> Self {
        Self {
            detector: SimilarityDetector::new(config.min_occurrences, config.similarity_threshold),
            config,
            existing_classes,
        }
    }

    /// Process HTML and return auto-grouping results.
    ///
    /// Returns `None` if auto-grouping is disabled or no patterns were found.
    pub fn process(&mut self, html: &[u8]) -> Option<AutoGroupRewrite> {
        if !self.config.enabled {
            return None;
        }

        // Scan for patterns
        self.detector.scan(html);

        // Get clusters
        let clusters = self.detector.cluster_patterns();
        if clusters.is_empty() {
            return None;
        }

        // Generate groups for each cluster
        let mut generator = ClassnameGenerator::new(self.existing_classes.clone());
        let mut groups = Vec::new();

        for cluster in clusters {
            let classes: Vec<String> = cluster.representative.classes().to_vec();

            // Check exclusions
            if self.is_excluded(&classes) {
                continue;
            }

            // Generate classname
            let alias = generator.generate(&classes);

            groups.push(AutoGroupInfo { alias, classes });
        }

        if groups.is_empty() {
            return None;
        }

        // Rewrite HTML if enabled
        let rewritten_html = if self.config.auto_rewrite {
            self.rewrite_html(html, &groups)
        } else {
            html.to_vec()
        };

        Some(AutoGroupRewrite {
            html: rewritten_html,
            groups,
        })
    }

    /// Apply auto-grouping results to a GroupRegistry.
    pub fn apply_to_registry(&self, groups: &[AutoGroupInfo], registry: &mut GroupRegistry) {
        for group in groups {
            registry.add_definition(
                group.alias.clone(),
                GroupDefinition {
                    utilities: group.classes.clone(),
                    allow_extend: false,
                    raw_tokens: Vec::new(),
                    dev_tokens: Vec::new(),
                },
            );
        }
    }

    /// Check if a pattern should be excluded based on configuration.
    fn is_excluded(&self, classes: &[String]) -> bool {
        for pattern in &self.config.excluded_patterns {
            for class in classes {
                if class.contains(pattern) {
                    return true;
                }
            }
        }
        false
    }

    /// Rewrite HTML to use generated group classnames.
    fn rewrite_html(&self, html: &[u8], groups: &[AutoGroupInfo]) -> Vec<u8> {
        let mut html_string = String::from_utf8_lossy(html).to_string();

        for group in groups {
            // Build a set of classes in this group for fast lookup
            let pattern_set: AHashSet<&String> = group.classes.iter().collect();

            // Replace matching class attributes
            html_string = self.replace_class_attrs(&html_string, &pattern_set, &group.alias);
        }

        html_string.into_bytes()
    }

    /// Replace class attributes that contain all classes in the pattern.
    fn replace_class_attrs(&self, html: &str, pattern: &AHashSet<&String>, alias: &str) -> String {
        // Simple regex-free approach: find class="..." and class='...' attributes
        let mut result = String::with_capacity(html.len());
        let mut pos = 0;
        let bytes = html.as_bytes();

        while pos < bytes.len() {
            // Look for class=" or class='
            if pos + 7 < bytes.len() && &bytes[pos..pos + 6] == b"class=" {
                let quote = bytes[pos + 6];
                if quote == b'"' || quote == b'\'' {
                    let start = pos + 7;
                    let mut end = start;

                    // Find closing quote
                    while end < bytes.len() && bytes[end] != quote {
                        end += 1;
                    }

                    if end < bytes.len() {
                        // Extract class value
                        let class_value = &html[start..end];
                        let classes: Vec<&str> = class_value.split_whitespace().collect();

                        // Check if all pattern classes are present
                        let class_set: AHashSet<String> =
                            classes.iter().map(|s| s.to_string()).collect();
                        let all_present = pattern.iter().all(|p| class_set.contains(*p));

                        // Only replace if pattern has multiple unique classes
                        if all_present && !pattern.is_empty() && pattern.len() > 1 {
                            // Replace pattern classes with alias
                            let mut new_classes: Vec<&str> = classes
                                .iter()
                                .filter(|c| !pattern.contains(&c.to_string()))
                                .copied()
                                .collect();
                            new_classes.push(alias);

                            // Write the modified attribute
                            result.push_str(&html[..pos]);
                            result.push_str("class=");
                            result.push(quote as char);
                            result.push_str(&new_classes.join(" "));
                            result.push(quote as char);

                            pos = end + 1;
                            continue;
                        }
                    }
                }
            }

            result.push(bytes[pos] as char);
            pos += 1;
        }

        result
    }

    /// Get the configuration.
    pub fn config(&self) -> &AutoGroupConfig {
        &self.config
    }

    /// Clear the detector's recorded patterns.
    pub fn clear(&mut self) {
        self.detector.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_group_config_default() {
        let config = AutoGroupConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_occurrences, 3);
        assert!((config.similarity_threshold - 0.6).abs() < 0.001);
        assert!(config.excluded_patterns.is_empty());
        assert!(config.auto_rewrite);
    }

    #[test]
    fn test_auto_grouper_disabled() {
        let config = AutoGroupConfig {
            enabled: false,
            ..Default::default()
        };
        let mut grouper = AutoGrouper::new(config, AHashSet::new());
        let html = b"<div class=\"flex items-center\"></div>";
        let result = grouper.process(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_grouper_no_patterns() {
        let config = AutoGroupConfig {
            min_occurrences: 10, // High threshold
            ..Default::default()
        };
        let mut grouper = AutoGrouper::new(config, AHashSet::new());
        let html = b"<div class=\"flex items-center\"></div>";
        let result = grouper.process(html);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_grouper_finds_patterns() {
        let config = AutoGroupConfig {
            min_occurrences: 2,
            ..Default::default()
        };
        let mut grouper = AutoGrouper::new(config, AHashSet::new());

        // HTML with repeated pattern
        let html = br#"
            <div class="flex items-center p-4">Content 1</div>
            <div class="flex items-center p-4">Content 2</div>
            <div class="flex items-center p-4">Content 3</div>
        "#;

        let result = grouper.process(html);
        assert!(result.is_some());

        let rewrite = result.unwrap();
        assert!(!rewrite.groups.is_empty());

        // Check that groups have valid aliases
        for group in &rewrite.groups {
            assert!(group.alias.starts_with("dxg-"));
            assert!(!group.classes.is_empty());
        }
    }

    #[test]
    fn test_auto_grouper_excludes_patterns() {
        let config = AutoGroupConfig {
            min_occurrences: 2,
            excluded_patterns: vec!["hover:".to_string()],
            ..Default::default()
        };
        let mut grouper = AutoGrouper::new(config, AHashSet::new());

        let html = br#"
            <div class="hover:bg-blue flex">Content 1</div>
            <div class="hover:bg-blue flex">Content 2</div>
            <div class="hover:bg-blue flex">Content 3</div>
        "#;

        let result = grouper.process(html);

        // Should either be None or have no groups with hover: classes
        if let Some(rewrite) = result {
            for group in &rewrite.groups {
                for class in &group.classes {
                    assert!(!class.contains("hover:"), "Group should not contain excluded pattern");
                }
            }
        }
    }

    #[test]
    fn test_is_excluded() {
        let config = AutoGroupConfig {
            excluded_patterns: vec!["hover:".to_string(), "focus:".to_string()],
            ..Default::default()
        };
        let grouper = AutoGrouper::new(config, AHashSet::new());

        assert!(grouper.is_excluded(&["hover:bg-blue".to_string(), "flex".to_string()]));
        assert!(grouper.is_excluded(&["focus:ring".to_string()]));
        assert!(!grouper.is_excluded(&["flex".to_string(), "items-center".to_string()]));
    }

    #[test]
    fn test_apply_to_registry() {
        let config = AutoGroupConfig::default();
        let grouper = AutoGrouper::new(config, AHashSet::new());

        let groups = vec![
            AutoGroupInfo {
                alias: "dxg-test1".to_string(),
                classes: vec!["flex".to_string(), "items-center".to_string()],
            },
            AutoGroupInfo {
                alias: "dxg-test2".to_string(),
                classes: vec!["bg-white".to_string(), "rounded".to_string()],
            },
        ];

        let mut registry = GroupRegistry::new();
        grouper.apply_to_registry(&groups, &mut registry);

        assert!(!registry.is_empty());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Generate a vector of class names
    fn arb_class_names() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec("[a-z][a-z0-9-]{0,10}", 2..6)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Feature: dx-style-production-ready, Property 10: HTML Rewrite Preservation
        /// *For any* HTML rewrite operation, all classes NOT in the grouped pattern
        /// SHALL remain in the output class attribute.
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_html_rewrite_preservation(
            pattern_classes in arb_class_names(),
            extra_class in "[a-z][a-z0-9-]{1,10}"
        ) {
            let config = AutoGroupConfig::default();
            let grouper = AutoGrouper::new(config, AHashSet::new());

            // Create a class attribute with pattern + extra class
            let mut all_classes = pattern_classes.clone();
            all_classes.push(extra_class.clone());
            let class_attr = all_classes.join(" ");

            let html = format!(r#"<div class="{}"></div>"#, class_attr);

            let pattern_set: AHashSet<&String> = pattern_classes.iter().collect();
            let alias = "dxg-test1";

            let result = grouper.replace_class_attrs(&html, &pattern_set, alias);

            // The extra class should still be present
            prop_assert!(
                result.contains(&extra_class),
                "Extra class '{}' should be preserved in result: {}",
                extra_class,
                result
            );

            // The alias should be present only if pattern has multiple UNIQUE classes
            if pattern_set.len() > 1 {
                prop_assert!(
                    result.contains(alias),
                    "Alias '{}' should be present in result: {}",
                    alias,
                    result
                );
            }
        }

        /// Property: Excluded patterns are never grouped
        #[test]
        fn prop_excluded_patterns_not_grouped(
            excluded in "[a-z][a-z0-9-]{1,5}",
            other_classes in arb_class_names()
        ) {
            let config = AutoGroupConfig {
                excluded_patterns: vec![excluded.clone()],
                ..Default::default()
            };
            let grouper = AutoGrouper::new(config, AHashSet::new());

            // Create classes that include the excluded pattern
            let mut classes = other_classes;
            classes.push(format!("{}:something", excluded));

            prop_assert!(
                grouper.is_excluded(&classes),
                "Classes containing '{}' should be excluded",
                excluded
            );
        }
    }
}
