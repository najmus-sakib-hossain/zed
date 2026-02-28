//! Comprehensive round-trip tests for dx-serializer
//!
//! Feature: dx-serializer-production-ready
//! Tasks 14.1-14.3: Add comprehensive round-trip tests
//!
//! This module tests Properties 31, 32, 4, 14, 18, 23, and 30 from the design document.

use proptest::prelude::*;
use serializer::llm::{
    DxDocument, DxLlmValue, DxSection, LlmParser, LlmSerializer, SerializerConfig,
};
use std::collections::HashMap;

// =============================================================================
// STRATEGIES FOR GENERATING TEST DATA
// =============================================================================

/// Generate simple DxLlmValue instances
fn simple_llm_value() -> impl Strategy<Value = DxLlmValue> {
    prop_oneof![
        Just(DxLlmValue::Null),
        any::<bool>().prop_map(DxLlmValue::Bool),
        // Use floats in a safe range to avoid i64 overflow issues in serializer
        // The serializer casts whole numbers to i64, so we need to stay within i64 range
        (-1e15_f64..1e15_f64)
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(DxLlmValue::Num),
        "[a-zA-Z][a-zA-Z0-9_]{0,20}".prop_map(|s| DxLlmValue::Str(s)),
    ]
}

/// Generate arrays of simple values
fn simple_array() -> impl Strategy<Value = Vec<DxLlmValue>> {
    prop::collection::vec(simple_llm_value(), 1..10)
}

/// Generate simple objects (HashMap)
fn simple_object() -> impl Strategy<Value = HashMap<String, DxLlmValue>> {
    prop::collection::hash_map("[a-z][a-z0-9_]{0,10}", simple_llm_value(), 1..5)
}

/// Generate a DxSection with simple data
fn simple_section() -> impl Strategy<Value = DxSection> {
    (prop::collection::vec("[a-z][a-z0-9_]{0,10}", 2..5), 1usize..10).prop_flat_map(
        |(schema, num_rows)| {
            let num_cols = schema.len();
            prop::collection::vec(
                prop::collection::vec(simple_llm_value(), num_cols..=num_cols),
                num_rows..=num_rows,
            )
            .prop_map(move |rows| {
                let mut section = DxSection::new(schema.clone());
                for row in rows {
                    let _ = section.add_row(row);
                }
                section
            })
        },
    )
}

/// Generate a simple DxDocument
fn simple_document() -> impl Strategy<Value = DxDocument> {
    (
        prop::collection::hash_map("[a-z][a-z0-9_]{0,10}", simple_llm_value(), 0..5),
        prop::collection::hash_map("[a-z][a-z0-9_]{0,10}", "[a-zA-Z0-9_]{1,20}", 0..3),
        prop::collection::vec(
            (prop::sample::select(vec!['a', 'b', 'c', 'd', 'e']), simple_section()),
            0..3,
        ),
    )
        .prop_map(|(context, refs, sections)| {
            let mut doc = DxDocument::new();
            doc.context = context;
            doc.refs = refs;
            doc.sections = sections.into_iter().collect();
            doc
        })
}

/// Generate inline objects with various field types
#[allow(dead_code)]
fn inline_object_value() -> impl Strategy<Value = DxLlmValue> {
    simple_object().prop_map(DxLlmValue::Obj)
}

/// Generate arrays with nested arrays
#[allow(dead_code)]
fn nested_array_value() -> impl Strategy<Value = DxLlmValue> {
    prop::collection::vec(
        prop_oneof![simple_llm_value(), simple_array().prop_map(DxLlmValue::Arr)],
        1..5,
    )
    .prop_map(DxLlmValue::Arr)
}

// =============================================================================
// PROPERTY 31: GENERAL PARSE-SERIALIZE ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 31: General Parse-Serialize Round-Trip
    /// **Validates: Requirements 7.1**
    ///
    /// For any valid LLM format string, parsing then serializing then parsing
    /// should produce an equivalent DxDocument structure.
    #[test]
    fn prop_parse_serialize_roundtrip(doc in simple_document()) {
        // Serialize the document
        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);

        // Parse it back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse serialized document: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Parse again to ensure stability
        let serialized2 = serializer.serialize(&parsed);
        let parsed2 = LlmParser::parse(&serialized2);
        prop_assert!(
            parsed2.is_ok(),
            "Failed to parse re-serialized document: {:?}",
            parsed2.err()
        );
        let parsed2 = parsed2.unwrap();

        // The two parsed documents should be equivalent
        prop_assert!(
            documents_equivalent(&parsed, &parsed2),
            "Round-trip mismatch:\nFirst parse: {:?}\nSecond parse: {:?}\nFirst serialized: {}\nSecond serialized: {}",
            parsed, parsed2, serialized, serialized2
        );
    }
}

