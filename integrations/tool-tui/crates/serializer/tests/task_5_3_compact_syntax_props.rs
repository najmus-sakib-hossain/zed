//! Property tests for Task 5.3: Compact syntax parsing
//!
//! Feature: dx-serializer-production-ready
//! Property 19: Compact Syntax Marker Recognition
//! Property 20: Compact Syntax Key-Value Parsing
//! Property 21: Compact Syntax Object Structure
//! Validates: Requirements 5.1, 5.2, 5.3

use proptest::prelude::*;
use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;
use std::collections::HashMap;

/// Strategy to generate valid identifier strings (keys)
fn valid_identifier() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
}

/// Strategy to generate valid simple values (no nested structures)
fn simple_value() -> impl Strategy<Value = String> {
    prop_oneof![
        // String values (alphanumeric, no spaces)
        "[a-zA-Z][a-zA-Z0-9._-]{0,20}".prop_map(|s| s.to_string()),
        // Integer values
        (-10000i64..10000i64).prop_map(|n| n.to_string()),
        // Float values
        (-1000.0f64..1000.0f64)
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(|f| format!("{:.2}", f)),
        // Boolean values
        prop::bool::ANY.prop_map(|b| if b {
            "true".to_string()
        } else {
            "false".to_string()
        }),
    ]
}

/// Strategy to generate a single key-value pair
fn key_value_pair() -> impl Strategy<Value = (String, String)> {
    (valid_identifier(), simple_value())
}

/// Strategy to generate a map of key-value pairs (1-10 pairs)
fn key_value_map() -> impl Strategy<Value = HashMap<String, String>> {
    prop::collection::vec(key_value_pair(), 1..=10)
        .prop_map(|pairs| {
            let mut map = HashMap::new();
            for (k, v) in pairs {
                map.insert(k, v);
            }
            map
        })
        .prop_filter("unique keys", |map| !map.is_empty())
}

