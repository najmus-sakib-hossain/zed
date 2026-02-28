//! Property tests for Task 8.4: Serializer output format
//!
//! Feature: dx-serializer-production-ready
//! Tests:
//! - Property 3: Inline Object Serialization Format
//! - Property 13: Schema Serialization Format
//! - Property 17: Array Serialization Format
//! Validates: Requirements 1.4, 3.3, 4.3

use proptest::prelude::*;
use serializer::llm::serializer::LlmSerializer;
use serializer::llm::types::{DxDocument, DxLlmValue, DxSection};
use std::collections::HashMap;

/// Strategy to generate valid identifier strings (keys)
fn valid_identifier() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
}

/// Strategy to generate valid simple values (no nested structures)
fn simple_value() -> impl Strategy<Value = DxLlmValue> {
    prop_oneof![
        // String values (alphanumeric, no spaces to avoid quoting complexity)
        "[a-zA-Z][a-zA-Z0-9._-]{0,20}".prop_map(|s| DxLlmValue::Str(s)),
        // Integer values
        (-10000i64..10000i64).prop_map(|n| DxLlmValue::Num(n as f64)),
        // Float values
        (-1000.0f64..1000.0f64)
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(DxLlmValue::Num),
        // Boolean values
        prop::bool::ANY.prop_map(DxLlmValue::Bool),
    ]
}

/// Strategy to generate a map of key-value pairs (1-10 pairs)
fn object_fields() -> impl Strategy<Value = HashMap<String, DxLlmValue>> {
    prop::collection::vec((valid_identifier(), simple_value()), 1..=10)
        .prop_map(|pairs| {
            let mut map = HashMap::new();
            for (k, v) in pairs {
                map.insert(k, v);
            }
            map
        })
        .prop_filter("unique keys", |map| !map.is_empty())
}

/// Strategy to generate an array of simple values (1-20 items)
fn array_values() -> impl Strategy<Value = Vec<DxLlmValue>> {
    prop::collection::vec(simple_value(), 1..=20)
}

