//! Property-based tests for feature detection object
//!
//! **Feature: production-readiness, Property 16: Feature Detection Object Structure**
//! **Validates: Requirements 10.2, 10.4**
//!
//! For any access to `dx.features`, the returned object SHALL be a valid JavaScript
//! object with boolean values for each feature key, and the set of keys SHALL include
//! at least: `es2015`, `es2016`, `es2017`, `es2018`, `es2019`, `es2020`, `es2021`,
//! `es2022`, `typescript`.

use dx_js_runtime::compiler::builtins_registry::BuiltinRegistry;
use dx_js_runtime::features::{DxFeatures, DxGlobal};
use dx_js_runtime::value::Value;
use proptest::prelude::*;

/// Required feature keys per Property 16
const REQUIRED_FEATURE_KEYS: &[&str] = &[
    "es2015",
    "es2016",
    "es2017",
    "es2018",
    "es2019",
    "es2020",
    "es2021",
    "es2022",
    "typescript",
];

/// All known feature keys
const ALL_FEATURE_KEYS: &[&str] = &[
    "es2015",
    "es2016",
    "es2017",
    "es2018",
    "es2019",
    "es2020",
    "es2021",
    "es2022",
    "typescript",
    "jsx",
    "decorators",
    "commonjs",
    "esm",
    "workers",
    "wasm",
    "hmr",
    "sourceMaps",
    "jit",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 16: Feature Detection Object Structure
    /// For any access to dx.features, the returned object contains all required keys
    #[test]
    fn prop_dx_features_contains_required_keys(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let registry = BuiltinRegistry::new();
        let dx_features = registry.get("dx.features")
            .expect("dx.features should be registered");

        let result = dx_features(&[]);

        match result {
            Value::Object(obj) => {
                for key in REQUIRED_FEATURE_KEYS {
                    let value = obj.get(*key);
                    prop_assert!(
                        value.is_some(),
                        "dx.features must contain required key: {}",
                        key
                    );
                }
            }
            _ => prop_assert!(false, "dx.features must return an Object"),
        }
    }

    /// Property 16: All values in dx.features must be booleans
    #[test]
    fn prop_dx_features_all_values_are_booleans(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let registry = BuiltinRegistry::new();
        let dx_features = registry.get("dx.features")
            .expect("dx.features should be registered");

        let result = dx_features(&[]);

        match result {
            Value::Object(obj) => {
                for (key, value) in obj.entries() {
                    prop_assert!(
                        matches!(value, Value::Boolean(_)),
                        "dx.features.{} must be a boolean, got {:?}",
                        key, value
                    );
                }
            }
            _ => prop_assert!(false, "dx.features must return an Object"),
        }
    }

    /// Property 16: dx.features is consistent across multiple accesses
    #[test]
    fn prop_dx_features_consistent_across_accesses(num_accesses in 2usize..10) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let registry = BuiltinRegistry::new();
        let dx_features = registry.get("dx.features")
            .expect("dx.features should be registered");

        // Get first result as reference
        let first_result = dx_features(&[]);
        let first_entries = match &first_result {
            Value::Object(obj) => obj.entries(),
            _ => {
                prop_assert!(false, "dx.features must return an Object");
                return Ok(());
            }
        };

        // Verify subsequent accesses return the same values
        for _ in 1..num_accesses {
            let result = dx_features(&[]);
            match result {
                Value::Object(obj) => {
                    let current_entries = obj.entries();
                    prop_assert_eq!(
                        first_entries.len(),
                        current_entries.len(),
                        "dx.features should have consistent number of keys"
                    );

                    for (key, expected_value) in &first_entries {
                        let actual_value = obj.get(key);
                        prop_assert!(
                            actual_value.is_some(),
                            "dx.features should consistently have key: {}",
                            key
                        );
                        prop_assert_eq!(
                            actual_value.unwrap(),
                            *expected_value,
                            "dx.features.{} should be consistent",
                            key
                        );
                    }
                }
                _ => prop_assert!(false, "dx.features must return an Object"),
            }
        }
    }

    /// Property: DxFeatures::to_map() produces valid feature map
    #[test]
    fn prop_dx_features_to_map_contains_required_keys(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let features = DxFeatures::current();
        let map = features.to_map();

        // All required keys must be present
        for key in REQUIRED_FEATURE_KEYS {
            prop_assert!(
                map.contains_key(*key),
                "DxFeatures::to_map() must contain required key: {}",
                key
            );
        }
    }

    /// Property: DxFeatures::is_supported() returns Some for all known features
    #[test]
    fn prop_is_supported_returns_some_for_known_features(
        feature_idx in 0usize..ALL_FEATURE_KEYS.len()
    ) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let features = DxFeatures::current();
        let feature_name = ALL_FEATURE_KEYS[feature_idx];

        let result = features.is_supported(feature_name);
        prop_assert!(
            result.is_some(),
            "is_supported('{}') should return Some",
            feature_name
        );
    }

    /// Property: DxFeatures::is_supported() returns None for unknown features
    #[test]
    fn prop_is_supported_returns_none_for_unknown_features(
        unknown_feature in "[a-z]{5,15}"
    ) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        // Skip if the generated string happens to be a known feature
        if ALL_FEATURE_KEYS.contains(&unknown_feature.as_str()) {
            return Ok(());
        }

        let features = DxFeatures::current();
        let result = features.is_supported(&unknown_feature);

        prop_assert!(
            result.is_none(),
            "is_supported('{}') should return None for unknown feature",
            unknown_feature
        );
    }

    /// Property: DxGlobal provides consistent features and version
    #[test]
    fn prop_dx_global_consistent(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let dx = DxGlobal::new();

        // Version should be non-empty
        prop_assert!(
            !dx.version.is_empty(),
            "dx.version should not be empty"
        );

        // Features should match DxFeatures::current()
        let current_features = DxFeatures::current();
        prop_assert_eq!(
            dx.features.es2022,
            current_features.es2022,
            "DxGlobal features should match DxFeatures::current()"
        );
        prop_assert_eq!(
            dx.features.typescript,
            current_features.typescript,
            "DxGlobal features should match DxFeatures::current()"
        );
    }

    /// Property: Required ES versions are all supported
    #[test]
    fn prop_required_es_versions_supported(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let features = DxFeatures::current();

        // All ES versions from 2015-2022 should be supported
        prop_assert!(features.es2015, "es2015 should be supported");
        prop_assert!(features.es2016, "es2016 should be supported");
        prop_assert!(features.es2017, "es2017 should be supported");
        prop_assert!(features.es2018, "es2018 should be supported");
        prop_assert!(features.es2019, "es2019 should be supported");
        prop_assert!(features.es2020, "es2020 should be supported");
        prop_assert!(features.es2021, "es2021 should be supported");
        prop_assert!(features.es2022, "es2022 should be supported");

        // TypeScript should be supported
        prop_assert!(features.typescript, "typescript should be supported");
    }

    /// Property: feature_names() returns all required keys
    #[test]
    fn prop_feature_names_contains_required(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let names = DxFeatures::feature_names();

        for required in REQUIRED_FEATURE_KEYS {
            prop_assert!(
                names.contains(required),
                "feature_names() must contain required key: {}",
                required
            );
        }
    }

    /// Property: required_feature_names() matches Property 16 specification
    #[test]
    fn prop_required_feature_names_matches_spec(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 16: Feature Detection Object Structure
        // Validates: Requirements 10.2, 10.4

        let required = DxFeatures::required_feature_names();

        // Should contain exactly the keys specified in Property 16
        prop_assert_eq!(
            required.len(),
            REQUIRED_FEATURE_KEYS.len(),
            "required_feature_names() should have {} keys",
            REQUIRED_FEATURE_KEYS.len()
        );

        for key in REQUIRED_FEATURE_KEYS {
            prop_assert!(
                required.contains(key),
                "required_feature_names() must contain: {}",
                key
            );
        }
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_dx_features_returns_object() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    let result = dx_features(&[]);
    assert!(matches!(result, Value::Object(_)), "dx.features should return an Object");
}