/// Helper to parse a simple value string into expected DxLlmValue
fn parse_expected_value(value_str: &str) -> DxLlmValue {
    // Try to parse as number
    if let Ok(num) = value_str.parse::<f64>() {
        return DxLlmValue::Num(num);
    }

    // Try to parse as boolean
    match value_str {
        "true" => return DxLlmValue::Bool(true),
        "false" => return DxLlmValue::Bool(false),
        _ => {}
    }

    // Otherwise it's a string
    DxLlmValue::Str(value_str.to_string())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 19: Compact Syntax Marker Recognition
    /// **Validates: Requirements 5.1**
    ///
    /// For any compact syntax string with the `@=` marker, parsing should recognize
    /// the marker and parse the content as a compact object.
    #[test]
    fn prop_compact_syntax_marker_recognition(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the compact syntax string with @= marker
        let count = fields.len();
        let tokens: Vec<String> = fields
            .iter()
            .flat_map(|(k, v)| vec![k.clone(), v.clone()])
            .collect();
        let input = format!("{}:{}@=[{}]", section_name, count, tokens.join(" "));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse valid compact syntax with @= marker: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the section exists in the document context
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context. Available keys: {:?}\nInput: {}",
            section_name,
            doc.context.keys().collect::<Vec<_>>(),
            input
        );

        // Check that it's an object (marker was recognized)
        prop_assert!(
            matches!(doc.context.get(&section_name), Some(DxLlmValue::Obj(_))),
            "Expected Obj variant for section '{}' (marker should be recognized), got {:?}\nInput: {}",
            section_name,
            doc.context.get(&section_name),
            input
        );
    }

    /// Feature: dx-serializer-production-ready, Property 20: Compact Syntax Key-Value Parsing
    /// **Validates: Requirements 5.2**
    ///
    /// For any compact syntax content in the format `@=[key value key value]`, parsing
    /// should correctly extract all key-value pairs without requiring `=` signs between them.
    #[test]
    fn prop_compact_syntax_key_value_parsing(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the compact syntax string with space-separated key-value pairs (no = signs)
        let count = fields.len();
        let tokens: Vec<String> = fields
            .iter()
            .flat_map(|(k, v)| vec![k.clone(), v.clone()])
            .collect();
        let input = format!("{}:{}@=[{}]", section_name, count, tokens.join(" "));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse compact syntax key-value pairs: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that it's an object
        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            // Check that all fields are present
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Expected {} fields, got {}. Input: {}\nExpected keys: {:?}\nGot keys: {:?}",
                fields.len(),
                parsed_fields.len(),
                input,
                fields.keys().collect::<Vec<_>>(),
                parsed_fields.keys().collect::<Vec<_>>()
            );

            // Check each field value
            for (key, expected_value_str) in &fields {
                prop_assert!(
                    parsed_fields.contains_key(key),
                    "Key '{}' not found in parsed object. Available keys: {:?}\nInput: {}",
                    key,
                    parsed_fields.keys().collect::<Vec<_>>(),
                    input
                );

                let parsed_value = &parsed_fields[key];
                let expected_value = parse_expected_value(expected_value_str);

                // Compare values based on type
                match (&expected_value, parsed_value) {
                    (DxLlmValue::Num(exp), DxLlmValue::Num(got)) => {
                        // Use approximate equality for floats
                        let diff = (exp - got).abs();
                        prop_assert!(
                            diff < 0.01,
                            "Numeric value mismatch for key '{}': expected {}, got {}\nInput: {}",
                            key,
                            exp,
                            got,
                            input
                        );
                    }
                    (DxLlmValue::Bool(exp), DxLlmValue::Bool(got)) => {
                        prop_assert_eq!(
                            exp,
                            got,
                            "Boolean value mismatch for key '{}'\nInput: {}",
                            key,
                            input
                        );
                    }
                    (DxLlmValue::Str(exp), DxLlmValue::Str(got)) => {
                        prop_assert_eq!(
                            exp,
                            got,
                            "String value mismatch for key '{}'\nInput: {}",
                            key,
                            input
                        );
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Type mismatch for key '{}': expected {:?}, got {:?}\nInput: {}",
                            key,
                            expected_value.type_name(),
                            parsed_value.type_name(),
                            input
                        );
                    }
                }
            }
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}', got {:?}\nInput: {}",
                section_name,
                doc.context.get(&section_name),
                input
            );
        }
    }

    /// Feature: dx-serializer-production-ready, Property 21: Compact Syntax Object Structure
    /// **Validates: Requirements 5.3**
    ///
    /// For any compact syntax string, parsing should produce a DxDocument with a properly
    /// structured object containing all key-value pairs.
    #[test]
    fn prop_compact_syntax_object_structure(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the compact syntax string
        let count = fields.len();
        let tokens: Vec<String> = fields
            .iter()
            .flat_map(|(k, v)| vec![k.clone(), v.clone()])
            .collect();
        let input = format!("{}:{}@=[{}]", section_name, count, tokens.join(" "));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse compact syntax: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Verify the document structure
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context\nInput: {}",
            section_name,
            input
        );

        // Verify it's a properly structured object
        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            // Check that the object is not empty (unless input was empty)
            if !fields.is_empty() {
                prop_assert!(
                    !parsed_fields.is_empty(),
                    "Parsed object is empty but input had {} fields\nInput: {}",
                    fields.len(),
                    input
                );
            }

            // Check that all keys are valid identifiers (no corruption)
            for key in parsed_fields.keys() {
                prop_assert!(
                    !key.is_empty(),
                    "Found empty key in parsed object\nInput: {}",
                    input
                );
                prop_assert!(
                    key.chars().all(|c| c.is_alphanumeric() || c == '_'),
                    "Found invalid key '{}' in parsed object\nInput: {}",
                    key,
                    input
                );
            }

            // Check that all values are valid (not corrupted)
            for (key, value) in parsed_fields.iter() {
                match value {
                    DxLlmValue::Str(s) => {
                        prop_assert!(
                            !s.is_empty() || fields.get(key).map(|v| v.is_empty()).unwrap_or(false),
                            "Found empty string value for key '{}' but input was not empty\nInput: {}",
                            key,
                            input
                        );
                    }
                    DxLlmValue::Num(n) => {
                        prop_assert!(
                            n.is_finite(),
                            "Found non-finite number for key '{}': {}\nInput: {}",
                            key,
                            n,
                            input
                        );
                    }
                    DxLlmValue::Bool(_) => {
                        // Booleans are always valid
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Found unexpected value type for key '{}': {:?}\nInput: {}",
                            key,
                            value.type_name(),
                            input
                        );
                    }
                }
            }

            // Check that the number of fields matches
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Field count mismatch\nInput: {}",
                input
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}', got {:?}\nInput: {}",
                section_name,
                doc.context.get(&section_name),
                input
            );
        }
    }

    /// Property 19 variant: Compact syntax with single field
    ///
    /// For any compact syntax with a single key-value pair, parsing should work correctly.
    #[test]
    fn prop_compact_syntax_single_field(
        section_name in valid_identifier(),
        key in valid_identifier(),
        value in simple_value()
    ) {
        let input = format!("{}:1@=[{} {}]", section_name, key, value);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse single-field compact syntax: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                1,
                "Expected 1 field, got {}\nInput: {}",
                parsed_fields.len(),
                input
            );

            prop_assert!(
                parsed_fields.contains_key(&key),
                "Key '{}' not found in parsed object\nInput: {}",
                key,
                input
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'\nInput: {}",
                section_name,
                input
            );
        }
    }

    /// Property 20 variant: Compact syntax with multiple spaces
    ///
    /// For any compact syntax with multiple spaces between tokens,
    /// parsing should handle whitespace correctly.
    #[test]
    fn prop_compact_syntax_multiple_spaces(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the compact syntax string with multiple spaces between tokens
        let count = fields.len();
        let tokens: Vec<String> = fields
            .iter()
            .flat_map(|(k, v)| vec![k.clone(), v.clone()])
            .collect();
        // Use 2-5 spaces between tokens
        let input = format!("{}:{}@=[{}]", section_name, count, tokens.join("   "));

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse compact syntax with multiple spaces: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Expected {} fields, got {}\nInput: {}",
                fields.len(),
                parsed_fields.len(),
                input
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'\nInput: {}",
                section_name,
                input
            );
        }
    }

    /// Property 20 variant: Compact syntax with odd number of tokens should fail
    ///
    /// For any compact syntax with an odd number of tokens (unpaired key-value),
    /// parsing should return an error.
    #[test]
    fn prop_compact_syntax_odd_tokens_fails(
        section_name in valid_identifier(),
        tokens in prop::collection::vec(valid_identifier(), 1..=9)
            .prop_filter("odd length", |v| v.len() % 2 == 1)
    ) {
        let count = tokens.len() / 2; // Intentionally wrong count
        let input = format!("{}:{}@=[{}]", section_name, count, tokens.join(" "));

        let result = LlmParser::parse(&input);

        // Should fail because odd number of tokens
        prop_assert!(
            result.is_err(),
            "Expected error for odd number of tokens, but parsing succeeded\nInput: {}",
            input
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_compact_syntax() {
        let input = "config:2@=[host localhost port 8080]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 2);
            assert!(fields.contains_key("host"));
            assert!(fields.contains_key("port"));
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
            assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_compact_syntax_with_numbers() {
        let input = "version:3@=[major 2 minor 1 patch 0]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("version") {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields.get("major").unwrap().as_num(), Some(2.0));
            assert_eq!(fields.get("minor").unwrap().as_num(), Some(1.0));
            assert_eq!(fields.get("patch").unwrap().as_num(), Some(0.0));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_compact_syntax_with_booleans() {
        let input = "flags:2@=[debug true production false]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("flags") {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
            assert_eq!(fields.get("production").unwrap().as_bool(), Some(false));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_compact_syntax_single_field() {
        let input = "name:1@=[value test]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("name") {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields.get("value").unwrap().as_str(), Some("test"));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_compact_syntax_empty() {
        let input = "empty:0@=[]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("empty"));
        // Empty compact syntax should produce an empty object or null
        let value = doc.context.get("empty").unwrap();
        assert!(
            matches!(value, DxLlmValue::Obj(map) if map.is_empty())
                || matches!(value, DxLlmValue::Null)
        );
    }

    #[test]
    fn test_compact_syntax_odd_tokens_error() {
        // Odd number of tokens should fail
        let input = "config:1@=[host localhost port]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Expected error for odd number of tokens");
    }

    #[test]
    fn test_compact_syntax_multiple_spaces() {
        let input = "config:2@=[host   localhost   port   8080]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 2);
            assert!(fields.contains_key("host"));
            assert!(fields.contains_key("port"));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_compact_syntax_marker_recognition() {
        // Test that @= marker is recognized and not confused with regular inline objects
        let input = "config:2@=[key value]";
        let doc = LlmParser::parse(input).unwrap();

        // Should parse as compact syntax (no = between key and value)
        assert!(doc.context.contains_key("config"));
        assert!(matches!(doc.context.get("config"), Some(DxLlmValue::Obj(_))));
    }

    #[test]
    fn test_compact_syntax_vs_regular_object() {
        // Regular object with = signs
        let input1 = "config:2[key=value other=data]";
        let doc1 = LlmParser::parse(input1).unwrap();

        // Compact syntax without = signs
        let input2 = "config:2@=[key value other data]";
        let doc2 = LlmParser::parse(input2).unwrap();

        // Both should produce objects with the same keys
        if let (Some(DxLlmValue::Obj(fields1)), Some(DxLlmValue::Obj(fields2))) =
            (doc1.context.get("config"), doc2.context.get("config"))
        {
            assert_eq!(fields1.len(), fields2.len());
            assert!(fields1.contains_key("key"));
            assert!(fields2.contains_key("key"));
            assert!(fields1.contains_key("other"));
            assert!(fields2.contains_key("other"));
        } else {
            panic!("Expected Obj variants for both");
        }
    }
}
