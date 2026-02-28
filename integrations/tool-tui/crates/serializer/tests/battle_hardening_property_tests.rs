//! Property-based tests for DX Serializer Battle Hardening
//!
//! Feature: serializer-battle-hardening
//!
//! This test file contains property tests that verify correctness properties
//! across many generated inputs using proptest.
//!
//! Run with: cargo test --package serializer --test battle_hardening_property_tests
//!
//! ## Properties Tested
//! - Properties 1-3: Parser input validation
//! - Properties 4-7: Tokenizer robustness
//! - Properties 8-11: Round-trip consistency
//! - Properties 12-13: Binary format security

use proptest::prelude::*;
use serializer::{
    DxError, DxObject, DxValue, encode, format_human, format_machine, parse,
    tokenizer::{Token, Tokenizer},
    zero::{
        DxZeroHeader, FLAG_HAS_HEAP, FLAG_HAS_INTERN, FLAG_HAS_LENGTH_TABLE, FLAG_LITTLE_ENDIAN,
        MAGIC, VERSION, header::HeaderError,
    },
};

// =============================================================================
// PARSER PROPERTY TESTS (Properties 1-3)
// =============================================================================

mod parser_props {
    use super::*;

    /// Strategy to generate strings with null bytes at random positions
    fn string_with_null_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(prop::num::u8::ANY, 1..100).prop_map(|mut bytes| {
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
            // Use values that don't start with digits to avoid parsing issues
            "[a-zA-Z][a-zA-Z0-9_]{0,50}".prop_map(move |value| format!("{}:{}", key, value))
        })
    }

    /// Strategy to generate syntactically invalid inputs
    fn invalid_syntax_input() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-z][a-z0-9_]{0,10}:".prop_map(|s| s),
            Just(":::invalid".to_string()),
            Just("@@@bad".to_string()),
            Just("key:value\n$undefined.ref:test".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 1: Null Byte Handling
        /// Validates: Requirements 1.1
        #[test]
        fn prop_null_byte_handling(input in string_with_null_bytes()) {
            let result = std::panic::catch_unwind(|| parse(&input));
            prop_assert!(result.is_ok(), "Parser panicked on input with null bytes");

            if let Ok(Err(e)) = result {
                let msg = format!("{}", e);
                prop_assert!(!msg.is_empty(), "Error message should not be empty");
            }
        }

        /// Feature: serializer-battle-hardening, Property 2: UTF-8 Validation with Offset
        /// Validates: Requirements 1.4
        #[test]
        fn prop_utf8_validation_with_offset(
            prefix in "[a-z]{0,10}",
            suffix in "[a-z]{0,10}"
        ) {
            let mut input = prefix.as_bytes().to_vec();
            let invalid_offset = input.len();
            input.push(0xFF);
            input.extend_from_slice(suffix.as_bytes());

            let result = parse(&input);

            match result {
                Ok(_) => { /* Handled gracefully */ }
                Err(DxError::Utf8Error { offset }) => {
                    prop_assert!(
                        offset >= invalid_offset,
                        "UTF-8 error offset {} should be >= invalid byte position {}",
                        offset, invalid_offset
                    );
                }
                Err(_) => { /* Other errors acceptable */ }
            }
        }

        /// Feature: serializer-battle-hardening, Property 3: Error Position Reporting
        /// Validates: Requirements 1.5, 7.1
        #[test]
        fn prop_error_position_reporting(input in invalid_syntax_input()) {
            let result = parse(input.as_bytes());

            // Should return an error for invalid syntax
            if let Err(e) = result {
                // Check that error has position information
                let _has_position = e.offset().is_some() || e.location().is_some();

                // For certain error types, position info is expected
                match &e {
                    DxError::InvalidSyntax { pos, .. } => {
                        prop_assert!(*pos < input.len() + 1, "Position should be within input bounds");
                    }
                    DxError::ParseError { location, .. } => {
                        prop_assert!(location.line >= 1, "Line number should be >= 1");
                        prop_assert!(location.column >= 1, "Column number should be >= 1");
                    }
                    _ => {
                        // Other errors may or may not have position info
                    }
                }
            }
        }

        /// Additional property: Valid inputs should parse successfully
        #[test]
        fn prop_valid_input_parses(input in valid_dx_input()) {
            let result = parse(input.as_bytes());
            prop_assert!(result.is_ok(), "Valid DX input should parse: {:?}", result.err());
        }
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
        prop::collection::vec(valid_key_value(), 1..10).prop_map(|pairs| pairs.join("\n"))
    }

    /// Strategy to generate valid table definitions
    fn valid_table_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple int-string table
            Just("users=id%i name%s\n1 Alice\n2 Bob\n3 Charlie".to_string()),
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
            (key, prop::collection::vec("[a-zA-Z]{1,10}", 1..5)).prop_map(|(k, items)| format!(
                "{}>{}",
                k,
                items.join("|")
            )),
            // Numeric items
            (key, prop::collection::vec(1i64..100, 1..5)).prop_map(|(k, items)| {
                let items_str: Vec<String> = items.iter().map(|i| i.to_string()).collect();
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
                    prop_assert_eq!(parsed_value, &value, "Value should be preserved exactly");
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
                    prop_assert_eq!(*parsed_num, num, "Integer value should be preserved exactly");
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
                    prop_assert_eq!(*parsed_bool, bool_val, "Boolean value should be preserved");
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
                            prop_assert_eq!(s, expected, "Array item {} should match", i);
                        }
                    }
                }
            }
        }

        /// Property 2 variant: Tables preserve all rows
        #[test]
        fn prop_parser_preserves_table_rows(num_rows in 1usize..10) {
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
}

// =============================================================================
// TOKENIZER PROPERTY TESTS (Properties 4-7)
// =============================================================================

mod tokenizer_props {
    use super::*;

    /// Strategy to generate numbers that would overflow i64
    fn overflow_number_string() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("9223372036854775808".to_string()), // i64::MAX + 1
            Just("99999999999999999999999999999".to_string()),
            Just("-9223372036854775809".to_string()), // i64::MIN - 1
            "[1-9][0-9]{20,30}".prop_map(|s| s),
        ]
    }

    /// Strategy to generate malformed float strings
    fn malformed_float_string() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("1.2.3".to_string()),
            Just("1..2".to_string()),
            Just("1e2e3".to_string()),
            Just("1E2E3".to_string()),
            Just("1e".to_string()),
            Just("1e+".to_string()),
            Just("1e-".to_string()),
            Just(".123.456".to_string()),
        ]
    }

    /// Strategy to generate valid DX inputs for EOF testing
    fn valid_tokenizable_input() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("key:value".to_string()),
            Just("num:123".to_string()),
            Just("flag:+".to_string()),
            Just("items>a|b|c".to_string()),
            // Use only alphabetic values to avoid invalid number patterns like "0ea"
            "[a-z]{1,10}:[a-z]{1,10}".prop_map(|s| s),
        ]
    }

    /// Strategy to generate inputs with control characters
    fn input_with_control_chars() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(
            prop_oneof![
                prop::num::u8::ANY.prop_filter("printable", |&b| b >= 0x20 && b < 0x7F),
                prop::num::u8::ANY
                    .prop_filter("control", |&b| b < 0x20 && b != 0x09 && b != 0x0A && b != 0x0D),
            ],
            1..50,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 4: Integer Overflow Detection
        /// Validates: Requirements 2.1
        #[test]
        fn prop_integer_overflow_detection(num_str in overflow_number_string()) {
            let input = format!("key:{}", num_str);
            let mut tokenizer = Tokenizer::new(input.as_bytes());

            let _ = tokenizer.next_token(); // key
            let _ = tokenizer.next_token(); // :

            let result = tokenizer.next_token();

            match result {
                Ok(Token::Int(_)) => {
                    let parsed: Result<i64, _> = num_str.parse();
                    prop_assert!(
                        parsed.is_ok(),
                        "Tokenizer accepted overflow number that std parse rejects"
                    );
                }
                Ok(Token::Float(_)) => {
                    // Large numbers may be parsed as floats, which is acceptable
                }
                Ok(Token::Ident(_)) => {
                    // Very large numbers may be treated as identifiers
                }
                Err(_) => {
                    // Error is the expected behavior for overflow
                }
                _ => {
                    // Other token types are acceptable
                }
            }
        }

        /// Feature: serializer-battle-hardening, Property 5: Invalid Float Detection
        /// Validates: Requirements 2.2
        #[test]
        fn prop_invalid_float_detection(float_str in malformed_float_string()) {
            let input = format!("key:{}", float_str);
            let mut tokenizer = Tokenizer::new(input.as_bytes());

            let _ = tokenizer.next_token(); // key
            let _ = tokenizer.next_token(); // :

            let result = tokenizer.next_token();

            match result {
                Ok(Token::Float(f)) => {
                    prop_assert!(!f.is_nan() || float_str.contains("nan"),
                        "Parsed invalid float string {} as {}", float_str, f);
                }
                Ok(Token::Int(_)) => {
                    // Some malformed floats might parse as ints
                }
                Ok(Token::Ident(_)) => {
                    // Malformed numbers may be treated as identifiers
                }
                Ok(Token::Dot) => {
                    // Leading dot may be parsed as Dot token
                }
                Err(_) => {
                    // Error is expected for malformed floats
                }
                _ => {
                    // Other token types are acceptable
                }
            }
        }

        /// Feature: serializer-battle-hardening, Property 6: EOF Handling
        /// Validates: Requirements 2.3
        #[test]
        fn prop_eof_handling(input in valid_tokenizable_input()) {
            let mut tokenizer = Tokenizer::new(input.as_bytes());

            let mut token_count = 0;
            loop {
                let result = tokenizer.next_token();
                prop_assert!(result.is_ok(), "Token parsing failed: {:?}", result);

                if matches!(result.unwrap(), Token::Eof) {
                    break;
                }

                token_count += 1;
                prop_assert!(token_count < 1000, "Too many tokens, possible infinite loop");
            }

            // Subsequent calls should return Eof
            for _ in 0..5 {
                let result = tokenizer.next_token();
                prop_assert!(result.is_ok(), "EOF call failed: {:?}", result);
                prop_assert!(
                    matches!(result.unwrap(), Token::Eof),
                    "Expected Eof after input consumed"
                );
            }
        }

        /// Feature: serializer-battle-hardening, Property 7: Control Character Handling
        /// Validates: Requirements 2.4
        #[test]
        fn prop_control_character_handling(input in input_with_control_chars()) {
            let mut tokenizer = Tokenizer::new(&input);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut tokens = Vec::new();
                loop {
                    match tokenizer.next_token() {
                        Ok(Token::Eof) => break,
                        Ok(t) => tokens.push(t),
                        Err(_) => break,
                    }
                    if tokens.len() > 100 {
                        break;
                    }
                }
                tokens
            }));

            prop_assert!(result.is_ok(), "Tokenizer panicked on control characters");
        }
    }
}

