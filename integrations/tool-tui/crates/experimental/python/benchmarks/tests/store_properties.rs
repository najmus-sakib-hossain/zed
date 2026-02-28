//! Property-based tests for ResultStore
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::core::{BenchmarkConfig, BenchmarkResult};
use dx_py_benchmarks::data::{BenchmarkResults, ResultStore};
use proptest::prelude::*;
use std::time::Duration;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 15: Metadata Recording Completeness**
    /// *For any* stored benchmark result, it SHALL include: all configuration parameters used,
    /// relevant environment variables, and a timestamp.
    /// **Validates: Requirements 7.1, 7.3, 7.5**
    #[test]
    fn property_metadata_recording_completeness(
        warmup in 1u32..50,
        measurement in 30u32..100,
        timeout in 1u64..3600,
        suite_name in "[a-z_]{3,15}"
    ) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = ResultStore::new(temp_dir.path().to_path_buf());

        let config = BenchmarkConfig {
            warmup_iterations: warmup,
            measurement_iterations: measurement,
            timeout_seconds: timeout,
            ..Default::default()
        };

        let mut results = BenchmarkResults::new(&suite_name, config.clone());
        let mut bench_result = BenchmarkResult::new("test_bench");
        bench_result.timings = vec![Duration::from_micros(100); 30];
        bench_result.warmup_completed = true;
        results.add_result(bench_result);

        // Save and load
        let id = store.save(&results, &config).expect("Should save");
        let loaded = store.load(&id).expect("Should load");

        // Verify all configuration parameters are recorded
        prop_assert_eq!(loaded.config.warmup_iterations, warmup,
            "Warmup iterations should be recorded");
        prop_assert_eq!(loaded.config.measurement_iterations, measurement,
            "Measurement iterations should be recorded");
        prop_assert_eq!(loaded.config.timeout_seconds, timeout,
            "Timeout should be recorded");

        // Verify timestamp is present and valid
        prop_assert!(loaded.timestamp.timestamp() > 0,
            "Timestamp should be valid (not epoch)");

        // Verify has_complete_metadata returns true
        prop_assert!(loaded.has_complete_metadata(),
            "Stored result should have complete metadata");
    }

    /// Test that save and load round-trip preserves data
    #[test]
    fn property_store_round_trip(
        suite_name in "[a-z_]{3,15}",
        bench_name in "[a-z_]{3,15}",
        timing_count in 30usize..50
    ) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = ResultStore::new(temp_dir.path().to_path_buf());

        let config = BenchmarkConfig::default();
        let mut results = BenchmarkResults::new(&suite_name, config.clone());

        let mut bench_result = BenchmarkResult::new(&bench_name);
        bench_result.timings = (0..timing_count)
            .map(|i| Duration::from_micros((i as u64 + 1) * 100))
            .collect();
        bench_result.warmup_completed = true;
        results.add_result(bench_result);

        // Save
        let id = store.save(&results, &config).expect("Should save");

        // Load
        let loaded = store.load(&id).expect("Should load");

        // Verify data is preserved
        prop_assert_eq!(&loaded.results.suite, &suite_name,
            "Suite name should be preserved");
        prop_assert_eq!(loaded.results.benchmarks.len(), 1,
            "Should have one benchmark");
        prop_assert_eq!(&loaded.results.benchmarks[0].name, &bench_name,
            "Benchmark name should be preserved");
        prop_assert_eq!(loaded.results.benchmarks[0].timings.len(), timing_count,
            "Timing count should be preserved");
    }

    /// Test that list_recent returns results in correct order
    #[test]
    fn property_list_recent_ordering(count in 1usize..5) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = ResultStore::new(temp_dir.path().to_path_buf());

        let config = BenchmarkConfig::default();
        let mut ids = Vec::new();

        // Save multiple results
        for i in 0..count {
            let mut results = BenchmarkResults::new(format!("suite_{}", i), config.clone());
            let mut bench_result = BenchmarkResult::new("test");
            bench_result.timings = vec![Duration::from_micros(100); 30];
            bench_result.warmup_completed = true;
            results.add_result(bench_result);

            let id = store.save(&results, &config).expect("Should save");
            ids.push(id);

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // List recent
        let recent = store.list_recent(count);

        prop_assert_eq!(recent.len(), count,
            "Should return {} results", count);

        // Verify ordering (newest first)
        for i in 1..recent.len() {
            prop_assert!(recent[i-1].timestamp >= recent[i].timestamp,
                "Results should be ordered by timestamp (newest first)");
        }
    }
}

/// Test that exists() correctly detects stored results
#[test]
fn test_exists() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store = ResultStore::new(temp_dir.path().to_path_buf());

    let config = BenchmarkConfig::default();
    let mut results = BenchmarkResults::new("test_suite", config.clone());
    let mut bench_result = BenchmarkResult::new("test");
    bench_result.timings = vec![Duration::from_micros(100); 30];
    results.add_result(bench_result);

    let id = store.save(&results, &config).expect("Should save");

    assert!(store.exists(&id), "Should exist after save");
    assert!(!store.exists("nonexistent_id"), "Should not exist for invalid ID");
}

/// Test that delete() removes stored results
#[test]
fn test_delete() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store = ResultStore::new(temp_dir.path().to_path_buf());

    let config = BenchmarkConfig::default();
    let mut results = BenchmarkResults::new("test_suite", config.clone());
    let mut bench_result = BenchmarkResult::new("test");
    bench_result.timings = vec![Duration::from_micros(100); 30];
    results.add_result(bench_result);

    let id = store.save(&results, &config).expect("Should save");
    assert!(store.exists(&id), "Should exist after save");

    store.delete(&id).expect("Should delete");
    assert!(!store.exists(&id), "Should not exist after delete");
}

/// Test get_historical filters by suite
#[test]
fn test_get_historical_filters_by_suite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store = ResultStore::new(temp_dir.path().to_path_buf());

    let config = BenchmarkConfig::default();

    // Save results for different suites
    for suite in &["runtime", "package", "test_runner"] {
        let mut results = BenchmarkResults::new(*suite, config.clone());
        let mut bench_result = BenchmarkResult::new("test");
        bench_result.timings = vec![Duration::from_micros(100); 30];
        results.add_result(bench_result);
        store.save(&results, &config).expect("Should save");
    }

    // Get historical for specific suite
    let runtime_results = store.get_historical("runtime", 10);
    assert_eq!(runtime_results.len(), 1, "Should have 1 runtime result");
    assert_eq!(runtime_results[0].results.suite, "runtime");

    let package_results = store.get_historical("package", 10);
    assert_eq!(package_results.len(), 1, "Should have 1 package result");
    assert_eq!(package_results[0].results.suite, "package");
}

/// Test loading non-existent result returns error
#[test]
fn test_load_nonexistent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store = ResultStore::new(temp_dir.path().to_path_buf());

    let result = store.load("nonexistent_id");
    assert!(result.is_err(), "Should return error for non-existent ID");
}
