//! Property tests for Task 10.3: Compact Syntax Serialization
//!
//! **Property 22: Compact Syntax Serialization**
//! **Validates: Requirements 5.4**
//!
//! For any object marked for compact serialization, the serializer should output
//! the format `section:count@=[key value key value]`.

use proptest::prelude::*;
use serializer::llm::{DxDocument, DxLlmValue, LlmSerializer, SerializerConfig};
use std::collections::HashMap;

/// Generate an arbitrary object with simple values
fn arb_simple_object() -> impl Strategy<Value = HashMap<String, DxLlmValue>> {
    prop::collection::hash_map(
        "[a-z]{3,8}",
        prop_oneof![
            any::<bool>().prop_map(DxLlmValue::Bool),
            (0.0..1000.0).prop_map(DxLlmValue::Num),
            "[a-zA-Z0-9]{1,10}".prop_map(|s| DxLlmValue::Str(s)),
        ],
        1..5,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 22: Compact Syntax Serialization
    /// Objects with compact_syntax enabled should use @= format
    #[test]
    fn compact_syntax_uses_at_equals_format(obj in arb_simple_object()) {
        let mut config = SerializerConfig::default();
        config.compact_syntax = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj.clone()));

        let output = serializer.serialize(&doc);

        // Should contain @= marker
        prop_assert!(output.contains("@="), "Compact syntax should use @= marker: {}", output);

        // Should have format: name:count@=[...]
        prop_assert!(output.contains(&format!("config:{}@=[", obj.len())),
            "Should have count prefix with @=: {}", output);
    }

    /// Feature: dx-serializer-production-ready, Property 22: Compact Syntax Serialization
    /// Compact syntax should use space-separated key-value pairs without = signs
    #[test]
    fn compact_syntax_space_separated_no_equals(obj in arb_simple_object()) {
        let mut config = SerializerConfig::default();
        config.compact_syntax = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("data".to_string(), DxLlmValue::Obj(obj.clone()));

        let output = serializer.serialize(&doc);

        // Extract the content between @=[ and ]
        if let Some(start) = output.find("@=[") {
            let content_start = start + 3;
            if let Some(end) = output[content_start..].find(']') {
                let content = &output[content_start..content_start + end];

                // Should be space-separated tokens
                let tokens: Vec<&str> = content.split_whitespace().collect();

                // Should have even number of tokens (key-value pairs)
                prop_assert_eq!(tokens.len() % 2, 0,
                    "Compact syntax should have even number of tokens: {}", content);

                // Should NOT contain = signs within the brackets (except in the @= marker)
                prop_assert!(!content.contains('='),
                    "Compact syntax content should not contain = signs: {}", content);
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 22: Compact Syntax Serialization
    /// Without compact_syntax, objects should use inline format with = signs
    #[test]
    fn without_compact_syntax_uses_inline_format(obj in arb_simple_object()) {
        let config = SerializerConfig::default(); // compact_syntax = false
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj.clone()));

        let output = serializer.serialize(&doc);

        // Should NOT contain @= marker
        prop_assert!(!output.contains("@="),
            "Without compact_syntax, should not use @= marker: {}", output);

        // Should use inline format with [ and = signs
        prop_assert!(output.contains("config:"), "Should have name prefix: {}", output);
        prop_assert!(output.contains('['), "Should use brackets: {}", output);

        // Should contain = signs for key-value pairs (not in @= format)
        let bracket_content = output.split('[').nth(1).unwrap_or("");
        if !obj.is_empty() {
            prop_assert!(bracket_content.contains('='),
                "Inline format should contain = signs: {}", output);
        }
    }

    /// Feature: dx-serializer-production-ready, Property 22: Compact Syntax Serialization
    /// Compact syntax should preserve all key-value pairs
    #[test]
    fn compact_syntax_preserves_all_fields(obj in arb_simple_object()) {
        let mut config = SerializerConfig::default();
        config.compact_syntax = true;
        let serializer = LlmSerializer::with_config(config);

        let mut doc = DxDocument::new();
        doc.context.insert("data".to_string(), DxLlmValue::Obj(obj.clone()));

        let output = serializer.serialize(&doc);

        // All keys should appear in the output
        for key in obj.keys() {
            prop_assert!(output.contains(key),
                "Key '{}' should appear in output: {}", key, output);
        }
    }
}
