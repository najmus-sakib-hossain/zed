//! Property-Based Tests for Array Parsing with Space Separators
//!
//! This module tests Requirements 4.1 and 4.2:
//! - Space-separated array parsing (new format)
//! - Comma-separated array parsing (backward compatibility)
//!
//! Feature: dx-serializer-production-ready
//! Task: 3.4 Write property tests for array parsing

use proptest::prelude::*;
use serializer::llm::{DxLlmValue, LlmParser};

// =============================================================================
// Property 15: Space-Separated Array Parsing
// =============================================================================

/// **Property 15: Space-Separated Array Parsing**
///
/// *For any* simple array in the format `name:count=item1 item2 item3` with
/// space-separated items, parsing should extract all items correctly.
///
/// **Validates: Requirements 4.1**
#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_space_separated_array_parsing(
            items in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]*",  // Start with letter or underscore
                1..10
            )
        ) {
            let count = items.len();
            let items_str = items.join(" ");
            let input = format!("tags:{}={}", count, items_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert!(doc.context.contains_key("tags"), "Missing 'tags' key");

            if let Some(DxLlmValue::Arr(parsed_items)) = doc.context.get("tags") {
                prop_assert_eq!(
                    parsed_items.len(),
                    items.len(),
                    "Item count mismatch for input: {}",
                    input
                );

                for (i, item) in items.iter().enumerate() {
                    if let Some(DxLlmValue::Str(s)) = parsed_items.get(i) {
                        prop_assert_eq!(s, item, "Item {} mismatch", i);
                    } else {
                        prop_assert!(false, "Item {} is not a string: {:?}", i, parsed_items.get(i));
                    }
                }
            } else {
                prop_assert!(
                    false,
                    "Expected array value, got: {:?}",
                    doc.context.get("tags")
                );
            }
        }
    }
}

// =============================================================================
// Property 16: Comma-Separated Array Backward Compatibility
// =============================================================================

/// **Property 16: Comma-Separated Array Backward Compatibility**
///
/// *For any* simple array in the legacy format `name:count=item1,item2,item3`
/// with comma-separated items, parsing should continue to extract all items
/// correctly.
///
/// **Validates: Requirements 4.2**
#[cfg(test)]
mod backward_compat_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_comma_separated_array_parsing(
            items in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]*",  // Start with letter or underscore
                1..10
            )
        ) {
            let count = items.len();
            let items_str = items.join(",");
            let input = format!("tags:{}={}", count, items_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert!(doc.context.contains_key("tags"), "Missing 'tags' key");

            if let Some(DxLlmValue::Arr(parsed_items)) = doc.context.get("tags") {
                prop_assert_eq!(
                    parsed_items.len(),
                    items.len(),
                    "Item count mismatch for input: {}",
                    input
                );

                for (i, item) in items.iter().enumerate() {
                    if let Some(DxLlmValue::Str(s)) = parsed_items.get(i) {
                        prop_assert_eq!(s, item, "Item {} mismatch", i);
                    } else {
                        prop_assert!(false, "Item {} is not a string: {:?}", i, parsed_items.get(i));
                    }
                }
            } else {
                prop_assert!(
                    false,
                    "Expected array value, got: {:?}",
                    doc.context.get("tags")
                );
            }
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_space_separated_simple_array() {
        let input = "tags:3=rust performance serialization";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("tags"));
        if let Some(DxLlmValue::Arr(items)) = doc.context.get("tags") {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], DxLlmValue::Str("rust".to_string()));
            assert_eq!(items[1], DxLlmValue::Str("performance".to_string()));
            assert_eq!(items[2], DxLlmValue::Str("serialization".to_string()));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_comma_separated_simple_array_legacy() {
        let input = "tags:3=rust,performance,serialization";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("tags"));
        if let Some(DxLlmValue::Arr(items)) = doc.context.get("tags") {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], DxLlmValue::Str("rust".to_string()));
            assert_eq!(items[1], DxLlmValue::Str("performance".to_string()));
            assert_eq!(items[2], DxLlmValue::Str("serialization".to_string()));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_space_separated_numeric_array() {
        let input = "numbers:5=1 2 3 4 5";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("numbers"));
        if let Some(DxLlmValue::Arr(items)) = doc.context.get("numbers") {
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], DxLlmValue::Num(1.0));
            assert_eq!(items[1], DxLlmValue::Num(2.0));
            assert_eq!(items[2], DxLlmValue::Num(3.0));
            assert_eq!(items[3], DxLlmValue::Num(4.0));
            assert_eq!(items[4], DxLlmValue::Num(5.0));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_space_separated_mixed_types() {
        let input = "mixed:4=hello 42 true null";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("mixed"));
        if let Some(DxLlmValue::Arr(items)) = doc.context.get("mixed") {
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], DxLlmValue::Str("hello".to_string()));
            assert_eq!(items[1], DxLlmValue::Num(42.0));
            assert_eq!(items[2], DxLlmValue::Bool(true));
            assert_eq!(items[3], DxLlmValue::Null);
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_single_item_space_array() {
        let input = "single:1=item";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("single"));
        if let Some(DxLlmValue::Arr(items)) = doc.context.get("single") {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], DxLlmValue::Str("item".to_string()));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_nested_array_in_object_space_separated() {
        let input = "config:2[tags[3]=rust wasm performance enabled=true]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert!(doc.context.contains_key("config"));
        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert!(fields.contains_key("tags"));
            if let Some(DxLlmValue::Arr(items)) = fields.get("tags") {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], DxLlmValue::Str("rust".to_string()));
                assert_eq!(items[1], DxLlmValue::Str("wasm".to_string()));
                assert_eq!(items[2], DxLlmValue::Str("performance".to_string()));
            } else {
                panic!("Expected array value for tags");
            }
        } else {
            panic!("Expected object value");
        }
    }
}
