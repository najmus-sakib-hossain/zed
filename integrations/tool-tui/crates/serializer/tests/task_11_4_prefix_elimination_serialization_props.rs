//! Property tests for Task 11.4: Prefix Elimination Serialization
//!
//! **Property 29: Prefix Detection and Elimination**
//! **Validates: Requirements 6.6**
//!
//! For any DxSection with repeated prefixes in string columns, serializing should
//! detect common prefixes and output @prefix markers to eliminate redundancy.

use proptest::prelude::*;
use serializer::llm::{DxDocument, DxLlmValue, DxSection, LlmSerializer, SerializerConfig};

/// Generate a section with common prefixes in a column
fn arb_section_with_prefix() -> impl Strategy<Value = DxSection> {
    (3..8_usize, "[a-z]{3,10}").prop_flat_map(|(row_count, prefix)| {
        let schema = vec!["id".to_string(), "path".to_string()];
        let rows = (0..row_count)
            .map(|i| {
                vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Str(format!("{}/item{}", prefix, i)),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

/// Generate a section without common prefixes
fn arb_section_without_prefix() -> impl Strategy<Value = DxSection> {
    (3..8_usize).prop_flat_map(|row_count| {
        let schema = vec!["id".to_string(), "name".to_string()];
        let rows = (0..row_count)
            .map(|i| {
                // Use different prefixes to avoid common prefix detection
                let prefix = match i % 3 {
                    0 => "alpha",
                    1 => "beta",
                    _ => "gamma",
                };
                vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Str(format!("{}{}", prefix, i)),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 29: Prefix Detection and Elimination
    /// Sections with common prefixes should use @prefix markers when enabled
    #[test]
    fn sections_with_prefix_use_markers(section in arb_section_with_prefix()) {
        let mut config = SerializerConfig::default();
        config.prefix_elimination = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Should contain @prefix marker
        prop_assert!(output.contains('@'), "Should contain @ prefix marker: {}", output);

        // The prefix marker should appear before the opening bracket
        let parts: Vec<&str> = output.split('[').collect();
        if parts.len() > 1 {
            prop_assert!(parts[0].contains('@'), "Prefix marker should appear before [: {}", output);
        }
    }

    /// Feature: dx-serializer-production-ready, Property 29: Prefix Detection and Elimination
    /// Sections without common prefixes should not use @prefix markers
    #[test]
    fn sections_without_prefix_no_markers(section in arb_section_without_prefix()) {
        let mut config = SerializerConfig::default();
        config.prefix_elimination = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Should NOT contain @prefix marker (no common prefix found)
        // Note: @ might appear in data, so check specifically for @prefix pattern before [
        let parts: Vec<&str> = output.split('[').collect();
        if parts.len() > 1 {
            // Check if there's a @ followed by space before the bracket
            let before_bracket = parts[0];
            let has_prefix_marker = before_bracket.contains("@") &&
                                   before_bracket.rfind('@').map(|pos| {
                                       before_bracket[pos..].contains(' ')
                                   }).unwrap_or(false);
            prop_assert!(!has_prefix_marker,
                "Should not have prefix marker for section without common prefix: {}", output);
        }
    }

    /// Feature: dx-serializer-production-ready, Property 29: Prefix Detection and Elimination
    /// Without prefix_elimination enabled, should not use @prefix markers
    #[test]
    fn without_prefix_elimination_no_markers(section in arb_section_with_prefix()) {
        let config = SerializerConfig::default(); // prefix_elimination = false
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Should NOT contain @prefix marker when feature is disabled
        let parts: Vec<&str> = output.split('[').collect();
        if parts.len() > 1 {
            let before_bracket = parts[0];
            let has_prefix_marker = before_bracket.contains("@") &&
                                   before_bracket.rfind('@').map(|pos| {
                                       before_bracket[pos..].contains(' ')
                                   }).unwrap_or(false);
            prop_assert!(!has_prefix_marker,
                "Should not have prefix marker when feature is disabled: {}", output);
        }
    }

    /// Feature: dx-serializer-production-ready, Property 29: Prefix Detection and Elimination
    /// Prefix elimination should reduce the length of values in output
    #[test]
    fn prefix_elimination_reduces_value_length(section in arb_section_with_prefix()) {
        let mut config = SerializerConfig::default();
        config.prefix_elimination = true;
        let serializer_with = LlmSerializer::with_config(config);

        let config_without = SerializerConfig::default();
        let serializer_without = LlmSerializer::with_config(config_without);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output_with = serializer_with.serialize(&doc);
        let output_without = serializer_without.serialize(&doc);

        // With prefix elimination, individual values should be shorter
        // (though total output might be similar due to @prefix marker)
        // Check that the content after [ is different
        let content_with = output_with.split('[').nth(1).unwrap_or("");
        let content_without = output_without.split('[').nth(1).unwrap_or("");

        prop_assert_ne!(content_with, content_without,
            "Prefix elimination should change the row content");
    }
}
