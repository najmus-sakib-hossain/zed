//! Property tests for Task 13.3: Legacy Mode Support
//!
//! **Property 33: Legacy Mode Serialization**
//! **Validates: Requirements 11.5**
//!
//! For any DxDocument, when serializing in legacy mode, the output should use
//! comma separators for arrays and schemas instead of space separators.

use proptest::prelude::*;
use serializer::llm::{DxDocument, DxLlmValue, DxSection, LlmSerializer, SerializerConfig};
use std::collections::HashMap;

/// Generate an arbitrary array
fn arb_array() -> impl Strategy<Value = Vec<DxLlmValue>> {
    prop::collection::vec(
        prop_oneof![
            any::<bool>().prop_map(DxLlmValue::Bool),
            (0.0..1000.0).prop_map(DxLlmValue::Num),
            "[a-zA-Z]{3,8}".prop_map(|s| DxLlmValue::Str(s)),
        ],
        2..6,
    )
}

/// Generate an arbitrary object
fn arb_object() -> impl Strategy<Value = HashMap<String, DxLlmValue>> {
    prop::collection::hash_map(
        "[a-z]{3,8}",
        prop_oneof![
            any::<bool>().prop_map(DxLlmValue::Bool),
            (0.0..1000.0).prop_map(DxLlmValue::Num),
            "[a-zA-Z]{3,8}".prop_map(|s| DxLlmValue::Str(s)),
        ],
        2..5,
    )
}

/// Generate an arbitrary section
fn arb_section() -> impl Strategy<Value = DxSection> {
    (2..5_usize, 2..5_usize).prop_flat_map(|(col_count, row_count)| {
        let schema: Vec<String> = (0..col_count).map(|i| format!("col{}", i)).collect();
        let rows = (0..row_count)
            .map(|i| (0..col_count).map(|j| DxLlmValue::Str(format!("val{}_{}", i, j))).collect())
            .collect();
        Just(DxSection { schema, rows })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 33: Legacy Mode Serialization
    /// Arrays in legacy mode should use comma separators
    #[test]
    fn legacy_mode_arrays_use_commas(arr in arb_array()) {
        let mut config = SerializerConfig::default();
        config.legacy_mode = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("items".to_string(), DxLlmValue::Arr(arr.clone()));

        let output = serializer.serialize(&doc);

        // Should contain comma separators in array
        if arr.len() > 1 {
            prop_assert!(output.contains(','), "Legacy mode arrays should use commas: {}", output);
        }

        // Should NOT use space-only separators (spaces may appear in values)
        // Check that after the = sign, we have comma-separated values
        if let Some(eq_pos) = output.find('=') {
            let after_eq = &output[eq_pos + 1..];
            if arr.len() > 1 {
                prop_assert!(after_eq.contains(','),
                    "Legacy mode array values should be comma-separated: {}", output);
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 33: Legacy Mode Serialization
    /// Objects in legacy mode should use comma separators
    #[test]
    fn legacy_mode_objects_use_commas(obj in arb_object()) {
        let mut config = SerializerConfig::default();
        config.legacy_mode = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj.clone()));

        let output = serializer.serialize(&doc);

        // Should contain comma separators in object fields
        if obj.len() > 1 {
            let bracket_content = output.split('[').nth(1).unwrap_or("");
            prop_assert!(bracket_content.contains(','),
                "Legacy mode objects should use commas: {}", output);
        }
    }

    /// Feature: dx-serializer-production-ready, Property 33: Legacy Mode Serialization
    /// Schemas in legacy mode should use comma separators
    #[test]
    fn legacy_mode_schemas_use_commas(section in arb_section()) {
        let mut config = SerializerConfig::default();
        config.legacy_mode = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Extract schema (between parentheses)
        if let Some(start) = output.find('(') {
            if let Some(end) = output[start..].find(')') {
                let schema = &output[start + 1..start + end];
                if section.schema.len() > 1 {
                    prop_assert!(schema.contains(','),
                        "Legacy mode schemas should use commas: {}", output);
                }
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 33: Legacy Mode Serialization
    /// Without legacy mode, arrays should use space separators
    #[test]
    fn default_mode_arrays_use_spaces(arr in arb_array()) {
        let config = SerializerConfig::default(); // legacy_mode = false
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("items".to_string(), DxLlmValue::Arr(arr.clone()));

        let output = serializer.serialize(&doc);

        // Should NOT contain commas in array (unless in values themselves)
        // Check that after the = sign, we don't have comma separators
        if let Some(eq_pos) = output.find('=') {
            let after_eq = &output[eq_pos + 1..].trim();
            // In default mode, array items are space-separated
            // We can check that there are spaces but no commas between items
            if arr.len() > 1 {
                // Should have spaces
                prop_assert!(after_eq.contains(' '),
                    "Default mode arrays should use spaces: {}", output);
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 33: Legacy Mode Serialization
    /// Without legacy mode, schemas should use space separators
    #[test]
    fn default_mode_schemas_use_spaces(section in arb_section()) {
        let config = SerializerConfig::default(); // legacy_mode = false
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Extract schema (between parentheses)
        if let Some(start) = output.find('(') {
            if let Some(end) = output[start..].find(')') {
                let schema = &output[start + 1..start + end];
                if section.schema.len() > 1 {
                    // Should have spaces, not commas
                    prop_assert!(schema.contains(' '),
                        "Default mode schemas should use spaces: {}", output);
                    prop_assert!(!schema.contains(','),
                        "Default mode schemas should not use commas: {}", output);
                }
            }
        }
    }
}
