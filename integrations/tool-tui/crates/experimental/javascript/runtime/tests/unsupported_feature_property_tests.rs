//! Property-based tests for unsupported feature error clarity
//!
//! **Feature: production-readiness, Property 15: Unsupported Feature Error Clarity**
//! **Validates: Requirements 10.1, 10.5**
//!
//! For any unsupported JavaScript feature F or unsupported API option O, the thrown
//! error message SHALL explicitly name the unsupported feature or option, not just
//! report a generic error.

use dx_js_runtime::error::{unsupported_feature, unsupported_options, DxError};
use proptest::prelude::*;

/// Known unsupported features that should produce clear error messages
const UNSUPPORTED_FEATURES: &[(&str, &str, &str)] = &[
    (
        "RegExp literals",
        "Regular expression literals are not yet supported",
        "Use RegExp constructor",
    ),
    (
        "optional chaining",
        "Optional chaining (e.g., obj?.prop) is not yet supported",
        "Use explicit null checks",
    ),
    (
        "class expressions",
        "Class expressions are not yet supported",
        "Use class declarations",
    ),
    (
        "dynamic import",
        "Dynamic import() expressions are not yet supported",
        "Use static imports",
    ),
    (
        "tagged template literals",
        "Tagged template literals are not yet supported",
        "Use regular function calls",
    ),
    (
        "yield expressions",
        "Generator functions and yield are not yet supported",
        "Use async/await",
    ),
    (
        "decorators",
        "Stage 3 decorators are not yet supported",
        "Use higher-order functions",
    ),
    (
        "with statement",
        "The 'with' statement is not supported",
        "Use explicit property access",
    ),
];