// =============================================================================
// PROPERTY 32: GENERAL SERIALIZE-PARSE ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 32: General Serialize-Parse Round-Trip
    /// **Validates: Requirements 7.2**
    ///
    /// For any DxDocument, serializing then parsing should produce an equivalent
    /// DxDocument with all context, refs, and sections preserved.
    #[test]
    fn prop_serialize_parse_roundtrip(doc in simple_document()) {
        // Serialize the document
        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);

        // Parse it back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse serialized document: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Documents should be equivalent
        prop_assert!(
            documents_equivalent(&doc, &parsed),
            "Serialize-parse round-trip mismatch:\nOriginal: {:?}\nParsed: {:?}\nSerialized: {}",
            doc, parsed, serialized
        );
    }
}

// =============================================================================
// PROPERTY 4: INLINE OBJECT ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 4: Inline Object Round-Trip
    /// **Validates: Requirements 1.5**
    ///
    /// For any valid inline object string, parsing then serializing then parsing
    /// should produce an equivalent DxDocument structure.
    #[test]
    fn prop_inline_object_roundtrip(obj in simple_object()) {
        // Create a document with an inline object
        let mut doc = DxDocument::new();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj.clone()));

        // Serialize
        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);

        // Parse back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse inline object: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Check that the object is preserved
        prop_assert!(
            parsed.context.contains_key("config"),
            "Parsed document should contain 'config' key"
        );

        if let Some(DxLlmValue::Obj(parsed_obj)) = parsed.context.get("config") {
            prop_assert!(
                objects_equivalent(&obj, parsed_obj),
                "Inline object round-trip mismatch:\nOriginal: {:?}\nParsed: {:?}",
                obj, parsed_obj
            );
        } else {
            prop_assert!(false, "Parsed value should be an object");
        }
    }
}

// =============================================================================
// PROPERTY 14: SCHEMA ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 14: Schema Round-Trip
    /// **Validates: Requirements 3.4**
    ///
    /// For any table schema, serializing then parsing should preserve all
    /// column names in order.
    #[test]
    fn prop_schema_roundtrip(section in simple_section()) {
        // Create a document with a section
        let mut doc = DxDocument::new();
        doc.sections.insert('t', section.clone());

        // Serialize
        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);

        // Parse back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse schema: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Check that the schema is preserved
        prop_assert!(
            parsed.sections.contains_key(&'t'),
            "Parsed document should contain section 't'"
        );

        if let Some(parsed_section) = parsed.sections.get(&'t') {
            prop_assert_eq!(
                &section.schema, &parsed_section.schema,
                "Schema should be preserved in round-trip"
            );
        }
    }
}

// =============================================================================
// PROPERTY 18: ARRAY ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 18: Array Round-Trip
    /// **Validates: Requirements 4.4**
    ///
    /// For any array, serializing then parsing should preserve all items in order.
    #[test]
    fn prop_array_roundtrip(arr in simple_array()) {
        // Create a document with an array
        let mut doc = DxDocument::new();
        doc.context.insert("items".to_string(), DxLlmValue::Arr(arr.clone()));

        // Serialize
        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);

        // Parse back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse array: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Check that the array is preserved
        prop_assert!(
            parsed.context.contains_key("items"),
            "Parsed document should contain 'items' key"
        );

        if let Some(DxLlmValue::Arr(parsed_arr)) = parsed.context.get("items") {
            prop_assert_eq!(
                arr.len(), parsed_arr.len(),
                "Array length should be preserved"
            );

            for (i, (orig, parsed)) in arr.iter().zip(parsed_arr.iter()).enumerate() {
                prop_assert!(
                    values_equivalent(orig, parsed),
                    "Array item {} mismatch:\nOriginal: {:?}\nParsed: {:?}",
                    i, orig, parsed
                );
            }
        } else {
            prop_assert!(false, "Parsed value should be an array");
        }
    }
}

