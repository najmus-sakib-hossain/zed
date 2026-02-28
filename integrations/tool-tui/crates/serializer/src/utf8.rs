//! UTF-8 validation utilities for dx-serializer
//!
//! This module provides UTF-8 validation functions that return detailed
//! error information including byte offsets for invalid sequences.

use crate::error::{DxError, Result};

/// Validate that a byte slice is valid UTF-8
///
/// Returns the validated string slice on success, or a `DxError::Utf8Error`
/// with the byte offset of the first invalid sequence on failure.
///
/// # Examples
///
/// ```
/// use serializer::utf8::validate_utf8;
///
/// // Valid UTF-8
/// let valid = b"Hello, World!";
/// assert!(validate_utf8(valid).is_ok());
///
/// // Invalid UTF-8 (invalid continuation byte)
/// let invalid = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x80]; // "Hello" + invalid byte
/// let err = validate_utf8(invalid).unwrap_err();
/// // Error contains offset 5 (position of invalid byte)
/// ```
pub fn validate_utf8(bytes: &[u8]) -> Result<&str> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Err(DxError::Utf8Error {
            offset: e.valid_up_to(),
        }),
    }
}

/// Validate UTF-8 and return owned String
///
/// Converts a byte vector to String, returning detailed error information
/// if the bytes are not valid UTF-8.
pub fn validate_utf8_owned(bytes: Vec<u8>) -> Result<String> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Err(DxError::Utf8Error {
            offset: e.utf8_error().valid_up_to(),
        }),
    }
}

/// Validate UTF-8 with detailed error information
///
/// Returns additional context about the invalid sequence including
/// the invalid byte value and expected continuation information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8ValidationError {
    /// Byte offset where the error occurred
    pub offset: usize,
    /// The invalid byte value (if available)
    pub invalid_byte: Option<u8>,
    /// Human-readable description of the error
    pub description: String,
}

impl Utf8ValidationError {
    /// Create a new validation error
    pub fn new(offset: usize, invalid_byte: Option<u8>, description: impl Into<String>) -> Self {
        Self {
            offset,
            invalid_byte,
            description: description.into(),
        }
    }
}

/// Validate UTF-8 with detailed error information
///
/// This function provides more detailed error information than `validate_utf8`,
/// including the specific invalid byte and a description of why it's invalid.
pub fn validate_utf8_detailed(bytes: &[u8]) -> std::result::Result<&str, Utf8ValidationError> {
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];

        // Determine expected sequence length from first byte
        let seq_len = if byte & 0x80 == 0 {
            // ASCII: 0xxxxxxx
            1
        } else if byte & 0xE0 == 0xC0 {
            // 2-byte: 110xxxxx
            2
        } else if byte & 0xF0 == 0xE0 {
            // 3-byte: 1110xxxx
            3
        } else if byte & 0xF8 == 0xF0 {
            // 4-byte: 11110xxx
            4
        } else if byte & 0xC0 == 0x80 {
            // Unexpected continuation byte
            return Err(Utf8ValidationError::new(
                i,
                Some(byte),
                format!("Unexpected continuation byte 0x{:02X} at position {}", byte, i),
            ));
        } else {
            // Invalid start byte
            return Err(Utf8ValidationError::new(
                i,
                Some(byte),
                format!("Invalid UTF-8 start byte 0x{:02X} at position {}", byte, i),
            ));
        };

        // Check if we have enough bytes
        if i + seq_len > bytes.len() {
            return Err(Utf8ValidationError::new(
                i,
                Some(byte),
                format!(
                    "Incomplete UTF-8 sequence at position {}: expected {} bytes, got {}",
                    i,
                    seq_len,
                    bytes.len() - i
                ),
            ));
        }

        // Validate continuation bytes
        for j in 1..seq_len {
            let cont_byte = bytes[i + j];
            if cont_byte & 0xC0 != 0x80 {
                return Err(Utf8ValidationError::new(
                    i + j,
                    Some(cont_byte),
                    format!(
                        "Invalid continuation byte 0x{:02X} at position {} (expected 10xxxxxx)",
                        cont_byte,
                        i + j
                    ),
                ));
            }
        }

        // Additional validation for overlong encodings and invalid code points
        if seq_len == 2 {
            // 2-byte sequences must encode values >= 0x80
            let code_point = ((byte as u32 & 0x1F) << 6) | (bytes[i + 1] as u32 & 0x3F);
            if code_point < 0x80 {
                return Err(Utf8ValidationError::new(
                    i,
                    Some(byte),
                    format!(
                        "Overlong encoding at position {}: 2-byte sequence for code point U+{:04X}",
                        i, code_point
                    ),
                ));
            }
        } else if seq_len == 3 {
            // 3-byte sequences must encode values >= 0x800 and not be surrogates
            let code_point = ((byte as u32 & 0x0F) << 12)
                | ((bytes[i + 1] as u32 & 0x3F) << 6)
                | (bytes[i + 2] as u32 & 0x3F);
            if code_point < 0x800 {
                return Err(Utf8ValidationError::new(
                    i,
                    Some(byte),
                    format!(
                        "Overlong encoding at position {}: 3-byte sequence for code point U+{:04X}",
                        i, code_point
                    ),
                ));
            }
            if (0xD800..=0xDFFF).contains(&code_point) {
                return Err(Utf8ValidationError::new(
                    i,
                    Some(byte),
                    format!("Invalid surrogate code point U+{:04X} at position {}", code_point, i),
                ));
            }
        } else if seq_len == 4 {
            // 4-byte sequences must encode values >= 0x10000 and <= 0x10FFFF
            let code_point = ((byte as u32 & 0x07) << 18)
                | ((bytes[i + 1] as u32 & 0x3F) << 12)
                | ((bytes[i + 2] as u32 & 0x3F) << 6)
                | (bytes[i + 3] as u32 & 0x3F);
            if code_point < 0x10000 {
                return Err(Utf8ValidationError::new(
                    i,
                    Some(byte),
                    format!(
                        "Overlong encoding at position {}: 4-byte sequence for code point U+{:04X}",
                        i, code_point
                    ),
                ));
            }
            if code_point > 0x10FFFF {
                return Err(Utf8ValidationError::new(
                    i,
                    Some(byte),
                    format!(
                        "Code point U+{:04X} exceeds maximum U+10FFFF at position {}",
                        code_point, i
                    ),
                ));
            }
        }

        i += seq_len;
    }

    // All validation passed, safe to convert
    // SAFETY: We've validated all UTF-8 sequences manually
    Ok(unsafe { std::str::from_utf8_unchecked(bytes) })
}

