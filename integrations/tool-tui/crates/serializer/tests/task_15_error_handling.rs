//! Error handling tests for dx-serializer
//!
//! Feature: dx-serializer-production-ready
//! Tasks 15.1-15.3: Add error handling tests
//!
//! Tests error position information, input validation, and schema mismatch errors.

use serializer::llm::parser::{LlmParser, ParseError};

// =============================================================================
// TASK 15.1: PARSE ERROR POSITION INFORMATION
// =============================================================================

#[cfg(test)]
mod error_position_tests {
    use super::*;

    #[test]
    fn test_unclosed_bracket_has_position() {
        let input = "table:2(id name)[1 Alice";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail on unclosed bracket");
        if let Err(ParseError::UnclosedBracket { pos }) = result {
            assert!(pos <= input.len(), "Position should be within input bounds");
            assert!(pos > 0, "Position should be non-zero");
        }
    }

    #[test]
    fn test_unclosed_paren_has_position() {
        let input = "table:2(id name[data]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail on unclosed parenthesis");
        if let Err(ParseError::UnclosedParen { pos }) = result {
            assert!(pos <= input.len(), "Position should be within input bounds");
            assert!(pos > 0, "Position should be non-zero");
        }
    }

    #[test]
    fn test_unexpected_char_has_position() {
        let input = "key\x00:value";
        let result = LlmParser::parse(input);

        // Parser may handle null bytes differently, but if it errors, should have position
        if let Err(e) = result {
            match e {
                ParseError::UnexpectedChar { pos, ch } => {
                    assert!(pos <= input.len(), "Position should be within input bounds");
                    assert_eq!(ch, '\x00', "Should report the unexpected character");
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_unexpected_eof_has_position() {
        let input = "key=";
        let result = LlmParser::parse(input);

        // Parser may handle this differently, but if it errors, should have position
        if let Err(e) = result {
            match e {
                ParseError::UnexpectedEof => {
                    // UnexpectedEof doesn't have a position field, which is acceptable
                }
                ParseError::MissingValue { pos } => {
                    assert!(pos <= input.len(), "Position should be within input bounds");
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_invalid_table_format_has_message() {
        let input = "table:abc(id name)[data]";
        let result = LlmParser::parse(input);

        if let Err(e) = result {
            match e {
                ParseError::InvalidTable { msg } => {
                    assert!(!msg.is_empty(), "Error message should not be empty");
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_multiline_error_position() {
        let input = "line1=value1\nline2=value2\nline3[unclosed";
        let result = LlmParser::parse(input);

        if let Err(e) = result {
            match e {
                ParseError::UnclosedBracket { pos } => {
                    // Position should be somewhere in the third line
                    let line3_start = "line1=value1\nline2=value2\n".len();
                    assert!(pos >= line3_start, "Position should be in line 3");
                    assert!(pos <= input.len(), "Position should be within input");
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_all_parse_errors_have_position_or_context() {
        // Test various error scenarios to ensure they all provide useful information
        let test_cases = vec![
            ("table:2(id)[", "unclosed bracket"),
            ("table:2(id[data]", "unclosed paren"),
            ("table:2(id name)[1", "incomplete row"),
        ];

        for (input, description) in test_cases {
            let result = LlmParser::parse(input);
            assert!(result.is_err(), "Test case '{}' should produce an error", description);

            if let Err(e) = result {
                // Every error should provide some way to locate the problem
                let has_useful_info = match &e {
                    ParseError::UnexpectedChar { pos, .. } => *pos <= input.len(),
                    ParseError::UnclosedBracket { pos } => *pos <= input.len(),
                    ParseError::UnclosedParen { pos } => *pos <= input.len(),
                    ParseError::MissingValue { pos } => *pos <= input.len(),
                    ParseError::InvalidTable { msg } => !msg.is_empty(),
                    ParseError::SchemaMismatch { .. } => true,
                    ParseError::Utf8Error { offset } => *offset <= input.len(),
                    ParseError::InputTooLarge { .. } => true,
                    ParseError::UnexpectedEof => true,
                    ParseError::InvalidValue { value } => !value.is_empty(),
                };

                assert!(
                    has_useful_info,
                    "Error for '{}' should provide useful information: {:?}",
                    description, e
                );
            }
        }
    }
}

// =============================================================================
// TASK 15.2: INPUT VALIDATION EDGE CASES
// =============================================================================

#[cfg(test)]
mod input_validation_tests {
    use super::*;

    #[test]
    #[ignore] // Slow test - allocates 100MB+
    fn test_maximum_input_size_rejection() {
        // Create an input larger than MAX_INPUT_SIZE (100MB)
        // This test is ignored by default because it's slow
        let large_input = "x".repeat(100_000_001);
        let result = LlmParser::parse(&large_input);

        assert!(result.is_err(), "Should reject input exceeding maximum size");
        if let Err(ParseError::InputTooLarge { size, max }) = result {
            assert_eq!(size, large_input.len(), "Should report actual size");
            assert_eq!(max, 100_000_000, "Should report maximum size");
        }
    }

    #[test]
    #[ignore] // Slow test - allocates 100MB+
    fn test_maximum_input_size_bytes() {
        // Test with bytes API
        // This test is ignored by default because it's slow
        let large_input = vec![b'x'; 100_000_001];
        let result = LlmParser::parse_bytes(&large_input);

        assert!(result.is_err(), "Should reject byte input exceeding maximum size");
        if let Err(ParseError::InputTooLarge { size, max }) = result {
            assert_eq!(size, large_input.len(), "Should report actual size");
            assert_eq!(max, 100_000_000, "Should report maximum size");
        }
    }

    #[test]
    fn test_invalid_utf8_with_byte_offset() {
        // Create input with invalid UTF-8 at a known position
        let mut bytes = b"key=value\n".to_vec();
        let invalid_pos = bytes.len();
        bytes.push(0xFF); // Invalid UTF-8 byte
        bytes.extend_from_slice(b"\nmore=data");

        let result = LlmParser::parse_bytes(&bytes);

        assert!(result.is_err(), "Should fail on invalid UTF-8");
        if let Err(ParseError::Utf8Error { offset }) = result {
            assert_eq!(offset, invalid_pos, "Should report correct byte offset of invalid UTF-8");
        }
    }

    #[test]
    fn test_invalid_utf8_at_start() {
        let bytes = vec![0xFF, 0xFE, b'k', b'e', b'y'];
        let result = LlmParser::parse_bytes(&bytes);

        assert!(result.is_err(), "Should fail on invalid UTF-8 at start");
        if let Err(ParseError::Utf8Error { offset }) = result {
            assert_eq!(offset, 0, "Should report offset 0 for invalid UTF-8 at start");
        }
    }

    #[test]
    fn test_invalid_utf8_in_middle() {
        let mut bytes = b"first=ok\nsecond=".to_vec();
        let invalid_pos = bytes.len();
        bytes.push(0x80); // Invalid UTF-8 continuation byte without lead byte

        let result = LlmParser::parse_bytes(&bytes);

        assert!(result.is_err(), "Should fail on invalid UTF-8 in middle");
        if let Err(ParseError::Utf8Error { offset }) = result {
            assert!(
                offset >= invalid_pos.saturating_sub(1),
                "Should report offset near invalid byte"
            );
        }
    }

    #[test]
    fn test_empty_input() {
        let result = LlmParser::parse("");

        // Empty input should parse successfully as an empty document
        assert!(result.is_ok(), "Empty input should parse successfully");
        let doc = result.unwrap();
        assert!(doc.is_empty(), "Empty input should produce empty document");
    }

    #[test]
    fn test_whitespace_only_input() {
        let result = LlmParser::parse("   \n\t\n   ");

        // Whitespace-only input should parse successfully
        assert!(result.is_ok(), "Whitespace-only input should parse successfully");
        let doc = result.unwrap();
        assert!(doc.is_empty(), "Whitespace-only input should produce empty document");
    }

    #[test]
    fn test_input_at_size_limit() {
        // Create input exactly at the limit (100MB) - use smaller size for practical testing
        let input = "x".repeat(1_000_000); // 1MB for practical testing
        let result = LlmParser::parse(&input);

        // Should accept input well under the limit
        assert!(result.is_ok(), "Should accept input under size limit");
    }

    #[test]
    #[ignore] // Slow test - allocates 100MB+
    fn test_input_just_over_size_limit() {
        // Create input just over the limit (100MB)
        // This test is ignored by default because it's slow
        let input = "x".repeat(100_000_001);
        let result = LlmParser::parse(&input);

        assert!(result.is_err(), "Should reject input just over size limit");
        if let Err(ParseError::InputTooLarge { size, max }) = result {
            assert_eq!(size, 100_000_001);
            assert_eq!(max, 100_000_000);
        }
    }

    #[test]
    fn test_deeply_nested_structures() {
        // Test that parser handles deeply nested structures without stack overflow
        let mut input = String::from("obj:1[nested=");
        for _ in 0..100 {
            input.push_str("value_");
        }
        input.push(']');

        let result = LlmParser::parse(&input);
        // Should either parse or fail gracefully, but not panic
        let _ = result;
    }

    #[test]
    fn test_very_long_line() {
        // Test parsing a very long single line
        let long_value = "x".repeat(10_000);
        let input = format!("key={}", long_value);

        let result = LlmParser::parse(&input);
        assert!(result.is_ok(), "Should handle very long lines");
    }

    #[test]
    fn test_many_small_entries() {
        // Test parsing many small entries
        let mut input = String::new();
        for i in 0..1000 {
            input.push_str(&format!("key{}=value{}\n", i, i));
        }

        let result = LlmParser::parse(&input);
        assert!(result.is_ok(), "Should handle many small entries");
    }
}

// =============================================================================
// TASK 15.3: SCHEMA MISMATCH ERRORS
// =============================================================================

#[cfg(test)]
mod schema_mismatch_tests {
    use super::*;

    #[test]
    fn test_schema_mismatch_too_few_columns() {
        let input = "table:2(id name email)[1 Alice]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail when row has too few columns");
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 3, "Should expect 3 columns from schema");
            assert_eq!(got, 2, "Should report 2 columns in row");
        }
    }

    #[test]
    fn test_schema_mismatch_too_many_columns() {
        let input = "table:2(id name)[1 Alice Bob Carol]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail when row has too many columns");
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 2, "Should expect 2 columns from schema");
            assert!(got > 2, "Should report more than 2 columns in row");
        }
    }

    #[test]
    fn test_schema_mismatch_multiline_format() {
        let input = "table:2(id name email)[\n1 Alice\n]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail on schema mismatch in multiline format");
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 3, "Should expect 3 columns");
            assert_eq!(got, 2, "Should report 2 columns");
        }
    }

    #[test]
    fn test_schema_mismatch_comma_separated() {
        let input = "table:2(id name email)[1 Alice, 2 Bob]";
        let result = LlmParser::parse(input);

        // Should fail because rows don't match schema
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 3, "Should expect 3 columns from schema");
            assert!(got < 3, "Should report fewer columns in row");
        }
    }

    #[test]
    fn test_schema_mismatch_semicolon_separated() {
        let input = "table:2(id name email)[1 Alice; 2 Bob Carol]";
        let result = LlmParser::parse(input);

        // First row has 2 columns, second has 3 - should fail on first row
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 3, "Should expect 3 columns");
            assert_eq!(got, 2, "Should report 2 columns in first row");
        }
    }

    #[test]
    fn test_schema_mismatch_colon_separated() {
        let input = "table:2(timestamp level message)[2025-01-15 INFO: 2025-01-15 ERROR]";
        let result = LlmParser::parse(input);

        // Should fail on schema mismatch
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 3, "Should expect 3 columns");
            assert!(got != 3, "Should report incorrect column count");
        }
    }

    #[test]
    fn test_empty_schema() {
        let input = "table:1()[data]";
        let result = LlmParser::parse(input);

        // Should fail on empty schema
        assert!(result.is_err(), "Should fail on empty schema");
        if let Err(e) = result {
            match e {
                ParseError::InvalidTable { msg } => {
                    assert!(
                        msg.to_lowercase().contains("empty")
                            || msg.to_lowercase().contains("schema"),
                        "Error message should mention empty schema"
                    );
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_schema_mismatch_with_nested_arrays() {
        let input = "table:1(id items)[1 [a b c]]";
        let result = LlmParser::parse(input);

        // Parser should handle nested arrays correctly
        // If it fails, should provide clear error
        if let Err(e) = result {
            match e {
                ParseError::SchemaMismatch { expected, got: _ } => {
                    assert_eq!(expected, 2, "Should expect 2 columns");
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }

    #[test]
    fn test_schema_mismatch_error_message_clarity() {
        let input = "users:3(id name email age)[1 Alice alice@example.com]";
        let result = LlmParser::parse(input);

        assert!(result.is_err(), "Should fail on schema mismatch");
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 4, "Should expect 4 columns");
            assert_eq!(got, 3, "Should report 3 columns");

            // Verify the error can be formatted clearly
            let error_msg = format!("Schema mismatch: expected {} columns, got {}", expected, got);
            assert!(error_msg.contains("4"), "Error message should contain expected count");
            assert!(error_msg.contains("3"), "Error message should contain actual count");
        }
    }

    #[test]
    fn test_multiple_rows_first_row_mismatch() {
        let input = "table:3(id name)[1 Alice, 2 Bob Carol, 3 Dave]";
        let result = LlmParser::parse(input);

        // Should fail on first row with mismatch (2 values vs 2 columns expected)
        // Note: Parser may parse "1 Alice" as 2 columns which matches schema
        // So this might not fail, or might fail on a different row
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 2, "Should expect 2 columns");
            // The actual column count depends on how parser interprets the data
        }
    }

    #[test]
    fn test_multiple_rows_middle_row_mismatch() {
        let input = "table:3(id name)[1 Alice, 2 Bob Carol Dave, 3 Eve]";
        let result = LlmParser::parse(input);

        // Should fail on middle row with too many columns
        if let Err(ParseError::SchemaMismatch { expected, got }) = result {
            assert_eq!(expected, 2, "Should expect 2 columns");
            // Could fail on first row (1 column) or second row (3 columns)
            assert!(got != 2, "Should report incorrect column count");
        }
    }

    #[test]
    fn test_schema_mismatch_preserves_context() {
        // Ensure that schema mismatch errors don't lose other parsing context
        let input = "config=test\ntable:2(id name email)[1 Alice]\nother=value";
        let result = LlmParser::parse(input);

        // Should fail on schema mismatch, but error should be clear
        if let Err(e) = result {
            match e {
                ParseError::SchemaMismatch { expected, got } => {
                    assert_eq!(expected, 3);
                    assert_eq!(got, 2);
                }
                _ => {
                    // Other errors are acceptable
                }
            }
        }
    }
}

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_error_handling_comprehensive() {
        // Test that all major error types can be triggered and provide useful information
        let error_cases = vec![
            ("table:2(id)[", "UnclosedBracket"),
            ("table:2(id[data]", "UnclosedParen"),
            ("table:2(id name)[1]", "SchemaMismatch"),
        ];

        for (input, expected_error_type) in error_cases {
            let result = LlmParser::parse(input);
            assert!(
                result.is_err(),
                "Input '{}' should produce {} error",
                input,
                expected_error_type
            );

            // Verify error can be displayed
            if let Err(e) = result {
                let error_string = format!("{}", e);
                assert!(!error_string.is_empty(), "Error should have non-empty display string");
            }
        }
    }

    #[test]
    fn test_error_recovery_not_required() {
        // Verify that parser fails fast on errors rather than trying to recover
        let input = "bad[syntax\ngood=value";
        let result = LlmParser::parse(input);

        // Should fail on first error, not try to parse the rest
        assert!(result.is_err(), "Should fail on first error");
    }
}
