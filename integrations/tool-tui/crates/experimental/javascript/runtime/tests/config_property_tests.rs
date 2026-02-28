//! Property-based tests for configuration file loading
//!
//! **Feature: production-readiness, Property 14: Configuration File Loading**
//! **Validates: Requirements 8.3**
//!
//! For any valid dx.config.json file in a project root, the loaded configuration
//! SHALL contain all specified values, and unspecified values SHALL use documented defaults.

use dx_js_runtime::{
    load_config, load_config_from_file, merge_with_defaults, BundlerConfigFile, DxConfig,
    PackageManagerConfigFile, ProjectConfig, RuntimeConfigFile, TestRunnerConfigFile,
};
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Generate valid heap sizes (16-16384 MB)
fn valid_heap_size() -> impl Strategy<Value = usize> {
    16usize..=16384
}

/// Generate valid module resolution modes
fn valid_module_resolution() -> impl Strategy<Value = String> {
    prop_oneof![Just("node".to_string()), Just("bundler".to_string())]
}

/// Generate valid ES targets
fn valid_target() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("es5".to_string()),
        Just("es6".to_string()),
        Just("es2015".to_string()),
        Just("es2016".to_string()),
        Just("es2017".to_string()),
        Just("es2018".to_string()),
        Just("es2019".to_string()),
        Just("es2020".to_string()),
        Just("es2021".to_string()),
        Just("es2022".to_string()),
        Just("es2023".to_string()),
        Just("esnext".to_string()),
    ]
}

/// Generate valid test timeouts (1-3600000 ms)
fn valid_timeout() -> impl Strategy<Value = u64> {
    1u64..=3600000
}

/// Generate valid worker counts
fn valid_workers() -> impl Strategy<Value = usize> {
    1usize..=64
}

