//! Property-based tests for UTF-8 validation
//!
//! **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
//! **Validates: Requirements 15.5**
//!
//! For any input containing invalid UTF-8 sequences, the parser SHALL return
//! a `Utf8Error` containing the byte offset of the first invalid byte.

#[cfg(test)]
mod property_tests {
    use crate::error::DxError;
    use crate::utf8::{validate_string_input, validate_utf8, validate_utf8_detailed};
    use proptest::prelude::*;

    /// Generate valid UTF-8 strings
    fn arb_valid_utf8() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-zA-Z0-9 .,!?\\-_]{0,100}").unwrap()
    }

    /// Generate valid UTF-8 with unicode characters
    fn arb_valid_utf8_unicode() -> impl Strategy<Value = String> {
        prop_oneof![
            // ASCII only
            proptest::string::string_regex("[a-zA-Z0-9]{0,50}").unwrap(),
            // With common unicode
            Just("Hello, 世界!".to_string()),
            Just("Привет мир".to_string()),
            Just("🎉🎊🎈".to_string()),
            Just("日本語テスト".to_string()),
            Just("مرحبا بالعالم".to_string()),
        ]
    }

    /// Generate invalid UTF-8 byte sequences
    fn arb_invalid_utf8() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Unexpected continuation byte
            Just(vec![0x80]),
            Just(vec![0xBF]),
            Just(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x80]), // "Hello" + continuation
            // Invalid start bytes (0xC0, 0xC1 are overlong, 0xF5+ are invalid)
            Just(vec![0xC0, 0x80]),
            Just(vec![0xC1, 0x80]),
            Just(vec![0xF5, 0x80, 0x80, 0x80]),
            Just(vec![0xFF]),
            Just(vec![0xFE]),
            // Incomplete sequences
            Just(vec![0xC2]),             // 2-byte start, no continuation
            Just(vec![0xE0, 0xA0]),       // 3-byte start, only 1 continuation
            Just(vec![0xF0, 0x90, 0x80]), // 4-byte start, only 2 continuations
            // Invalid continuation bytes
            Just(vec![0xC2, 0x00]),       // 2-byte with invalid continuation
            Just(vec![0xC2, 0xC0]),       // 2-byte with another start byte
            Just(vec![0xE0, 0xA0, 0x00]), // 3-byte with invalid continuation
            // Overlong encodings
            Just(vec![0xC0, 0xAF]),       // Overlong '/'
            Just(vec![0xE0, 0x80, 0xAF]), // Overlong '/'
            // Surrogate pairs (invalid in UTF-8)
            Just(vec![0xED, 0xA0, 0x80]), // U+D800 (high surrogate)
            Just(vec![0xED, 0xBF, 0xBF]), // U+DFFF (low surrogate)
            // Code points > U+10FFFF
            Just(vec![0xF4, 0x90, 0x80, 0x80]), // U+110000
        ]
    }

    /// Generate bytes with invalid UTF-8 at a specific position
    fn arb_invalid_at_position() -> impl Strategy<Value = (Vec<u8>, usize)> {
        (0usize..20).prop_flat_map(|prefix_len| {
            let prefix: Vec<u8> = (0..prefix_len).map(|i| b'a' + (i % 26) as u8).collect();
            let invalid_byte = 0x80u8; // Unexpected continuation byte
            let mut bytes = prefix.clone();
            bytes.push(invalid_byte);
            Just((bytes, prefix_len))
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6a: Valid UTF-8 strings SHALL be accepted
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_valid_utf8_accepted(s in arb_valid_utf8()) {
            let bytes = s.as_bytes();
            let result = validate_utf8(bytes);
            prop_assert!(result.is_ok(), "Valid UTF-8 should be accepted: {:?}", s);
            prop_assert_eq!(result.unwrap(), s.as_str());
        }

        /// Property 6b: Valid UTF-8 with unicode SHALL be accepted
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_valid_utf8_unicode_accepted(s in arb_valid_utf8_unicode()) {
            let bytes = s.as_bytes();
            let result = validate_utf8(bytes);
            prop_assert!(result.is_ok(), "Valid UTF-8 unicode should be accepted: {:?}", s);
        }

        /// Property 6c: Invalid UTF-8 SHALL return Utf8Error
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_invalid_utf8_returns_error(bytes in arb_invalid_utf8()) {
            let result = validate_utf8(&bytes);
            prop_assert!(result.is_err(), "Invalid UTF-8 should return error: {:?}", bytes);

            if let Err(DxError::Utf8Error { offset }) = result {
                prop_assert!(offset <= bytes.len(), "Offset should be within bounds");
            } else {
                prop_assert!(false, "Expected Utf8Error");
            }
        }

        /// Property 6d: Error offset SHALL point to first invalid byte
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_error_offset_correct((bytes, expected_offset) in arb_invalid_at_position()) {
            let result = validate_utf8(&bytes);
            prop_assert!(result.is_err(), "Should return error for invalid UTF-8");

            if let Err(DxError::Utf8Error { offset }) = result {
                prop_assert_eq!(
                    offset, expected_offset,
                    "Offset should be {} (position of invalid byte), got {}",
                    expected_offset, offset
                );
            }
        }

        /// Property 6e: validate_string_input SHALL add base_offset to error offset
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_base_offset_added(
            base_offset in 0usize..1000,
            (bytes, local_offset) in arb_invalid_at_position()
        ) {
            let result = validate_string_input(&bytes, base_offset);
            prop_assert!(result.is_err(), "Should return error for invalid UTF-8");

            if let Err(DxError::Utf8Error { offset }) = result {
                prop_assert_eq!(
                    offset, base_offset + local_offset,
                    "Offset should be base_offset + local_offset = {} + {} = {}, got {}",
                    base_offset, local_offset, base_offset + local_offset, offset
                );
            }
        }

        /// Property 6f: Detailed validation SHALL provide description
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_detailed_validation_has_description(bytes in arb_invalid_utf8()) {
            let result = validate_utf8_detailed(&bytes);
            prop_assert!(result.is_err(), "Invalid UTF-8 should return error");

            if let Err(err) = result {
                prop_assert!(!err.description.is_empty(), "Description should not be empty");
                prop_assert!(err.offset <= bytes.len(), "Offset should be within bounds");
            }
        }

        /// Property 6g: Empty input SHALL be valid UTF-8
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_empty_is_valid(_dummy in Just(())) {
            let result = validate_utf8(b"");
            prop_assert!(result.is_ok(), "Empty input should be valid UTF-8");
            prop_assert_eq!(result.unwrap(), "");
        }

        /// Property 6h: ASCII-only input SHALL always be valid
        ///
        /// **Feature: serializer-production-hardening, Property 6: Invalid UTF-8 Returns Error with Offset**
        /// **Validates: Requirements 15.5**
        #[test]
        fn prop_ascii_always_valid(bytes in proptest::collection::vec(0x00u8..0x80, 0..100)) {
            let result = validate_utf8(&bytes);
            prop_assert!(result.is_ok(), "ASCII-only input should be valid UTF-8: {:?}", bytes);
        }
    }

    // Unit tests for specific edge cases

    #[test]
    fn test_overlong_encoding_detected() {
        // Overlong encoding of ASCII 'A' (0x41) as 2-byte sequence
        let overlong = &[0xC1, 0x81];
        let result = validate_utf8_detailed(overlong);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.description.contains("Overlong") || err.description.contains("Invalid"));
    }

    #[test]
    fn test_surrogate_detected() {
        // UTF-16 surrogate U+D800
        let surrogate = &[0xED, 0xA0, 0x80];
        let result = validate_utf8_detailed(surrogate);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.description.contains("surrogate") || err.description.contains("Invalid"));
    }

    #[test]
    fn test_code_point_too_large() {
        // Code point > U+10FFFF
        let too_large = &[0xF4, 0x90, 0x80, 0x80];
        let result = validate_utf8_detailed(too_large);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.description.contains("exceeds") || err.description.contains("Invalid"));
    }

    #[test]
    fn test_incomplete_sequence_at_end() {
        // Valid ASCII followed by incomplete 2-byte sequence
        let incomplete = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xC2]; // "Hello" + start of 2-byte
        let result = validate_utf8(incomplete);
        assert!(result.is_err());
        if let Err(DxError::Utf8Error { offset }) = result {
            assert_eq!(offset, 5); // Position of incomplete sequence
        }
    }

    #[test]
    fn test_valid_multibyte_sequences() {
        // 2-byte: é (U+00E9)
        assert!(validate_utf8(&[0xC3, 0xA9]).is_ok());

        // 3-byte: 中 (U+4E2D)
        assert!(validate_utf8(&[0xE4, 0xB8, 0xAD]).is_ok());

        // 4-byte: 𝄞 (U+1D11E)
        assert!(validate_utf8(&[0xF0, 0x9D, 0x84, 0x9E]).is_ok());

        // Mixed
        let mixed = "Hello, 世界! 🎉";
        assert!(validate_utf8(mixed.as_bytes()).is_ok());
    }

    #[test]
    fn test_boundary_code_points() {
        // U+007F (highest 1-byte)
        assert!(validate_utf8(&[0x7F]).is_ok());

        // U+0080 (lowest 2-byte)
        assert!(validate_utf8(&[0xC2, 0x80]).is_ok());

        // U+07FF (highest 2-byte)
        assert!(validate_utf8(&[0xDF, 0xBF]).is_ok());

        // U+0800 (lowest 3-byte)
        assert!(validate_utf8(&[0xE0, 0xA0, 0x80]).is_ok());

        // U+FFFF (highest 3-byte, excluding surrogates)
        assert!(validate_utf8(&[0xEF, 0xBF, 0xBF]).is_ok());

        // U+10000 (lowest 4-byte)
        assert!(validate_utf8(&[0xF0, 0x90, 0x80, 0x80]).is_ok());

        // U+10FFFF (highest valid code point)
        assert!(validate_utf8(&[0xF4, 0x8F, 0xBF, 0xBF]).is_ok());
    }
}
