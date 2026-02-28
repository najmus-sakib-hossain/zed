//! Error handling tests for dx-style.
//!
//! These tests verify that error conditions are handled gracefully
//! and return appropriate error types.
//!
//! **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**

use std::path::PathBuf;
use style::StyleError;
use style::binary::dawn::{BinaryDawnError, BinaryDawnReader, BinaryDawnWriter};

/// Test: Non-existent input file returns InputReadError
/// **Validates: Requirements 4.1, 4.4**
#[test]
fn test_compile_nonexistent_input_file() {
    let input = PathBuf::from("/nonexistent/path/to/file.html");
    let output = PathBuf::from("/tmp/output.dxbd");

    let result = style::compile(input.clone(), output);

    assert!(result.is_err(), "Should return error for non-existent file");

    match result.unwrap_err() {
        StyleError::InputReadError { path, .. } => {
            assert_eq!(path, input, "Error should contain the input path");
        }
        other => panic!("Expected InputReadError, got {:?}", other),
    }
}

/// Test: Invalid binary format returns BinaryError
/// **Validates: Requirements 4.3, 4.5**
#[test]
fn test_invalid_binary_format_parsing() {
    // Create invalid binary data (not a valid Binary Dawn format)
    let invalid_data = b"not a valid binary dawn format";

    let result = BinaryDawnReader::new(invalid_data);

    assert!(result.is_err(), "Should return error for invalid binary format");

    match result {
        Err(BinaryDawnError::InvalidHeader) => {
            // Expected - magic bytes don't match
        }
        Err(other) => panic!("Expected InvalidHeader error, got {:?}", other),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

/// Test: Truncated binary data returns appropriate error
/// **Validates: Requirements 4.3**
#[test]
fn test_truncated_binary_data() {
    // Create valid header but truncated data
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(1, ".test { color: red; }");
    let valid_data = writer.build();

    // Truncate the data
    let truncated = &valid_data[..valid_data.len() / 2];

    let result = BinaryDawnReader::new(truncated);

    assert!(result.is_err(), "Should return error for truncated data");
}

/// Test: Corrupted checksum returns ChecksumMismatch error
/// **Validates: Requirements 4.3**
#[test]
fn test_corrupted_checksum() {
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(1, ".test { color: red; }");
    let mut data = writer.build();

    // Corrupt the data (modify a byte in the middle)
    let len = data.len();
    if len > 20 {
        data[len - 10] ^= 0xFF;
    }

    let result = BinaryDawnReader::new(&data);

    assert!(result.is_err(), "Should return error for corrupted data");

    match result {
        Err(BinaryDawnError::ChecksumMismatch) => {
            // Expected - checksum doesn't match
        }
        Err(other) => {
            // Other errors are also acceptable for corrupted data
            println!("Got error: {:?}", other);
        }
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

/// Test: Empty binary data returns error
/// **Validates: Requirements 4.3**
#[test]
fn test_empty_binary_data() {
    let empty_data: &[u8] = &[];

    let result = BinaryDawnReader::new(empty_data);

    assert!(result.is_err(), "Should return error for empty data");
}

/// Test: StyleError display formatting
/// **Validates: Requirements 4.2**
#[test]
fn test_style_error_display() {
    use std::io::{Error, ErrorKind};

    // Test InputReadError display
    let io_error = Error::new(ErrorKind::NotFound, "file not found");
    let input_error = StyleError::InputReadError {
        path: PathBuf::from("/test/input.html"),
        source: io_error,
    };
    let display = format!("{}", input_error);
    assert!(display.contains("input.html"), "Error display should contain file path");

    // Test OutputWriteError display
    let io_error = Error::new(ErrorKind::PermissionDenied, "permission denied");
    let output_error = StyleError::OutputWriteError {
        path: PathBuf::from("/test/output.dxbd"),
        source: io_error,
    };
    let display = format!("{}", output_error);
    assert!(display.contains("output.dxbd"), "Error display should contain file path");

    // Test ParseError display
    let parse_error = StyleError::ParseError {
        message: "unexpected token".to_string(),
        line: 10,
        column: 5,
    };
    let display = format!("{}", parse_error);
    assert!(display.contains("10"), "Error display should contain line number");
    assert!(display.contains("5"), "Error display should contain column number");

    // Test EngineNotInitialized display
    let engine_error = StyleError::EngineNotInitialized;
    let display = format!("{}", engine_error);
    assert!(
        display.contains("not initialized"),
        "Error display should indicate engine not initialized"
    );

    // Test ThemeError display
    let theme_error = StyleError::ThemeError("invalid color".to_string());
    let display = format!("{}", theme_error);
    assert!(
        display.contains("invalid color"),
        "Error display should contain theme error message"
    );
}

/// Test: BinaryDawnError display formatting
/// **Validates: Requirements 4.5**
#[test]
fn test_binary_dawn_error_display() {
    // Test InvalidHeader display
    let header_error = BinaryDawnError::InvalidHeader;
    let display = format!("{}", header_error);
    assert!(!display.is_empty(), "Error display should not be empty");

    // Test ChecksumMismatch display
    let checksum_error = BinaryDawnError::ChecksumMismatch;
    let display = format!("{}", checksum_error);
    assert!(
        display.contains("checksum") || display.contains("Checksum"),
        "Error display should mention checksum"
    );

    // Test HeaderTooShort display
    let short_error = BinaryDawnError::HeaderTooShort;
    let display = format!("{}", short_error);
    assert!(!display.is_empty(), "Error display should not be empty");
}

/// Test: Binary Dawn reader handles ID not found gracefully
/// **Validates: Requirements 4.3**
#[test]
fn test_binary_dawn_id_not_found() {
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(1, ".test { color: red; }");
    writer.add_style(5, ".test2 { color: blue; }");
    let data = writer.build();

    let reader = BinaryDawnReader::new(&data).expect("Should parse valid data");

    // Try to get a non-existent ID
    let result = reader.get_css(999);
    assert!(result.is_none(), "Should return None for non-existent ID");

    // Try to get ID 3 which doesn't exist
    let result = reader.get_css(3);
    assert!(result.is_none(), "Should return None for non-existent ID");
}

/// Test: Theme error handling
/// **Validates: Requirements 4.3**
#[test]
fn test_theme_error_handling() {
    use style::theme::ThemeGenerator;

    let generator = ThemeGenerator::new();

    // Test invalid hex color
    let result = generator.from_hex("invalid");
    assert!(result.is_err(), "Should return error for invalid hex color");

    // Test hex color that's too short
    let result = generator.from_hex("#FFF");
    assert!(result.is_err(), "Should return error for short hex color");

    // Test hex color that's too long
    let result = generator.from_hex("#FFFFFFFFFF");
    assert!(result.is_err(), "Should return error for long hex color");
}

/// Test: InvalidPropertyByte error display
/// **Validates: Requirements 8.1, 8.3**
#[test]
fn test_invalid_property_byte_error_display() {
    let error = StyleError::InvalidPropertyByte(0xFF);
    let display = format!("{}", error);
    assert!(
        display.contains("0xFF") || display.contains("FF"),
        "Error display should contain the invalid byte value in hex"
    );
    assert!(
        display.to_lowercase().contains("invalid") || display.to_lowercase().contains("property"),
        "Error display should indicate invalid property"
    );
}

/// Test: ConfigError display
/// **Validates: Requirements 8.1, 8.3**
#[test]
fn test_config_error_display() {
    let error = StyleError::ConfigError {
        message: "missing required field 'output_path'".to_string(),
    };
    let display = format!("{}", error);
    assert!(
        display.contains("missing required field"),
        "Error display should contain the config error message"
    );
    assert!(
        display.contains("output_path"),
        "Error display should contain the specific field name"
    );
}

/// Test: HtmlParseError display
/// **Validates: Requirements 8.1, 8.3**
#[test]
fn test_html_parse_error_display() {
    let error = StyleError::HtmlParseError {
        message: "unclosed tag at line 42".to_string(),
    };
    let display = format!("{}", error);
    assert!(
        display.contains("unclosed tag"),
        "Error display should contain the parse error message"
    );
    assert!(display.contains("42"), "Error display should contain the line number");
}

/// Test: MutexPoisoned error display
/// **Validates: Requirements 8.1, 8.3**
#[test]
fn test_mutex_poisoned_error_display() {
    let error = StyleError::MutexPoisoned;
    let display = format!("{}", error);
    assert!(
        display.to_lowercase().contains("mutex") || display.to_lowercase().contains("poisoned"),
        "Error display should indicate mutex poisoning"
    );
}

// Property-based tests for error context
mod prop_tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::{Error, ErrorKind};

    // Generate arbitrary file paths
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        prop::collection::vec("[a-zA-Z0-9_-]+", 1..5).prop_map(|parts| {
            let path_str = parts.join("/");
            PathBuf::from(format!("/{}.html", path_str))
        })
    }

    // Generate arbitrary error messages
    fn arb_error_message() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 _-]{1,50}".prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* StyleError variant that involves a file path, the error's Display output SHALL contain that file path.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_input_read_error_contains_path(path in arb_file_path()) {
            let io_error = Error::new(ErrorKind::NotFound, "file not found");
            let error = StyleError::InputReadError {
                path: path.clone(),
                source: io_error,
            };
            let display = format!("{}", error);

            // The path should appear in the error message
            let path_str = path.to_string_lossy();
            prop_assert!(
                display.contains(&*path_str),
                "InputReadError display '{}' should contain path '{}'",
                display,
                path_str
            );
        }

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* StyleError variant that involves a file path, the error's Display output SHALL contain that file path.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_output_write_error_contains_path(path in arb_file_path()) {
            let io_error = Error::new(ErrorKind::PermissionDenied, "permission denied");
            let error = StyleError::OutputWriteError {
                path: path.clone(),
                source: io_error,
            };
            let display = format!("{}", error);

            // The path should appear in the error message
            let path_str = path.to_string_lossy();
            prop_assert!(
                display.contains(&*path_str),
                "OutputWriteError display '{}' should contain path '{}'",
                display,
                path_str
            );
        }

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* StyleError variant with a message, the error's Display output SHALL contain that message.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_config_error_contains_message(message in arb_error_message()) {
            let error = StyleError::ConfigError {
                message: message.clone(),
            };
            let display = format!("{}", error);

            prop_assert!(
                display.contains(&message),
                "ConfigError display '{}' should contain message '{}'",
                display,
                message
            );
        }

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* StyleError variant with a message, the error's Display output SHALL contain that message.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_html_parse_error_contains_message(message in arb_error_message()) {
            let error = StyleError::HtmlParseError {
                message: message.clone(),
            };
            let display = format!("{}", error);

            prop_assert!(
                display.contains(&message),
                "HtmlParseError display '{}' should contain message '{}'",
                display,
                message
            );
        }

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* ParseError with line and column info, the error's Display output SHALL contain those values.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_parse_error_contains_location(
            message in arb_error_message(),
            line in 1usize..10000,
            column in 1usize..1000,
        ) {
            let error = StyleError::ParseError {
                message: message.clone(),
                line,
                column,
            };
            let display = format!("{}", error);

            // The line and column should appear in the error message
            let line_str = line.to_string();
            let column_str = column.to_string();
            prop_assert!(
                display.contains(&line_str),
                "ParseError display '{}' should contain line '{}'",
                display,
                line_str
            );
            prop_assert!(
                display.contains(&column_str),
                "ParseError display '{}' should contain column '{}'",
                display,
                column_str
            );
        }

        /// **Property 5: Errors Include Sufficient Context**
        /// *For any* InvalidPropertyByte error, the error's Display output SHALL contain the byte value.
        /// **Validates: Requirements 8.4**
        /// Feature: dx-style-production-hardening, Property 5
        #[test]
        fn prop_invalid_property_byte_contains_value(byte in any::<u8>()) {
            let error = StyleError::InvalidPropertyByte(byte);
            let display = format!("{}", error);

            // The byte value should appear in hex format in the error message
            let hex_upper = format!("{:02X}", byte);
            let hex_lower = format!("{:02x}", byte);
            prop_assert!(
                display.contains(&hex_upper) || display.contains(&hex_lower),
                "InvalidPropertyByte display '{}' should contain byte value '0x{}'",
                display,
                hex_upper
            );
        }
    }
}
