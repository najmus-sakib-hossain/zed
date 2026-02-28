//! Property-based tests for environment marker evaluation
//!
//! **Feature: dx-py-hardening, Property 4: Marker Evaluation Consistency**
//! **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

use dx_py_compat::markers::{MarkerEnvironment, MarkerEvaluator};
use proptest::prelude::*;

/// Strategy for generating Python versions
fn python_version_strategy() -> impl Strategy<Value = String> {
    (3u32..4, 6u32..15).prop_map(|(major, minor)| format!("{}.{}", major, minor))
}

/// Strategy for generating simple marker expressions
fn simple_marker_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Python version comparisons
        python_version_strategy().prop_map(|v| format!("python_version >= '{}'", v)),
        python_version_strategy().prop_map(|v| format!("python_version < '{}'", v)),
        python_version_strategy().prop_map(|v| format!("python_version == '{}'", v)),
        // Platform comparisons
        Just("sys_platform == 'win32'".to_string()),
        Just("sys_platform == 'linux'".to_string()),
        Just("sys_platform == 'darwin'".to_string()),
        Just("platform_system == 'Windows'".to_string()),
        Just("platform_system == 'Linux'".to_string()),
        Just("platform_system == 'Darwin'".to_string()),
        // Implementation
        Just("implementation_name == 'cpython'".to_string()),
        Just("implementation_name == 'pypy'".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: Marker Evaluation Consistency - Determinism**
    ///
    /// For any valid marker expression and environment, evaluating the marker
    /// SHALL produce a deterministic boolean result.
    ///
    /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
    #[test]
    fn prop_marker_evaluation_deterministic(
        marker in simple_marker_strategy(),
        python_version in python_version_strategy()
    ) {
        let env = MarkerEnvironment::current().with_python(&python_version);

        // Evaluate twice
        let result1 = MarkerEvaluator::evaluate(&marker, &env, &[]);
        let result2 = MarkerEvaluator::evaluate(&marker, &env, &[]);

        // Should be deterministic
        prop_assert_eq!(
            result1, result2,
            "Marker evaluation not deterministic for '{}' with Python {}",
            marker, python_version
        );
    }

    /// **Property 4: Marker Evaluation Consistency - Parse Roundtrip**
    ///
    /// For any parseable marker expression, parsing should succeed consistently.
    ///
    /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
    #[test]
    fn prop_marker_parse_consistency(marker in simple_marker_strategy()) {
        // Parse twice
        let result1 = MarkerEvaluator::parse(&marker);
        let result2 = MarkerEvaluator::parse(&marker);

        // Both should succeed or both should fail
        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Marker parsing not consistent for '{}'",
            marker
        );
    }

    /// **Property 4: Marker Evaluation Consistency - Boolean Logic**
    ///
    /// AND and OR operations should follow boolean logic rules.
    ///
    /// **Validates: Requirements 3.6**
    #[test]
    fn prop_marker_boolean_logic(
        python_version in python_version_strategy()
    ) {
        let env = MarkerEnvironment::current().with_python(&python_version);

        // Test AND: true AND x == x
        let always_true = "python_version >= '0.0'";
        let test_marker = "python_version >= '3.8'";
        let combined = format!("{} and {}", always_true, test_marker);

        let result_combined = MarkerEvaluator::evaluate(&combined, &env, &[]);
        let result_single = MarkerEvaluator::evaluate(test_marker, &env, &[]);

        prop_assert_eq!(
            result_combined, result_single,
            "AND with true should equal original: {} vs {}",
            combined, test_marker
        );

        // Test OR: false OR x == x
        let always_false = "python_version >= '99.0'";
        let combined_or = format!("{} or {}", always_false, test_marker);

        let result_or = MarkerEvaluator::evaluate(&combined_or, &env, &[]);

        prop_assert_eq!(
            result_or, result_single,
            "OR with false should equal original: {} vs {}",
            combined_or, test_marker
        );
    }

    /// **Property 4: Marker Evaluation Consistency - Version Ordering**
    ///
    /// Version comparisons should be consistent with ordering.
    ///
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_marker_version_ordering(
        v1 in 6u32..15,
        v2 in 6u32..15
    ) {
        let env = MarkerEnvironment::current().with_python(&format!("3.{}", v1));

        let gte_marker = format!("python_version >= '3.{}'", v2);
        let lt_marker = format!("python_version < '3.{}'", v2);

        let gte_result = MarkerEvaluator::evaluate(&gte_marker, &env, &[]);
        let lt_result = MarkerEvaluator::evaluate(&lt_marker, &env, &[]);

        // >= and < should be mutually exclusive and exhaustive
        prop_assert_ne!(
            gte_result, lt_result,
            ">= and < should be opposites for 3.{} vs 3.{}",
            v1, v2
        );
    }
}