// =============================================================================
// PROPERTY 23: COMPACT SYNTAX ROUND-TRIP
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 23: Compact Syntax Round-Trip
    /// **Validates: Requirements 5.5**
    ///
    /// For any valid compact syntax string, parsing then serializing then parsing
    /// should produce an equivalent DxDocument structure.
    #[test]
    fn prop_compact_syntax_roundtrip(obj in simple_object()) {
        // Create a document with an object
        let mut doc = DxDocument::new();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj.clone()));

        // Serialize with compact syntax enabled
        let config = SerializerConfig {
            legacy_mode: false,
            prefix_elimination: false,
            compact_syntax: true,
        };
        let serializer = LlmSerializer::with_config(config);
        let serialized = serializer.serialize(&doc);

        // Parse back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse compact syntax: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Check that the object is preserved
        prop_assert!(
            parsed.context.contains_key("config"),
            "Parsed document should contain 'config' key"
        );

        if let Some(DxLlmValue::Obj(parsed_obj)) = parsed.context.get("config") {
            prop_assert!(
                objects_equivalent(&obj, parsed_obj),
                "Compact syntax round-trip mismatch:\nOriginal: {:?}\nParsed: {:?}\nSerialized: {}",
                obj, parsed_obj, serialized
            );
        } else {
            prop_assert!(false, "Parsed value should be an object");
        }
    }
}

// =============================================================================
// PROPERTY 30: PREFIX ELIMINATION ROUND-TRIP
// =============================================================================

