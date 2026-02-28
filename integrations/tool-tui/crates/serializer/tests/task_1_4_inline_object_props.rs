//! Property tests for Task 1.4: Inline object parsing
//!
//! Feature: dx-serializer-production-ready
//! Property 1: Inline Object Parsing with Count and Space Separators
//! Validates: Requirements 1.1, 1.2

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

    /// Feature: dx-serializer-production-ready, Property 1: Inline Object Parsing with Count and Space Separators
    /// **Validates: Requirements 1.1, 1.2**
    ///
    /// For any inline object string in the format `section:count[key=value key2=value2]`
    /// with space-separated fields, parsing should produce a DxDocument with an object
    /// containing all specified key-value pairs.
    #[test]
    fn prop_inline_object_space_separated(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the inline object string with space separators
        let count = fields.len();
        let fields_str: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let input = format!("{}:{}[{}]", section_name, count, fields_str.join(" "));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse valid inline object: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the section exists in the document context
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context. Available keys: {:?}",
            section_name,
            doc.context.keys().collect::<Vec<_>>()
        );

        // Check that it's an object
        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            // Check that all fields are present
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Expected {} fields, got {}. Input: {}\nExpected: {:?}\nGot: {:?}",
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
                    "Key '{}' not found in parsed object. Available keys: {:?}",
                    key,
                    parsed_fields.keys().collect::<Vec<_>>()
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
                            "Numeric value mismatch for key '{}': expected {}, got {}",
                            key,
                            exp,
                            got
                        );
                    }
                    (DxLlmValue::Bool(exp), DxLlmValue::Bool(got)) => {
                        prop_assert_eq!(
                            exp,
                            got,
                            "Boolean value mismatch for key '{}'",
                            key
                        );
                    }
                    (DxLlmValue::Str(exp), DxLlmValue::Str(got)) => {
                        prop_assert_eq!(
                            exp,
                            got,
                            "String value mismatch for key '{}'",
                            key
                        );
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Type mismatch for key '{}': expected {:?}, got {:?}",
                            key,
                            expected_value.type_name(),
                            parsed_value.type_name()
                        );
                    }
                }
            }
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}', got {:?}",
                section_name,
                doc.context.get(&section_name)
            );
        }
    }

    /// Property 1 variant: Comma-separated backward compatibility
    ///
    /// For any inline object string with comma separators (legacy format),
    /// parsing should still work correctly.
    #[test]
    fn prop_inline_object_comma_separated_backward_compat(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the inline object string with comma separators (legacy)
        let count = fields.len();
        let fields_str: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let input = format!("{}:{}[{}]", section_name, count, fields_str.join(","));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse valid inline object with comma separators: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the section exists
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context",
            section_name
        );

        // Check that it's an object with all fields
        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Expected {} fields, got {}",
                fields.len(),
                parsed_fields.len()
            );

            // Verify all keys are present
            for key in fields.keys() {
                prop_assert!(
                    parsed_fields.contains_key(key),
                    "Key '{}' not found in parsed object",
                    key
                );
            }
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'",
                section_name
            );
        }
    }

    /// Property 1 variant: Empty objects
    ///
    /// For any inline object with count=0 and no fields, parsing should
    /// produce a valid result (either empty object or null).
    #[test]
    fn prop_inline_object_empty(section_name in valid_identifier()) {
        let input = format!("{}:0[]", section_name);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse empty inline object: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the section exists
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context",
            section_name
        );

        // Empty objects may be represented as Null or empty Obj
        let value = doc.context.get(&section_name).unwrap();
        let is_valid = match value {
            DxLlmValue::Null => true,
            DxLlmValue::Obj(map) if map.is_empty() => true,
            _ => false,
        };
        prop_assert!(
            is_valid,
            "Expected Null or empty Obj for empty inline object, got {:?}",
            value
        );
    }

    /// Property 1 variant: Single field objects
    ///
    /// For any inline object with a single field, parsing should work correctly.
    #[test]
    fn prop_inline_object_single_field(
        section_name in valid_identifier(),
        key in valid_identifier(),
        value in simple_value()
    ) {
        let input = format!("{}:1[{}={}]", section_name, key, value);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse single-field inline object: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                1,
                "Expected 1 field, got {}",
                parsed_fields.len()
            );

            prop_assert!(
                parsed_fields.contains_key(&key),
                "Key '{}' not found in parsed object",
                key
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'",
                section_name
            );
        }
    }

    /// Property 1 variant: Multiple spaces between fields
    ///
    /// For any inline object with multiple spaces between fields,
    /// parsing should handle whitespace correctly.
    #[test]
    fn prop_inline_object_multiple_spaces(
        section_name in valid_identifier(),
        fields in key_value_map()
    ) {
        // Build the inline object string with multiple spaces between fields
        let count = fields.len();
        let fields_str: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        // Use 2-5 spaces between fields
        let input = format!("{}:{}[{}]", section_name, count, fields_str.join("   "));

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with multiple spaces: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                fields.len(),
                "Expected {} fields, got {}",
                fields.len(),
                parsed_fields.len()
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'",
                section_name
            );
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_inline_object() {
        let input = "config:2[host=localhost port=8080]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 2);
            assert!(fields.contains_key("host"));
            assert!(fields.contains_key("port"));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_inline_object_with_numbers() {
        let input = "version:3[major=2 minor=1 patch=0]";
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
    fn test_inline_object_with_booleans() {
        let input = "flags:2[debug=true production=false]";
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
    fn test_inline_object_comma_separated() {
        let input = "config:2[host=localhost,port=8080]";
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
    fn test_empty_inline_object() {
        let input = "empty:0[]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("empty"));
        // Empty objects are represented as Null
        assert!(matches!(doc.context.get("empty"), Some(DxLlmValue::Null)));
    }
}