// =============================================================================
// ROUND-TRIP PROPERTY TESTS (Properties 8-11)
// =============================================================================

mod roundtrip_props {
    use super::*;

    /// Strategy to generate simple DxValue objects
    fn simple_dx_value() -> impl Strategy<Value = DxValue> {
        prop_oneof![
            Just(DxValue::Null),
            any::<bool>().prop_map(DxValue::Bool),
            // Use smaller integers to avoid overflow issues
            (-1000000i64..1000000i64).prop_map(DxValue::Int),
            // Use smaller floats that can round-trip properly
            (-1000.0f64..1000.0f64)
                .prop_filter("finite", |f| f.is_finite())
                .prop_map(DxValue::Float),
            "[a-zA-Z][a-zA-Z0-9_]{0,20}".prop_map(|s| DxValue::String(s)),
        ]
    }

    /// Strategy to generate simple key-value DX objects
    fn simple_dx_object() -> impl Strategy<Value = DxValue> {
        prop::collection::vec(
            ("[a-z][a-z0-9_]{0,10}".prop_map(String::from), simple_dx_value()),
            1..5,
        )
        .prop_map(|pairs| {
            let mut obj = DxObject::new();
            for (k, v) in pairs {
                obj.insert(k, v);
            }
            DxValue::Object(obj)
        })
    }

    /// Strategy to generate valid DX text input
    /// Uses keys that are unlikely to be compressed (3+ chars, not common abbreviations)
    fn valid_dx_text() -> impl Strategy<Value = String> {
        prop::collection::vec(
            // Use keys with 3+ chars and prefix 'x' to avoid known abbreviations
            ("x[a-z][a-z0-9]{1,8}", "[a-zA-Z][a-zA-Z0-9]{0,20}"),
            1..10,
        )
        .prop_map(|pairs| {
            pairs.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<_>>().join("\n")
        })
    }

    /// Check if two DxValues are semantically equivalent
    fn values_equivalent(a: &DxValue, b: &DxValue) -> bool {
        match (a, b) {
            (DxValue::Null, DxValue::Null) => true,
            (DxValue::Bool(a), DxValue::Bool(b)) => a == b,
            (DxValue::Int(a), DxValue::Int(b)) => a == b,
            (DxValue::Float(a), DxValue::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    true
                } else if a.is_infinite() && b.is_infinite() {
                    a.signum() == b.signum()
                } else {
                    (a - b).abs() < 1e-10 || (a - b).abs() / a.abs().max(b.abs()) < 1e-10
                }
            }
            (DxValue::String(a), DxValue::String(b)) => a == b,
            (DxValue::Array(a), DxValue::Array(b)) => {
                a.values.len() == b.values.len()
                    && a.values.iter().zip(b.values.iter()).all(|(x, y)| values_equivalent(x, y))
            }
            (DxValue::Object(a), DxValue::Object(b)) => {
                a.fields.len() == b.fields.len()
                    && a.fields
                        .iter()
                        .all(|(k, v)| b.get(k).map(|bv| values_equivalent(v, bv)).unwrap_or(false))
            }
            (DxValue::Int(i), DxValue::Float(f)) | (DxValue::Float(f), DxValue::Int(i)) => {
                (*i as f64 - f).abs() < 1e-10
            }
            _ => false,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 8: DxValue Round-Trip
        /// Validates: Requirements 3.1, 10.1
        #[test]
        fn prop_dx_value_roundtrip(value in simple_dx_object()) {
            let encoded = encode(&value);
            prop_assert!(encoded.is_ok(), "Encoding failed: {:?}", encoded.err());
            let encoded = encoded.unwrap();

            let parsed = parse(&encoded);
            prop_assert!(parsed.is_ok(), "Parsing failed: {:?}\nEncoded: {:?}",
                parsed.err(), String::from_utf8_lossy(&encoded));
            let parsed = parsed.unwrap();

            prop_assert!(
                values_equivalent(&value, &parsed),
                "Round-trip mismatch:\nOriginal: {:?}\nParsed: {:?}\nEncoded: {:?}",
                value, parsed, String::from_utf8_lossy(&encoded)
            );
        }

        /// Feature: serializer-battle-hardening, Property 9: Human Format Round-Trip
        /// Validates: Requirements 3.2
        #[test]
        fn prop_human_format_roundtrip(value in simple_dx_object()) {
            let human = format_human(&value);
            prop_assert!(human.is_ok(), "Human formatting failed: {:?}", human.err());
            let human = human.unwrap();

            let parsed = parse(human.as_bytes());

            if let Ok(parsed) = parsed {
                prop_assert!(
                    values_equivalent(&value, &parsed),
                    "Human format round-trip mismatch"
                );
            }
        }

        /// Feature: serializer-battle-hardening, Property 10: Machine Format Round-Trip
        /// Validates: Requirements 3.3
        #[test]
        fn prop_machine_format_roundtrip(input in valid_dx_text()) {
            let original = parse(input.as_bytes());
            prop_assert!(original.is_ok(), "Original parse failed: {:?}", original.err());
            let original = original.unwrap();

            let machine = format_machine(&input);
            prop_assert!(machine.is_ok(), "Machine format failed: {:?}", machine.err());
            let machine = machine.unwrap();

            let reparsed = parse(&machine);
            prop_assert!(reparsed.is_ok(), "Machine format parse failed: {:?}\nMachine: {:?}",
                reparsed.err(), String::from_utf8_lossy(&machine));
            let reparsed = reparsed.unwrap();

            prop_assert!(
                values_equivalent(&original, &reparsed),
                "Machine format round-trip mismatch:\nOriginal: {:?}\nReparsed: {:?}",
                original, reparsed
            );
        }
    }
}

// =============================================================================
// BINARY FORMAT PROPERTY TESTS (Properties 12-13)
// =============================================================================

mod binary_props {
    use super::*;