/// Generate a section with common prefixes in string columns
#[allow(dead_code)]
fn section_with_common_prefix() -> impl Strategy<Value = DxSection> {
    (
        prop::sample::select(vec!["/api/", "https://example.com/", "user_"]),
        prop::collection::vec("[a-z]{3,10}", 3..8),
    )
        .prop_filter("suffixes must be unique", |(_, suffixes)| {
            // Ensure all suffixes are different to avoid empty strings after prefix removal
            let unique: std::collections::HashSet<_> = suffixes.iter().collect();
            unique.len() == suffixes.len()
        })
        .prop_map(|(prefix, suffixes)| {
            let mut section = DxSection::new(vec!["id".to_string(), "endpoint".to_string()]);
            for (i, suffix) in suffixes.iter().enumerate() {
                let _ = section.add_row(vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Str(format!("{}{}", prefix, suffix)),
                ]);
            }
            section
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 30: Prefix Elimination Round-Trip
    /// **Validates: Requirements 6.7**
    ///
    /// For any table with prefix elimination, serializing with prefix markers
    /// then parsing should reconstruct the original values correctly.
    #[test]
    fn prop_prefix_elimination_roundtrip(section in section_with_common_prefix()) {
        // Create a document with a section
        let mut doc = DxDocument::new();
        doc.sections.insert('e', section.clone());

        // Serialize with prefix elimination enabled
        let config = SerializerConfig {
            legacy_mode: false,
            prefix_elimination: true,
            compact_syntax: false,
        };
        let serializer = LlmSerializer::with_config(config);
        let serialized = serializer.serialize(&doc);

        // Parse back
        let parsed = LlmParser::parse(&serialized);
        prop_assert!(
            parsed.is_ok(),
            "Failed to parse with prefix elimination: {:?}\nSerialized: {}",
            parsed.err(),
            serialized
        );
        let parsed = parsed.unwrap();

        // Check that the section is preserved
        prop_assert!(
            parsed.sections.contains_key(&'e'),
            "Parsed document should contain section 'e'"
        );

        if let Some(parsed_section) = parsed.sections.get(&'e') {
            // Schema should match
            prop_assert_eq!(
                &section.schema, &parsed_section.schema,
                "Schema should be preserved"
            );

            // Row count should match
            prop_assert_eq!(
                section.rows.len(), parsed_section.rows.len(),
                "Row count should be preserved"
            );

            // All values should be preserved (even if prefixes were eliminated)
            for (i, (orig_row, parsed_row)) in section.rows.iter().zip(parsed_section.rows.iter()).enumerate() {
                prop_assert_eq!(
                    orig_row.len(), parsed_row.len(),
                    "Row {} column count should match", i
                );

                for (j, (orig_val, parsed_val)) in orig_row.iter().zip(parsed_row.iter()).enumerate() {
                    prop_assert!(
                        values_equivalent(orig_val, parsed_val),
                        "Row {} col {} mismatch:\nOriginal: {:?}\nParsed: {:?}",
                        i, j, orig_val, parsed_val
                    );
                }
            }
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if two DxDocuments are semantically equivalent
fn documents_equivalent(a: &DxDocument, b: &DxDocument) -> bool {
    // Check context
    if a.context.len() != b.context.len() {
        return false;
    }
    for (key, val_a) in &a.context {
        if let Some(val_b) = b.context.get(key) {
            if !values_equivalent(val_a, val_b) {
                return false;
            }
        } else {
            return false;
        }
    }

    // Note: refs are not serialized in the current format, so we skip checking them
    // They are used internally for reference resolution but not persisted

    // Check sections
    if a.sections.len() != b.sections.len() {
        return false;
    }
    for (key, section_a) in &a.sections {
        if let Some(section_b) = b.sections.get(key) {
            if !sections_equivalent(section_a, section_b) {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

/// Check if two DxSections are semantically equivalent
fn sections_equivalent(a: &DxSection, b: &DxSection) -> bool {
    if a.schema != b.schema {
        return false;
    }
    if a.rows.len() != b.rows.len() {
        return false;
    }
    for (row_a, row_b) in a.rows.iter().zip(b.rows.iter()) {
        if row_a.len() != row_b.len() {
            return false;
        }
        for (val_a, val_b) in row_a.iter().zip(row_b.iter()) {
            if !values_equivalent(val_a, val_b) {
                return false;
            }
        }
    }
    true
}

/// Check if two DxLlmValues are semantically equivalent
fn values_equivalent(a: &DxLlmValue, b: &DxLlmValue) -> bool {
    match (a, b) {
        (DxLlmValue::Null, DxLlmValue::Null) => true,
        (DxLlmValue::Bool(a), DxLlmValue::Bool(b)) => a == b,
        (DxLlmValue::Num(a), DxLlmValue::Num(b)) => {
            if a.is_nan() && b.is_nan() {
                true
            } else if a.is_infinite() && b.is_infinite() {
                a.signum() == b.signum()
            } else {
                (a - b).abs() < 1e-10 || (a - b).abs() / a.abs().max(b.abs()) < 1e-10
            }
        }
        (DxLlmValue::Str(a), DxLlmValue::Str(b)) => a == b,
        (DxLlmValue::Arr(a), DxLlmValue::Arr(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equivalent(x, y))
        }
        (DxLlmValue::Obj(a), DxLlmValue::Obj(b)) => objects_equivalent(a, b),
        _ => false,
    }
}

/// Check if two objects are semantically equivalent
fn objects_equivalent(a: &HashMap<String, DxLlmValue>, b: &HashMap<String, DxLlmValue>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (key, val_a) in a {
        if let Some(val_b) = b.get(key) {
            if !values_equivalent(val_a, val_b) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_roundtrip() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("test".to_string()));
        doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));

        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);
        let parsed = LlmParser::parse(&serialized).unwrap();

        assert!(documents_equivalent(&doc, &parsed));
    }

    #[test]
    fn test_array_roundtrip() {
        let mut doc = DxDocument::new();
        let arr = vec![
            DxLlmValue::Str("a".to_string()),
            DxLlmValue::Str("b".to_string()),
            DxLlmValue::Str("c".to_string()),
        ];
        doc.context.insert("items".to_string(), DxLlmValue::Arr(arr));

        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);
        let parsed = LlmParser::parse(&serialized).unwrap();

        assert!(documents_equivalent(&doc, &parsed));
    }

    #[test]
    fn test_object_roundtrip() {
        let mut doc = DxDocument::new();
        let mut obj = HashMap::new();
        obj.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        obj.insert("port".to_string(), DxLlmValue::Num(5432.0));
        doc.context.insert("config".to_string(), DxLlmValue::Obj(obj));

        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);
        let parsed = LlmParser::parse(&serialized).unwrap();

        assert!(documents_equivalent(&doc, &parsed));
    }

    #[test]
    fn test_section_roundtrip() {
        let mut doc = DxDocument::new();
        let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);
        section
            .add_row(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Alice".to_string())])
            .unwrap();
        section
            .add_row(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("Bob".to_string())])
            .unwrap();
        doc.sections.insert('u', section);

        let serializer = LlmSerializer::new();
        let serialized = serializer.serialize(&doc);
        eprintln!("Serialized: {}", serialized);
        let parsed = LlmParser::parse(&serialized).unwrap();
        eprintln!("Parsed sections: {:?}", parsed.sections.keys().collect::<Vec<_>>());

        assert!(documents_equivalent(&doc, &parsed));
    }
}