/// Generate optional values
fn optional<T: Clone + std::fmt::Debug + 'static>(
    strategy: impl Strategy<Value = T>,
) -> impl Strategy<Value = Option<T>> {
    prop_oneof![Just(None), strategy.prop_map(Some)]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14: Configuration File Loading
    /// For any valid dx.config.json, loaded config contains all specified values
    #[test]
    fn prop_config_preserves_specified_values(
        heap_size in optional(valid_heap_size()),
        type_check in optional(any::<bool>()),
        workers in optional(valid_workers()),
        module_resolution in optional(valid_module_resolution()),
        registry in optional("[a-z]+://[a-z.]+".prop_map(String::from)),
        cache_dir in optional("[a-z_-]+".prop_map(String::from)),
        target in optional(valid_target()),
        minify in optional(any::<bool>()),
        source_maps in optional(any::<bool>()),
        parallel in optional(any::<bool>()),
        coverage in optional(any::<bool>()),
        timeout in optional(valid_timeout()),
    ) {
        // Feature: production-readiness, Property 14: Configuration File Loading
        // Validates: Requirements 8.3

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("dx.config.json");

        // Build config JSON
        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                max_heap_size: heap_size,
                type_check,
                workers,
                module_resolution: module_resolution.clone(),
                experimental: vec![],
            },
            package_manager: PackageManagerConfigFile {
                registry: registry.clone(),
                cache_dir: cache_dir.clone(),
                strict_peer_deps: None,
            },
            bundler: BundlerConfigFile {
                target: target.clone(),
                minify,
                source_maps,
                entry: None,
                out_dir: None,
            },
            test_runner: TestRunnerConfigFile {
                parallel,
                coverage,
                timeout,
                include: None,
                exclude: None,
            },
        };

        // Write config to file
        let json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, &json).unwrap();

        // Load config
        let loaded = load_config(temp_dir.path()).unwrap();
        prop_assert!(loaded.is_some(), "Config file should be loaded");

        let loaded_config = loaded.unwrap().config;

        // Verify all specified values are preserved
        prop_assert_eq!(loaded_config.runtime.max_heap_size, heap_size);
        prop_assert_eq!(loaded_config.runtime.type_check, type_check);
        prop_assert_eq!(loaded_config.runtime.workers, workers);
        prop_assert_eq!(loaded_config.runtime.module_resolution, module_resolution);

        prop_assert_eq!(loaded_config.package_manager.registry, registry);
        prop_assert_eq!(loaded_config.package_manager.cache_dir, cache_dir);

        prop_assert_eq!(loaded_config.bundler.target, target);
        prop_assert_eq!(loaded_config.bundler.minify, minify);
        prop_assert_eq!(loaded_config.bundler.source_maps, source_maps);

        prop_assert_eq!(loaded_config.test_runner.parallel, parallel);
        prop_assert_eq!(loaded_config.test_runner.coverage, coverage);
        prop_assert_eq!(loaded_config.test_runner.timeout, timeout);
    }

    /// Property: Unspecified values use defaults when merged
    #[test]
    fn prop_unspecified_values_use_defaults(
        heap_size in optional(valid_heap_size()),
        type_check in optional(any::<bool>()),
        workers in optional(valid_workers()),
    ) {
        // Feature: production-readiness, Property 14: Configuration File Loading
        // Validates: Requirements 8.3

        let config = ProjectConfig {
            runtime: RuntimeConfigFile {
                max_heap_size: heap_size,
                type_check,
                workers,
                ..Default::default()
            },
            ..Default::default()
        };

        let defaults = DxConfig::default();
        let merged = merge_with_defaults(&config, &defaults);

        // Specified values should be used
        if let Some(hs) = heap_size {
            prop_assert_eq!(merged.max_heap_size_mb, hs);
        } else {
            // Unspecified should use default
            prop_assert_eq!(merged.max_heap_size_mb, defaults.max_heap_size_mb);
        }

        if let Some(tc) = type_check {
            prop_assert_eq!(merged.type_check, tc);
        } else {
            prop_assert_eq!(merged.type_check, defaults.type_check);
        }

        if let Some(w) = workers {
            prop_assert_eq!(merged.workers, w);
        } else {
            prop_assert_eq!(merged.workers, defaults.workers);
        }
    }

    /// Property: Empty config file loads successfully with all None values
    #[test]
    fn prop_empty_config_loads_successfully(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 14: Configuration File Loading
        // Validates: Requirements 8.3

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("dx.config.json");

        // Write empty config
        fs::write(&config_path, "{}").unwrap();

        // Load should succeed
        let loaded = load_config(temp_dir.path()).unwrap();
        prop_assert!(loaded.is_some());

        let config = loaded.unwrap().config;

        // All values should be None/default
        prop_assert!(config.runtime.max_heap_size.is_none());
        prop_assert!(config.runtime.type_check.is_none());
        prop_assert!(config.runtime.workers.is_none());
        prop_assert!(config.bundler.target.is_none());
        prop_assert!(config.test_runner.timeout.is_none());
    }

    /// Property: No config file returns None (not error)
    #[test]
    fn prop_missing_config_returns_none(_seed in any::<u64>()) {
        // Feature: production-readiness, Property 14: Configuration File Loading
        // Validates: Requirements 8.3

        let temp_dir = TempDir::new().unwrap();

        // No config file exists
        let result = load_config(temp_dir.path()).unwrap();
        prop_assert!(result.is_none(), "Missing config should return None, not error");
    }
}

/// Test invalid configurations are rejected
#[test]
fn test_invalid_heap_size_too_small() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.json");

    let json = r#"{"runtime": {"maxHeapSize": 8}}"#;
    fs::write(&config_path, json).unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_invalid_heap_size_too_large() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.json");

    let json = r#"{"runtime": {"maxHeapSize": 32768}}"#;
    fs::write(&config_path, json).unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_invalid_module_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.json");

    let json = r#"{"runtime": {"moduleResolution": "invalid"}}"#;
    fs::write(&config_path, json).unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_invalid_target() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.json");

    let json = r#"{"bundler": {"target": "es3"}}"#;
    fs::write(&config_path, json).unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_zero_timeout_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.json");

    let json = r#"{"testRunner": {"timeout": 0}}"#;
    fs::write(&config_path, json).unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_js_config_not_supported() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.config.js");

    fs::write(&config_path, "module.exports = {}").unwrap();

    let result = load_config(temp_dir.path());
    assert!(result.is_err());
}