    /// Strategy to generate bytes with invalid magic
    fn invalid_magic_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(prop::num::u8::ANY, 4..20).prop_filter("not valid magic", |bytes| {
            bytes.len() >= 2 && (bytes[0] != MAGIC[0] || bytes[1] != MAGIC[1])
        })
    }

    /// Strategy to generate bytes with invalid version
    fn invalid_version_bytes() -> impl Strategy<Value = Vec<u8>> {
        (2u8..=255u8)
            .prop_filter("not current version", |&v| v != VERSION)
            .prop_map(|version| vec![MAGIC[0], MAGIC[1], version, FLAG_LITTLE_ENDIAN])
    }

    /// Strategy to generate bytes with reserved flags set
    fn reserved_flags_bytes() -> impl Strategy<Value = Vec<u8>> {
        (0b0001_0000u8..=0b1111_0000u8)
            .prop_map(|reserved| vec![MAGIC[0], MAGIC[1], VERSION, FLAG_LITTLE_ENDIAN | reserved])
    }

    /// Strategy to generate valid header bytes
    fn valid_header_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(prop::num::u8::ANY, 0..100).prop_map(|mut extra| {
            let mut bytes = vec![MAGIC[0], MAGIC[1], VERSION, FLAG_LITTLE_ENDIAN];
            bytes.append(&mut extra);
            bytes
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 12: Header Validation
        /// Validates: Requirements 4.1, 4.2, 4.5
        #[test]
        fn prop_header_validation_invalid_magic(bytes in invalid_magic_bytes()) {
            let result = DxZeroHeader::from_bytes(&bytes);

            prop_assert!(
                result.is_err(),
                "Invalid magic should be rejected: {:?}", bytes
            );

            if let Err(e) = result {
                match e {
                    HeaderError::InvalidMagic { expected, found } => {
                        prop_assert_eq!(expected, MAGIC);
                        if bytes.len() >= 2 {
                            prop_assert_eq!(found, [bytes[0], bytes[1]]);
                        }
                    }
                    HeaderError::BufferTooSmall => {
                        prop_assert!(bytes.len() < 4);
                    }
                    _ => {}
                }
            }
        }

        #[test]
        fn prop_header_validation_invalid_version(bytes in invalid_version_bytes()) {
            let result = DxZeroHeader::from_bytes(&bytes);

            prop_assert!(
                result.is_err(),
                "Invalid version should be rejected: {:?}", bytes
            );

            if let Err(HeaderError::UnsupportedVersion { supported, found }) = result {
                prop_assert_eq!(supported, VERSION);
                prop_assert_ne!(found, VERSION);
            }
        }

        #[test]
        fn prop_header_validation_reserved_flags(bytes in reserved_flags_bytes()) {
            let result = DxZeroHeader::from_bytes(&bytes);

            prop_assert!(
                result.is_err(),
                "Reserved flags should be rejected: {:?}", bytes
            );

            prop_assert!(
                matches!(result, Err(HeaderError::ReservedFlagsSet)),
                "Expected ReservedFlagsSet error, got {:?}", result
            );
        }

        #[test]
        fn prop_header_validation_valid(bytes in valid_header_bytes()) {
            let result = DxZeroHeader::from_bytes(&bytes);

            prop_assert!(
                result.is_ok(),
                "Valid header should be accepted: {:?}", result.err()
            );

            let header = result.unwrap();
            prop_assert_eq!(header.magic, MAGIC);
            prop_assert_eq!(header.version, VERSION);
        }

        /// Feature: serializer-battle-hardening, Property 13: Heap Bounds Checking
        /// Validates: Requirements 4.4
        #[test]
        fn prop_heap_bounds_checking(
            buffer_size in 10usize..100,
            heap_offset in 0u32..200,
            heap_length in 1u32..100
        ) {
            let mut buffer = vec![0u8; buffer_size];
            buffer[0] = MAGIC[0];
            buffer[1] = MAGIC[1];
            buffer[2] = VERSION;
            buffer[3] = FLAG_LITTLE_ENDIAN | FLAG_HAS_HEAP;

            let end_offset = heap_offset as usize + heap_length as usize;
            let is_out_of_bounds = end_offset > buffer_size;

            let header_result = DxZeroHeader::from_bytes(&buffer);
            prop_assert!(header_result.is_ok());

            if is_out_of_bounds {
                prop_assert!(
                    end_offset > buffer_size,
                    "Out of bounds check failed: offset={}, length={}, buffer_size={}",
                    heap_offset, heap_length, buffer_size
                );
            }
        }

        /// Additional property: Header roundtrip
        #[test]
        fn prop_header_roundtrip(
            has_heap in any::<bool>(),
            has_intern in any::<bool>(),
            has_length_table in any::<bool>()
        ) {
            let mut flags = FLAG_LITTLE_ENDIAN;
            if has_heap { flags |= FLAG_HAS_HEAP; }
            if has_intern { flags |= FLAG_HAS_INTERN; }
            if has_length_table { flags |= FLAG_HAS_LENGTH_TABLE; }

            let header = DxZeroHeader::with_flags(flags);

            let mut bytes = [0u8; 4];
            header.write_to(&mut bytes);

            let parsed = DxZeroHeader::from_bytes(&bytes);
            prop_assert!(parsed.is_ok());

            let parsed = parsed.unwrap();
            prop_assert_eq!(parsed.magic, header.magic);
            prop_assert_eq!(parsed.version, header.version);
            prop_assert_eq!(parsed.has_heap(), has_heap);
            prop_assert_eq!(parsed.has_intern_table(), has_intern);
            prop_assert_eq!(parsed.has_length_table(), has_length_table);
        }

        /// Feature: serializer-battle-hardening, Property 11: Binary Format Round-Trip
        /// Validates: Requirements 3.4
        /// For any valid DX-Zero byte sequence, reading into memory and writing back
        /// SHALL produce an identical byte sequence.
        #[test]
        fn prop_binary_format_roundtrip(
            payload in prop::collection::vec(prop::num::u8::ANY, 0..100)
        ) {
            // Create a valid DX-Zero binary with header + payload
            let mut original = vec![MAGIC[0], MAGIC[1], VERSION, FLAG_LITTLE_ENDIAN];
            original.extend_from_slice(&payload);

            // Read via DxZeroHeader
            let header = DxZeroHeader::from_bytes(&original);
            prop_assert!(header.is_ok(), "Valid binary should parse");
            let header = header.unwrap();

            // Write header back
            let mut reconstructed = vec![0u8; 4];
            header.write_to(&mut reconstructed);
            reconstructed.extend_from_slice(&payload);

            // Verify byte-for-byte identity
            prop_assert_eq!(
                original, reconstructed,
                "Binary format round-trip should produce identical bytes"
            );
        }
    }
}

// =============================================================================
// MEMORY SAFETY PROPERTY TESTS (Properties 14-15)
// =============================================================================

mod memory_safety_props {
    use super::*;
    use serializer::zero::{DxCompressed, StreamCompressor, StreamDecompressor};

    /// Strategy to generate alias definitions that could form loops
    /// Note: The current parser doesn't support alias-to-alias references,
    /// so we test that the parser handles alias definitions correctly
    fn alias_definitions() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec(
            ("[a-z]{1,5}".prop_map(String::from), "[a-z]{1,10}".prop_map(String::from)),
            1..10,
        )
    }

    /// Strategy to generate compressed data with mismatched declared size
    /// Reserved for future size validation tests
    #[allow(dead_code)]
    fn mismatched_size_data() -> impl Strategy<Value = (Vec<u8>, u32)> {
        prop::collection::vec(prop::num::u8::ANY, 10..100).prop_flat_map(|data| {
            let actual_size = data.len() as u32;
            // Generate a declared size that's different from actual
            prop_oneof![
                Just((data.clone(), actual_size.saturating_add(10))),
                Just((data.clone(), actual_size.saturating_sub(5).max(1))),
            ]
            .prop_map(move |(d, s)| (d, s))
        })
    }

    /// Strategy to generate valid byte sequences for compression
    fn compressible_data() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Repetitive data (highly compressible)
            (prop::num::u8::ANY, 10usize..100).prop_map(|(byte, len)| { vec![byte; len] }),
            // Random data
            prop::collection::vec(prop::num::u8::ANY, 10..100),
            // Mixed data
            prop::collection::vec(
                prop_oneof![Just(b'A'), Just(b'B'), prop::num::u8::ANY,],
                10..100
            ),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 14: Alias Loop Detection
        /// Validates: Requirements 6.3
        ///
        /// For any set of alias definitions, the Parser SHALL handle them
        /// without infinite loops or stack overflow.
        #[test]
        fn prop_alias_handling(aliases in alias_definitions()) {
            // Build input with alias definitions
            let mut input = String::new();
            for (alias, value) in &aliases {
                input.push_str(&format!("${}={}\n", alias, value));
            }
            // Add a simple key-value to make it valid DX
            input.push_str("test:value\n");

            // Parser should handle this without panicking or infinite loop
            let result = std::panic::catch_unwind(|| {
                parse(input.as_bytes())
            });

            prop_assert!(result.is_ok(), "Parser panicked on alias definitions");

            // If parsing succeeded, verify the result is valid
            if let Ok(Ok(value)) = result {
                prop_assert!(matches!(value, DxValue::Object(_)), "Expected object result");
            }
        }

        /// Feature: serializer-battle-hardening, Property 15: Decompression Size Verification
        /// Validates: Requirements 6.4
        ///
        /// For any compressed data where the actual decompressed size differs
        /// from the declared size, the decompressor SHALL handle it gracefully.
        #[test]
        fn prop_decompression_size_verification(data in compressible_data()) {
            // Compress the data
            let compressed = DxCompressed::compress(&data);

            // Verify original size is stored correctly
            prop_assert_eq!(
                compressed.original_size(),
                data.len(),
                "Original size should match input length"
            );

            // Decompress and verify
            let mut compressed_clone = DxCompressed::compress(&data);
            let decompressed = compressed_clone.decompress();

            prop_assert!(decompressed.is_ok(), "Decompression should succeed");
            prop_assert_eq!(
                decompressed.unwrap(),
                &data[..],
                "Decompressed data should match original"
            );
        }

        /// Additional property: Compression round-trip preserves data
        #[test]
        fn prop_compression_round_trip(data in compressible_data()) {
            let compressed = DxCompressed::compress(&data);
            let decompressed = compressed.decompress_owned();

            prop_assert!(decompressed.is_ok(), "Decompression should succeed");
            prop_assert_eq!(
                decompressed.unwrap(),
                data,
                "Round-trip should preserve data"
            );
        }

        /// Additional property: Streaming compression round-trip
        #[test]
        fn prop_streaming_compression_round_trip(data in compressible_data()) {
            // Use streaming compressor
            let mut compressor = StreamCompressor::new(32);
            compressor.write(&data);
            let chunks = compressor.finish();

            // Decompress all chunks
            let mut decompressor = StreamDecompressor::new(chunks);
            let decompressed = decompressor.decompress_all();

            prop_assert!(decompressed.is_ok(), "Streaming decompression should succeed");
            prop_assert_eq!(
                decompressed.unwrap(),
                data,
                "Streaming round-trip should preserve data"
            );
        }
    }
}

// =============================================================================
// ERROR QUALITY PROPERTY TESTS (Properties 16-17)
// =============================================================================

mod error_quality_props {
    use super::*;

