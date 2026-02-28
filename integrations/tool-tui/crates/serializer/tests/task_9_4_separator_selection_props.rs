//! Property tests for Task 9.4: Serializer Separator Selection
//!
//! **Property 10: Serializer Separator Selection**
//! **Validates: Requirements 2.6**
//!
//! For any DxSection, the serializer should choose an appropriate row separator
//! based on data characteristics (table size, complexity, schema).

use proptest::prelude::*;
use serializer::llm::{DxDocument, DxLlmValue, DxSection, LlmSerializer};

/// Generate a section with many rows (> 10) to test newline separator
fn arb_large_section() -> impl Strategy<Value = DxSection> {
    (11..20_usize).prop_flat_map(|row_count| {
        let schema = vec!["id".to_string(), "name".to_string()];
        let rows = (0..row_count)
            .map(|i| {
                vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Str(format!("item{}", i)),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

/// Generate a section with timestamp/log columns to test colon separator
fn arb_log_section() -> impl Strategy<Value = DxSection> {
    (1..5_usize).prop_flat_map(|row_count| {
        let schema = vec![
            "timestamp".to_string(),
            "level".to_string(),
            "message".to_string(),
        ];
        let rows = (0..row_count)
            .map(|i| {
                vec![
                    DxLlmValue::Str(format!("2025-01-{}T10:00:00Z", i + 1)),
                    DxLlmValue::Str("info".to_string()),
                    DxLlmValue::Str(format!("msg{}", i)),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

/// Generate a section with complex data (nested arrays/objects) to test semicolon separator
fn arb_complex_section() -> impl Strategy<Value = DxSection> {
    (1..5_usize).prop_flat_map(|row_count| {
        let schema = vec!["id".to_string(), "data".to_string()];
        let rows = (0..row_count)
            .map(|i| {
                vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Arr(vec![
                        DxLlmValue::Str("a".to_string()),
                        DxLlmValue::Str("b".to_string()),
                    ]),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

/// Generate a simple small section to test comma separator (default)
fn arb_simple_section() -> impl Strategy<Value = DxSection> {
    (1..5_usize).prop_flat_map(|row_count| {
        let schema = vec!["id".to_string(), "value".to_string()];
        let rows = (0..row_count)
            .map(|i| {
                vec![
                    DxLlmValue::Num(i as f64),
                    DxLlmValue::Str(format!("val{}", i)),
                ]
            })
            .collect();
        Just(DxSection { schema, rows })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 10: Serializer Separator Selection
    /// Large tables (> 10 rows) should use newline separator
    #[test]
    fn large_tables_use_newline_separator(section in arb_large_section()) {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Large tables should use newline separator (multi-line format)
        // Check that rows are on separate lines
        let lines: Vec<&str> = output.lines().collect();
        prop_assert!(lines.len() > section.rows.len(), "Expected multi-line format for large table");

        // Should NOT use inline separators (comma, semicolon, colon)
        let table_content = output.split('[').nth(1).unwrap_or("");
        prop_assert!(!table_content.contains(", "), "Large table should not use comma separator");
        prop_assert!(!table_content.contains("; "), "Large table should not use semicolon separator");
        prop_assert!(!table_content.contains(": "), "Large table should not use colon separator");
    }

    /// Feature: dx-serializer-production-ready, Property 10: Serializer Separator Selection
    /// Tables with timestamp/log columns should use colon separator
    #[test]
    fn log_tables_use_colon_separator(section in arb_log_section()) {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Log tables should use colon separator (inline format)
        // Check for colon separator between rows
        let table_content = output.split('[').nth(1).unwrap_or("");
        if section.rows.len() > 1 {
            prop_assert!(table_content.contains(": "), "Log table should use colon separator: {}", output);
        }

        // Should be inline (not multi-line)
        let lines: Vec<&str> = output.lines().collect();
        prop_assert!(lines.len() <= 3, "Log table should use inline format, got {} lines", lines.len());
    }

    /// Feature: dx-serializer-production-ready, Property 10: Serializer Separator Selection
    /// Tables with complex data (nested arrays/objects) should use semicolon separator
    #[test]
    fn complex_tables_use_semicolon_separator(section in arb_complex_section()) {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Complex tables should use semicolon separator (inline format)
        let table_content = output.split('[').nth(1).unwrap_or("");
        if section.rows.len() > 1 {
            prop_assert!(table_content.contains("; "), "Complex table should use semicolon separator: {}", output);
        }

        // Should be inline (not multi-line)
        let lines: Vec<&str> = output.lines().collect();
        prop_assert!(lines.len() <= 3, "Complex table should use inline format, got {} lines", lines.len());
    }

    /// Feature: dx-serializer-production-ready, Property 10: Serializer Separator Selection
    /// Simple small tables should use comma separator (default)
    #[test]
    fn simple_tables_use_comma_separator(section in arb_simple_section()) {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.sections.insert('d', section.clone());

        let output = serializer.serialize(&doc);

        // Simple tables should use comma separator (inline format)
        let table_content = output.split('[').nth(1).unwrap_or("");
        if section.rows.len() > 1 {
            prop_assert!(table_content.contains(", "), "Simple table should use comma separator: {}", output);
        }

        // Should be inline (not multi-line)
        let lines: Vec<&str> = output.lines().collect();
        prop_assert!(lines.len() <= 3, "Simple table should use inline format, got {} lines", lines.len());
    }
}
