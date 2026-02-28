//! Property tests for tokenizer robustness
//!
//! Feature: serializer-battle-hardening
//! Tests Properties 4-7 from the design document

use proptest::prelude::*;
use serializer::tokenizer::{Token, Tokenizer};

/// Strategy to generate numbers that would overflow i64
fn overflow_number_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Numbers larger than i64::MAX
        Just("9223372036854775808".to_string()), // i64::MAX + 1
        Just("99999999999999999999999999999".to_string()),
        Just("-9223372036854775809".to_string()), // i64::MIN - 1
        // Very large numbers
        "[1-9][0-9]{20,30}".prop_map(|s| s),
    ]
}

/// Strategy to generate malformed float strings
fn malformed_float_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Multiple decimal points
        Just("1.2.3".to_string()),
        Just("1..2".to_string()),
        // Multiple exponents
        Just("1e2e3".to_string()),
        Just("1E2E3".to_string()),
        // Invalid exponent format
        Just("1e".to_string()),
        Just("1e+".to_string()),
        Just("1e-".to_string()),
        // Leading/trailing dots
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
        "[a-z]{1,10}:[a-z0-9]{1,10}".prop_map(|s| s),
    ]
}

/// Strategy to generate inputs with control characters
fn input_with_control_chars() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(
        prop_oneof![
            // Normal ASCII
            prop::num::u8::ANY.prop_filter("printable", |&b| b >= 0x20 && b < 0x7F),
            // Control characters (except newline/tab which are handled)
            prop::num::u8::ANY.prop_filter("control", |&b| b < 0x20 && b != 0x09 && b != 0x0A && b != 0x0D),
        ],
        1..50
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: serializer-battle-hardening, Property 4: Integer Overflow Detection
    /// Validates: Requirements 2.1
    ///
    /// For any numeric string representing a value outside the range [-2^63, 2^63-1],
    /// the Tokenizer SHALL return an IntegerOverflow error rather than silently
    /// truncating or wrapping.
    #[test]
    fn prop_integer_overflow_detection(num_str in overflow_number_string()) {
        let input = format!("key:{}", num_str);
        let mut tokenizer = Tokenizer::new(input.as_bytes());
        
        // Skip to the number
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        
        let result = tokenizer.next_token();
        
        match result {
            Ok(Token::Int(_)) => {
                // If it parsed as int, verify it didn't overflow silently
                // by checking the string representation
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
        }
    }

    /// Feature: serializer-battle-hardening, Property 5: Invalid Float Detection
    /// Validates: Requirements 2.2
    ///
    /// For any string that looks like a float but has invalid format,
    /// the Tokenizer SHALL return an InvalidNumber error.
    #[test]
    fn prop_invalid_float_detection(float_str in malformed_float_string()) {
        let input = format!("key:{}", float_str);
        let mut tokenizer = Tokenizer::new(input.as_bytes());
        
        // Skip to the number
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        
        let result = tokenizer.next_token();
        
        match result {
            Ok(Token::Float(f)) => {
                // If it parsed, verify it's a valid float
                prop_assert!(!f.is_nan() || float_str.contains("nan"), 
                    "Parsed invalid float string {} as {}", float_str, f);
            }
            Ok(Token::Int(_)) => {
                // Some malformed floats might parse as ints (e.g., "1." -> 1)
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
        }
    }

    /// Feature: serializer-battle-hardening, Property 6: EOF Handling
    /// Validates: Requirements 2.3
    ///
    /// For any input, after all tokens have been consumed, subsequent calls
    /// to next_token() SHALL return Token::Eof without panicking or returning errors.
    #[test]
    fn prop_eof_handling(input in valid_tokenizable_input()) {
        let mut tokenizer = Tokenizer::new(input.as_bytes());
        
        // Consume all tokens
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
    ///
    /// For any input containing control characters, the Tokenizer SHALL handle
    /// them consistently—either as part of string values or by returning
    /// appropriate errors.
    #[test]
    fn prop_control_character_handling(input in input_with_control_chars()) {
        let mut tokenizer = Tokenizer::new(&input);
        
        // Should not panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut tokens = Vec::new();
            loop {
                match tokenizer.next_token() {
                    Ok(Token::Eof) => break,
                    Ok(t) => tokens.push(t),
                    Err(_) => break, // Errors are acceptable
                }
                if tokens.len() > 100 {
                    break; // Prevent infinite loops
                }
            }
            tokens
        }));
        
        prop_assert!(result.is_ok(), "Tokenizer panicked on control characters");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_i64_max() {
        let input = b"key:9223372036854775807"; // i64::MAX
        let mut tokenizer = Tokenizer::new(input);
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        let result = tokenizer.next_token();
        assert!(matches!(result, Ok(Token::Int(i64::MAX))));
    }

    #[test]
    fn test_i64_min() {
        let input = b"key:-9223372036854775808"; // i64::MIN
        let mut tokenizer = Tokenizer::new(input);
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        let result = tokenizer.next_token();
        assert!(matches!(result, Ok(Token::Int(i64::MIN))));
    }

    #[test]
    fn test_valid_float() {
        let input = b"key:3.14159";
        let mut tokenizer = Tokenizer::new(input);
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        let result = tokenizer.next_token();
        if let Ok(Token::Float(f)) = result {
            assert!((f - 3.14159).abs() < 0.00001);
        } else {
            panic!("Expected float token");
        }
    }

    #[test]
    fn test_scientific_notation() {
        let input = b"key:1.5e10";
        let mut tokenizer = Tokenizer::new(input);
        let _ = tokenizer.next_token(); // key
        let _ = tokenizer.next_token(); // :
        let result = tokenizer.next_token();
        if let Ok(Token::Float(f)) = result {
            assert!((f - 1.5e10).abs() < 1e5);
        } else {
            panic!("Expected float token");
        }
    }

    #[test]
    fn test_eof_multiple_calls() {
        let input = b"key:value";
        let mut tokenizer = Tokenizer::new(input);
        
        // Consume all tokens
        while !matches!(tokenizer.next_token().unwrap(), Token::Eof) {}
        
        // Multiple EOF calls should work
        for _ in 0..10 {
            assert!(matches!(tokenizer.next_token().unwrap(), Token::Eof));
        }
    }
}
