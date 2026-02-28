//! Property tests for Task 1.5: Nested array parsing in inline objects
//!
//! Feature: dx-serializer-production-ready
//! Property 2: Nested Array Parsing in Objects
//! Validates: Requirements 1.3

use proptest::prelude::*;
use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

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

/// Strategy to generate an array of simple values (1-10 items)
fn simple_array() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(simple_value(), 1..=10)
}

/// Strategy to generate a nested array field (key with array value)
fn nested_array_field() -> impl Strategy<Value = (String, Vec<String>)> {
    (valid_identifier(), simple_array())
}

/// Strategy to generate a simple key-value field (non-array)
fn simple_field() -> impl Strategy<Value = (String, String)> {
    (valid_identifier(), simple_value())
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

/// Strategy to generate an inline object with nested arrays and simple fields
/// Returns (section_name, nested_arrays, simple_fields)
fn inline_object_with_nested_arrays()
-> impl Strategy<Value = (String, Vec<(String, Vec<String>)>, Vec<(String, String)>)> {
    (
        valid_identifier(),
        prop::collection::vec(nested_array_field(), 1..=3),
        prop::collection::vec(simple_field(), 0..=3),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 2: Nested Array Parsing in Objects
    /// **Validates: Requirements 1.3**
    ///
    /// For any inline object containing nested arrays in the format `key[count]=item1 item2 item3`,
    /// parsing should correctly extract the array with all items preserved.
    #[test]
    fn prop_nested_array_space_separated(
        section_name in valid_identifier(),
        array_key in valid_identifier(),
        array_items in simple_array()
    ) {
        // Build inline object with nested array: section:1[key[count]=item1 item2 item3]
        let count = array_items.len();
        let items_str = array_items.join(" ");
        let input = format!("{}:1[{}[{}]={}]", section_name, array_key, count, items_str);

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with nested array: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the section exists
        prop_assert!(
            doc.context.contains_key(&section_name),
            "Section '{}' not found in document context. Available keys: {:?}",
            section_name,
            doc.context.keys().collect::<Vec<_>>()
        );

        // Check that it's an object
        if let Some(DxLlmValue::Obj(fields)) = doc.context.get(&section_name) {
            // Check that the array field exists
            prop_assert!(
                fields.contains_key(&array_key),
                "Array key '{}' not found in parsed object. Available keys: {:?}",
                array_key,
                fields.keys().collect::<Vec<_>>()
            );

            // Check that it's an array
            if let Some(DxLlmValue::Arr(parsed_items)) = fields.get(&array_key) {
                prop_assert_eq!(
                    parsed_items.len(),
                    array_items.len(),
                    "Expected {} array items, got {}. Input: {}",
                    array_items.len(),
                    parsed_items.len(),
                    input
                );

                // Check each item value
                for (i, expected_str) in array_items.iter().enumerate() {
                    let expected_value = parse_expected_value(expected_str);
                    let parsed_value = &parsed_items[i];

                    match (&expected_value, parsed_value) {
                        (DxLlmValue::Num(exp), DxLlmValue::Num(got)) => {
                            let diff = (exp - got).abs();
                            prop_assert!(
                                diff < 0.01,
                                "Numeric value mismatch at index {}: expected {}, got {}",
                                i,
                                exp,
                                got
                            );
                        }
                        (DxLlmValue::Bool(exp), DxLlmValue::Bool(got)) => {
                            prop_assert_eq!(
                                exp,
                                got,
                                "Boolean value mismatch at index {}",
                                i
                            );
                        }
                        (DxLlmValue::Str(exp), DxLlmValue::Str(got)) => {
                            prop_assert_eq!(
                                exp,
                                got,
                                "String value mismatch at index {}",
                                i
                            );
                        }
                        _ => {
                            prop_assert!(
                                false,
                                "Type mismatch at index {}: expected {:?}, got {:?}",
                                i,
                                expected_value.type_name(),
                                parsed_value.type_name()
                            );
                        }
                    }
                }
            } else {
                prop_assert!(
                    false,
                    "Expected Arr variant for key '{}', got {:?}",
                    array_key,
                    fields.get(&array_key)
                );
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

    /// Property 2 variant: Nested array with mixed field types
    ///
    /// For any inline object with nested arrays and simple fields,
    /// parsing should correctly extract both arrays and simple values.
    #[test]
    fn prop_nested_array_with_simple_fields(
        (section_name, nested_arrays, simple_fields) in inline_object_with_nested_arrays()
    ) {
        // Ensure no duplicate keys in nested arrays
        let array_keys: std::collections::HashSet<_> = nested_arrays.iter().map(|(k, _)| k).collect();
        prop_assume!(array_keys.len() == nested_arrays.len());

        // Ensure no key conflicts between nested arrays and simple fields
        let simple_keys: std::collections::HashSet<_> = simple_fields.iter().map(|(k, _)| k).collect();
        prop_assume!(array_keys.is_disjoint(&simple_keys));
        prop_assume!(simple_keys.len() == simple_fields.len());

        // Build inline object with nested arrays and simple fields
        let total_fields = nested_arrays.len() + simple_fields.len();

        let mut fields_str = Vec::new();

        // Add nested array fields
        for (key, items) in &nested_arrays {
            let items_str = items.join(" ");
            fields_str.push(format!("{}[{}]={}", key, items.len(), items_str));
        }

        // Add simple fields
        for (key, value) in &simple_fields {
            fields_str.push(format!("{}={}", key, value));
        }

        let input = format!("{}:{}[{}]", section_name, total_fields, fields_str.join(" "));

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with nested arrays and simple fields: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                total_fields,
                "Expected {} fields, got {}. Input: {}",
                total_fields,
                parsed_fields.len(),
                input
            );

            // Verify nested arrays
            for (key, expected_items) in &nested_arrays {
                prop_assert!(
                    parsed_fields.contains_key(key),
                    "Nested array key '{}' not found",
                    key
                );

                if let Some(DxLlmValue::Arr(parsed_items)) = parsed_fields.get(key) {
                    prop_assert_eq!(
                        parsed_items.len(),
                        expected_items.len(),
                        "Array '{}' length mismatch",
                        key
                    );
                } else {
                    prop_assert!(
                        false,
                        "Expected Arr variant for key '{}'",
                        key
                    );
                }
            }

            // Verify simple fields
            for (key, _value) in &simple_fields {
                prop_assert!(
                    parsed_fields.contains_key(key),
                    "Simple field key '{}' not found",
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

    /// Property 2 variant: Multiple nested arrays in same object
    ///
    /// For any inline object with multiple nested arrays,
    /// parsing should correctly extract all arrays independently.
    #[test]
    fn prop_multiple_nested_arrays(
        section_name in valid_identifier(),
        arrays in prop::collection::vec(nested_array_field(), 2..=4)
    ) {
        // Ensure no duplicate keys
        let keys: std::collections::HashSet<_> = arrays.iter().map(|(k, _)| k).collect();
        prop_assume!(keys.len() == arrays.len());

        let total_fields = arrays.len();

        let fields_str: Vec<String> = arrays
            .iter()
            .map(|(key, items)| {
                let items_str = items.join(" ");
                format!("{}[{}]={}", key, items.len(), items_str)
            })
            .collect();

        let input = format!("{}:{}[{}]", section_name, total_fields, fields_str.join(" "));

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with multiple nested arrays: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                total_fields,
                "Expected {} fields, got {}",
                total_fields,
                parsed_fields.len()
            );

            // Verify each nested array
            for (key, expected_items) in &arrays {
                prop_assert!(
                    parsed_fields.contains_key(key),
                    "Nested array key '{}' not found",
                    key
                );

                if let Some(DxLlmValue::Arr(parsed_items)) = parsed_fields.get(key) {
                    prop_assert_eq!(
                        parsed_items.len(),
                        expected_items.len(),
                        "Array '{}' length mismatch: expected {}, got {}",
                        key,
                        expected_items.len(),
                        parsed_items.len()
                    );
                } else {
                    prop_assert!(
                        false,
                        "Expected Arr variant for key '{}', got {:?}",
                        key,
                        parsed_fields.get(key)
                    );
                }
            }
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'",
                section_name
            );
        }
    }

    /// Property 2 variant: Empty nested arrays
    ///
    /// For any inline object with empty nested arrays (count=0),
    /// parsing should produce empty array values.
    #[test]
    fn prop_nested_array_empty(
        section_name in valid_identifier(),
        array_key in valid_identifier(),
        other_key in valid_identifier(),
        other_value in simple_value()
    ) {
        // Ensure keys are different
        prop_assume!(array_key != other_key);

        let input = format!("{}:2[{}[0]= {}={}]", section_name, array_key, other_key, other_value);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with empty nested array: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            prop_assert_eq!(
                parsed_fields.len(),
                2,
                "Expected 2 fields, got {}",
                parsed_fields.len()
            );

            // Check empty array
            if let Some(DxLlmValue::Arr(items)) = parsed_fields.get(&array_key) {
                prop_assert_eq!(
                    items.len(),
                    0,
                    "Expected empty array, got {} items",
                    items.len()
                );
            } else {
                prop_assert!(
                    false,
                    "Expected Arr variant for key '{}'",
                    array_key
                );
            }

            // Check other field exists
            prop_assert!(
                parsed_fields.contains_key(&other_key),
                "Other field '{}' not found",
                other_key
            );
        } else {
            prop_assert!(
                false,
                "Expected Obj variant for section '{}'",
                section_name
            );
        }
    }

    /// Property 2 variant: Single item nested arrays
    ///
    /// For any inline object with single-item nested arrays,
    /// parsing should correctly extract the single item.
    #[test]
    fn prop_nested_array_single_item(
        section_name in valid_identifier(),
        array_key in valid_identifier(),
        item in simple_value()
    ) {
        let input = format!("{}:1[{}[1]={}]", section_name, array_key, item);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with single-item nested array: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            if let Some(DxLlmValue::Arr(items)) = parsed_fields.get(&array_key) {
                prop_assert_eq!(
                    items.len(),
                    1,
                    "Expected 1 item, got {}",
                    items.len()
                );

                // Verify the item value
                let expected_value = parse_expected_value(&item);
                let parsed_value = &items[0];

                match (&expected_value, parsed_value) {
                    (DxLlmValue::Num(exp), DxLlmValue::Num(got)) => {
                        let diff = (exp - got).abs();
                        prop_assert!(
                            diff < 0.01,
                            "Numeric value mismatch: expected {}, got {}",
                            exp,
                            got
                        );
                    }
                    (DxLlmValue::Bool(exp), DxLlmValue::Bool(got)) => {
                        prop_assert_eq!(exp, got, "Boolean value mismatch");
                    }
                    (DxLlmValue::Str(exp), DxLlmValue::Str(got)) => {
                        prop_assert_eq!(exp, got, "String value mismatch");
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Type mismatch: expected {:?}, got {:?}",
                            expected_value.type_name(),
                            parsed_value.type_name()
                        );
                    }
                }
            } else {
                prop_assert!(
                    false,
                    "Expected Arr variant for key '{}'",
                    array_key
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

    /// Property 2 variant: Comma-separated nested arrays (backward compatibility)
    ///
    /// For any inline object with comma-separated nested arrays,
    /// parsing should still work correctly for backward compatibility.
    #[test]
    fn prop_nested_array_comma_separated(
        section_name in valid_identifier(),
        array_key in valid_identifier(),
        array_items in simple_array()
    ) {
        // Build inline object with comma-separated nested array
        let count = array_items.len();
        let items_str = array_items.join(",");
        let input = format!("{}:1[{}[{}]={}]", section_name, array_key, count, items_str);

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse inline object with comma-separated nested array: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        if let Some(DxLlmValue::Obj(parsed_fields)) = doc.context.get(&section_name) {
            if let Some(DxLlmValue::Arr(parsed_items)) = parsed_fields.get(&array_key) {
                prop_assert_eq!(
                    parsed_items.len(),
                    array_items.len(),
                    "Expected {} array items, got {}",
                    array_items.len(),
                    parsed_items.len()
                );
            } else {
                prop_assert!(
                    false,
                    "Expected Arr variant for key '{}'",
                    array_key
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
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_nested_array_basic() {
        let input = "config:1[tags[3]=web api mobile]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 1);

            if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
                assert_eq!(tags.len(), 3);
                assert_eq!(tags[0].as_str(), Some("web"));
                assert_eq!(tags[1].as_str(), Some("api"));
                assert_eq!(tags[2].as_str(), Some("mobile"));
            } else {
                panic!("Expected tags to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_with_numbers() {
        let input = "data:1[ports[3]=8080 8081 8082]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("data") {
            if let Some(DxLlmValue::Arr(ports)) = fields.get("ports") {
                assert_eq!(ports.len(), 3);
                assert_eq!(ports[0].as_num(), Some(8080.0));
                assert_eq!(ports[1].as_num(), Some(8081.0));
                assert_eq!(ports[2].as_num(), Some(8082.0));
            } else {
                panic!("Expected ports to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_with_booleans() {
        let input = "flags:1[enabled[3]=true false true]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("flags") {
            if let Some(DxLlmValue::Arr(enabled)) = fields.get("enabled") {
                assert_eq!(enabled.len(), 3);
                assert_eq!(enabled[0].as_bool(), Some(true));
                assert_eq!(enabled[1].as_bool(), Some(false));
                assert_eq!(enabled[2].as_bool(), Some(true));
            } else {
                panic!("Expected enabled to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_mixed_with_simple_field() {
        let input = "config:2[tags[3]=web api mobile host=localhost]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 2);

            // Check nested array
            if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
                assert_eq!(tags.len(), 3);
            } else {
                panic!("Expected tags to be an array");
            }

            // Check simple field
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_multiple_nested_arrays() {
        let input = "server:2[hosts[2]=web1 web2 ports[2]=80 443]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("server") {
            assert_eq!(fields.len(), 2);

            // Check hosts array
            if let Some(DxLlmValue::Arr(hosts)) = fields.get("hosts") {
                assert_eq!(hosts.len(), 2);
                assert_eq!(hosts[0].as_str(), Some("web1"));
                assert_eq!(hosts[1].as_str(), Some("web2"));
            } else {
                panic!("Expected hosts to be an array");
            }

            // Check ports array
            if let Some(DxLlmValue::Arr(ports)) = fields.get("ports") {
                assert_eq!(ports.len(), 2);
                assert_eq!(ports[0].as_num(), Some(80.0));
                assert_eq!(ports[1].as_num(), Some(443.0));
            } else {
                panic!("Expected ports to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_empty() {
        let input = "config:2[tags[0]= host=localhost]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            assert_eq!(fields.len(), 2);

            if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
                assert_eq!(tags.len(), 0);
            } else {
                panic!("Expected tags to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_single_item() {
        let input = "config:1[tags[1]=production]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
                assert_eq!(tags.len(), 1);
                assert_eq!(tags[0].as_str(), Some("production"));
            } else {
                panic!("Expected tags to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_comma_separated() {
        let input = "config:1[tags[3]=web,api,mobile]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
            if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
                assert_eq!(tags.len(), 3);
                assert_eq!(tags[0].as_str(), Some("web"));
                assert_eq!(tags[1].as_str(), Some("api"));
                assert_eq!(tags[2].as_str(), Some("mobile"));
            } else {
                panic!("Expected tags to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }

    #[test]
    fn test_nested_array_mixed_types() {
        let input = "data:1[values[4]=100 test 3.14 true]";
        let doc = LlmParser::parse(input).unwrap();

        if let Some(DxLlmValue::Obj(fields)) = doc.context.get("data") {
            if let Some(DxLlmValue::Arr(values)) = fields.get("values") {
                assert_eq!(values.len(), 4);
                assert_eq!(values[0].as_num(), Some(100.0));
                assert_eq!(values[1].as_str(), Some("test"));
                assert_eq!(values[2].as_num(), Some(3.14));
                assert_eq!(values[3].as_bool(), Some(true));
            } else {
                panic!("Expected values to be an array");
            }
        } else {
            panic!("Expected Obj variant");
        }
    }
}
