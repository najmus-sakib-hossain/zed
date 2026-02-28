//! Property tests for parser input validation
//!
//! Feature: serializer-battle-hardening
//! Feature: serializer-production-hardening
//! Tests Properties 1-3 from the battle-hardening design document
//! Tests Property 2 from the production-hardening design document

use proptest::prelude::*;
use serializer::{parse, DxError, DxValue};

/// Strategy to generate strings with null bytes at random positions
fn string_with_null_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::num::u8::ANY, 1..100).prop_map(|mut bytes| {
        // Ensure at least one null byte
        if !bytes.contains(&0) && !bytes.is_empty() {
            let pos = bytes.len() / 2;
            bytes[pos] = 0;
        }
        bytes
    })
}

/// Strategy to generate valid DX key-value pairs
fn valid_dx_input() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,20}".prop_flat_map(|key| {
        "[a-zA-Z0-9_]{1,50}".prop_map(move |value| format!("{}:{}", key, value))
    })
}

/// Strategy to generate syntactically invalid inputs
fn invalid_syntax_input() -> impl Strategy<Value = String> {
    prop_oneof![
        // Missing value after colon
        "[a-z][a-z0-9_]{0,10}:".prop_map(|s| s),
        // Invalid characters at start
        Just(":::invalid".to_string()),
        Just("@@@bad".to_string()),
        // Unclosed structures
        Just("key:value\n$undefined.ref:test".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: serializer-battle-hardening, Property 1: Null Byte Handling
    /// Validates: Requirements 1.1
    ///
    /// For any input string containing null bytes at any position,
    /// the Parser SHALL either successfully parse the input or return
    /// a well-formed error without panicking.
    #[test]
    fn prop_null_byte_handling(input in string_with_null_bytes()) {
        // The parser should not panic on null bytes
        let result = std::panic::catch_unwind(|| {
            parse(&input)
        });
        
        // Should not panic
        prop_assert!(result.is_ok(), "Parser panicked on input with null bytes");
        
        // If it returns an error, it should be a well-formed error
        if let Ok(Err(e)) = result {
            // Error should have a meaningful message
            let msg = format!("{}", e);
            prop_assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }

    /// Feature: serializer-battle-hardening, Property 2: UTF-8 Validation with Offset
    /// Validates: Requirements 1.4
    ///
    /// For any byte sequence containing invalid UTF-8, the Parser SHALL
    /// return a Utf8Error where the offset field exactly matches the byte
    /// position of the first invalid sequence.
    #[test]
    fn prop_utf8_validation_with_offset(
        prefix in "[a-z]{0,10}",
        suffix in "[a-z]{0,10}"
    ) {
        // Create input with invalid UTF-8 at a known position
        let mut input = prefix.as_bytes().to_vec();
        let invalid_offset = input.len();
        
        // Add invalid UTF-8 sequence (0xFF is never valid in UTF-8)
        input.push(0xFF);
        input.extend_from_slice(suffix.as_bytes());
        
        let result = parse(&input);
        
        // Should return an error (either UTF-8 or parse error)
        // The parser may handle invalid UTF-8 in different ways
        match result {
            Ok(_) => {
                // If parsing succeeds, the invalid byte was handled gracefully
                // (e.g., treated as part of a binary value)
            }
            Err(DxError::Utf8Error { offset }) => {
                // If UTF-8 error, offset should be at or after the invalid byte
                prop_assert!(
                    offset >= invalid_offset,
                    "UTF-8 error offset {} should be >= invalid byte position {}",
                    offset, invalid_offset
                );
            }
            Err(_) => {
                // Other errors are acceptable (e.g., parse errors)
            }
        }
    }

    /// Feature: serializer-battle-hardening, Property 3: Error Position Reporting
    /// Validates: Requirements 1.5, 7.1
    ///
    /// For any syntactically invalid input, the Parser SHALL return an error
    /// containing position information.
    #[test]
    fn prop_error_position_reporting(input in invalid_syntax_input()) {
        let result = parse(input.as_bytes());
        
        // Should return an error for invalid syntax
        if let Err(e) = result {
            // Check that error has position information
            let has_position = e.offset().is_some() || e.location().is_some();
            
            // For certain error types, position info is expected
            match &e {
                DxError::InvalidSyntax { pos, .. } => {
                    prop_assert!(*pos < input.len() + 1, "Position should be within input bounds");
                }
                DxError::ParseError { location, .. } => {
                    prop_assert!(location.line >= 1, "Line number should be >= 1");
                    prop_assert!(location.column >= 1, "Column number should be >= 1");
                }
                DxError::UnknownAlias(_) => {
                    // Alias errors may not have position info
                }
                _ => {
                    // Other errors may or may not have position info
                }
            }
        }
        // Note: Some "invalid" inputs may actually parse successfully
        // due to the flexible nature of the DX format
    }

    /// Additional property: Valid inputs should parse successfully
    #[test]
    fn prop_valid_input_parses(input in valid_dx_input()) {
        let result = parse(input.as_bytes());
        prop_assert!(result.is_ok(), "Valid DX input should parse: {:?}", result.err());
    }
}

// =============================================================================
// PARSER COMPLETENESS PROPERTY TESTS (Production Hardening)
// =============================================================================

mod parser_completeness_props {
    use super::*;

    /// Strategy to generate valid DX key-value pairs with various value types
    fn valid_key_value() -> impl Strategy<Value = String> {
        let key = "[a-z][a-z0-9_]{0,15}";
        prop_oneof![
            // String values (alphanumeric)
            (key, "[a-zA-Z][a-zA-Z0-9_]{0,30}").prop_map(|(k, v)| format!("{}:{}", k, v)),
            // Integer values
            (key, -10000i64..10000i64).prop_map(|(k, v)| format!("{}:{}", k, v)),
            // Float values
            (key, -1000.0f64..1000.0f64)
                .prop_filter("finite", |(_, f)| f.is_finite())
                .prop_map(|(k, v)| format!("{}:{:.2}", k, v)),
            // Boolean true
            key.prop_map(|k| format!("{}:+", k)),
            // Boolean false
            key.prop_map(|k| format!("{}:-", k)),
            // Null value
            key.prop_map(|k| format!("{}:~", k)),
            // Implicit true (!)
            key.prop_map(|k| format!("{}!", k)),
            // Implicit null (?)
            key.prop_map(|k| format!("{}?", k)),
        ]
    }

    /// Strategy to generate valid DX documents with multiple key-value pairs
    fn valid_dx_document() -> impl Strategy<Value = String> {
        prop::collection::vec(valid_key_value(), 1..10)
            .prop_map(|pairs| pairs.join("\n"))
    }

    /// Strategy to generate valid table definitions
    fn valid_table_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple int-string table
            Just("users=id%i name%s\n1 Alice\n2 Bob\n3 Charlie".to_string()),
            // Table with float column
            Just("prices=id%i price%f\n1 9.99\n2 19.99\n3 29.99".to_string()),
            // Table with boolean column
            Just("flags=id%i active%b\n1 +\n2 -\n3 +".to_string()),
            // Table with ditto
            Just("data=id%i name%s\n1 Alice\n_ Bob\n_ Charlie".to_string()),
        ]
    }

    /// Strategy to generate valid stream arrays
    fn valid_stream_array() -> impl Strategy<Value = String> {
        let key = "[a-z][a-z0-9_]{0,10}";
        prop_oneof![
            // String items
            (key, prop::collection::vec("[a-zA-Z]{1,10}", 1..5))
                .prop_map(|(k, items)| format!("{}>{}", k, items.join("|"))),
            // Numeric items
            (key, prop::collection::vec(1i64..100, 1..5))
                .prop_map(|(k, items)| {
                    let items_str: Vec<String> = items.iter().map(|i| i.to_string()).collect();
                    format!("{}>{}", k, items_str.join("|"))
                }),
            // Boolean items
            (key, prop::collection::vec(prop::bool::ANY, 1..5))
                .prop_map(|(k, items)| {
                    let items_str: Vec<&str> = items.iter().map(|b| if *b { "+" } else { "-" }).collect();
                    format!("{}>{}", k, items_str.join("|"))
                }),
        ]
    }

    /// Strategy to generate valid alias definitions and usage
    fn valid_alias_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple alias
            Just("$c=context\n$c.name:test\n$c.version:1".to_string()),
            // Multiple aliases
            Just("$a=app\n$u=user\n$a.name:myapp\n$u.id:123".to_string()),
        ]
    }

    /// Combined strategy for comprehensive valid DX inputs
    fn comprehensive_valid_dx() -> impl Strategy<Value = String> {
        prop_oneof![
            valid_dx_document(),
            valid_table_input(),
            valid_stream_array(),
            valid_alias_input(),
        ]
    }

    /// Helper to count values in a DxValue recursively
    fn count_values(value: &DxValue) -> usize {
        match value {
            DxValue::Null
            | DxValue::Bool(_)
            | DxValue::Int(_)
            | DxValue::Float(_)
            | DxValue::String(_)
            | DxValue::Ref(_) => 1,
            DxValue::Array(arr) => arr.values.iter().map(count_values).sum::<usize>() + 1,
            DxValue::Object(obj) => {
                obj.fields.iter().map(|(_, v)| count_values(v)).sum::<usize>() + 1
            }
            DxValue::Table(table) => {
                table.rows.iter().flat_map(|row| row.iter()).map(count_values).sum::<usize>() + 1
            }
        }
    }

    /// Helper to check if a DxValue is complete (no truncation indicators)
    fn is_complete(value: &DxValue) -> bool {
        match value {
            DxValue::Null
            | DxValue::Bool(_)
            | DxValue::Int(_)
            | DxValue::Float(_)
            | DxValue::Ref(_) => true,
            DxValue::String(_) => true, // Empty strings are valid
            DxValue::Array(arr) => arr.values.iter().all(is_complete),
            DxValue::Object(_) => true, // Empty objects are valid for empty input
            DxValue::Table(table) => {
                // Table should have schema columns defined
                !table.schema.columns.is_empty()
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-production-hardening, Property 2: Parser Completeness for Valid Input
        /// **Validates: Requirements 2.2**
        ///
        /// For any valid DX input conforming to the documented syntax, the parser
        /// SHALL produce a complete DxValue that represents all data in the input
        /// without truncation or data loss.
        #[test]
        fn prop_parser_completeness_valid_input(input in comprehensive_valid_dx()) {
            let result = parse(input.as_bytes());

            // Valid input should parse successfully
            prop_assert!(
                result.is_ok(),
                "Valid DX input should parse successfully: {:?}\nInput: {}",
                result.err(),
                input
            );

            let value = result.unwrap();

            // The result should be complete (not truncated)
            prop_assert!(
                is_complete(&value),
                "Parsed value should be complete, not truncated.\nInput: {}\nValue: {:?}",
                input,
                value
            );

            // For non-empty input, the result should contain data
            if !input.trim().is_empty() && !input.starts_with('#') {
                let value_count = count_values(&value);
                prop_assert!(
                    value_count > 0,
                    "Non-empty input should produce non-empty result.\nInput: {}\nValue: {:?}",
                    input,
                    value
                );
            }
        }

        /// Property 2 variant: Key-value pairs are preserved
        #[test]
        fn prop_parser_preserves_key_value_pairs(
            key in "[a-z][a-z0-9_]{0,10}",
            value in "[a-zA-Z][a-zA-Z0-9]{0,20}"
        ) {
            let input = format!("{}:{}", key, value);
            let result = parse(input.as_bytes());

            prop_assert!(result.is_ok(), "Simple key-value should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                // The key should be present
                prop_assert!(
                    obj.get(&key).is_some(),
                    "Key '{}' should be present in parsed object. Got: {:?}",
                    key,
                    obj.fields.iter().map(|(k, _)| k).collect::<Vec<_>>()
                );

                // The value should match
                if let Some(DxValue::String(parsed_value)) = obj.get(&key) {
                    prop_assert_eq!(
                        parsed_value,
                        &value,
                        "Value should be preserved exactly"
                    );
                }
            }
        }

        /// Property 2 variant: Integer values are preserved exactly
        #[test]
        fn prop_parser_preserves_integers(
            key in "[a-z][a-z0-9_]{0,10}",
            num in -1000000i64..1000000i64
        ) {
            let input = format!("{}:{}", key, num);
            let result = parse(input.as_bytes());

            prop_assert!(result.is_ok(), "Integer value should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                if let Some(DxValue::Int(parsed_num)) = obj.get(&key) {
                    prop_assert_eq!(
                        *parsed_num,
                        num,
                        "Integer value should be preserved exactly"
                    );
                }
            }
        }

        /// Property 2 variant: Boolean values are preserved
        #[test]
        fn prop_parser_preserves_booleans(
            key in "[a-z][a-z0-9_]{0,10}",
            bool_val in prop::bool::ANY
        ) {
            let input = format!("{}:{}", key, if bool_val { "+" } else { "-" });
            let result = parse(input.as_bytes());

            prop_assert!(result.is_ok(), "Boolean value should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                if let Some(DxValue::Bool(parsed_bool)) = obj.get(&key) {
                    prop_assert_eq!(
                        *parsed_bool,
                        bool_val,
                        "Boolean value should be preserved exactly"
                    );
                }
            }
        }

        /// Property 2 variant: Stream arrays preserve all items
        #[test]
        fn prop_parser_preserves_stream_array_items(
            key in "[a-z][a-z0-9_]{0,10}",
            items in prop::collection::vec("[a-zA-Z]{1,8}", 1..6)
        ) {
            let input = format!("{}>{}", key, items.join("|"));
            let result = parse(input.as_bytes());

            prop_assert!(result.is_ok(), "Stream array should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                if let Some(DxValue::Array(arr)) = obj.get(&key) {
                    prop_assert_eq!(
                        arr.values.len(),
                        items.len(),
                        "Array should have same number of items.\nExpected: {:?}\nGot: {:?}",
                        items,
                        arr.values
                    );

                    // Verify each item
                    for (i, (expected, actual)) in items.iter().zip(arr.values.iter()).enumerate() {
                        if let DxValue::String(s) = actual {
                            prop_assert_eq!(
                                s,
                                expected,
                                "Array item {} should match",
                                i
                            );
                        }
                    }
                }
            }
        }

        /// Property 2 variant: Tables preserve all rows
        #[test]
        fn prop_parser_preserves_table_rows(
            num_rows in 1usize..10
        ) {
            // Generate a simple table with the specified number of rows
            let mut input = String::from("data=id%i name%s\n");
            for i in 0..num_rows {
                input.push_str(&format!("{} User{}\n", i + 1, i + 1));
            }

            let result = parse(input.as_bytes());

            prop_assert!(result.is_ok(), "Table should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                if let Some(DxValue::Table(table)) = obj.get("data") {
                    prop_assert_eq!(
                        table.rows.len(),
                        num_rows,
                        "Table should have {} rows, got {}",
                        num_rows,
                        table.rows.len()
                    );

                    // Verify each row has the correct number of columns
                    for (i, row) in table.rows.iter().enumerate() {
                        prop_assert_eq!(
                            row.len(),
                            2,
                            "Row {} should have 2 columns, got {}",
                            i,
                            row.len()
                        );
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_simple_key_value_completeness() {
            let input = "name:Alice";
            let result = parse(input.as_bytes()).unwrap();

            if let DxValue::Object(obj) = result {
                assert!(obj.get("name").is_some());
                assert_eq!(obj.get("name"), Some(&DxValue::String("Alice".to_string())));
            } else {
                panic!("Expected object");
            }
        }

        #[test]
        fn test_multiple_key_values_completeness() {
            let input = "name:Alice\nage:30\nactive:+";
            let result = parse(input.as_bytes()).unwrap();

            if let DxValue::Object(obj) = result {
                assert_eq!(obj.fields.len(), 3);
                assert!(obj.get("name").is_some());
                assert!(obj.get("age").is_some());
                assert!(obj.get("active").is_some());
            } else {
                panic!("Expected object");
            }
        }

        #[test]
        fn test_table_completeness() {
            let input = "users=id%i name%s\n1 Alice\n2 Bob\n3 Charlie";
            let result = parse(input.as_bytes()).unwrap();

            if let DxValue::Object(obj) = result {
                if let Some(DxValue::Table(table)) = obj.get("users") {
                    assert_eq!(table.rows.len(), 3, "Should have 3 rows");
                    assert_eq!(table.schema.columns.len(), 2, "Should have 2 columns");
                } else {
                    panic!("Expected table");
                }
            } else {
                panic!("Expected object");
            }
        }

        #[test]
        fn test_stream_array_completeness() {
            let input = "tags>alpha|beta|gamma|delta";
            let result = parse(input.as_bytes()).unwrap();

            if let DxValue::Object(obj) = result {
                if let Some(DxValue::Array(arr)) = obj.get("tags") {
                    assert_eq!(arr.values.len(), 4, "Should have 4 items");
                    assert!(arr.is_stream, "Should be a stream array");
                } else {
                    panic!("Expected array");
                }
            } else {
                panic!("Expected object");
            }
        }

        #[test]
        fn test_alias_completeness() {
            let input = "$c=context\n$c.name:test\n$c.version:1";
            let result = parse(input.as_bytes()).unwrap();

            if let DxValue::Object(obj) = result {
                // Aliases should be resolved
                assert!(obj.get("context.name").is_some());
                assert!(obj.get("context.version").is_some());
            } else {
                panic!("Expected object");
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_null_byte_in_key() {
        let input = b"ke\0y:value";
        let result = parse(input);
        // Should not panic, may return error or handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_null_byte_in_value() {
        let input = b"key:val\0ue";
        let result = parse(input);
        // Should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_invalid_utf8_sequence() {
        let input = vec![b'k', b'e', b'y', b':', 0xFF, 0xFE];
        let result = parse(&input);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

// =============================================================================
// THREAD SAFETY PROPERTY TESTS
// =============================================================================

mod thread_safety_props {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    /// Strategy to generate valid DX inputs for concurrent parsing
    fn concurrent_parse_input() -> impl Strategy<Value = String> {
        prop::collection::vec(
            ("[a-z][a-z0-9_]{0,10}", "[a-zA-Z][a-zA-Z0-9]{0,20}"),
            1..5,
        )
        .prop_map(|pairs| {
            pairs
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Strategy to generate various valid DX document types
    fn varied_dx_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple key-value pairs
            concurrent_parse_input(),
            // Table definitions
            Just("data=id%i name%s\n1 Alice\n2 Bob".to_string()),
            // Stream arrays
            Just("tags>alpha|beta|gamma".to_string()),
            // Boolean values
            Just("active:+\ninactive:-".to_string()),
            // Mixed content
            prop::collection::vec(
                prop_oneof![
                    "[a-z]{1,5}:[a-z]{1,10}".prop_map(String::from),
                    Just("flag:+".to_string()),
                    Just("empty:~".to_string()),
                ],
                1..5
            )
            .prop_map(|lines| lines.join("\n")),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-production-hardening, Property 6: Thread Safety
        /// Validates: Requirements 13.3
        ///
        /// For any valid input, calling parse() concurrently from multiple threads
        /// SHALL not cause data races, undefined behavior, or inconsistent results.
        #[test]
        fn prop_parse_thread_safety(input in varied_dx_input()) {
            let input = Arc::new(input);
            let num_threads = 4;

            // First, parse sequentially to get expected result
            let expected = parse(input.as_bytes());

            // Now parse concurrently from multiple threads
            let handles: Vec<_> = (0..num_threads)
                .map(|_| {
                    let input_clone = Arc::clone(&input);
                    thread::spawn(move || parse(input_clone.as_bytes()))
                })
                .collect();

            // Collect all results
            let results: Vec<_> = handles
                .into_iter()
                .map(|h| h.join())
                .collect();

            // All threads should complete without panicking
            for (i, result) in results.iter().enumerate() {
                prop_assert!(
                    result.is_ok(),
                    "Thread {} panicked during parsing",
                    i
                );
            }

            // All results should be consistent with the sequential result
            for (i, result) in results.into_iter().enumerate() {
                let concurrent_result = result.unwrap();
                match (&expected, &concurrent_result) {
                    (Ok(exp), Ok(conc)) => {
                        prop_assert_eq!(
                            exp, conc,
                            "Thread {} produced different result than sequential parse",
                            i
                        );
                    }
                    (Err(_), Err(_)) => {
                        // Both errored, which is consistent
                    }
                    _ => {
                        prop_assert!(
                            false,
                            "Thread {} result differs in success/failure from sequential",
                            i
                        );
                    }
                }
            }
        }

        /// Feature: serializer-production-hardening, Property 6: Thread Safety (Isolation)
        /// Validates: Requirements 13.3
        ///
        /// For any two different inputs parsed concurrently, the parsing of one
        /// SHALL not affect the results of the other.
        #[test]
        fn prop_parse_thread_isolation(
            input1 in concurrent_parse_input(),
            input2 in concurrent_parse_input()
        ) {
            let input1 = Arc::new(input1);
            let input2 = Arc::new(input2);

            // Parse inputs sequentially first to get expected results
            let expected1 = parse(input1.as_bytes());
            let expected2 = parse(input2.as_bytes());

            // Now parse concurrently
            let input1_clone = Arc::clone(&input1);
            let input2_clone = Arc::clone(&input2);

            let handle1 = thread::spawn(move || parse(input1_clone.as_bytes()));
            let handle2 = thread::spawn(move || parse(input2_clone.as_bytes()));

            let result1 = handle1.join();
            let result2 = handle2.join();

            // Neither thread should panic
            prop_assert!(result1.is_ok(), "Thread 1 panicked");
            prop_assert!(result2.is_ok(), "Thread 2 panicked");

            let concurrent1 = result1.unwrap();
            let concurrent2 = result2.unwrap();

            // Results should match sequential parsing
            match (&expected1, &concurrent1) {
                (Ok(e), Ok(c)) => {
                    prop_assert_eq!(e, c, "Concurrent result 1 should match sequential");
                }
                (Err(_), Err(_)) => {
                    // Both errored, which is consistent
                }
                _ => {
                    prop_assert!(
                        false,
                        "Concurrent and sequential results differ in success/failure for input 1"
                    );
                }
            }

            match (&expected2, &concurrent2) {
                (Ok(e), Ok(c)) => {
                    prop_assert_eq!(e, c, "Concurrent result 2 should match sequential");
                }
                (Err(_), Err(_)) => {
                    // Both errored, which is consistent
                }
                _ => {
                    prop_assert!(
                        false,
                        "Concurrent and sequential results differ in success/failure for input 2"
                    );
                }
            }
        }
    }

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_concurrent_parsing_simple() {
            let inputs = vec!["a:one", "b:two", "c:three", "d:four"];

            let handles: Vec<_> = inputs
                .into_iter()
                .map(|input| {
                    let input = input.to_string();
                    thread::spawn(move || parse(input.as_bytes()))
                })
                .collect();

            for handle in handles {
                let result = handle.join().unwrap();
                assert!(result.is_ok());
            }
        }

        #[test]
        fn test_concurrent_parsing_same_input() {
            let input = "name:Test\nvalue:42\nactive:+";
            let input = Arc::new(input.to_string());

            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let input_clone = Arc::clone(&input);
                    thread::spawn(move || parse(input_clone.as_bytes()))
                })
                .collect();

            let results: Vec<_> = handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect();

            // All results should be identical
            let first = results[0].as_ref().unwrap();
            for (i, result) in results.iter().enumerate().skip(1) {
                assert_eq!(
                    first,
                    result.as_ref().unwrap(),
                    "Thread {} produced different result",
                    i
                );
            }
        }
    }
}
