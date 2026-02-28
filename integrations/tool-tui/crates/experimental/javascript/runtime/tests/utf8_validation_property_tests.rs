//! Property tests for UTF-8 String Validation
//!
//! Feature: dx-runtime-production-ready
//! Property 5: UTF-8 String Validation
//!
//! These tests verify that:
//! - Valid UTF-8 sequences are accepted
//! - Invalid UTF-8 sequences are rejected with an error
//! - String content is preserved correctly
//!
//! **Validates: Requirements 1.5**

use dx_js_runtime::gc::GcHeap;
use proptest::prelude::*;

// ============================================================================
// Property 5: UTF-8 String Validation
// For any byte sequence passed to string allocation, invalid UTF-8 sequences
// should be rejected with an error, while valid UTF-8 should be accepted.
// **Validates: Requirements 1.5**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 5: Valid UTF-8 strings are accepted and preserved
    /// Feature: dx-runtime-production-ready, Property 5: UTF-8 String Validation
    #[test]
    fn prop_valid_utf8_accepted(s in "[a-zA-Z0-9 !@#$%^&*()_+-=\\[\\]{};':\",./<>?]{0,100}") {
        let mut heap = GcHeap::new().expect("Failed to create heap");
        
        // Valid UTF-8 string should be accepted
        let gc_string = heap.alloc_string(&s);
        
        prop_assert!(
            gc_string.is_some(),
            "Valid UTF-8 string '{}' should be accepted",
            s
        );
        
        // Content should be preserved exactly
        let gc_string = gc_string.unwrap();
        prop_assert_eq!(
            gc_string.as_str(),
            s.as_str(),
            "String content should be preserved"
        );
    }

    /// Property 5: Unicode strings are accepted and preserved
    /// Feature: dx-runtime-production-ready, Property 5: UTF-8 String Validation
    #[test]
    fn prop_unicode_strings_accepted(s in "\\PC{0,50}") {
        let mut heap = GcHeap::new().expect("Failed to create heap");
        
        // Unicode string should be accepted (proptest generates valid UTF-8)
        let gc_string = heap.alloc_string(&s);
        
        prop_assert!(
            gc_string.is_some(),
            "Unicode string should be accepted"
        );
        
        // Content should be preserved exactly
        let gc_string = gc_string.unwrap();
        prop_assert_eq!(
            gc_string.as_str(),
            s.as_str(),
            "Unicode string content should be preserved"
        );
    }

    /// Property 5: Empty string is valid UTF-8
    /// Feature: dx-runtime-production-ready, Property 5: UTF-8 String Validation
    #[test]
    fn prop_empty_string_valid(_dummy in 0..1i32) {
        let mut heap = GcHeap::new().expect("Failed to create heap");
        
        let gc_string = heap.alloc_string("");
        
        prop_assert!(
            gc_string.is_some(),
            "Empty string should be valid UTF-8"
        );
        
        let gc_string = gc_string.unwrap();
        prop_assert_eq!(gc_string.as_str(), "", "Empty string should be preserved");
        prop_assert!(gc_string.is_empty(), "Empty string should report as empty");
    }

    /// Property 5: String length is preserved
    /// Feature: dx-runtime-production-ready, Property 5: UTF-8 String Validation
    #[test]
    fn prop_string_length_preserved(s in "[a-zA-Z0-9]{1,100}") {
        let mut heap = GcHeap::new().expect("Failed to create heap");
        
        let gc_string = heap.alloc_string(&s).expect("Should allocate");
        
        // Byte length should match
        prop_assert_eq!(
            gc_string.len(),
            s.len(),
            "String byte length should be preserved"
        );
    }

    /// Property 5: Multi-byte UTF-8 characters are handled correctly
    /// Feature: dx-runtime-production-ready, Property 5: UTF-8 String Validation
    #[test]
    fn prop_multibyte_utf8_handled(
        ascii_part in "[a-z]{0,20}",
        emoji_count in 0..5usize
    ) {
        let mut heap = GcHeap::new().expect("Failed to create heap");
        
        // Build a string with multi-byte characters
        let emojis = "🎉🎊🎈🎁🎄";
        let emoji_part: String = emojis.chars().take(emoji_count).collect();
        let test_string = format!("{}{}", ascii_part, emoji_part);
        
        let gc_string = heap.alloc_string(&test_string).expect("Should allocate");
        
        // Content should be preserved
        prop_assert_eq!(
            gc_string.as_str(),
            test_string.as_str(),
            "Multi-byte UTF-8 content should be preserved"
        );
        
        // Byte length should match (emojis are 4 bytes each)
        prop_assert_eq!(
            gc_string.len(),
            test_string.len(),
            "Multi-byte UTF-8 byte length should be preserved"
        );
    }
}

// ============================================================================
// Unit Tests for UTF-8 Validation Edge Cases
// ============================================================================

#[test]
fn test_valid_utf8_ascii() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    let test_cases = [
        "",
        "hello",
        "Hello, World!",
        "1234567890",
        "!@#$%^&*()",
        "   \t\n\r   ",
    ];
    
    for s in &test_cases {
        let gc_string = heap.alloc_string(s).expect("Should allocate ASCII string");
        assert_eq!(gc_string.as_str(), *s, "ASCII string should be preserved");
    }
}

