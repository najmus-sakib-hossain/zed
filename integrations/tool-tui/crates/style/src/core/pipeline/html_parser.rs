//! HTML parsing phase of the rebuild pipeline
//!
//! This module extracts class names and group events from HTML content.
//! It uses the incremental parser for efficient re-parsing of changed content.

use ahash::AHasher;
use std::hash::Hasher;

use crate::parser::IncrementalParser;

use super::types::ExtractedContent;

/// Parse HTML and extract class information
///
/// This function extracts all class names and group events from the provided HTML bytes.
/// It uses the incremental parser for efficient re-parsing when only parts of the HTML
/// have changed.
///
/// # Arguments
///
/// * `html_bytes` - The raw HTML content as bytes
/// * `incremental_parser` - The incremental parser instance for efficient parsing
/// * `capacity_hint` - Hint for initial capacity of the class set
///
/// # Returns
///
/// An `ExtractedContent` struct containing:
/// - All unique class names found in the HTML
/// - Group events for auto-grouping analysis
/// - A hash of the HTML content for change detection
#[allow(dead_code)]
pub fn parse_html(
    html_bytes: &[u8],
    incremental_parser: &mut IncrementalParser,
    capacity_hint: usize,
) -> ExtractedContent {
    // Use incremental parser for efficient parsing
    let extracted = incremental_parser.parse_incremental(html_bytes, capacity_hint);

    // Compute content hash for change detection
    let content_hash = {
        let mut hasher = AHasher::default();
        hasher.write(html_bytes);
        hasher.finish()
    };

    ExtractedContent {
        classes: extracted.classes,
        group_events: extracted.group_events,
        content_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_empty() {
        let mut parser = IncrementalParser::new();
        let result = parse_html(b"", &mut parser, 16);

        assert!(result.classes.is_empty());
        assert!(result.group_events.is_empty());
    }

    #[test]
    fn test_parse_html_simple_class() {
        let mut parser = IncrementalParser::new();
        let html = b"<div class=\"flex p-4\">content</div>";
        let result = parse_html(html, &mut parser, 16);

        assert!(result.classes.contains("flex"));
        assert!(result.classes.contains("p-4"));
    }

    #[test]
    fn test_parse_html_content_hash_changes() {
        let mut parser = IncrementalParser::new();

        let html1 = b"<div class=\"flex\">content</div>";
        let result1 = parse_html(html1, &mut parser, 16);

        let html2 = b"<div class=\"block\">content</div>";
        let result2 = parse_html(html2, &mut parser, 16);

        // Different content should produce different hashes
        assert_ne!(result1.content_hash, result2.content_hash);
    }

    #[test]
    fn test_parse_html_same_content_same_hash() {
        let mut parser = IncrementalParser::new();

        let html = b"<div class=\"flex\">content</div>";
        let result1 = parse_html(html, &mut parser, 16);
        let result2 = parse_html(html, &mut parser, 16);

        // Same content should produce same hash
        assert_eq!(result1.content_hash, result2.content_hash);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // **Property 3: HTML Parsing Preserves Content**
    // **Validates: Requirements 7.2, 7.3, 7.4**
    // Feature: dx-style-production-hardening, Property 3: HTML Parsing Preserves Content
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_html_parsing_preserves_class_names(
            class1 in "[a-z][a-z0-9-]{0,10}",
            class2 in "[a-z][a-z0-9-]{0,10}",
        ) {
            let html = format!(r#"<div class="{} {}">content</div>"#, class1, class2);
            let mut parser = IncrementalParser::new();
            let result = parse_html(html.as_bytes(), &mut parser, 16);

            // Both class names should be preserved
            prop_assert!(
                result.classes.contains(&class1),
                "Class '{}' should be preserved in {:?}",
                class1,
                result.classes
            );
            prop_assert!(
                result.classes.contains(&class2),
                "Class '{}' should be preserved in {:?}",
                class2,
                result.classes
            );
        }

        #[test]
        fn prop_html_parsing_handles_utf8(
            // Generate ASCII class names (UTF-8 compatible)
            class_name in "[a-z][a-z0-9]{0,8}",
        ) {
            // Test with UTF-8 content around the class
            let html = format!(r#"<div class="{}">日本語コンテンツ</div>"#, class_name);
            let mut parser = IncrementalParser::new();
            let result = parse_html(html.as_bytes(), &mut parser, 16);

            // Class name should be preserved even with UTF-8 content
            prop_assert!(
                result.classes.contains(&class_name),
                "Class '{}' should be preserved with UTF-8 content",
                class_name
            );
        }

        #[test]
        fn prop_html_parsing_same_content_same_hash(
            class_name in "[a-z][a-z0-9-]{0,10}",
        ) {
            let html = format!(r#"<div class="{}">content</div>"#, class_name);
            let mut parser1 = IncrementalParser::new();
            let mut parser2 = IncrementalParser::new();

            let result1 = parse_html(html.as_bytes(), &mut parser1, 16);
            let result2 = parse_html(html.as_bytes(), &mut parser2, 16);

            // Same content should produce same hash
            prop_assert_eq!(
                result1.content_hash,
                result2.content_hash,
                "Same content should produce same hash"
            );
        }
    }
}
