//! Output writing phase of the rebuild pipeline
//!
//! This module handles writing CSS output to the configured destination,
//! supporting both full and incremental write modes.

use super::types::{GeneratedCss, WriteMode};

/// Statistics from write operation
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct WriteStats {
    /// Number of rules written
    pub rules_written: usize,
    /// Total bytes written
    pub bytes_written: usize,
}

/// Prepare CSS content for writing
///
/// This function prepares the CSS content based on the write mode.
/// The actual writing is handled by the caller using CssOutput.
///
/// # Arguments
///
/// * `css` - The generated CSS to prepare
/// * `mode` - The write mode (Full or Incremental)
///
/// # Returns
///
/// A tuple of (css_bytes, stats) where css_bytes is the prepared CSS content
/// and stats contains write statistics.
#[allow(dead_code)]
pub fn prepare_output(css: &GeneratedCss, mode: &WriteMode) -> (Vec<u8>, WriteStats) {
    let mut stats = WriteStats::default();
    let mut output = Vec::new();

    match mode {
        WriteMode::Full => {
            // Full rebuild - include all CSS rules
            for rule in &css.rules {
                if !output.is_empty() {
                    output.push(b'\n');
                }
                output.extend_from_slice(rule.css.as_bytes());
                stats.rules_written += 1;
                stats.bytes_written += rule.css.len();
            }
        }
        WriteMode::Incremental { added, .. } => {
            // Incremental update - only include added rules
            for rule in &css.rules {
                if added.contains(&rule.class_name) {
                    if !output.is_empty() {
                        output.push(b'\n');
                    }
                    output.extend_from_slice(rule.css.as_bytes());
                    stats.rules_written += 1;
                    stats.bytes_written += rule.css.len();
                }
            }
        }
    }

    (output, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::types::CssRule;

    #[test]
    fn test_write_stats_default() {
        let stats = WriteStats::default();
        assert_eq!(stats.rules_written, 0);
        assert_eq!(stats.bytes_written, 0);
    }

    #[test]
    fn test_prepare_output_full() {
        let css = GeneratedCss {
            rules: vec![
                CssRule {
                    class_name: "flex".to_string(),
                    css: "display:flex".to_string(),
                },
                CssRule {
                    class_name: "p-4".to_string(),
                    css: "padding:1rem".to_string(),
                },
            ],
            total_bytes: 24,
        };

        let (output, stats) = prepare_output(&css, &WriteMode::Full);

        assert_eq!(stats.rules_written, 2);
        assert!(output.len() > 0);
    }

    #[test]
    fn test_prepare_output_incremental() {
        let css = GeneratedCss {
            rules: vec![
                CssRule {
                    class_name: "flex".to_string(),
                    css: "display:flex".to_string(),
                },
                CssRule {
                    class_name: "p-4".to_string(),
                    css: "padding:1rem".to_string(),
                },
            ],
            total_bytes: 24,
        };

        let (output, stats) = prepare_output(
            &css,
            &WriteMode::Incremental {
                added: vec!["flex".to_string()],
                removed: vec![],
            },
        );

        // Only "flex" should be included
        assert_eq!(stats.rules_written, 1);
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("display:flex"));
        assert!(!output_str.contains("padding:1rem"));
    }
}