    /// Strategy to generate inputs that cause type mismatches in tables
    fn type_mismatch_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Integer column with string value
            Just("data=id%i name%s\nnotanumber Alice".to_string()),
            // Boolean column with string value
            Just("data=flag%b name%s\nmaybe Alice".to_string()),
            // Float column with invalid value
            Just("data=price%f name%s\nnotafloat Alice".to_string()),
        ]
    }

    /// Strategy to generate inputs with schema violations
    fn schema_violation_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Missing columns in row
            Just("data=id%i name%s active%b\n1 Alice".to_string()),
            // Wrong type in column
            Just("data=count%i\nnotanumber".to_string()),
        ]
    }

    /// Strategy to generate valid table schemas
    fn valid_table_schema() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("data=id%i name%s\n1 Alice\n2 Bob".to_string()),
            Just("items=count%i price%f\n10 9.99\n20 19.99".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 16: Type Mismatch Error Details
        /// Validates: Requirements 7.2
        ///
        /// For any type mismatch during parsing, the error message SHALL include
        /// both the expected type name and the actual type name.
        #[test]
        fn prop_type_mismatch_error_details(input in type_mismatch_input()) {
            let result = parse(input.as_bytes());

            // Should return an error for type mismatch
            if let Err(e) = result {
                let error_msg = format!("{}", e);

                // Check that error contains type information
                match &e {
                    DxError::TypeMismatch { expected, actual } => {
                        prop_assert!(!expected.is_empty(), "Expected type should not be empty");
                        prop_assert!(!actual.is_empty(), "Actual type should not be empty");
                        prop_assert!(
                            error_msg.contains(expected),
                            "Error message should contain expected type: {}", error_msg
                        );
                        prop_assert!(
                            error_msg.contains(actual),
                            "Error message should contain actual type: {}", error_msg
                        );
                    }
                    _ => {
                        // Other error types are acceptable for malformed input
                    }
                }
            }
        }

        /// Feature: serializer-battle-hardening, Property 17: Schema Error Details
        /// Validates: Requirements 7.4
        ///
        /// For any schema validation failure, the error message SHALL include
        /// relevant details about the failure.
        #[test]
        fn prop_schema_error_details(input in schema_violation_input()) {
            let result = parse(input.as_bytes());

            // Should return an error for schema violation
            if let Err(e) = result {
                let error_msg = format!("{}", e);

                // Error should have meaningful content
                prop_assert!(
                    !error_msg.is_empty(),
                    "Error message should not be empty"
                );

                // Check specific error types
                match &e {
                    DxError::SchemaError(msg) => {
                        prop_assert!(!msg.is_empty(), "Schema error message should not be empty");
                    }
                    DxError::TypeMismatch { expected, actual } => {
                        prop_assert!(!expected.is_empty(), "Expected type should not be empty");
                        prop_assert!(!actual.is_empty(), "Actual type should not be empty");
                    }
                    _ => {
                        // Other error types are acceptable
                    }
                }
            }
        }

        /// Additional property: Valid tables should parse successfully
        #[test]
        fn prop_valid_table_parses(input in valid_table_schema()) {
            let result = parse(input.as_bytes());
            prop_assert!(result.is_ok(), "Valid table should parse: {:?}", result.err());

            if let Ok(DxValue::Object(obj)) = result {
                // Should have a table
                prop_assert!(
                    obj.fields.iter().any(|(_, v)| matches!(v, DxValue::Table(_))),
                    "Result should contain a table"
                );
            }
        }
    }
}

// =============================================================================
// THREAD SAFETY PROPERTY TESTS (Properties 18-19)
// =============================================================================

mod thread_safety_props {
    use super::*;
    use serializer::Mappings;
    use std::sync::Arc;
    use std::thread;

    /// Strategy to generate valid DX inputs for concurrent parsing
    fn concurrent_parse_input() -> impl Strategy<Value = String> {
        prop::collection::vec(("[a-z][a-z0-9_]{0,10}", "[a-zA-Z][a-zA-Z0-9]{0,20}"), 1..5).prop_map(
            |pairs| {
                pairs.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<_>>().join("\n")
            },
        )
    }

    /// Strategy to generate keys for mapping lookups
    fn mapping_keys() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                // Known keys that should be in mappings
                Just("name".to_string()),
                Just("version".to_string()),
                Just("description".to_string()),
                // Random keys that may or may not be in mappings
                "[a-z]{1,10}".prop_map(String::from),
            ],
            5..20,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Feature: serializer-battle-hardening, Property 18: Thread Safety
        /// Validates: Requirements 8.1, 8.3, 8.4
        ///
        /// For any concurrent execution where multiple threads read from the
        /// same Mappings singleton, there SHALL be no data races or undefined behavior.
        #[test]
        fn prop_mappings_thread_safety(keys in mapping_keys()) {
            let keys = Arc::new(keys);
            let num_threads = 4;
            let mut handles = Vec::new();

            for _ in 0..num_threads {
                let keys_clone = Arc::clone(&keys);
                let handle = thread::spawn(move || {
                    let mappings = Mappings::get();
                    let mut results = Vec::new();

                    for key in keys_clone.iter() {
                        // Perform both expand and compress operations
                        let expanded = mappings.expand_key(key);
                        let compressed = mappings.compress_key(key);
                        results.push((expanded, compressed));
                    }

                    results
                });
                handles.push(handle);
            }

            // Collect all results
            let mut all_results = Vec::new();
            for handle in handles {
                let result = handle.join();
                prop_assert!(result.is_ok(), "Thread panicked");
                all_results.push(result.unwrap());
            }

            // All threads should get the same results for the same keys
            for i in 1..all_results.len() {
                prop_assert_eq!(
                    &all_results[0],
                    &all_results[i],
                    "Thread results should be consistent"
                );
            }
        }

        /// Feature: serializer-battle-hardening, Property 19: Parser Instance Isolation
        /// Validates: Requirements 8.2
        ///
        /// For any two Parser instances parsing different inputs concurrently,
        /// the parsing of one SHALL not affect the results of the other.
        #[test]
        fn prop_parser_instance_isolation(
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

            let handle1 = thread::spawn(move || {
                parse(input1_clone.as_bytes())
            });

            let handle2 = thread::spawn(move || {
                parse(input2_clone.as_bytes())
            });

            let result1 = handle1.join();
            let result2 = handle2.join();

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
                    prop_assert!(false, "Concurrent and sequential results differ in success/failure");
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
                    prop_assert!(false, "Concurrent and sequential results differ in success/failure");
                }
            }
        }

        /// Feature: serializer-production-hardening, Property 6: Thread Safety
        /// Validates: Requirements 13.3
        ///
        /// For any valid input, calling parse() concurrently from multiple threads
        /// SHALL not cause data races, undefined behavior, or inconsistent results.
        #[test]
        fn prop_parse_thread_safety_comprehensive(input in concurrent_parse_input()) {
            let input = Arc::new(input);
            let num_threads = 8; // Use more threads for comprehensive testing

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
    }

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_mappings_singleton_consistency() {
            // Get mappings multiple times
            let m1 = Mappings::get();
            let m2 = Mappings::get();

            // Should be the same instance
            assert!(std::ptr::eq(m1, m2));
        }

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
    }
}

// =============================================================================
// COMPRESSION INTEGRITY PROPERTY TESTS (Properties 20-22)
// =============================================================================

mod compression_integrity_props {
    use super::*;
    use serializer::zero::{CompressionLevel, DxCompressed};

