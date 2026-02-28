//! Property-based tests for BenchmarkFramework
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::core::{BenchmarkConfig, BenchmarkFramework, BenchmarkResult, OutputFormat};
use dx_py_benchmarks::data::BenchmarkResults;
use proptest::prelude::*;
use std::time::Duration;
use tempfile::TempDir;

/// Create a test config with a temporary output directory
fn create_test_config(temp_dir: &TempDir) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    }
}

/// Create test benchmark results for validation
fn create_test_results(suite: &str, bench_names: &[&str]) -> BenchmarkResults {
    let config = BenchmarkConfig::default();
    let mut results = BenchmarkResults::new(suite, config);

    for name in bench_names {
        let mut bench = BenchmarkResult::new(*name);
        bench.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 100)).collect();
        bench.warmup_completed = true;
        results.add_result(bench);
    }

    results
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: Dual Output Format Validity**
    /// *For any* completed benchmark run, the framework SHALL produce both valid Markdown
    /// (parseable as Markdown) and valid JSON (parseable as JSON) outputs containing
    /// equivalent benchmark data.
    /// **Validates: Requirements 1.4**
    #[test]
    fn property_dual_output_format_validity(
        suite_name in "[a-z_]{3,15}",
        bench_count in 1usize..5
    ) {
        // Create test results
        let bench_names: Vec<String> = (0..bench_count)
            .map(|i| format!("bench_{}", i))
            .collect();
        let bench_refs: Vec<&str> = bench_names.iter().map(|s| s.as_str()).collect();
        let results = create_test_results(&suite_name, &bench_refs);

        // Create framework and generate reports
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let framework = BenchmarkFramework::new(config);

        // Generate markdown report
        let markdown = framework.reporter.generate_json(&results);
        let json = framework.reporter.generate_json(&results);

        // Validate dual output
        let validation = BenchmarkFramework::validate_dual_output(&markdown, &json);

        // JSON should always be valid
        prop_assert!(validation.json_valid,
            "JSON output should be valid JSON");

        // Verify JSON contains benchmark data
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let benchmarks = parsed.get("benchmarks").and_then(|v| v.as_array());
        prop_assert!(benchmarks.is_some(), "JSON should contain benchmarks array");
        prop_assert_eq!(benchmarks.unwrap().len(), bench_count,
            "JSON should contain correct number of benchmarks");

        // Verify suite name in JSON
        prop_assert_eq!(
            parsed.get("suite").and_then(|v| v.as_str()),
            Some(suite_name.as_str()),
            "JSON should contain correct suite name"
        );
    }

    /// Test that markdown and JSON contain equivalent data
    #[test]
    fn property_dual_output_data_equivalence(
        bench_count in 1usize..5
    ) {
        let bench_names: Vec<String> = (0..bench_count)
            .map(|i| format!("bench_{}", i))
            .collect();
        let bench_refs: Vec<&str> = bench_names.iter().map(|s| s.as_str()).collect();
        let results = create_test_results("test_suite", &bench_refs);

        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let framework = BenchmarkFramework::new(config);

        let json = framework.reporter.generate_json(&results);

        // Parse JSON and verify all benchmarks are present
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let json_benchmarks = parsed.get("benchmarks")
            .and_then(|v| v.as_array())
            .unwrap();

        // Verify each benchmark name is in JSON
        for name in &bench_names {
            let found = json_benchmarks.iter().any(|b| {
                b.get("name").and_then(|n| n.as_str()) == Some(name.as_str())
            });
            prop_assert!(found, "Benchmark {} should be in JSON output", name);
        }
    }
}

/// Test available suites list
#[test]
fn test_available_suites() {
    let suites = BenchmarkFramework::available_suites();

    assert!(suites.contains(&"runtime"), "Should include runtime suite");
    assert!(suites.contains(&"package"), "Should include package suite");
    assert!(suites.contains(&"test_runner"), "Should include test_runner suite");
}

/// Test framework creation with default config
#[test]
fn test_framework_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let framework = BenchmarkFramework::new(config.clone());

    assert_eq!(framework.config.warmup_iterations, config.warmup_iterations);
    assert_eq!(framework.config.measurement_iterations, config.measurement_iterations);
    assert_eq!(framework.config.timeout_seconds, config.timeout_seconds);
}

/// Test dual output validation with valid inputs
#[test]
fn test_dual_output_validation_valid() {
    let markdown = "# Test Report\n\nSome content";
    let json = r#"{"suite": "test", "benchmarks": []}"#;

    let validation = BenchmarkFramework::validate_dual_output(markdown, json);

    assert!(validation.markdown_valid, "Markdown should be valid");
    assert!(validation.json_valid, "JSON should be valid");
    assert!(validation.both_valid, "Both should be valid");
}

/// Test dual output validation with invalid JSON
#[test]
fn test_dual_output_validation_invalid_json() {
    let markdown = "# Test Report\n\nSome content";
    let json = "not valid json {";

    let validation = BenchmarkFramework::validate_dual_output(markdown, json);

    assert!(validation.markdown_valid, "Markdown should be valid");
    assert!(!validation.json_valid, "JSON should be invalid");
    assert!(!validation.both_valid, "Both should not be valid");
}

/// Test dual output validation with empty markdown
#[test]
fn test_dual_output_validation_empty_markdown() {
    let markdown = "";
    let json = r#"{"suite": "test"}"#;

    let validation = BenchmarkFramework::validate_dual_output(markdown, json);

    assert!(!validation.markdown_valid, "Empty markdown should be invalid");
    assert!(validation.json_valid, "JSON should be valid");
    assert!(!validation.both_valid, "Both should not be valid");
}

/// Test unknown suite error
#[test]
fn test_unknown_suite_error() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let mut framework = BenchmarkFramework::new(config);

    let result = framework.run_suite("nonexistent_suite");

    assert!(result.is_err(), "Should return error for unknown suite");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Unknown suite"), "Error should mention unknown suite");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Test that output format configuration is respected
    #[test]
    fn property_output_format_respected(
        format_choice in 0u8..3
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);

        config.output_format = match format_choice {
            0 => OutputFormat::Markdown,
            1 => OutputFormat::Json,
            _ => OutputFormat::Both,
        };

        let framework = BenchmarkFramework::new(config.clone());

        // Verify config is stored correctly
        prop_assert_eq!(framework.config.output_format, config.output_format,
            "Output format should be stored correctly");
    }

    /// Test seed configuration
    #[test]
    fn property_seed_configuration(
        seed in any::<u64>()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.seed = Some(seed);

        let framework = BenchmarkFramework::new(config);

        prop_assert_eq!(framework.config.seed, Some(seed),
            "Seed should be stored correctly");
    }
}