#[test]
fn test_dx_features_ignores_arguments() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    // Call with various arguments - should all return the same result
    let result_no_args = dx_features(&[]);
    let result_with_args = dx_features(&[Value::Number(42.0), Value::String("test".to_string())]);

    match (&result_no_args, &result_with_args) {
        (Value::Object(obj1), Value::Object(obj2)) => {
            // Both should have the same keys
            let keys1: Vec<String> = obj1.entries().iter().map(|(k, _)| (*k).clone()).collect();
            let keys2: Vec<String> = obj2.entries().iter().map(|(k, _)| (*k).clone()).collect();
            assert_eq!(keys1.len(), keys2.len());
        }
        _ => panic!("Both calls should return Objects"),
    }
}

#[test]
fn test_unsupported_features_list() {
    let features = DxFeatures::current();
    let unsupported = features.unsupported_features();

    // decorators and hmr are currently not supported
    assert!(unsupported.contains(&"decorators"), "decorators should be unsupported");
    assert!(unsupported.contains(&"hmr"), "hmr should be unsupported");

    // ES versions should NOT be in unsupported list
    assert!(!unsupported.contains(&"es2022"), "es2022 should be supported");
    assert!(!unsupported.contains(&"typescript"), "typescript should be supported");
}

#[test]
fn test_source_maps_alias() {
    let features = DxFeatures::current();

    // Both "sourceMaps" and "source_maps" should work
    assert_eq!(
        features.is_supported("sourceMaps"),
        features.is_supported("source_maps"),
        "sourceMaps and source_maps should be aliases"
    );
}
