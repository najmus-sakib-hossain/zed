//! Property-based tests for ReportGenerator
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::core::{BenchmarkConfig, BenchmarkResult};
use dx_py_benchmarks::data::BenchmarkResults;
use dx_py_benchmarks::report::{BenchmarkComparison, ComparisonReport, FeatureCoverage, ReportGenerator};
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

/// Generate a valid BenchmarkResults for testing
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

/// Generate a comparison report for testing
fn create_test_comparison(suite: &str, comparisons: Vec<BenchmarkComparison>) -> ComparisonReport {
    let total = comparisons.len();
    let successful = comparisons.iter().filter(|c| c.is_valid).count();
    
    ComparisonReport {
        suite: suite.to_string(),
        comparisons,
        methodology: "Test methodology".to_string(),
        feature_coverage: FeatureCoverage {
            total_benchmarks: total,
            successful_benchmarks: successful,
            validated_benchmarks: successful,
            not_supported_benchmarks: 0,
            output_mismatch_benchmarks: 0,
            execution_failed_benchmarks: 0,
            coverage_percentage: if total > 0 { (successful as f64 / total as f64) * 100.0 } else { 0.0 },
        },
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 12: Report Content Completeness**
    /// *For any* generated Markdown report, it SHALL contain: a comparison table with benchmark
    /// names and timings, speedup factors for each benchmark, confidence intervals for speedups,
    /// clear indication when speedup < 1.0 (slower), and a methodology section.
    /// **Validates: Requirements 6.1, 6.2, 6.3, 6.5**
    #[test]
    fn property_report_content_completeness(
        suite_name in "[a-z_]{3,15}",
        bench_count in 1usize..5,
        has_slower in any::<bool>()
    ) {
        let generator = ReportGenerator::new(PathBuf::from("test_output"));

        // Create test results
        let bench_names: Vec<String> = (0..bench_count)
            .map(|i| format!("bench_{}", i))
            .collect();
        let bench_refs: Vec<&str> = bench_names.iter().map(|s| s.as_str()).collect();
        let results = create_test_results(&suite_name, &bench_refs);

        // Create comparison with at least one slower benchmark if has_slower is true
        let comparisons: Vec<BenchmarkComparison> = bench_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let speedup = if has_slower && i == 0 { 0.8 } else { 1.5 };
                BenchmarkComparison {
                    name: name.clone(),
                    baseline_mean_ms: 100.0,
                    subject_mean_ms: if speedup > 1.0 { 66.67 } else { 125.0 },
                    speedup,
                    speedup_ci: (speedup * 0.9, speedup * 1.1),
                    is_significant: true,
                    p_value: 0.01,
                    is_slower: speedup < 1.0,
                    validation_status: "✅ Validated".to_string(),
                    is_valid: true,
                    error_message: None,
                }
            })
            .collect();

        let comparison = create_test_comparison(&suite_name, comparisons);

        // Generate markdown report
        let markdown = generator.generate_markdown(&results, &comparison);

        // Validate report content
        let validation = generator.validate_markdown_report(&markdown);

        prop_assert!(validation.has_comparison_table,
            "Report should contain comparison table");
        prop_assert!(validation.has_speedup_factors,
            "Report should contain speedup factors");
        prop_assert!(validation.has_confidence_intervals,
            "Report should contain confidence intervals");
        prop_assert!(validation.has_methodology,
            "Report should contain methodology section");

        // If there's a slower benchmark, check for slowdown indication
        if has_slower {
            prop_assert!(validation.has_slowdown_indication,
                "Report should indicate slowdown when speedup < 1.0");
        }

        // Verify all benchmarks are in the report
        for name in &bench_names {
            prop_assert!(markdown.contains(name),
                "Report should contain benchmark name: {}", name);
        }
    }

    /// Test that speedup values are correctly formatted
    #[test]
    fn property_speedup_formatting(
        speedup in 0.1f64..10.0
    ) {
        let generator = ReportGenerator::new(PathBuf::from("test_output"));
        let results = create_test_results("test_suite", &["test_bench"]);

        let comparison = create_test_comparison("test_suite", vec![
            BenchmarkComparison {
                name: "test_bench".to_string(),
                baseline_mean_ms: 100.0,
                subject_mean_ms: 100.0 / speedup,
                speedup,
                speedup_ci: (speedup * 0.9, speedup * 1.1),
                is_significant: true,
                p_value: 0.01,
                is_slower: speedup < 1.0,
                validation_status: "✅ Validated".to_string(),
                is_valid: true,
                error_message: None,
            }
        ]);

        let markdown = generator.generate_markdown(&results, &comparison);

        // Speedup should be formatted with 2 decimal places
        let speedup_str = format!("{:.2}x", speedup);
        prop_assert!(markdown.contains(&speedup_str),
            "Report should contain formatted speedup: {}", speedup_str);
    }
}