/// Strategy to generate a schema (column names)
fn schema_columns() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(valid_identifier(), 1..=10).prop_filter("unique columns", |cols| {
        let mut unique = std::collections::HashSet::new();
        cols.iter().all(|c| unique.insert(c.clone()))
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 3: Inline Object Serialization Format
    /// **Validates: Requirements 1.4**
    ///
    /// For any DxDocument containing inline objects, serializing should produce
    /// space-separated key-value pairs in the format `section:count[key=value key2=value2]`.
    #[test]
    fn prop_inline_object_serialization_format(
        section_name in valid_identifier(),
        fields in object_fields()
    ) {
        let mut doc = DxDocument::new();
        doc.context.insert(section_name.clone(), DxLlmValue::Obj(fields.clone()));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains the section name
        prop_assert!(
            output.contains(&section_name),
            "Output does not contain section name '{}': {}",
            section_name,
            output
        );

        // Check that output contains count prefix
        let count_prefix = format!("{}:{}", section_name, fields.len());
        prop_assert!(
            output.contains(&count_prefix),
            "Output does not contain count prefix '{}': {}",
            count_prefix,
            output
        );

        // Check that output uses brackets
        prop_assert!(
            output.contains('[') && output.contains(']'),
            "Output does not contain brackets: {}",
            output
        );

        // Check that all keys are present in the output
        for key in fields.keys() {
            prop_assert!(
                output.contains(key),
                "Output does not contain key '{}': {}",
                key,
                output
            );
        }

        // Check that output uses space separators (not commas between fields)
        // Extract the object portion: section:count[...]
        if let Some(start) = output.find('[') {
            if let Some(end) = output[start..].find(']') {
                let object_content = &output[start + 1..start + end];

                // Count equals signs (one per field)
                let equals_count = object_content.matches('=').count();
                prop_assert_eq!(
                    equals_count,
                    fields.len(),
                    "Expected {} equals signs, got {}. Content: {}",
                    fields.len(),
                    equals_count,
                    object_content
                );

                // If there are multiple fields, check for space separators
                if fields.len() > 1 {
                    // Count spaces between fields (should have at least len-1 spaces)
                    let space_count = object_content.matches(' ').count();
                    prop_assert!(
                        space_count >= fields.len() - 1,
                        "Expected at least {} spaces for {} fields, got {}. Content: {}",
                        fields.len() - 1,
                        fields.len(),
                        space_count,
                        object_content
                    );
                }
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 3 variant: Nested arrays in objects
    /// **Validates: Requirements 1.4**
    ///
    /// For any object containing nested arrays, serializing should produce
    /// the format `key[count]=item1 item2 item3` with space-separated items.
    #[test]
    fn prop_inline_object_with_nested_array_serialization(
        section_name in valid_identifier(),
        key in valid_identifier(),
        array_items in array_values()
    ) {
        let mut doc = DxDocument::new();
        let mut fields = HashMap::new();
        fields.insert(key.clone(), DxLlmValue::Arr(array_items.clone()));
        doc.context.insert(section_name.clone(), DxLlmValue::Obj(fields));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains the nested array format: key[count]=
        let array_prefix = format!("{}[{}]=", key, array_items.len());
        prop_assert!(
            output.contains(&array_prefix),
            "Output does not contain nested array prefix '{}': {}",
            array_prefix,
            output
        );

        // Check that array items are space-separated (not comma-separated)
        if let Some(start) = output.find(&array_prefix) {
            let after_prefix = &output[start + array_prefix.len()..];
            // Extract until the next space or bracket
            let array_content = if let Some(end) = after_prefix.find(|c| c == ']' || c == '\n') {
                &after_prefix[..end]
            } else {
                after_prefix
            };

            // If there are multiple items, check for space separators
            if array_items.len() > 1 {
                let space_count = array_content.matches(' ').count();
                prop_assert!(
                    space_count >= array_items.len() - 1,
                    "Expected at least {} spaces for {} array items, got {}. Content: {}",
                    array_items.len() - 1,
                    array_items.len(),
                    space_count,
                    array_content
                );
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 13: Schema Serialization Format
    /// **Validates: Requirements 3.3**
    ///
    /// For any DxSection, serializing should output space-separated column names
    /// in the format `(col1 col2 col3)`.
    #[test]
    fn prop_schema_serialization_format(
        schema in schema_columns()
    ) {
        let mut doc = DxDocument::new();
        let section = DxSection::new(schema.clone());
        doc.sections.insert('t', section);

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains parentheses for schema
        prop_assert!(
            output.contains('(') && output.contains(')'),
            "Output does not contain schema parentheses: {}",
            output
        );

        // Check that all column names are present
        for col in &schema {
            prop_assert!(
                output.contains(col),
                "Output does not contain column '{}': {}",
                col,
                output
            );
        }

        // Extract the schema portion: (col1 col2 col3)
        if let Some(start) = output.find('(') {
            if let Some(end) = output[start..].find(')') {
                let schema_content = &output[start + 1..start + end];

                // Check that schema uses space separators (not commas)
                if schema.len() > 1 {
                    let space_count = schema_content.matches(' ').count();
                    prop_assert!(
                        space_count >= schema.len() - 1,
                        "Expected at least {} spaces for {} columns, got {}. Content: {}",
                        schema.len() - 1,
                        schema.len(),
                        space_count,
                        schema_content
                    );

                    // Should not contain commas
                    prop_assert!(
                        !schema_content.contains(','),
                        "Schema should not contain commas (use spaces): {}",
                        schema_content
                    );
                }

                // Verify all columns are in the schema content
                for col in &schema {
                    prop_assert!(
                        schema_content.contains(col),
                        "Schema content does not contain column '{}': {}",
                        col,
                        schema_content
                    );
                }
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 17: Array Serialization Format
    /// **Validates: Requirements 4.3**
    ///
    /// For any array value, serializing should output space-separated items
    /// in the format `name:count=item1 item2 item3`.
    #[test]
    fn prop_array_serialization_format(
        array_name in valid_identifier(),
        items in array_values()
    ) {
        let mut doc = DxDocument::new();
        doc.context.insert(array_name.clone(), DxLlmValue::Arr(items.clone()));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains the array name
        prop_assert!(
            output.contains(&array_name),
            "Output does not contain array name '{}': {}",
            array_name,
            output
        );

        // Check that output contains count prefix and equals sign
        let array_prefix = format!("{}:{}=", array_name, items.len());
        prop_assert!(
            output.contains(&array_prefix),
            "Output does not contain array prefix '{}': {}",
            array_prefix,
            output
        );

        // Extract the array content after the prefix
        if let Some(start) = output.find(&array_prefix) {
            let after_prefix = &output[start + array_prefix.len()..];
            // Extract until newline
            let array_content = if let Some(end) = after_prefix.find('\n') {
                &after_prefix[..end]
            } else {
                after_prefix
            };

            // Check that array uses space separators (not commas)
            if items.len() > 1 {
                let space_count = array_content.matches(' ').count();
                prop_assert!(
                    space_count >= items.len() - 1,
                    "Expected at least {} spaces for {} items, got {}. Content: {}",
                    items.len() - 1,
                    items.len(),
                    space_count,
                    array_content
                );

                // Should not contain commas between items
                // Note: commas might appear in nested structures, so we check the top level
                let comma_count = array_content.matches(',').count();
                prop_assert!(
                    comma_count == 0,
                    "Array should not contain commas at top level (use spaces): {}",
                    array_content
                );
            }

            // Verify the array content is not empty
            prop_assert!(
                !array_content.trim().is_empty(),
                "Array content should not be empty for {} items",
                items.len()
            );
        }
    }

    /// Feature: dx-serializer-production-ready, Property 17 variant: Single item arrays
    /// **Validates: Requirements 4.3**
    ///
    /// For any array with a single item, serializing should still use the
    /// correct format `name:1=item`.
    #[test]
    fn prop_single_item_array_serialization(
        array_name in valid_identifier(),
        item in simple_value()
    ) {
        let mut doc = DxDocument::new();
        doc.context.insert(array_name.clone(), DxLlmValue::Arr(vec![item]));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains the correct format
        let array_prefix = format!("{}:1=", array_name);
        prop_assert!(
            output.contains(&array_prefix),
            "Output does not contain single-item array prefix '{}': {}",
            array_prefix,
            output
        );
    }

    /// Feature: dx-serializer-production-ready, Property 3 variant: Empty objects
    /// **Validates: Requirements 1.4**
    ///
    /// For any empty object, serializing should produce a valid format.
    #[test]
    fn prop_empty_object_serialization(
        section_name in valid_identifier()
    ) {
        let mut doc = DxDocument::new();
        let fields = HashMap::new();
        doc.context.insert(section_name.clone(), DxLlmValue::Obj(fields));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Empty objects should serialize as section:0[]
        let expected_format = format!("{}:0[]", section_name);
        prop_assert!(
            output.contains(&expected_format),
            "Output does not contain empty object format '{}': {}",
            expected_format,
            output
        );
    }

    /// Feature: dx-serializer-production-ready, Property 13 variant: Single column schema
    /// **Validates: Requirements 3.3**
    ///
    /// For any schema with a single column, serializing should produce
    /// the correct format `(col)`.
    #[test]
    fn prop_single_column_schema_serialization(
        column_name in valid_identifier()
    ) {
        let mut doc = DxDocument::new();
        let section = DxSection::new(vec![column_name.clone()]);
        doc.sections.insert('t', section);

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Check that output contains the single column in parentheses
        let schema_format = format!("({})", column_name);
        prop_assert!(
            output.contains(&schema_format),
            "Output does not contain single column schema '{}': {}",
            schema_format,
            output
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_inline_object_space_separated() {
        let mut doc = DxDocument::new();
        let mut fields = HashMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context.insert("config".to_string(), DxLlmValue::Obj(fields));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have count prefix
        assert!(output.contains("config:2["), "Output: {}", output);
        // Should have both fields
        assert!(output.contains("host=localhost"), "Output: {}", output);
        assert!(output.contains("port=8080"), "Output: {}", output);
        // Should use space separator (not comma)
        assert!(!output.contains("host=localhost,port=8080"), "Output: {}", output);
    }

    #[test]
    fn test_nested_array_in_object() {
        let mut doc = DxDocument::new();
        let mut fields = HashMap::new();
        fields.insert(
            "tags".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("rust".to_string()),
                DxLlmValue::Str("fast".to_string()),
            ]),
        );
        doc.context.insert("item".to_string(), DxLlmValue::Obj(fields));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have nested array format
        assert!(output.contains("tags[2]=rust fast"), "Output: {}", output);
    }

    #[test]
    fn test_schema_space_separated() {
        let mut doc = DxDocument::new();
        let section =
            DxSection::new(vec!["id".to_string(), "name".to_string(), "email".to_string()]);
        doc.sections.insert('t', section);

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have space-separated schema
        assert!(output.contains("(id name email)"), "Output: {}", output);
        // Should not have comma-separated schema
        assert!(!output.contains("(id,name,email)"), "Output: {}", output);
    }

    #[test]
    fn test_array_space_separated() {
        let mut doc = DxDocument::new();
        doc.context.insert(
            "tags".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("rust".to_string()),
                DxLlmValue::Str("performance".to_string()),
                DxLlmValue::Str("serialization".to_string()),
            ]),
        );

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have space-separated array
        assert!(output.contains("tags:3=rust performance serialization"), "Output: {}", output);
        // Should not have comma-separated array
        assert!(!output.contains("tags:3=rust,performance,serialization"), "Output: {}", output);
    }

    #[test]
    fn test_empty_object() {
        let mut doc = DxDocument::new();
        let fields = HashMap::new();
        doc.context.insert("empty".to_string(), DxLlmValue::Obj(fields));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have empty object format
        assert!(output.contains("empty:0[]"), "Output: {}", output);
    }

    #[test]
    fn test_single_item_array() {
        let mut doc = DxDocument::new();
        doc.context
            .insert("tag".to_string(), DxLlmValue::Arr(vec![DxLlmValue::Str("rust".to_string())]));

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have single-item array format
        assert!(output.contains("tag:1=rust"), "Output: {}", output);
    }

    #[test]
    fn test_single_column_schema() {
        let mut doc = DxDocument::new();
        let section = DxSection::new(vec!["id".to_string()]);
        doc.sections.insert('t', section);

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        // Should have single column schema
        assert!(output.contains("(id)"), "Output: {}", output);
    }
}