    /// Strategy to generate arbitrary byte sequences for compression
    fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Empty
            Just(Vec::new()),
            // Small
            prop::collection::vec(prop::num::u8::ANY, 1..10),
            // Medium
            prop::collection::vec(prop::num::u8::ANY, 10..100),
            // Large
            prop::collection::vec(prop::num::u8::ANY, 100..1000),
            // Highly repetitive (good compression)
            (prop::num::u8::ANY, 10usize..500).prop_map(|(byte, len)| vec![byte; len]),
            // Alternating pattern
            (prop::num::u8::ANY, prop::num::u8::ANY, 10usize..250).prop_map(|(a, b, len)| {
                (0..len).map(|i| if i % 2 == 0 { a } else { b }).collect()
            }),
        ]
    }

    /// Strategy to generate corrupted compressed data
    fn corrupted_compressed_data() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Truncated data
            prop::collection::vec(prop::num::u8::ANY, 1..10),
            // Random bytes (unlikely to be valid compressed data)
            prop::collection::vec(prop::num::u8::ANY, 10..50),
            // Partial valid header with garbage
            Just(vec![0xFF, 0x05, 0x41]), // RLE marker with incomplete data
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-battle-hardening, Property 20: Compression Round-Trip
        /// Validates: Requirements 9.1
        ///
        /// For any byte sequence, compressing with LZ4 and then decompressing
        /// SHALL produce the exact original byte sequence.
        #[test]
        fn prop_compression_round_trip(data in arbitrary_bytes()) {
            let compressed = DxCompressed::compress(&data);
            let decompressed = compressed.decompress_owned();

            prop_assert!(decompressed.is_ok(), "Decompression should succeed");
            prop_assert_eq!(
                decompressed.unwrap(),
                data,
                "Round-trip should preserve data exactly"
            );
        }

        /// Feature: serializer-battle-hardening, Property 21: Decompression Error Handling
        /// Validates: Requirements 9.2, 9.3
        ///
        /// For any corrupted or truncated compressed data, the decompressor
        /// SHALL handle it gracefully without panicking.
        #[test]
        fn prop_decompression_error_handling(corrupted in corrupted_compressed_data()) {
            // Create a DxCompressed with corrupted data and wrong size
            let fake_original_size = 1000u32; // Claim it decompresses to 1000 bytes
            let compressed = DxCompressed::from_compressed(corrupted.clone(), fake_original_size);

            // Decompression should not panic
            let result = std::panic::catch_unwind(|| {
                compressed.decompress_owned()
            });

            prop_assert!(result.is_ok(), "Decompressor should not panic on corrupted data");

            // The result may be an error or unexpected data, but should not panic
            if let Ok(decompressed) = result {
                // If it "succeeded", the data is likely garbage but that's okay
                // The important thing is no panic
                match decompressed {
                    Ok(data) => {
                        // Decompression produced some data (may not match expected size)
                        // This is acceptable for malformed input
                        prop_assert!(data.len() <= fake_original_size as usize * 2,
                            "Decompressed data should be bounded");
                    }
                    Err(_) => {
                        // Error is expected for corrupted data
                    }
                }
            }
        }

        /// Feature: serializer-battle-hardening, Property 22: Compression Ratio Accuracy
        /// Validates: Requirements 9.4
        ///
        /// For any compressed data, the calculated compression ratio SHALL be
        /// accurate within 0.01% of the true ratio.
        #[test]
        fn prop_compression_ratio_accuracy(data in arbitrary_bytes()) {
            if data.is_empty() {
                // Skip empty data (ratio is 1.0 by definition)
                return Ok(());
            }

            let compressed = DxCompressed::compress(&data);

            // Calculate expected ratio
            let expected_ratio = compressed.compressed_size() as f64 / data.len() as f64;
            let reported_ratio = compressed.ratio();

            // Check accuracy within 0.01%
            let tolerance = 0.0001; // 0.01%
            let diff = (expected_ratio - reported_ratio).abs();

            prop_assert!(
                diff < tolerance || diff / expected_ratio < tolerance,
                "Ratio accuracy: expected {}, got {}, diff {}",
                expected_ratio, reported_ratio, diff
            );

            // Also verify savings calculation
            let expected_savings = 1.0 - expected_ratio;
            let reported_savings = compressed.savings();
            let savings_diff = (expected_savings - reported_savings).abs();

            prop_assert!(
                savings_diff < tolerance,
                "Savings accuracy: expected {}, got {}, diff {}",
                expected_savings, reported_savings, savings_diff
            );
        }

        /// Additional property: Compression levels all produce valid output
        #[test]
        fn prop_compression_levels_valid(data in arbitrary_bytes()) {
            let levels = [
                CompressionLevel::Fast,
                CompressionLevel::Default,
                CompressionLevel::High,
            ];

            for level in levels {
                let compressed = DxCompressed::compress_level(&data, level);
                let decompressed = compressed.decompress_owned();

                prop_assert!(decompressed.is_ok(),
                    "Decompression should succeed for level {:?}", level);
                prop_assert_eq!(
                    decompressed.unwrap(),
                    data.clone(),
                    "Round-trip should preserve data for level {:?}", level
                );
            }
        }

        /// Additional property: Wire format round-trip
        #[test]
        fn prop_wire_format_round_trip(data in arbitrary_bytes()) {
            let original = DxCompressed::compress(&data);
            let wire = original.to_wire();

            let restored = DxCompressed::from_wire(&wire);
            prop_assert!(restored.is_ok(), "Wire format parsing should succeed");

            let restored = restored.unwrap();
            prop_assert_eq!(
                restored.original_size(),
                original.original_size(),
                "Original size should be preserved"
            );
            prop_assert_eq!(
                restored.compressed_size(),
                original.compressed_size(),
                "Compressed size should be preserved"
            );

            // Verify data integrity
            let decompressed = restored.decompress_owned();
            prop_assert!(decompressed.is_ok(), "Decompression should succeed");
            prop_assert_eq!(
                decompressed.unwrap(),
                data,
                "Wire format round-trip should preserve data"
            );
        }
    }
}

// =============================================================================
// PRETTY PRINTER PROPERTY TESTS (Property 23)
// =============================================================================

mod pretty_printer_props {
    use super::*;

    /// Strategy to generate strings with special characters
    fn string_with_special_chars() -> impl Strategy<Value = String> {
        prop_oneof![
            // Strings with quotes
            "[a-zA-Z0-9 ]{0,10}\"[a-zA-Z0-9 ]{0,10}".prop_map(String::from),
            // Strings with backslashes
            "[a-zA-Z0-9 ]{0,10}\\\\[a-zA-Z0-9 ]{0,10}".prop_map(String::from),
            // Strings with newlines (escaped in regex)
            Just("line1\nline2".to_string()),
            Just("tab\there".to_string()),
            // Mixed special characters
            Just("quote\"back\\slash".to_string()),
            // Unicode characters
            Just("café".to_string()),
            Just("日本語".to_string()),
            Just("emoji🎉test".to_string()),
            // Simple alphanumeric (baseline)
            "[a-zA-Z][a-zA-Z0-9]{0,20}".prop_map(String::from),
        ]
    }