/// Validate a string value during parsing
///
/// This is the main entry point for UTF-8 validation during parsing.
/// It validates the input and returns a DxError with byte offset on failure.
pub fn validate_string_input(bytes: &[u8], base_offset: usize) -> Result<&str> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Err(DxError::Utf8Error {
            offset: base_offset + e.valid_up_to(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ascii() {
        let input = b"Hello, World!";
        assert_eq!(validate_utf8(input).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_valid_utf8_multibyte() {
        // "Hello, 世界!" in UTF-8
        let input = "Hello, 世界!".as_bytes();
        assert_eq!(validate_utf8(input).unwrap(), "Hello, 世界!");
    }

    #[test]
    fn test_valid_utf8_emoji() {
        // Emoji (4-byte sequence)
        let input = "Hello 🌍!".as_bytes();
        assert_eq!(validate_utf8(input).unwrap(), "Hello 🌍!");
    }

    #[test]
    fn test_invalid_continuation_byte() {
        // Invalid continuation byte at position 5
        let input = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x80]; // "Hello" + 0x80
        let err = validate_utf8(input).unwrap_err();
        if let DxError::Utf8Error { offset } = err {
            assert_eq!(offset, 5);
        } else {
            panic!("Expected Utf8Error");
        }
    }

    #[test]
    fn test_invalid_start_byte() {
        // Invalid start byte 0xFF
        let input = &[0x48, 0x65, 0xFF, 0x6c, 0x6f]; // "He" + 0xFF + "lo"
        let err = validate_utf8(input).unwrap_err();
        if let DxError::Utf8Error { offset } = err {
            assert_eq!(offset, 2);
        } else {
            panic!("Expected Utf8Error");
        }
    }

    #[test]
    fn test_incomplete_sequence() {
        // Incomplete 2-byte sequence at end
        let input = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xC2]; // "Hello" + start of 2-byte
        let err = validate_utf8(input).unwrap_err();
        if let DxError::Utf8Error { offset } = err {
            assert_eq!(offset, 5);
        } else {
            panic!("Expected Utf8Error");
        }
    }

    #[test]
    fn test_detailed_validation_overlong() {
        // Overlong encoding of ASCII 'A' (0x41) as 2-byte sequence
        let input = &[0xC1, 0x81]; // Should be just 0x41
        let err = validate_utf8_detailed(input).unwrap_err();
        assert_eq!(err.offset, 0);
        assert!(err.description.contains("Overlong"));
    }

    #[test]
    fn test_detailed_validation_surrogate() {
        // UTF-16 surrogate (U+D800) encoded as UTF-8
        let input = &[0xED, 0xA0, 0x80]; // U+D800 (invalid in UTF-8)
        let err = validate_utf8_detailed(input).unwrap_err();
        assert_eq!(err.offset, 0);
        assert!(err.description.contains("surrogate"));
    }

    #[test]
    fn test_detailed_validation_too_large() {
        // Code point > U+10FFFF
        let input = &[0xF4, 0x90, 0x80, 0x80]; // U+110000 (invalid)
        let err = validate_utf8_detailed(input).unwrap_err();
        assert_eq!(err.offset, 0);
        assert!(err.description.contains("exceeds"));
    }

    #[test]
    fn test_validate_string_input_with_offset() {
        // Invalid byte at position 2 in the slice, but base_offset is 10
        let input = &[0x48, 0x65, 0xFF]; // "He" + invalid
        let err = validate_string_input(input, 10).unwrap_err();
        if let DxError::Utf8Error { offset } = err {
            assert_eq!(offset, 12); // 10 + 2
        } else {
            panic!("Expected Utf8Error");
        }
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(validate_utf8(b"").unwrap(), "");
    }

    #[test]
    fn test_validate_utf8_owned() {
        let valid = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        assert_eq!(validate_utf8_owned(valid).unwrap(), "Hello");

        let invalid = vec![0x48, 0x65, 0xFF]; // "He" + invalid
        let err = validate_utf8_owned(invalid).unwrap_err();
        if let DxError::Utf8Error { offset } = err {
            assert_eq!(offset, 2);
        } else {
            panic!("Expected Utf8Error");
        }
    }
}
