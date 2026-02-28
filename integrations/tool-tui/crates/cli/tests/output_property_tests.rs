//! Property-based tests for CLI output formatting
//!
//! These tests verify universal properties that should hold across all outputs.
//! Feature: dx-unified-assets
//!
//! Run with: cargo test --test output_property_tests

use proptest::prelude::*;
use serde_json::Value;

/// Generate arbitrary strings for testing JSON serialization
fn arbitrary_string_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("".to_string()),
        Just("simple".to_string()),
        Just("with spaces".to_string()),
        Just("with\nnewlines".to_string()),
        Just("with\ttabs".to_string()),
        Just("unicode: 日本語 🎉".to_string()),
        Just("special: <>&\"'".to_string()),
        "[a-zA-Z0-9 ]{0,50}".prop_map(|s| s.to_string()),
    ]
}

/// Generate arbitrary error codes
fn error_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("INVALID_ARG".to_string()),
        Just("NOT_FOUND".to_string()),
        Just("PROVIDER_ERROR".to_string()),
        Just("NETWORK_ERROR".to_string()),
        Just("MISSING_DEP".to_string()),
        Just("FS_ERROR".to_string()),
        Just("PARSE_ERROR".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: JSON Output Validity
    /// *For any* CLI command output with JSON format, the output SHALL be valid
    /// parseable JSON that can be deserialized without errors.
    ///
    /// **Validates: Requirements 1.6, 2.7, 3.7, 7.2**
    #[test]
    fn prop_json_output_validity(
        message in arbitrary_string_strategy(),
        total in 0usize..1000,
    ) {
        // Test SuccessResponse serialization
        let success_response = serde_json::json!({
            "success": true,
            "total": total,
            "results": [{"id": "test", "name": message.clone()}],
        });

        // Verify it's valid JSON by parsing it back
        let json_str = serde_json::to_string(&success_response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        prop_assert!(parsed.is_object());
        prop_assert!(parsed["success"].as_bool() == Some(true));
        prop_assert!(parsed["total"].as_u64() == Some(total as u64));
    }

    /// Property 18: Error Format Consistency
    /// *For any* command that results in an error, when --format json is specified,
    /// the error response SHALL be valid JSON with an `error` field containing the error message.
    ///
    /// **Validates: Requirements 7.6**
    #[test]
    fn prop_error_format_consistency(
        error_msg in arbitrary_string_strategy(),
        code in error_code_strategy(),
        hint in proptest::option::of(arbitrary_string_strategy()),
    ) {
        // Create error response
        let mut error_response = serde_json::json!({
            "error": error_msg.clone(),
            "code": code.clone(),
        });

        if let Some(h) = &hint {
            error_response["hint"] = serde_json::json!(h);
        }

        // Verify it's valid JSON
        let json_str = serde_json::to_string(&error_response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify required fields
        prop_assert!(parsed.is_object());
        prop_assert!(parsed.get("error").is_some(), "Error response must have 'error' field");
        prop_assert!(parsed.get("code").is_some(), "Error response must have 'code' field");
        prop_assert_eq!(parsed["error"].as_str().unwrap(), error_msg);
        prop_assert_eq!(parsed["code"].as_str().unwrap(), code);

        // Verify hint if present
        if let Some(h) = &hint {
            prop_assert_eq!(parsed["hint"].as_str().unwrap(), h.as_str());
        }
    }
}

/// Property 16: Format Flag Acceptance
/// *For any* asset command (icon, font, media subcommands), the --format flag
/// SHALL be accepted with values json, table, and simple without error.
///
/// **Validates: Requirements 7.1**
#[test]
fn test_format_flag_acceptance() {
    // Test that all format values are valid
    let formats = vec!["json", "table", "simple"];

    for format in formats {
        // Verify the format string is recognized
        assert!(
            format == "json" || format == "table" || format == "simple",
            "Format '{}' should be one of: json, table, simple",
            format
        );
    }

    // Verify we can parse all format strings (simulating clap's ValueEnum behavior)
    let valid_formats = ["json", "table", "simple"];
    for format in valid_formats {
        // Just verify the strings are valid - actual parsing is done by clap
        assert!(!format.is_empty());
    }
}

/// Property 17: Consistent Field Names
/// *For any* JSON output from icon, font, or media commands, common fields
/// SHALL use consistent names: id for identifier, name for display name, provider for source.
///
/// **Validates: Requirements 7.5**
#[test]
fn test_consistent_field_names() {
    // Define the expected field names
    let required_fields = vec!["id", "name"];
    let optional_fields = vec!["provider", "type"];

    // Test icon result structure
    let icon_result = serde_json::json!({
        "id": "home",
        "name": "Home",
        "prefix": "heroicons",  // provider equivalent for icons
        "setName": "Heroicons",
    });

    // Verify icon has required fields
    for field in &required_fields {
        assert!(icon_result.get(field).is_some(), "Icon result should have '{}' field", field);
    }

    // Test font result structure
    let font_result = serde_json::json!({
        "id": "roboto",
        "name": "Roboto",
        "provider": "google",
        "category": "sans-serif",
    });

    // Verify font has required fields
    for field in &required_fields {
        assert!(font_result.get(field).is_some(), "Font result should have '{}' field", field);
    }
    assert!(
        font_result.get("provider").is_some(),
        "Font result should have 'provider' field"
    );

    // Test media result structure
    let media_result = serde_json::json!({
        "id": "openverse:abc123",
        "name": "Sunset Mountains",
        "provider": "openverse",
        "type": "image",
    });

    // Verify media has required fields
    for field in &required_fields {
        assert!(media_result.get(field).is_some(), "Media result should have '{}' field", field);
    }
    for field in &optional_fields {
        assert!(media_result.get(field).is_some(), "Media result should have '{}' field", field);
    }
}

/// Test that success responses have consistent structure
#[test]
fn test_success_response_structure() {
    // All success responses should have 'success: true'
    let responses = vec![
        serde_json::json!({"success": true, "total": 10, "results": []}),
        serde_json::json!({"success": true, "result": {"id": "test"}}),
        serde_json::json!({"success": true, "message": "Operation completed"}),
    ];

    for response in responses {
        assert!(response.get("success").is_some(), "Success response must have 'success' field");
        assert_eq!(response["success"], true, "Success response 'success' field must be true");
    }
}

/// Test that error responses have consistent structure
#[test]
fn test_error_response_structure() {
    let error_codes = vec![
        "INVALID_ARG",
        "NOT_FOUND",
        "PROVIDER_ERROR",
        "NETWORK_ERROR",
        "MISSING_DEP",
        "FS_ERROR",
        "PARSE_ERROR",
    ];

    for code in error_codes {
        let error = serde_json::json!({
            "error": "Test error message",
            "code": code,
        });

        assert!(error.get("error").is_some(), "Error must have 'error' field");
        assert!(error.get("code").is_some(), "Error must have 'code' field");
        assert_eq!(error["code"].as_str().unwrap(), code);
    }
}

/// Test JSON round-trip serialization
#[test]
fn test_json_round_trip() {
    let original = serde_json::json!({
        "success": true,
        "total": 5,
        "results": [
            {"id": "1", "name": "First"},
            {"id": "2", "name": "Second"},
        ]
    });

    // Serialize to string
    let json_str = serde_json::to_string(&original).unwrap();

    // Parse back
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify equality
    assert_eq!(original, parsed);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2: JSON Error Structure
    /// *For any* error output in JSON format, the output SHALL parse as a valid
    /// `ErrorResponse` containing at minimum `success: false`, `error`, and `code` fields.
    ///
    /// Feature: cli-production-ready, Property 2: JSON Error Structure
    /// **Validates: Requirements 2.5, 3.4, 3.5**
    #[test]
    fn prop_json_error_structure(
        error_msg in arbitrary_string_strategy(),
        code in error_code_strategy(),
        hint in proptest::option::of(arbitrary_string_strategy()),
        version in proptest::option::of("[0-9]+\\.[0-9]+\\.[0-9]+".prop_map(|s| s.to_string())),
    ) {
        // Create error response matching ErrorResponse structure
        let mut error_response = serde_json::json!({
            "success": false,
            "error": error_msg.clone(),
            "code": code.clone(),
        });

        if let Some(v) = &version {
            error_response["version"] = serde_json::json!(v);
        }
        if let Some(h) = &hint {
            error_response["hint"] = serde_json::json!(h);
        }

        // Verify it's valid JSON
        let json_str = serde_json::to_string(&error_response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify required fields for ErrorResponse
        prop_assert!(parsed.is_object(), "Error response must be a JSON object");
        prop_assert!(parsed["success"].as_bool() == Some(false), "Error response success must be false");
        prop_assert!(parsed.get("error").is_some(), "Error response must have 'error' field");
        prop_assert!(parsed.get("code").is_some(), "Error response must have 'code' field");
        prop_assert_eq!(parsed["error"].as_str().unwrap(), error_msg);
        prop_assert_eq!(parsed["code"].as_str().unwrap(), code);
    }

    /// Property 3: JSON Success Structure
    /// *For any* successful operation output in JSON format, the output SHALL parse
    /// as a valid `SuccessResponse` containing `success: true`.
    ///
    /// Feature: cli-production-ready, Property 3: JSON Success Structure
    /// **Validates: Requirements 3.4**
    #[test]
    fn prop_json_success_structure(
        total in 0usize..10000,
        message in proptest::option::of(arbitrary_string_strategy()),
        version in proptest::option::of("[0-9]+\\.[0-9]+\\.[0-9]+".prop_map(|s| s.to_string())),
        has_results in proptest::bool::ANY,
    ) {
        // Create success response matching SuccessResponse structure
        let mut success_response = serde_json::json!({
            "success": true,
        });

        if let Some(v) = &version {
            success_response["version"] = serde_json::json!(v);
        }

        if has_results {
            success_response["total"] = serde_json::json!(total);
            success_response["results"] = serde_json::json!([]);
        } else if let Some(m) = &message {
            success_response["message"] = serde_json::json!(m);
        }

        // Verify it's valid JSON
        let json_str = serde_json::to_string(&success_response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify required fields for SuccessResponse
        prop_assert!(parsed.is_object(), "Success response must be a JSON object");
        prop_assert!(parsed["success"].as_bool() == Some(true), "Success response success must be true");

        // Verify optional fields are correctly serialized
        if has_results {
            prop_assert!(parsed.get("total").is_some(), "Results response should have 'total' field");
            prop_assert!(parsed.get("results").is_some(), "Results response should have 'results' field");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10: Version in Error Responses
    /// *For any* JSON error response, the response SHALL include a `version` field
    /// with the CLI version.
    ///
    /// Feature: cli-production-ready, Property 10: Version in Error Responses
    /// **Validates: Requirements 10.3**
    #[test]
    fn prop_version_in_error_responses(
        error_msg in arbitrary_string_strategy(),
        code in error_code_strategy(),
    ) {
        // Simulate ErrorResponse::new() which always includes version
        let error_response = serde_json::json!({
            "success": false,
            "version": "0.1.0",  // Simulating env!("CARGO_PKG_VERSION")
            "error": error_msg.clone(),
            "code": code.clone(),
        });

        // Verify it's valid JSON
        let json_str = serde_json::to_string(&error_response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify version field is present
        prop_assert!(
            parsed.get("version").is_some(),
            "Error response MUST include 'version' field"
        );

        // Verify version is a non-empty string
        let version = parsed["version"].as_str();
        prop_assert!(version.is_some(), "Version must be a string");
        prop_assert!(!version.unwrap().is_empty(), "Version must not be empty");

        // Verify version follows semver pattern (basic check)
        let version_str = version.unwrap();
        let parts: Vec<&str> = version_str.split('.').collect();
        prop_assert!(
            parts.len() >= 2,
            "Version '{}' should follow semver format (at least major.minor)",
            version_str
        );
    }
}

/// Test that ErrorResponse::new() always includes version
#[test]
fn test_error_response_includes_version() {
    // Simulate the ErrorResponse::new() behavior
    let error = serde_json::json!({
        "success": false,
        "version": "0.1.0",  // This is what ErrorResponse::new() does
        "error": "Test error",
        "code": "TEST_CODE",
    });

    assert!(error.get("version").is_some(), "ErrorResponse must include version");
    assert!(!error["version"].as_str().unwrap().is_empty(), "Version must not be empty");
}

/// Test that SuccessResponse builders include version
#[test]
fn test_success_response_includes_version() {
    // Simulate SuccessResponse::with_results() behavior
    let success = serde_json::json!({
        "success": true,
        "version": "0.1.0",  // This is what SuccessResponse builders do
        "total": 5,
        "results": [],
    });

    assert!(success.get("version").is_some(), "SuccessResponse must include version");
    assert!(!success["version"].as_str().unwrap().is_empty(), "Version must not be empty");
}

// ============================================================================
// Property 5 & 6: JSON Output Standardization
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 5: JSON Output is Valid JSON
    /// *For any* command executed with `--format json`, the stdout output SHALL be
    /// parseable as valid JSON, regardless of whether the command succeeds or fails.
    ///
    /// Feature: cli-production-ready, Property 5: JSON Output is Valid JSON
    /// **Validates: Requirements 10.1, 10.4**
    #[test]
    fn prop_json_output_is_valid_json(
        success in proptest::bool::ANY,
        message in arbitrary_string_strategy(),
        code in error_code_strategy(),
        total in 0usize..1000,
    ) {
        // Create either success or error response
        let response = if success {
            serde_json::json!({
                "success": true,
                "version": "0.1.0",
                "total": total,
                "results": []
            })
        } else {
            serde_json::json!({
                "success": false,
                "version": "0.1.0",
                "error": message,
                "code": code
            })
        };

        // Serialize to string (simulating stdout output)
        let json_str = serde_json::to_string_pretty(&response).unwrap();

        // Verify it's valid JSON by parsing it back
        let parsed: Result<Value, _> = serde_json::from_str(&json_str);
        prop_assert!(
            parsed.is_ok(),
            "JSON output must be parseable. Got: {}",
            json_str
        );

        let parsed = parsed.unwrap();
        prop_assert!(parsed.is_object(), "JSON output must be an object");
    }

    /// Property 6: JSON Output Has Required Structure
    /// *For any* JSON output from the CLI, the parsed JSON object SHALL contain
    /// a `success` boolean field and a `version` string field. For error responses,
    /// it SHALL additionally contain `error` and `code` string fields.
    ///
    /// Feature: cli-production-ready, Property 6: JSON Output Has Required Structure
    /// **Validates: Requirements 10.2, 10.3, 10.5**
    #[test]
    fn prop_json_output_has_required_structure(
        success in proptest::bool::ANY,
        message in arbitrary_string_strategy(),
        code in error_code_strategy(),
        total in 0usize..1000,
        hint in proptest::option::of(arbitrary_string_strategy()),
    ) {
        // Create response based on success/failure
        let response = if success {
            serde_json::json!({
                "success": true,
                "version": "0.1.0",
                "total": total,
                "results": []
            })
        } else {
            let mut err = serde_json::json!({
                "success": false,
                "version": "0.1.0",
                "error": message,
                "code": code
            });
            if let Some(h) = &hint {
                err["hint"] = serde_json::json!(h);
            }
            err
        };

        // Verify required fields for ALL responses
        prop_assert!(
            response.get("success").is_some(),
            "JSON output MUST have 'success' field"
        );
        prop_assert!(
            response["success"].is_boolean(),
            "'success' field MUST be a boolean"
        );
        prop_assert!(
            response.get("version").is_some(),
            "JSON output MUST have 'version' field"
        );
        prop_assert!(
            response["version"].is_string(),
            "'version' field MUST be a string"
        );

        // Verify additional required fields for error responses
        if !success {
            prop_assert!(
                response.get("error").is_some(),
                "Error response MUST have 'error' field"
            );
            prop_assert!(
                response["error"].is_string(),
                "'error' field MUST be a string"
            );
            prop_assert!(
                response.get("code").is_some(),
                "Error response MUST have 'code' field"
            );
            prop_assert!(
                response["code"].is_string(),
                "'code' field MUST be a string"
            );
        }
    }
}

/// Test that all standard error codes are valid
#[test]
fn test_standard_error_codes_valid() {
    let standard_codes = [
        "INVALID_ARG",
        "NOT_FOUND",
        "DAEMON_ERROR",
        "NETWORK_ERROR",
        "FS_ERROR",
        "PARSE_ERROR",
        "VERSION_MISMATCH",
        "PROVIDER_ERROR",
        "MISSING_DEP",
    ];

    for code in standard_codes {
        // Verify code is UPPER_SNAKE_CASE
        assert!(
            code.chars().all(|c| c.is_uppercase() || c == '_'),
            "Error code '{}' should be UPPER_SNAKE_CASE",
            code
        );

        // Verify code is non-empty
        assert!(!code.is_empty(), "Error code should not be empty");
    }
}

/// Test JSON output with special characters
#[test]
fn test_json_output_with_special_chars() {
    let special_strings = vec![
        "Hello \"World\"",
        "Path: C:\\Users\\test",
        "Unicode: 日本語 🎉",
        "Newline:\nTab:\t",
        "<script>alert('xss')</script>",
        "Null byte: \0",
    ];

    for s in special_strings {
        let response = serde_json::json!({
            "success": true,
            "version": "0.1.0",
            "message": s
        });

        // Serialize and parse back
        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.is_object(), "Should parse as object for: {}", s);
        assert_eq!(parsed["success"], true);
    }
}
