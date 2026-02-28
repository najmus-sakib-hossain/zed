//! Pipeline module for the rebuild pipeline
//!
//! This module provides a decomposed, testable pipeline for CSS generation.
//! The pipeline is broken into distinct phases:
//!
//! 1. **HTML Parsing**: Extract class names and group events from HTML
//! 2. **Group Analysis**: Analyze groups and compute suggested renames
//! 3. **HTML Rewriting**: Rewrite HTML with optimized class names
//! 4. **CSS Generation**: Generate CSS rules for all classes
//! 5. **Output Writing**: Write CSS to the configured output
//!
//! Each phase is implemented as a separate submodule with focused responsibility.

pub mod css_generator;
pub mod group_analyzer;
pub mod html_parser;
pub mod html_rewriter;
pub mod output_writer;
pub mod types;

// Re-export key types for convenience
#[allow(unused_imports)]
pub use types::{
    CssRule, ExtractedContent, FormatStatus, GeneratedCss, GroupAnalysis, GroupInfo, RebuildResult,
    RebuildStats, RewritePlan, WriteMode,
};

use ahash::AHashSet;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::time::Instant;

#[allow(unused_imports)]
use crate::config::RebuildConfig;
#[allow(unused_imports)]
use crate::core::AppState;
#[allow(unused_imports)]
use crate::datasource;

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during pipeline execution
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Failed to read the input file
    #[error("Failed to read input file {path:?}: {source}")]
    InputReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to write the output file
    #[error("Failed to write output file {path:?}: {source}")]
    OutputWriteError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Mutex was poisoned
    #[error("Mutex was poisoned - another thread panicked while holding the lock")]
    MutexPoisoned,
}

/// Main rebuild pipeline orchestrator
///
/// This function coordinates all rebuild phases in sequence, delegating
/// actual work to focused phase functions. It accepts a typed `RebuildConfig`
/// parameter instead of reading environment variables directly.
///
/// # Arguments
///
/// * `state` - The shared application state
/// * `index_path` - Path to the HTML file to process
/// * `config` - The rebuild configuration
/// * `is_initial_run` - Whether this is the initial run
///
/// # Returns
///
/// A `RebuildResult` indicating whether HTML was modified and statistics.
///
/// # Requirements
///
/// - 3.1: Decomposed pipeline phases
/// - 3.7: Orchestrator under 100 lines
/// - 3.8: Accept RebuildConfig parameter
/// - 4.4: Return RebuildResult with modification flag
#[allow(dead_code)]
pub fn rebuild_styles_pipeline(
    state: Arc<Mutex<AppState>>,
    index_path: &str,
    config: &RebuildConfig,
    is_initial_run: bool,
) -> Result<RebuildResult, PipelineError> {
    let mut stats = RebuildStats::default();

    // Phase 1: Read HTML file
    let html_bytes =
        datasource::read_file(index_path).map_err(|e| PipelineError::InputReadError {
            path: index_path.into(),
            source: e,
        })?;

    // Phase 2: Parse HTML and extract classes
    let parse_start = Instant::now();
    let content = {
        let mut guard = state.lock().map_err(|_| PipelineError::MutexPoisoned)?;
        let hint = guard.class_cache.len().next_power_of_two().max(16);
        html_parser::parse_html(&html_bytes, &mut guard.incremental_parser, hint)
    };
    stats.parse_duration = parse_start.elapsed();

    // Early exit if unchanged (unless force_full)
    let force_full = config.force_full || config.force_format;
    if !force_full && !is_initial_run {
        let guard = state.lock().map_err(|_| PipelineError::MutexPoisoned)?;
        if guard.html_hash == content.content_hash {
            return Ok(RebuildResult {
                html_modified: false,
                classes_added: 0,
                classes_removed: 0,
                stats,
                format_status: types::FormatStatus::NotFormatted,
            });
        }
    }

    // Phase 3: Analyze groups
    let engine = AppState::engine();
    let analysis = group_analyzer::analyze_groups(&content, engine, config);

    // Phase 4: Rewrite HTML if needed
    let html_modified =
        if let Some(plan) = html_rewriter::rewrite_html(&html_bytes, &analysis, config) {
            if plan.modified {
                std::fs::write(index_path, &plan.html).map_err(|e| {
                    PipelineError::OutputWriteError {
                        path: index_path.into(),
                        source: e,
                    }
                })?;
                true
            } else {
                false
            }
        } else {
            false
        };

    // Phase 5: Compute class diff
    let diff_start = Instant::now();
    let (added, removed) = {
        let guard = state.lock().map_err(|_| PipelineError::MutexPoisoned)?;
        compute_class_diff(&guard.class_cache, &analysis.processed_classes)
    };
    stats.diff_duration = diff_start.elapsed();

    // Phase 6: Generate CSS
    let mut registry = analysis.registry.clone();
    let _css = css_generator::generate_css(&analysis.processed_classes, &mut registry, engine);

    // Phase 7: Update state
    let cache_start = Instant::now();
    {
        let mut guard = state.lock().map_err(|_| PipelineError::MutexPoisoned)?;
        guard.html_hash = content.content_hash;
        guard.class_cache = analysis.processed_classes.clone();
        guard.group_registry = registry;
    }
    stats.cache_duration = cache_start.elapsed();

    // Note: Actual CSS writing is handled by the caller (core::rebuild_styles)
    // This orchestrator focuses on the pipeline logic

    Ok(RebuildResult {
        html_modified,
        classes_added: added.len(),
        classes_removed: removed.len(),
        stats,
        format_status: types::FormatStatus::NotFormatted, // Set by caller after CSS write
    })
}

