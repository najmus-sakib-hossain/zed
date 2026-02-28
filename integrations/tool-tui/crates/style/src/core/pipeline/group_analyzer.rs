//! Group analysis phase of the rebuild pipeline
//!
//! This module analyzes groups from extracted content and computes
//! suggested renames for better group organization.

use ahash::AHashSet;

use crate::config::RebuildConfig;
use crate::core::engine::StyleEngine;
use crate::core::group::GroupRegistry;

use super::types::{ExtractedContent, GroupAnalysis};

/// Analyze groups from extracted content
///
/// This function processes the extracted content to analyze groups and
/// compute suggested renames based on similarity thresholds.
///
/// # Arguments
///
/// * `content` - The extracted content from HTML parsing
/// * `engine` - The style engine for CSS generation
/// * `config` - The rebuild configuration
///
/// # Returns
///
/// A `GroupAnalysis` struct containing:
/// - The group registry with analyzed groups
/// - Processed classes after group analysis
/// - Suggested renames for group optimization
#[allow(dead_code)]
pub fn analyze_groups(
    content: &ExtractedContent,
    engine: &StyleEngine,
    config: &RebuildConfig,
) -> GroupAnalysis {
    let mut classes = content.classes.clone();

    // Analyze groups from group events
    let registry = GroupRegistry::analyze(&content.group_events, &mut classes, Some(engine));

    // Compute suggested renames based on similarity threshold
    let suggested_renames = compute_suggested_renames(&registry, config.group_rename_threshold);

    GroupAnalysis {
        registry,
        processed_classes: classes,
        suggested_renames,
    }
}

/// Compute suggested renames for groups based on similarity
///
/// This function analyzes the group registry and suggests renames
/// for groups that have high similarity to existing groups.
///
/// # Arguments
///
/// * `registry` - The group registry to analyze
/// * `threshold` - The similarity threshold (0.0 - 1.0)
///
/// # Returns
///
/// A vector of (old_name, new_name) pairs for suggested renames
#[allow(dead_code)]
pub fn compute_suggested_renames(
    registry: &GroupRegistry,
    threshold: f64,
) -> Vec<(String, String)> {
    let mut renames = Vec::new();

    // Collect current alias names
    let current_alias_names: AHashSet<String> =
        registry.definitions().map(|(name, _)| name.clone()).collect();

    // Normalize definitions for comparison
    let mut current_defs_norm: Vec<(String, AHashSet<String>)> = Vec::new();
    for (name, def) in registry.definitions() {
        let mut set: AHashSet<String> = AHashSet::default();
        for u in &def.utilities {
            if u.is_empty() || u.contains('@') || current_alias_names.contains(u) {
                continue;
            }
            if registry.is_internal_token(u) {
                continue;
            }
            set.insert(u.clone());
        }
        if !set.is_empty() {
            current_defs_norm.push((name.clone(), set));
        }
    }

    // Find similar groups and suggest renames
    for (i, (name1, set1)) in current_defs_norm.iter().enumerate() {
        for (name2, set2) in current_defs_norm.iter().skip(i + 1) {
            let intersection = set1.iter().filter(|x| set2.contains(*x)).count();
            let union = set1.len() + set2.len() - intersection;

            if union > 0 {
                let similarity = intersection as f64 / union as f64;
                if similarity >= threshold && name1 != name2 {
                    // Suggest renaming the shorter name to the longer one
                    // (assuming longer names are more descriptive)
                    if name1.len() < name2.len() {
                        renames.push((name1.clone(), name2.clone()));
                    } else {
                        renames.push((name2.clone(), name1.clone()));
                    }
                }
            }
        }
    }

    renames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_suggested_renames_empty() {
        let registry = GroupRegistry::default();
        let renames = compute_suggested_renames(&registry, 0.6);
        assert!(renames.is_empty());
    }
}