/// **Property 13: JSON Output Validity**
/// *For any* generated JSON output, it SHALL be valid JSON that can be parsed
/// and SHALL contain all benchmark results with their statistical metrics.
/// **Validates: Requirements 6.4**
#[test]
fn test_json_output_validity() {
    let generator = ReportGenerator::new(PathBuf::from("test_output"));
    let results = create_test_results("test_suite", &["bench_1", "bench_2"]);

    let json = generator.generate_json(&results);

    // Verify it's valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(parsed.is_ok(), "JSON output should be valid JSON");

    let value = parsed.unwrap();

    // Verify required fields
    assert!(value.get("suite").is_some(), "JSON should contain suite");
    assert!(value.get("benchmarks").is_some(), "JSON should contain benchmarks");
    assert!(value.get("system_info").is_some(), "JSON should contain system_info");
    assert!(value.get("config").is_some(), "JSON should contain config");
    assert!(value.get("timestamp").is_some(), "JSON should contain timestamp");

    // Verify benchmarks array
    let benchmarks = value.get("benchmarks").unwrap().as_array().unwrap();
    assert_eq!(benchmarks.len(), 2, "Should have 2 benchmarks");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property test for JSON validity with various inputs
    #[test]
    fn property_json_output_validity(
        suite_name in "[a-z_]{3,15}",
        bench_count in 1usize..5
    ) {
        let generator = ReportGenerator::new(PathBuf::from("test_output"));

        let bench_names: Vec<String> = (0..bench_count)
            .map(|i| format!("bench_{}", i))
            .collect();
        let bench_refs: Vec<&str> = bench_names.iter().map(|s| s.as_str()).collect();
        let results = create_test_results(&suite_name, &bench_refs);

        let json = generator.generate_json(&results);

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "JSON output should be valid JSON");

        let value = parsed.unwrap();

        // Verify suite name
        prop_assert_eq!(
            value.get("suite").and_then(|v| v.as_str()),
            Some(suite_name.as_str()),
            "JSON should contain correct suite name"
        );

        // Verify benchmark count
        let benchmarks = value.get("benchmarks").and_then(|v| v.as_array());
        prop_assert!(benchmarks.is_some(), "JSON should contain benchmarks array");
        prop_assert_eq!(benchmarks.unwrap().len(), bench_count,
            "JSON should contain correct number of benchmarks");
    }
}

/// Test slowdown indication is present when speedup < 1.0
#[test]
fn test_slowdown_indication() {
    let generator = ReportGenerator::new(PathBuf::from("test_output"));
    let results = create_test_results("test_suite", &["slow_bench"]);

    let comparison = create_test_comparison(
        "test_suite",
        vec![BenchmarkComparison {
            name: "slow_bench".to_string(),
            baseline_mean_ms: 100.0,
            subject_mean_ms: 150.0,
            speedup: 0.67,
            speedup_ci: (0.6, 0.74),
            is_significant: true,
            p_value: 0.01,
            is_slower: true,
            validation_status: "✅ Validated".to_string(),
            is_valid: true,
            error_message: None,
        }],
    );

    let markdown = generator.generate_markdown(&results, &comparison);

    assert!(
        markdown.contains("⚠️ Slower") || markdown.contains("Slower"),
        "Report should indicate slowdown"
    );
}