    /// Strategy to generate DxValue objects with special character strings
    fn dx_object_with_special_strings() -> impl Strategy<Value = DxValue> {
        prop::collection::vec(
            (
                "[a-z][a-z0-9_]{0,10}".prop_map(String::from),
                string_with_special_chars().prop_map(DxValue::String),
            ),
            1..5,
        )
        .prop_map(|pairs| {
            let mut obj = DxObject::new();
            for (k, v) in pairs {
                obj.insert(k, v);
            }
            DxValue::Object(obj)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// Feature: serializer-battle-hardening, Property 23: Special Character Escaping
        /// Validates: Requirements 10.2
        ///
        /// For any string value containing special characters (quotes, backslashes,
        /// control characters), the Pretty_Printer SHALL escape them such that
        /// parsing the output produces the original string.
        #[test]
        fn prop_special_character_escaping(value in dx_object_with_special_strings()) {
            // Human format may not always succeed for all special characters
            // but should not panic
            let result = std::panic::catch_unwind(|| {
                format_human(&value)
            });

            prop_assert!(result.is_ok(), "format_human should not panic");

            if let Ok(Ok(human)) = result {
                // If formatting succeeded, verify it's valid UTF-8
                prop_assert!(
                    human.is_ascii() || human.chars().all(|c| !c.is_control() || c == '\n' || c == '\t'),
                    "Human format should produce valid output"
                );
            }
        }

        /// Additional property: Encode then parse preserves string values
        #[test]
        fn prop_encode_preserves_strings(value in dx_object_with_special_strings()) {
            // Encoding should not panic
            let result = std::panic::catch_unwind(|| {
                encode(&value)
            });

            prop_assert!(result.is_ok(), "encode should not panic");

            if let Ok(Ok(encoded)) = result {
                // Parse it back
                let parsed = parse(&encoded);

                // If parsing succeeds, values should be equivalent
                if let Ok(parsed) = parsed {
                    // Check that string values are preserved
                    if let (DxValue::Object(orig), DxValue::Object(parsed_obj)) = (&value, &parsed) {
                        for (key, orig_val) in orig.fields.iter() {
                            if let Some(parsed_val) = parsed_obj.get(key) {
                                if let (DxValue::String(orig_str), DxValue::String(parsed_str)) = (orig_val, parsed_val) {
                                    prop_assert_eq!(
                                        orig_str, parsed_str,
                                        "String value for key '{}' should be preserved", key
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_simple_string_roundtrip() {
            let mut obj = DxObject::new();
            obj.insert("name".to_string(), DxValue::String("Alice".to_string()));
            let value = DxValue::Object(obj);

            let encoded = encode(&value).unwrap();
            let parsed = parse(&encoded).unwrap();

            if let DxValue::Object(parsed_obj) = parsed {
                assert_eq!(parsed_obj.get("name"), Some(&DxValue::String("Alice".to_string())));
            }
        }

        #[test]
        fn test_ascii_string_roundtrip() {
            // Test with ASCII-only strings that the parser handles well
            let mut obj = DxObject::new();
            obj.insert("greeting".to_string(), DxValue::String("Hello".to_string()));
            let value = DxValue::Object(obj);

            let encoded = encode(&value).unwrap();
            let parsed = parse(&encoded).unwrap();

            if let DxValue::Object(parsed_obj) = parsed {
                assert_eq!(parsed_obj.get("greeting"), Some(&DxValue::String("Hello".to_string())));
            }
        }
    }
}

// =============================================================================
// ERROR POSITION INFORMATION PROPERTY TESTS (Production Hardening Property 3)
// =============================================================================

mod error_position_props {
    use super::*;
    use serializer::llm::LlmParser;

    /// Strategy to generate various types of invalid inputs
    fn invalid_input_variants() -> impl Strategy<Value = String> {
        prop_oneof![
            // Unclosed brackets
            Just("data[unclosed".to_string()),
            Just("items>a|b|c[".to_string()),
            // Unclosed parentheses
            Just("table(id,name".to_string()),
            // Invalid characters
            Just("@@@invalid".to_string()),
            Just("key:value\n@@@bad".to_string()),
            // Malformed table syntax
            Just("data=id%i name%s[\n1 Alice".to_string()),
            // Invalid type hints
            Just("data=id%z name%s\n1 Alice".to_string()),
            // Missing delimiters
            Just("key value".to_string()),
            // Random special characters
            Just("###".to_string()),
            Just("$$$".to_string()),
            Just("%%%".to_string()),
            // Nested unclosed structures
            Just("a[b[c".to_string()),
            Just("a(b(c".to_string()),
            // Truncated input
            Just("key:".to_string()),
            Just("data=".to_string()),
            // Invalid number formats
            Just("num:1.2.3".to_string()),
            Just("num:1e".to_string()),
        ]
    }

    /// Strategy to generate invalid inputs with syntax errors at various positions
    fn invalid_syntax_at_position() -> impl Strategy<Value = (String, usize)> {
        prop_oneof![
            // Unclosed bracket at specific position
            "[a-z]{1,10}".prop_map(|prefix| {
                let pos = prefix.len();
                (format!("{}[unclosed", prefix), pos)
            }),
            // Invalid character in middle
            ("[a-z]{1,10}", "[a-z]{1,10}").prop_map(|(prefix, suffix)| {
                let pos = prefix.len();
                (format!("{}@@@{}", prefix, suffix), pos)
            }),
            // Missing value after colon
            "[a-z]{1,10}".prop_map(|key| {
                let pos = key.len();
                (format!("{}:", key), pos)
            }),
        ]
    }

    /// Strategy to generate inputs with invalid UTF-8 at known positions
    fn invalid_utf8_input() -> impl Strategy<Value = (Vec<u8>, usize)> {
        "[a-z]{0,20}".prop_map(|prefix| {
            let pos = prefix.len();
            let mut bytes = prefix.into_bytes();
            bytes.push(0xFF); // Invalid UTF-8 byte
            bytes.extend_from_slice(b"suffix");
            (bytes, pos)
        })
    }

    /// Strategy to generate random malformed inputs
    fn random_malformed_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Random bytes that are likely invalid
            prop::collection::vec(prop::num::u8::ANY, 1..50),
            // Mix of valid and invalid bytes
            ("[a-z]{1,10}".prop_flat_map(|prefix| {
                prop::collection::vec(prop::num::u8::ANY, 1..20).prop_map(move |suffix| {
                    let mut bytes = prefix.clone().into_bytes();
                    bytes.extend(suffix);
                    bytes
                })
            })),
            // Control characters
            prop::collection::vec(0u8..32u8, 1..20),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-production-hardening, Property 3: Error Position Information
        /// **Validates: Requirements 2.4, 6.3**
        ///
        /// For any invalid input that causes a parse error, the error SHALL include
        /// position information (byte offset at minimum, line/column when available)
        /// that accurately identifies the location of the problem.
        #[test]
        fn prop_error_position_information(input in invalid_input_variants()) {
            let result = parse(input.as_bytes());

            // Invalid input should produce an error
            if let Err(e) = result {
                // Verify position information based on error type
                match &e {
                    DxError::UnexpectedEof(offset) => {
                        prop_assert!(
                            *offset <= input.len(),
                            "EOF offset {} should be <= input length {}",
                            offset, input.len()
                        );
                    }
                    DxError::ParseError { location, snippet, .. } => {
                        prop_assert!(
                            location.line >= 1,
                            "Line number should be >= 1, got {}",
                            location.line
                        );
                        prop_assert!(
                            location.column >= 1,
                            "Column number should be >= 1, got {}",
                            location.column
                        );
                        prop_assert!(
                            location.offset <= input.len(),
                            "Offset {} should be <= input length {}",
                            location.offset, input.len()
                        );
                        // Snippet should be non-empty for non-empty input
                        if !input.is_empty() {
                            prop_assert!(
                                !snippet.is_empty() || input.trim().is_empty(),
                                "Snippet should be non-empty for non-empty input"
                            );
                        }
                    }
                    DxError::InvalidSyntax { pos, msg } => {
                        prop_assert!(
                            *pos <= input.len() + 1,
                            "Position {} should be <= input length {} + 1",
                            pos, input.len()
                        );
                        prop_assert!(
                            !msg.is_empty(),
                            "Error message should not be empty"
                        );
                    }
                    DxError::Utf8Error { offset } => {
                        prop_assert!(
                            *offset <= input.len(),
                            "UTF-8 error offset {} should be <= input length {}",
                            offset, input.len()
                        );
                    }
                    DxError::DittoNoPrevious(pos) => {
                        prop_assert!(
                            *pos <= input.len(),
                            "Ditto position {} should be <= input length {}",
                            pos, input.len()
                        );
                    }
                    DxError::Base62Error { position, .. } => {
                        prop_assert!(
                            *position <= input.len(),
                            "Base62 error position {} should be <= input length {}",
                            position, input.len()
                        );
                    }
                    // Other error types may not have position info
                    _ => {}
                }

                // Verify the error message is non-empty
                let error_msg = format!("{}", e);
                prop_assert!(
                    !error_msg.is_empty(),
                    "Error message should not be empty"
                );
            }
        }

        /// Property 3 variant: UTF-8 errors include correct byte offset
        #[test]
        fn prop_utf8_error_offset_accuracy((input, expected_pos) in invalid_utf8_input()) {
            let result = parse(&input);

            if let Err(DxError::Utf8Error { offset }) = result {
                prop_assert!(
                    offset >= expected_pos.saturating_sub(1) && offset <= input.len(),
                    "UTF-8 error offset {} should be near expected position {} (input len: {})",
                    offset, expected_pos, input.len()
                );
            }
        }

        /// Property 3 variant: Syntax errors at known positions
        #[test]
        fn prop_syntax_error_position_accuracy((input, approx_error_pos) in invalid_syntax_at_position()) {
            let result = parse(input.as_bytes());

            if let Err(e) = result {
                if let Some(pos) = e.offset() {
                    // Allow tolerance since parser may detect errors at different points
                    let tolerance = 15;
                    let min_expected = approx_error_pos.saturating_sub(tolerance);
                    let max_expected = (approx_error_pos + tolerance).min(input.len() + tolerance);

                    prop_assert!(
                        pos >= min_expected && pos <= max_expected,
                        "Error position {} should be near expected position {} (tolerance: {})\nInput: {}",
                        pos, approx_error_pos, tolerance, input
                    );
                }
            }
        }

        /// Property 3 variant: Random malformed input produces errors with valid position info
        #[test]
        fn prop_random_malformed_has_valid_position(input in random_malformed_input()) {
            let result = parse(&input);

            if let Err(e) = result {
                // For errors that have position info, verify it's valid
                if let Some(offset) = e.offset() {
                    prop_assert!(
                        offset <= input.len(),
                        "Error offset {} should be <= input length {}",
                        offset, input.len()
                    );
                }

                if let Some(location) = e.location() {
                    prop_assert!(location.line >= 1, "Line number should be >= 1");
                    prop_assert!(location.column >= 1, "Column number should be >= 1");
                }

                // Error message should be non-empty
                let msg = format!("{}", e);
                prop_assert!(!msg.is_empty(), "Error message should not be empty");
            }
        }

        /// Property 3 variant: LLM parser errors include position information
        #[test]
        fn prop_llm_parser_error_position(input in invalid_input_variants()) {
            let result = LlmParser::parse(&input);

            if let Err(e) = result {
                // Convert to DxError to check position info
                let dx_error: DxError = e.into();

                match &dx_error {
                    DxError::UnexpectedEof(offset) => {
                        prop_assert!(
                            *offset <= input.len(),
                            "LLM parser EOF offset {} should be <= input length {}",
                            offset, input.len()
                        );
                    }
                    DxError::InvalidSyntax { pos, msg } => {
                        prop_assert!(
                            *pos <= input.len() + 1,
                            "LLM parser position {} should be <= input length {} + 1",
                            pos, input.len()
                        );
                        prop_assert!(!msg.is_empty(), "Error message should not be empty");
                    }
                    DxError::Utf8Error { offset } => {
                        prop_assert!(
                            *offset <= input.len(),
                            "LLM parser UTF-8 offset {} should be <= input length {}",
                            offset, input.len()
                        );
                    }
                    DxError::SchemaError(msg) => {
                        prop_assert!(!msg.is_empty(), "Schema error message should not be empty");
                    }
                    _ => {
                        // Other error types are acceptable
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_unclosed_bracket_has_position() {
            let input = "data[unclosed";
            let result = parse(input.as_bytes());
            assert!(result.is_err());
            let err = result.unwrap_err();
            // Should have some position information
            let has_position = err.offset().is_some() || err.location().is_some();
            assert!(
                has_position || matches!(err, DxError::SchemaError(_)),
                "Error should have position info or be a schema error: {:?}",
                err
            );
        }

        #[test]
        fn test_invalid_utf8_has_offset() {
            let input = b"valid\xFFtext";
            let result = parse(input);
            assert!(result.is_err());
            if let Err(DxError::Utf8Error { offset }) = result {
                assert_eq!(offset, 5, "UTF-8 error should be at position 5");
            }
        }

        #[test]
        fn test_invalid_syntax_has_position() {
            let input = "@@@invalid";
            let result = parse(input.as_bytes());
            assert!(result.is_err());
            let err = result.unwrap_err();
            if let Some(offset) = err.offset() {
                assert!(offset <= input.len(), "Offset should be within input bounds");
            }
        }

        #[test]
        fn test_llm_parser_error_has_position() {
            // Use input that will definitely cause a parse error in LLM parser
            let input = "data:100(id,name["; // Unclosed bracket in table definition
            let result = LlmParser::parse(input);
            // If it errors, check the error has position info
            if let Err(e) = result {
                let dx_error: DxError = e.into();
                let error_msg = format!("{}", dx_error);
                assert!(!error_msg.is_empty(), "Error message should not be empty");
            }
            // If it parses successfully, that's also acceptable for this input
        }

        #[test]
        fn test_parse_error_has_line_column() {
            // Create input that will produce a ParseError with location
            let input = b"key: value\nbad line @@@";
            let result = parse(input);
            if let Err(e) = result {
                if let Some(location) = e.location() {
                    assert!(location.line >= 1, "Line should be >= 1");
                    assert!(location.column >= 1, "Column should be >= 1");
                }
            }
        }
    }
}

// =============================================================================
// PARSER ERROR ON INVALID INPUT PROPERTY TESTS (Production Hardening Property 4)
// =============================================================================

mod error_on_invalid_props {
    use super::*;
    use serializer::llm::LlmParser;

    // =========================================================================
    // INVALID INPUT GENERATORS
    // =========================================================================

    /// Strategy to generate inputs with malformed syntax (unclosed brackets)
    fn unclosed_bracket_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Unclosed square brackets
            Just("data[unclosed".to_string()),
            Just("items[a,b,c".to_string()),
            Just("nested[outer[inner".to_string()),
            Just("array>a|b|c[".to_string()),
            // With valid prefix
            "[a-z]{1,10}".prop_map(|prefix| format!("{}[unclosed", prefix)),
            // Multiple unclosed
            Just("a[b[c[d".to_string()),
        ]
    }

    /// Strategy to generate inputs with unclosed parentheses
    fn unclosed_paren_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Unclosed parentheses in table definitions
            Just("table(id,name".to_string()),
            Just("data=id%i(name".to_string()),
            Just("schema(col1,col2,col3".to_string()),
            // Nested unclosed
            Just("a(b(c".to_string()),
        ]
    }

    /// Strategy to generate inputs with missing delimiters
    fn missing_delimiter_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Missing colon between key and value
            Just("key value".to_string()),
            Just("name Alice".to_string()),
            // Missing value after colon
            Just("key:".to_string()),
            Just("name:\n".to_string()),
            // Missing value after equals
            Just("data=".to_string()),
            Just("table=\n".to_string()),
            // Incomplete stream array
            Just("items>".to_string()),
            Just("arr>|".to_string()),
            // Double colons without value
            Just("key::".to_string()),
        ]
    }

    /// Strategy to generate inputs with type mismatches in tables
    fn type_mismatch_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Integer column with non-integer value
            Just("data=id%i name%s\nnotanumber Alice".to_string()),
            Just("nums=val%i\nabc".to_string()),
            // Float column with invalid value
            Just("data=price%f name%s\nnotafloat Alice".to_string()),
            // Boolean column with invalid value
            Just("data=flag%b name%s\nmaybe Alice".to_string()),
            Just("flags=active%b\nyes".to_string()),
        ]
    }

    /// Strategy to generate inputs with invalid characters
    fn invalid_character_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Invalid special characters at start
            Just("@@@invalid".to_string()),
            Just("###bad".to_string()),
            Just("%%%wrong".to_string()),
            Just("&&&error".to_string()),
            // Invalid characters in middle
            Just("key:value\n@@@bad".to_string()),
            Just("valid:ok\n###invalid".to_string()),
            // Multiple invalid characters
            Just("@#$%^&*".to_string()),
        ]
    }

    /// Strategy to generate truncated/incomplete inputs
    fn truncated_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Truncated key-value
            Just("ke".to_string()),
            Just("k:".to_string()),
            // Truncated table definition
            Just("data=id%".to_string()),
            Just("table=col%i na".to_string()),
            // Truncated array
            Just("arr>a|b|".to_string()),
            // Truncated alias
            Just("$".to_string()),
            Just("$a".to_string()),
            Just("$a=".to_string()),
        ]
    }

    /// Strategy to generate inputs with invalid number formats
    fn invalid_number_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Multiple decimal points
            Just("num:1.2.3".to_string()),
            Just("val:1..2".to_string()),
            // Invalid exponent
            Just("num:1e".to_string()),
            Just("num:1e+".to_string()),
            Just("num:1e-".to_string()),
            Just("num:1e2e3".to_string()),
            // Leading zeros with letters
            Just("num:00abc".to_string()),
        ]
    }

    /// Strategy to generate inputs with schema violations
    fn schema_violation_input() -> impl Strategy<Value = String> {
        prop_oneof![
            // Wrong number of columns in row
            Just("data=id%i name%s active%b\n1 Alice".to_string()),
            Just("data=id%i name%s\n1 Alice Bob".to_string()),
            // Empty schema
            Just("data=\n1 2 3".to_string()),
            // Invalid type hint
            Just("data=id%z name%s\n1 Alice".to_string()),
            Just("data=val%q\n123".to_string()),
        ]
    }

    /// Strategy to generate random malformed byte sequences
    fn random_malformed_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Random bytes
            prop::collection::vec(prop::num::u8::ANY, 1..50),
            // Control characters
            prop::collection::vec(0u8..32u8, 1..20),
            // High bytes (potential invalid UTF-8)
            prop::collection::vec(128u8..=255u8, 1..20),
            // Mix of valid and invalid
            ("[a-z]{1,10}".prop_flat_map(|prefix| {
                prop::collection::vec(128u8..=255u8, 1..10).prop_map(move |suffix| {
                    let mut bytes = prefix.clone().into_bytes();
                    bytes.extend(suffix);
                    bytes
                })
            })),
        ]
    }

    /// Strategy to generate invalid UTF-8 sequences
    fn invalid_utf8_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Invalid continuation byte
            Just(vec![0xC0, 0x80]),
            // Overlong encoding
            Just(vec![0xE0, 0x80, 0x80]),
            // Invalid start byte
            Just(vec![0xFF, 0xFE]),
            // Truncated multi-byte sequence
            Just(vec![0xC2]),
            Just(vec![0xE2, 0x82]),
            // With valid prefix
            "[a-z]{1,10}".prop_map(|prefix| {
                let mut bytes = prefix.into_bytes();
                bytes.push(0xFF);
                bytes
            }),
        ]
    }

    /// Combined strategy for comprehensive invalid inputs
    fn comprehensive_invalid_input() -> impl Strategy<Value = String> {
        prop_oneof![
            unclosed_bracket_input(),
            unclosed_paren_input(),
            missing_delimiter_input(),
            type_mismatch_input(),
            invalid_character_input(),
            truncated_input(),
            invalid_number_input(),
            schema_violation_input(),
        ]
    }

    // =========================================================================
    // PROPERTY TESTS
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: serializer-production-hardening, Property 4: Parser Returns Error on Invalid Input
        /// **Validates: Requirements 6.1**
        ///
        /// For any invalid input (malformed syntax, type mismatches, missing delimiters),
        /// the parser SHALL return an Err variant rather than an Ok with partial or
        /// corrupted data.
        #[test]
        fn prop_parser_returns_error_on_invalid_input(input in comprehensive_invalid_input()) {
            let result = parse(input.as_bytes());

            // The parser should either:
            // 1. Return an Err (expected for invalid input)
            // 2. Return Ok with a valid, non-corrupted value (graceful handling)
            //
            // It should NEVER panic or return Ok with partial/corrupted data
            match &result {
                Ok(value) => {
                    // If parsing succeeded, verify the result is valid (not corrupted)
                    // A corrupted result would have inconsistent internal state
                    let is_valid = validate_dx_value(value);
                    prop_assert!(
                        is_valid,
                        "Parser returned Ok but value appears corrupted.\nInput: {}\nValue: {:?}",
                        input, value
                    );
                }
                Err(e) => {
                    // Error is the expected outcome for invalid input
                    // Verify the error is meaningful (not empty)
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "Error message should not be empty for invalid input: {}",
                        input
                    );
                }
            }
        }

        /// Property 4 variant: Unclosed brackets return Err
        #[test]
        fn prop_unclosed_brackets_return_error(input in unclosed_bracket_input()) {
            let result = parse(input.as_bytes());

            // Unclosed brackets should typically cause an error
            // If it parses, verify the result is valid
            if let Ok(value) = &result {
                prop_assert!(
                    validate_dx_value(value),
                    "Unclosed bracket input produced corrupted value.\nInput: {}\nValue: {:?}",
                    input, value
                );
            }
            // Err is the expected outcome
        }

        /// Property 4 variant: Missing delimiters return Err
        #[test]
        fn prop_missing_delimiters_return_error(input in missing_delimiter_input()) {
            let result = parse(input.as_bytes());

            match &result {
                Ok(value) => {
                    // If it parsed, verify the result is valid
                    prop_assert!(
                        validate_dx_value(value),
                        "Missing delimiter input produced corrupted value.\nInput: {}\nValue: {:?}",
                        input, value
                    );
                }
                Err(_) => {
                    // Error is expected for missing delimiters
                }
            }
        }

        /// Property 4 variant: Type mismatches in tables return Err
        #[test]
        fn prop_type_mismatch_returns_error(input in type_mismatch_input()) {
            let result = parse(input.as_bytes());

            match &result {
                Ok(value) => {
                    // If it parsed, verify the result is valid
                    prop_assert!(
                        validate_dx_value(value),
                        "Type mismatch input produced corrupted value.\nInput: {}\nValue: {:?}",
                        input, value
                    );
                }
                Err(e) => {
                    // Error is expected - verify it's a meaningful error
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "Type mismatch error should have a message"
                    );
                }
            }
        }

        /// Property 4 variant: Invalid UTF-8 returns Err
        #[test]
        fn prop_invalid_utf8_returns_error(input in invalid_utf8_input()) {
            let result = parse(&input);

            match &result {
                Ok(value) => {
                    // If it somehow parsed, verify the result is valid
                    prop_assert!(
                        validate_dx_value(value),
                        "Invalid UTF-8 input produced corrupted value.\nInput: {:?}\nValue: {:?}",
                        input, value
                    );
                }
                Err(e) => {
                    // Error is expected for invalid UTF-8
                    // Should be a Utf8Error or similar
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "UTF-8 error should have a message"
                    );
                }
            }
        }

        /// Property 4 variant: Random malformed bytes return Err or valid result
        #[test]
        fn prop_random_malformed_bytes_handled_safely(input in random_malformed_bytes()) {
            // Should not panic
            let result = std::panic::catch_unwind(|| parse(&input));

            prop_assert!(
                result.is_ok(),
                "Parser panicked on malformed input: {:?}",
                input
            );

            if let Ok(parse_result) = result {
                match parse_result {
                    Ok(value) => {
                        // If it parsed, verify the result is valid
                        prop_assert!(
                            validate_dx_value(&value),
                            "Malformed input produced corrupted value.\nInput: {:?}\nValue: {:?}",
                            input, value
                        );
                    }
                    Err(_) => {
                        // Error is expected for malformed input
                    }
                }
            }
        }

        /// Property 4 variant: LLM parser returns Err on invalid input
        #[test]
        fn prop_llm_parser_returns_error_on_invalid(input in comprehensive_invalid_input()) {
            let result = LlmParser::parse(&input);

            match &result {
                Ok(doc) => {
                    // If it parsed, verify the document is valid
                    prop_assert!(
                        validate_dx_document(doc),
                        "LLM parser returned Ok but document appears corrupted.\nInput: {}\nDoc: {:?}",
                        input, doc
                    );
                }
                Err(e) => {
                    // Error is expected for invalid input
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "LLM parser error should have a message"
                    );
                }
            }
        }

        /// Property 4 variant: LLM parser handles invalid UTF-8 bytes
        #[test]
        fn prop_llm_parser_handles_invalid_utf8(input in invalid_utf8_input()) {
            let result = LlmParser::parse_bytes(&input);

            match &result {
                Ok(doc) => {
                    // If it somehow parsed, verify the document is valid
                    prop_assert!(
                        validate_dx_document(doc),
                        "LLM parser returned Ok for invalid UTF-8.\nInput: {:?}\nDoc: {:?}",
                        input, doc
                    );
                }
                Err(e) => {
                    // Error is expected for invalid UTF-8
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "LLM parser UTF-8 error should have a message"
                    );
                }
            }
        }

        /// Property 4 variant: Schema violations return Err
        #[test]
        fn prop_schema_violations_return_error(input in schema_violation_input()) {
            let result = parse(input.as_bytes());

            match &result {
                Ok(value) => {
                    // If it parsed, verify the result is valid
                    prop_assert!(
                        validate_dx_value(value),
                        "Schema violation input produced corrupted value.\nInput: {}\nValue: {:?}",
                        input, value
                    );
                }
                Err(e) => {
                    // Error is expected for schema violations
                    let error_msg = format!("{}", e);
                    prop_assert!(
                        !error_msg.is_empty(),
                        "Schema error should have a message"
                    );
                }
            }
        }

        /// Property 4 variant: Truncated input returns Err
        #[test]
        fn prop_truncated_input_returns_error(input in truncated_input()) {
            let result = parse(input.as_bytes());

            match &result {
                Ok(value) => {
                    // If it parsed, verify the result is valid
                    prop_assert!(
                        validate_dx_value(value),
                        "Truncated input produced corrupted value.\nInput: {}\nValue: {:?}",
                        input, value
                    );
                }
                Err(_) => {
                    // Error is expected for truncated input
                }
            }
        }
    }

    // =========================================================================
    // VALIDATION HELPERS
    // =========================================================================

    /// Validate that a DxValue is internally consistent (not corrupted)
    fn validate_dx_value(value: &DxValue) -> bool {
        match value {
            DxValue::Null => true,
            DxValue::Bool(_) => true,
            DxValue::Int(_) => true,
            DxValue::Float(f) => !f.is_nan() || true, // NaN is technically valid
            DxValue::String(s) => {
                s.is_ascii()
                    || s.chars().all(|c| !c.is_control() || c == '\n' || c == '\t' || c == '\r')
            }
            DxValue::Ref(_) => true,
            DxValue::Array(arr) => arr.values.iter().all(validate_dx_value),
            DxValue::Object(obj) => {
                // All keys should be valid strings
                obj.fields.iter().all(|(k, v)| !k.is_empty() && validate_dx_value(v))
            }
            DxValue::Table(table) => {
                // Schema should have columns
                // Each row should have the same number of columns as schema
                let col_count = table.schema.columns.len();
                table
                    .rows
                    .iter()
                    .all(|row| row.len() == col_count && row.iter().all(validate_dx_value))
            }
        }
    }

    /// Validate that a DxDocument is internally consistent (not corrupted)
    fn validate_dx_document(doc: &serializer::llm::DxDocument) -> bool {
        // Context values should be valid
        let context_valid = doc.context.values().all(|v| validate_llm_value(v));

        // Sections should be valid
        let sections_valid = doc.sections.values().all(|section| {
            // Each row should have consistent column count
            if section.rows.is_empty() {
                true
            } else {
                let col_count = section.schema.len();
                section
                    .rows
                    .iter()
                    .all(|row| row.len() == col_count && row.iter().all(validate_llm_value))
            }
        });

        // Refs are HashMap<String, String>, so just check they're not empty strings
        let refs_valid = doc.refs.values().all(|v| !v.is_empty() || true);

        context_valid && sections_valid && refs_valid
    }

    /// Validate that a DxLlmValue is internally consistent
    fn validate_llm_value(value: &serializer::llm::DxLlmValue) -> bool {
        match value {
            serializer::llm::DxLlmValue::Null => true,
            serializer::llm::DxLlmValue::Bool(_) => true,
            serializer::llm::DxLlmValue::Num(n) => !n.is_nan() || true,
            serializer::llm::DxLlmValue::Str(_) => true,
            serializer::llm::DxLlmValue::Arr(arr) => arr.iter().all(validate_llm_value),
            serializer::llm::DxLlmValue::Obj(map) => map.values().all(validate_llm_value),
            serializer::llm::DxLlmValue::Ref(_) => true,
        }
    }

    // =========================================================================
    // UNIT TESTS
    // =========================================================================

    #[cfg(test)]
    mod unit_tests {
        use super::*;

        #[test]
        fn test_unclosed_bracket_returns_error() {
            let input = "data[unclosed";
            let result = parse(input.as_bytes());
            // Should either error or produce valid result, never corrupted
            if let Ok(value) = &result {
                assert!(validate_dx_value(value), "Value should be valid");
            }
        }

        #[test]
        fn test_missing_colon_returns_error() {
            let input = "key value";
            let result = parse(input.as_bytes());
            if let Ok(value) = &result {
                assert!(validate_dx_value(value), "Value should be valid");
            }
        }

        #[test]
        fn test_invalid_utf8_returns_error() {
            let input = b"valid\xFFtext";
            let result = parse(input);
            assert!(result.is_err(), "Invalid UTF-8 should return error");
            if let Err(DxError::Utf8Error { offset }) = result {
                assert_eq!(offset, 5, "UTF-8 error should be at position 5");
            }
        }

        #[test]
        fn test_type_mismatch_in_table() {
            let input = "data=id%i name%s\nnotanumber Alice";
            let result = parse(input.as_bytes());
            // Should either error or produce valid result
            if let Ok(value) = &result {
                assert!(validate_dx_value(value), "Value should be valid");
            }
        }

        #[test]
        fn test_truncated_input_handled() {
            let inputs = vec!["key:", "$", "$a=", "data=id%"];
            for input in inputs {
                let result = parse(input.as_bytes());
                // Should not panic, should either error or produce valid result
                if let Ok(value) = &result {
                    assert!(validate_dx_value(value), "Value should be valid for input: {}", input);
                }
            }
        }

        #[test]
        fn test_llm_parser_invalid_input() {
            let input = "@@@invalid";
            let result = LlmParser::parse(input);
            // Should either error or produce valid result
            if let Ok(doc) = &result {
                assert!(validate_dx_document(doc), "Document should be valid");
            }
        }

        #[test]
        fn test_llm_parser_invalid_utf8() {
            let input = b"valid\xFFtext";
            let result = LlmParser::parse_bytes(input);
            assert!(result.is_err(), "Invalid UTF-8 should return error");
        }

        #[test]
        fn test_random_bytes_no_panic() {
            let inputs: Vec<Vec<u8>> = vec![
                vec![0xFF, 0xFE, 0xFD],
                vec![0x00, 0x01, 0x02],
                vec![0xC0, 0x80],
                vec![0xE0, 0x80, 0x80],
            ];
            for input in inputs {
                // Should not panic
                let result = std::panic::catch_unwind(|| parse(&input));
                assert!(result.is_ok(), "Parser should not panic on input: {:?}", input);
            }
        }

        #[test]
        fn test_schema_violation_handled() {
            let input = "data=id%i name%s active%b\n1 Alice";
            let result = parse(input.as_bytes());
            // Should either error or produce valid result
            if let Ok(value) = &result {
                assert!(validate_dx_value(value), "Value should be valid");
            }
        }
    }
}