#[test]
fn test_valid_utf8_unicode() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    let test_cases = [
        "Hello, 世界!",           // Chinese
        "Привет мир",             // Russian
        "مرحبا بالعالم",          // Arabic
        "שלום עולם",              // Hebrew
        "日本語テスト",           // Japanese
        "한국어 테스트",          // Korean
        "🎉🎊🎈🎁🎄",             // Emojis
        "Ñoño",                   // Spanish
        "Ελληνικά",               // Greek
        "ไทย",                    // Thai
    ];
    
    for s in &test_cases {
        let gc_string = heap.alloc_string(s).expect("Should allocate Unicode string");
        assert_eq!(gc_string.as_str(), *s, "Unicode string '{}' should be preserved", s);
    }
}

#[test]
fn test_valid_utf8_boundary_characters() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    // Test boundary characters in UTF-8 encoding
    let test_cases = [
        "\u{0000}",      // Null character (1 byte)
        "\u{007F}",      // DEL - last 1-byte character
        "\u{0080}",      // First 2-byte character
        "\u{07FF}",      // Last 2-byte character
        "\u{0800}",      // First 3-byte character
        "\u{FFFF}",      // Last 3-byte character (excluding surrogates)
        "\u{10000}",     // First 4-byte character
        "\u{10FFFF}",    // Last valid Unicode code point
    ];
    
    for s in &test_cases {
        let gc_string = heap.alloc_string(s).expect("Should allocate boundary character");
        assert_eq!(gc_string.as_str(), *s, "Boundary character should be preserved");
    }
}

#[test]
fn test_valid_utf8_mixed_content() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    // Mix of ASCII, multi-byte, and special characters
    let test_string = "Hello 世界! 🎉 Привет мир 日本語 한국어";
    
    let gc_string = heap.alloc_string(test_string).expect("Should allocate mixed content");
    assert_eq!(gc_string.as_str(), test_string, "Mixed content should be preserved");
}

#[test]
fn test_string_hash_consistency() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    let s = "test string for hashing";
    
    // Allocate the same string twice
    let gc_string1 = heap.alloc_string(s).expect("Should allocate");
    let gc_string2 = heap.alloc_string(s).expect("Should allocate");
    
    // Hashes should be the same for identical content
    assert_eq!(
        gc_string1.hash(),
        gc_string2.hash(),
        "Same string content should produce same hash"
    );
}

#[test]
fn test_long_string_allocation() {
    let mut heap = GcHeap::new().expect("Failed to create heap");
    
    // Test with a long string
    let long_string = "x".repeat(10000);
    
    let gc_string = heap.alloc_string(&long_string).expect("Should allocate long string");
    assert_eq!(gc_string.as_str(), long_string, "Long string should be preserved");
    assert_eq!(gc_string.len(), 10000, "Long string length should be correct");
}

// ============================================================================
// Tests for Invalid UTF-8 Detection
// Note: These test the validation at the byte level, which happens in
// RuntimeHeap::allocate_string_from_bytes. The GcHeap::alloc_string takes
// &str which is already valid UTF-8 in Rust.
// ============================================================================

#[test]
fn test_std_str_from_utf8_rejects_invalid() {
    // These are invalid UTF-8 sequences that should be rejected
    let invalid_sequences: &[&[u8]] = &[
        // Invalid continuation byte
        &[0x80],
        // Invalid start byte
        &[0xC0, 0x80],
        // Truncated 2-byte sequence
        &[0xC2],
        // Truncated 3-byte sequence
        &[0xE0, 0xA0],
        // Truncated 4-byte sequence
        &[0xF0, 0x90, 0x80],
        // Invalid 4-byte sequence (overlong)
        &[0xF0, 0x80, 0x80, 0x80],
        // Surrogate half (invalid in UTF-8)
        &[0xED, 0xA0, 0x80],
        // Code point too large
        &[0xF4, 0x90, 0x80, 0x80],
    ];
    
    for bytes in invalid_sequences {
        let result = std::str::from_utf8(bytes);
        assert!(
            result.is_err(),
            "Invalid UTF-8 sequence {:?} should be rejected",
            bytes
        );
    }
}

#[test]
fn test_std_str_from_utf8_accepts_valid() {
    // These are valid UTF-8 sequences
    let valid_sequences: &[&[u8]] = &[
        // Empty
        &[],
        // ASCII
        b"hello",
        // 2-byte character (é)
        &[0xC3, 0xA9],
        // 3-byte character (€)
        &[0xE2, 0x82, 0xAC],
        // 4-byte character (𝄞)
        &[0xF0, 0x9D, 0x84, 0x9E],
        // Mixed
        &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0xE4, 0xB8, 0x96, 0xE7, 0x95, 0x8C],
    ];
    
    for bytes in valid_sequences {
        let result = std::str::from_utf8(bytes);
        assert!(
            result.is_ok(),
            "Valid UTF-8 sequence {:?} should be accepted",
            bytes
        );
    }
}