/// Known APIs with unsupported options
const APIS_WITH_UNSUPPORTED_OPTIONS: &[(&str, &[&str], &[&str])] = &[
    ("fs.readFile", &["signal", "flag"], &["encoding", "mode"]),
    ("fs.writeFile", &["signal"], &["encoding", "mode", "flag"]),
    ("http.request", &["agent", "createConnection"], &["method", "headers", "path"]),
    ("crypto.createHash", &["outputLength"], &["algorithm"]),
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 15: Unsupported feature errors explicitly name the feature
    /// For any unsupported feature, the error message must contain the feature name
    #[test]
    fn prop_unsupported_feature_error_names_feature(
        feature_idx in 0usize..UNSUPPORTED_FEATURES.len()
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let (feature_name, description, suggestion) = UNSUPPORTED_FEATURES[feature_idx];

        let error = unsupported_feature(feature_name, description, suggestion);
        let error_message = error.to_string();

        // The error message must contain the feature name
        prop_assert!(
            error_message.contains(feature_name),
            "Error message must contain feature name '{}', got: {}",
            feature_name,
            error_message
        );

        // The error message must contain "Unsupported feature"
        prop_assert!(
            error_message.contains("Unsupported feature"),
            "Error message must indicate it's an unsupported feature, got: {}",
            error_message
        );
    }

    /// Property 15: Unsupported feature errors include suggestions
    /// For any unsupported feature, the error message should include a suggestion
    #[test]
    fn prop_unsupported_feature_error_includes_suggestion(
        feature_idx in 0usize..UNSUPPORTED_FEATURES.len()
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let (feature_name, description, suggestion) = UNSUPPORTED_FEATURES[feature_idx];

        let error = unsupported_feature(feature_name, description, suggestion);
        let error_message = error.to_string();

        // The error message must contain "Suggestion"
        prop_assert!(
            error_message.contains("Suggestion"),
            "Error message must include a suggestion section, got: {}",
            error_message
        );
    }

    /// Property 15: Unsupported options errors list the unsupported options
    /// For any API with unsupported options, the error must list those options
    #[test]
    fn prop_unsupported_options_error_lists_options(
        api_idx in 0usize..APIS_WITH_UNSUPPORTED_OPTIONS.len()
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let (api_name, unsupported, supported) = APIS_WITH_UNSUPPORTED_OPTIONS[api_idx];

        let error = unsupported_options(api_name, unsupported, supported);
        let error_message = error.to_string();

        // The error message must contain the API name
        prop_assert!(
            error_message.contains(api_name),
            "Error message must contain API name '{}', got: {}",
            api_name,
            error_message
        );

        // The error message must contain "Unsupported option"
        prop_assert!(
            error_message.contains("Unsupported option"),
            "Error message must indicate unsupported options, got: {}",
            error_message
        );

        // The error message must list each unsupported option
        for option in unsupported.iter() {
            prop_assert!(
                error_message.contains(option),
                "Error message must list unsupported option '{}', got: {}",
                option,
                error_message
            );
        }
    }

    /// Property 15: Unsupported options errors list supported alternatives
    /// For any API with unsupported options, the error should list supported options
    #[test]
    fn prop_unsupported_options_error_lists_supported(
        api_idx in 0usize..APIS_WITH_UNSUPPORTED_OPTIONS.len()
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let (api_name, unsupported, supported) = APIS_WITH_UNSUPPORTED_OPTIONS[api_idx];

        let error = unsupported_options(api_name, unsupported, supported);
        let error_message = error.to_string();

        // The error message must contain "Supported options"
        prop_assert!(
            error_message.contains("Supported options"),
            "Error message must list supported options, got: {}",
            error_message
        );

        // The error message should list each supported option
        for option in supported.iter() {
            prop_assert!(
                error_message.contains(option),
                "Error message should list supported option '{}', got: {}",
                option,
                error_message
            );
        }
    }

    /// Property 15: Random feature names produce clear errors
    /// For any randomly generated feature name, the error must still be clear
    #[test]
    fn prop_random_feature_names_produce_clear_errors(
        feature_name in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
        description in "[a-zA-Z ]{10,50}",
        suggestion in "[a-zA-Z ]{10,30}"
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let error = unsupported_feature(&feature_name, &description, &suggestion);
        let error_message = error.to_string();

        // The error message must contain the feature name
        prop_assert!(
            error_message.contains(&feature_name),
            "Error message must contain feature name '{}', got: {}",
            feature_name,
            error_message
        );

        // The error message must be a SyntaxError (per Requirement 10.1)
        prop_assert!(
            error_message.contains("SyntaxError"),
            "Unsupported feature error must be a SyntaxError, got: {}",
            error_message
        );
    }

    /// Property 15: Random API names with options produce clear errors
    /// For any randomly generated API and options, the error must be clear
    #[test]
    fn prop_random_api_options_produce_clear_errors(
        api_name in "[a-zA-Z][a-zA-Z0-9.]{2,20}",
        unsupported_count in 1usize..5,
        supported_count in 0usize..5
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        // Generate option names
        let unsupported: Vec<&str> = (0..unsupported_count)
            .map(|i| match i {
                0 => "optionA",
                1 => "optionB",
                2 => "optionC",
                3 => "optionD",
                _ => "optionE",
            })
            .collect();

        let supported: Vec<&str> = (0..supported_count)
            .map(|i| match i {
                0 => "supportedA",
                1 => "supportedB",
                2 => "supportedC",
                3 => "supportedD",
                _ => "supportedE",
            })
            .collect();

        let error = unsupported_options(&api_name, &unsupported, &supported);
        let error_message = error.to_string();

        // The error message must contain the API name
        prop_assert!(
            error_message.contains(&api_name),
            "Error message must contain API name '{}', got: {}",
            api_name,
            error_message
        );

        // The error message must be a TypeError (per Requirement 10.5)
        prop_assert!(
            error_message.contains("TypeError"),
            "Unsupported options error must be a TypeError, got: {}",
            error_message
        );
    }

    /// Property 15: DxError::UnsupportedFeature variant produces correct format
    #[test]
    fn prop_dx_error_unsupported_feature_format(
        feature in "[a-zA-Z][a-zA-Z0-9_]{2,15}",
        desc in "[a-zA-Z ]{5,30}",
        sugg in "[a-zA-Z ]{5,20}"
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let error = DxError::unsupported_feature(&feature, &desc, &sugg);
        let error_message = error.to_string();

        // Must follow format: "SyntaxError: Unsupported feature '{feature}'"
        let expected_prefix = format!("SyntaxError: Unsupported feature '{}'", feature);
        prop_assert!(
            error_message.starts_with(&expected_prefix),
            "Error must start with '{}', got: {}",
            expected_prefix,
            error_message
        );
    }

    /// Property 15: DxError::UnsupportedOptions variant produces correct format
    #[test]
    fn prop_dx_error_unsupported_options_format(
        api in "[a-zA-Z][a-zA-Z0-9.]{2,15}"
    ) {
        // Feature: production-readiness, Property 15: Unsupported Feature Error Clarity
        // Validates: Requirements 10.1, 10.5

        let unsupported = &["opt1", "opt2"];
        let supported = &["sup1", "sup2"];

        let error = DxError::unsupported_options(&api, unsupported, supported);
        let error_message = error.to_string();

        // Must follow format: "TypeError: Unsupported option(s) for '{api}'"
        let expected_prefix = format!("TypeError: Unsupported option(s) for '{}'", api);
        prop_assert!(
            error_message.starts_with(&expected_prefix),
            "Error must start with '{}', got: {}",
            expected_prefix,
            error_message
        );
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_unsupported_feature_error_format() {
    let error = unsupported_feature(
        "decorators",
        "Stage 3 decorators are not yet supported",
        "Use higher-order functions instead",
    );

    let message = error.to_string();

    assert!(message.contains("SyntaxError"), "Should be a SyntaxError");
    assert!(message.contains("Unsupported feature"), "Should indicate unsupported feature");
    assert!(message.contains("decorators"), "Should name the feature");
    assert!(message.contains("Suggestion"), "Should include suggestion");
}

#[test]
fn test_unsupported_options_error_format() {
    let error = unsupported_options("fs.readFile", &["signal", "flag"], &["encoding", "mode"]);

    let message = error.to_string();

    assert!(message.contains("TypeError"), "Should be a TypeError");
    assert!(message.contains("Unsupported option"), "Should indicate unsupported options");
    assert!(message.contains("fs.readFile"), "Should name the API");
    assert!(message.contains("signal"), "Should list unsupported option 'signal'");
    assert!(message.contains("flag"), "Should list unsupported option 'flag'");
    assert!(message.contains("Supported options"), "Should list supported options");
    assert!(message.contains("encoding"), "Should list supported option 'encoding'");
}

#[test]
fn test_unsupported_options_with_no_supported() {
    let error = unsupported_options("some.api", &["badOption"], &[]);

    let message = error.to_string();

    assert!(message.contains("some.api"), "Should name the API");
    assert!(message.contains("badOption"), "Should list the unsupported option");
    assert!(
        message.contains("none") || message.contains("Supported options"),
        "Should handle empty supported options"
    );
}

#[test]
fn test_unsupported_feature_special_characters() {
    // Test that special characters in feature names are handled correctly
    let error = unsupported_feature(
        "private field 'in' operator",
        "The 'in' operator for private fields is not supported",
        "Use try-catch patterns",
    );

    let message = error.to_string();

    assert!(
        message.contains("private field 'in' operator"),
        "Should handle special characters in feature name"
    );
}

#[test]
fn test_dx_error_unsupported_feature_variant() {
    let error = DxError::UnsupportedFeature {
        feature: "test_feature".to_string(),
        description: "Test description".to_string(),
        suggestion: "Test suggestion".to_string(),
    };

    let message = error.to_string();

    assert!(message.contains("test_feature"));
    assert!(message.contains("Test description"));
    assert!(message.contains("Test suggestion"));
}

#[test]
fn test_dx_error_unsupported_options_variant() {
    let error = DxError::UnsupportedOptions {
        api: "test.api".to_string(),
        options: "opt1, opt2".to_string(),
        supported: "sup1, sup2".to_string(),
    };

    let message = error.to_string();

    assert!(message.contains("test.api"));
    assert!(message.contains("opt1, opt2"));
    assert!(message.contains("sup1, sup2"));
}
