//! HTML rewriting phase of the rebuild pipeline
//!
//! This module handles rewriting HTML with optimized class names
//! based on group analysis results.

use crate::config::RebuildConfig;

use super::types::{GroupAnalysis, GroupInfo, RewritePlan};

/// Rewrite HTML based on group analysis
///
/// This function applies suggested renames and group optimizations
/// to the HTML content.
///
/// # Arguments
///
/// * `html_bytes` - The original HTML content as bytes
/// * `analysis` - The group analysis results
/// * `config` - The rebuild configuration
///
/// # Returns
///
/// An `Option<RewritePlan>` containing the rewritten HTML if changes were made,
/// or `None` if no changes are needed.
#[allow(dead_code)]
pub fn rewrite_html(
    html_bytes: &[u8],
    analysis: &GroupAnalysis,
    config: &RebuildConfig,
) -> Option<RewritePlan> {
    // If no suggested renames and not aggressive rewrite, no changes needed
    if analysis.suggested_renames.is_empty() && !config.aggressive_rewrite {
        return None;
    }

    let html_string = String::from_utf8_lossy(html_bytes).to_string();
    let mut modified_html = html_string.clone();
    let mut modified = false;
    let mut groups = Vec::new();

    // Apply suggested renames
    for (old_name, new_name) in &analysis.suggested_renames {
        // Replace @old_name( with @new_name(
        let old_with_paren = format!("@{}(", old_name);
        let new_with_paren = format!("@{}(", new_name);

        if modified_html.contains(&old_with_paren) {
            modified_html = modified_html.replace(&old_with_paren, &new_with_paren);
            modified = true;

            groups.push(GroupInfo {
                alias: new_name.clone(),
                classes: Vec::new(), // Classes will be populated by caller if needed
            });
        }

        // Also replace standalone @old_name with @new_name
        let old_at = format!("@{}", old_name);
        let new_at = format!("@{}", new_name);

        if modified_html.contains(&old_at) {
            modified_html = modified_html.replace(&old_at, &new_at);
            modified = true;
        }
    }

    if modified {
        Some(RewritePlan {
            html: modified_html.into_bytes(),
            groups,
            modified: true,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::group::GroupRegistry;
    use ahash::AHashSet;

    fn make_analysis(renames: Vec<(String, String)>) -> GroupAnalysis {
        GroupAnalysis {
            registry: GroupRegistry::default(),
            processed_classes: AHashSet::new(),
            suggested_renames: renames,
        }
    }

    #[test]
    fn test_rewrite_html_no_changes() {
        let html = b"<div class=\"flex\">content</div>";
        let analysis = make_analysis(vec![]);
        let config = RebuildConfig::default();

        let result = rewrite_html(html, &analysis, &config);
        assert!(result.is_none());
    }

    #[test]
    fn test_rewrite_html_with_rename() {
        let html = b"<div class=\"@old(flex p-4)\">content</div>";
        let analysis = make_analysis(vec![("old".to_string(), "new".to_string())]);
        let config = RebuildConfig::default();

        let result = rewrite_html(html, &analysis, &config);
        assert!(result.is_some());

        let plan = result.unwrap();
        assert!(plan.modified);
        let html_str = String::from_utf8_lossy(&plan.html);
        assert!(html_str.contains("@new("));
        assert!(!html_str.contains("@old("));
    }
}