/// Test methodology section is present
#[test]
fn test_methodology_section() {
    let generator = ReportGenerator::new(PathBuf::from("test_output"));
    let results = create_test_results("test_suite", &["bench_1"]);

    let comparison = create_test_comparison(
        "test_suite",
        vec![BenchmarkComparison {
            name: "bench_1".to_string(),
            baseline_mean_ms: 100.0,
            subject_mean_ms: 50.0,
            speedup: 2.0,
            speedup_ci: (1.8, 2.2),
            is_significant: true,
            p_value: 0.01,
            is_slower: false,
            validation_status: "✅ Validated".to_string(),
            is_valid: true,
            error_message: None,
        }],
    );

    let markdown = generator.generate_markdown(&results, &comparison);

    assert!(
        markdown.contains("## Methodology") || markdown.contains("### Methodology"),
        "Report should contain methodology section"
    );
    assert!(markdown.contains("Warmup"), "Methodology should mention warmup");
    assert!(markdown.contains("Measurement"), "Methodology should mention measurement");
}

use dx_py_benchmarks::data::StoredResult;

/// **Property 14: Historical Comparison Generation**
/// *For any* benchmark run where previous results exist in the result store,
/// the Report_Generator SHALL include a historical comparison section showing performance trends.
/// **Validates: Requirements 6.6**
#[test]
fn test_historical_comparison_generation() {
    let generator = ReportGenerator::new(PathBuf::from("test_output"));

    // Create current results
    let current = create_test_results("test_suite", &["bench_1", "bench_2"]);

    // Create previous results
    let prev_config = BenchmarkConfig::default();
    let mut prev_results = BenchmarkResults::new("test_suite", prev_config.clone());

    let mut bench1 = BenchmarkResult::new("bench_1");
    bench1.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 120)).collect();
    bench1.warmup_completed = true;
    prev_results.add_result(bench1);

    let mut bench2 = BenchmarkResult::new("bench_2");
    bench2.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 80)).collect();
    bench2.warmup_completed = true;
    prev_results.add_result(bench2);

    let stored = StoredResult::new(prev_results, prev_config);

    // Generate historical comparison
    let historical = generator.generate_historical_comparison(&current, &[stored]);

    // Verify historical comparison content
    assert!(
        historical.contains("Historical Comparison"),
        "Should contain historical comparison header"
    );
    assert!(
        historical.contains("Performance Trends"),
        "Should contain performance trends section"
    );
    assert!(historical.contains("bench_1"), "Should contain benchmark names");
    assert!(historical.contains("bench_2"), "Should contain benchmark names");
}

/// Test historical comparison with no previous results
#[test]
fn test_historical_comparison_no_previous() {
    let generator = ReportGenerator::new(PathBuf::from("test_output"));
    let current = create_test_results("test_suite", &["bench_1"]);

    let historical = generator.generate_historical_comparison(&current, &[]);

    assert!(
        historical.contains("No previous results"),
        "Should indicate no previous results available"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property test for historical comparison with various inputs
    #[test]
    fn property_historical_comparison_generation(
        bench_count in 1usize..5,
        prev_count in 1usize..3
    ) {
        let generator = ReportGenerator::new(PathBuf::from("test_output"));

        // Create current results
        let bench_names: Vec<String> = (0..bench_count)
            .map(|i| format!("bench_{}", i))
            .collect();
        let bench_refs: Vec<&str> = bench_names.iter().map(|s| s.as_str()).collect();
        let current = create_test_results("test_suite", &bench_refs);

        // Create previous results
        let mut previous = Vec::new();
        for _ in 0..prev_count {
            let prev_config = BenchmarkConfig::default();
            let mut prev_results = BenchmarkResults::new("test_suite", prev_config.clone());

            for name in &bench_names {
                let mut bench = BenchmarkResult::new(name.as_str());
                bench.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 100)).collect();
                bench.warmup_completed = true;
                prev_results.add_result(bench);
            }

            previous.push(StoredResult::new(prev_results, prev_config));
        }

        // Generate historical comparison
        let historical = generator.generate_historical_comparison(&current, &previous);

        // Verify content
        prop_assert!(historical.contains("Historical Comparison"),
            "Should contain historical comparison header");
        prop_assert!(historical.contains("Performance Trends"),
            "Should contain performance trends section");

        // All benchmarks should be mentioned
        for name in &bench_names {
            prop_assert!(historical.contains(name),
                "Should contain benchmark name: {}", name);
        }
    }
}