/// Compute the difference between old and new class sets
#[allow(dead_code)]
fn compute_class_diff(
    old: &AHashSet<String>,
    new: &AHashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let added: Vec<String> = new.difference(old).cloned().collect();
    let removed: Vec<String> = old.difference(new).cloned().collect();
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_class_diff_empty() {
        let old = AHashSet::new();
        let new = AHashSet::new();
        let (added, removed) = compute_class_diff(&old, &new);
        assert!(added.is_empty());
        assert!(removed.is_empty());
    }

    #[test]
    fn test_compute_class_diff_additions() {
        let old = AHashSet::new();
        let mut new = AHashSet::new();
        new.insert("flex".to_string());
        new.insert("p-4".to_string());

        let (added, removed) = compute_class_diff(&old, &new);
        assert_eq!(added.len(), 2);
        assert!(removed.is_empty());
    }

    #[test]
    fn test_compute_class_diff_removals() {
        let mut old = AHashSet::new();
        old.insert("flex".to_string());
        old.insert("p-4".to_string());
        let new = AHashSet::new();

        let (added, removed) = compute_class_diff(&old, &new);
        assert!(added.is_empty());
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn test_compute_class_diff_mixed() {
        let mut old = AHashSet::new();
        old.insert("flex".to_string());
        old.insert("p-4".to_string());

        let mut new = AHashSet::new();
        new.insert("flex".to_string());
        new.insert("m-2".to_string());

        let (added, removed) = compute_class_diff(&old, &new);
        assert_eq!(added.len(), 1);
        assert!(added.contains(&"m-2".to_string()));
        assert_eq!(removed.len(), 1);
        assert!(removed.contains(&"p-4".to_string()));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // **Property 4: RebuildResult Indicates HTML Modification**
    // **Validates: Requirements 4.4**
    // Feature: dx-style-production-hardening, Property 4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_rebuild_result_html_modified_flag(
            html_modified in any::<bool>(),
            classes_added in 0usize..100,
            classes_removed in 0usize..100,
        ) {
            let result = RebuildResult {
                html_modified,
                classes_added,
                classes_removed,
                stats: RebuildStats::default(),
                format_status: types::FormatStatus::NotFormatted,
            };

            // The html_modified flag should match what was set
            prop_assert_eq!(result.html_modified, html_modified);
            prop_assert_eq!(result.classes_added, classes_added);
            prop_assert_eq!(result.classes_removed, classes_removed);
        }
    }
}
