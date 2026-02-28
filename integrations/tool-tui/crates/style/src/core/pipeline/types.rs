//! Pipeline data types for the rebuild pipeline
//!
//! This module defines the data structures used to pass information between
//! pipeline phases, enabling clean separation of concerns and testability.

use ahash::AHashSet;
use std::time::Duration;

use crate::core::group::GroupRegistry;
use crate::parser::GroupEvent;

/// Result of HTML parsing phase
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Set of unique class names found
    pub classes: AHashSet<String>,
    /// Group events for auto-grouping analysis
    pub group_events: Vec<GroupEvent>,
    /// Hash of the parsed HTML content
    pub content_hash: u64,
}

/// Result of group analysis phase
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GroupAnalysis {
    /// The group registry with analyzed groups
    pub registry: GroupRegistry,
    /// Classes after group processing
    pub processed_classes: AHashSet<String>,
    /// Suggested renames (old_name -> new_name)
    pub suggested_renames: Vec<(String, String)>,
}

/// Plan for HTML rewriting
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RewritePlan {
    /// The rewritten HTML bytes
    pub html: Vec<u8>,
    /// Groups that were created/modified
    pub groups: Vec<GroupInfo>,
    /// Whether any changes were made
    pub modified: bool,
}

/// Information about a group
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// The alias name for the group
    pub alias: String,
    /// The classes in the group
    pub classes: Vec<String>,
}

/// Result of CSS generation phase
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GeneratedCss {
    /// The generated CSS rules
    pub rules: Vec<CssRule>,
    /// Total bytes of CSS generated
    pub total_bytes: usize,
}

/// A single CSS rule
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CssRule {
    /// The class name (selector)
    pub class_name: String,
    /// The CSS content
    pub css: String,
}

/// Format status indicating what happened during a format operation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatStatus {
    /// CSS was rewritten
    Rewritten,
    /// CSS was unchanged
    Unchanged,
    /// No format operation was performed
    NotFormatted,
}

#[allow(dead_code)]
impl FormatStatus {
    /// Convert to string representation (for compatibility with external tools)
    pub fn as_str(&self) -> &'static str {
        match self {
            FormatStatus::Rewritten => "rewritten",
            FormatStatus::Unchanged => "unchanged",
            FormatStatus::NotFormatted => "not_formatted",
        }
    }
}

/// Result of the entire rebuild operation
#[allow(dead_code)]
#[derive(Debug)]
pub struct RebuildResult {
    /// Whether HTML was modified during this rebuild
    pub html_modified: bool,
    /// Number of classes added
    pub classes_added: usize,
    /// Number of classes removed
    pub classes_removed: usize,
    /// Processing statistics
    pub stats: RebuildStats,
    /// Format status (replaces DX_FORMAT_STATUS env var)
    pub format_status: FormatStatus,
}

/// Statistics from rebuild operation
#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct RebuildStats {
    /// Time spent computing content hash
    pub hash_duration: Duration,
    /// Time spent parsing HTML
    pub parse_duration: Duration,
    /// Time spent computing class diff
    pub diff_duration: Duration,
    /// Time spent on cache operations
    pub cache_duration: Duration,
    /// Time spent writing output
    pub write_duration: Duration,
}

/// Mode for writing CSS output
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum WriteMode {
    /// Full rebuild - write all CSS
    Full,
    /// Incremental update - only write changed classes
    Incremental {
        /// Classes that were added
        added: Vec<String>,
        /// Classes that were removed
        removed: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_content_default() {
        let content = ExtractedContent {
            classes: AHashSet::new(),
            group_events: Vec::new(),
            content_hash: 0,
        };
        assert!(content.classes.is_empty());
        assert!(content.group_events.is_empty());
    }

    #[test]
    fn test_rebuild_stats_default() {
        let stats = RebuildStats::default();
        assert_eq!(stats.hash_duration, Duration::ZERO);
        assert_eq!(stats.parse_duration, Duration::ZERO);
    }

    #[test]
    fn test_write_mode_variants() {
        let full = WriteMode::Full;
        let incremental = WriteMode::Incremental {
            added: vec!["flex".to_string()],
            removed: vec!["block".to_string()],
        };

        match full {
            WriteMode::Full => {}
            _ => panic!("Expected Full variant"),
        }

        match incremental {
            WriteMode::Incremental { added, removed } => {
                assert_eq!(added.len(), 1);
                assert_eq!(removed.len(), 1);
            }
            _ => panic!("Expected Incremental variant"),
        }
    }
}