/// Test specific marker evaluation cases
#[test]
fn test_marker_evaluation_specific_cases() {
    let env = MarkerEnvironment::current().with_python("3.12");

    // Python version tests
    assert!(MarkerEvaluator::evaluate("python_version >= '3.8'", &env, &[]));
    assert!(MarkerEvaluator::evaluate("python_version >= '3.12'", &env, &[]));
    assert!(!MarkerEvaluator::evaluate("python_version >= '3.13'", &env, &[]));
    assert!(MarkerEvaluator::evaluate("python_version < '4.0'", &env, &[]));
    assert!(MarkerEvaluator::evaluate("python_version != '3.11'", &env, &[]));

    // Platform tests (platform-specific)
    #[cfg(target_os = "windows")]
    {
        assert!(MarkerEvaluator::evaluate("sys_platform == 'win32'", &env, &[]));
        assert!(MarkerEvaluator::evaluate("platform_system == 'Windows'", &env, &[]));
    }

    #[cfg(target_os = "linux")]
    {
        assert!(MarkerEvaluator::evaluate("sys_platform == 'linux'", &env, &[]));
        assert!(MarkerEvaluator::evaluate("platform_system == 'Linux'", &env, &[]));
    }

    #[cfg(target_os = "macos")]
    {
        assert!(MarkerEvaluator::evaluate("sys_platform == 'darwin'", &env, &[]));
        assert!(MarkerEvaluator::evaluate("platform_system == 'Darwin'", &env, &[]));
    }

    // Implementation tests
    assert!(MarkerEvaluator::evaluate("implementation_name == 'cpython'", &env, &[]));
}

/// Test complex marker expressions
#[test]
fn test_complex_marker_expressions() {
    let env = MarkerEnvironment::current().with_python("3.12");

    // Compound AND
    let marker = "python_version >= '3.8' and implementation_name == 'cpython'";
    assert!(MarkerEvaluator::evaluate(marker, &env, &[]));

    // Compound OR
    let marker = "python_version < '3.0' or python_version >= '3.8'";
    assert!(MarkerEvaluator::evaluate(marker, &env, &[]));

    // Nested parentheses
    let marker = "(python_version >= '3.8') and (implementation_name == 'cpython')";
    assert!(MarkerEvaluator::evaluate(marker, &env, &[]));

    // Complex real-world example
    let marker = "python_version >= '3.8' and (sys_platform == 'win32' or sys_platform == 'linux' or sys_platform == 'darwin')";
    assert!(MarkerEvaluator::evaluate(marker, &env, &[]));
}

/// Test extra marker evaluation
#[test]
fn test_extra_marker() {
    let env = MarkerEnvironment::current();

    // Without extras
    assert!(!MarkerEvaluator::evaluate("extra == 'dev'", &env, &[]));
    assert!(!MarkerEvaluator::evaluate("extra == 'test'", &env, &[]));

    // With extras
    let extras = vec!["dev".to_string()];
    assert!(MarkerEvaluator::evaluate("extra == 'dev'", &env, &extras));

    let extras = vec!["dev".to_string(), "test".to_string()];
    assert!(MarkerEvaluator::evaluate("extra == 'dev'", &env, &extras));
}
