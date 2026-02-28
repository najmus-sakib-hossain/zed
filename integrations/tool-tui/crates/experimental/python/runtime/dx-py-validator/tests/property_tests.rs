//! Property-based tests for dx-py-validator
//!
//! **Feature: dx-py-game-changer, Property 9: Compatibility Matrix Completeness**
//! **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

use dx_py_validator::*;
use proptest::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

// Generators for test data

fn arb_framework_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Django".to_string()),
        Just("Flask".to_string()),
        Just("FastAPI".to_string()),
        Just("NumPy".to_string()),
        Just("Pandas".to_string()),
        "[a-zA-Z][a-zA-Z0-9_]{2,15}".prop_map(|s| s),
    ]
}

fn arb_version() -> impl Strategy<Value = String> {
    (1u32..10, 0u32..20, 0u32..10)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

fn arb_framework_info() -> impl Strategy<Value = FrameworkInfo> {
    (arb_framework_name(), arb_version(), 0.5f64..1.0).prop_map(|(name, version, min_rate)| {
        FrameworkInfo::new(name, version).with_min_pass_rate(min_rate)
    })
}

fn arb_test_counts() -> impl Strategy<Value = (usize, usize, usize, usize)> {
    // (passed, failed, skipped, errors)
    (0usize..1000, 0usize..100, 0usize..50, 0usize..20)
}

fn arb_failure_category() -> impl Strategy<Value = FailureCategory> {
    prop_oneof![
        Just(FailureCategory::CExtensionLoad),
        Just(FailureCategory::MissingApi),
        Just(FailureCategory::AsyncBehavior),
        Just(FailureCategory::ImportError),
        Just(FailureCategory::RuntimeError),
        Just(FailureCategory::TypeMismatch),
        Just(FailureCategory::MemoryError),
        Just(FailureCategory::AssertionError),
        Just(FailureCategory::Timeout),
        Just(FailureCategory::Unknown),
    ]
}

fn arb_test_failure() -> impl Strategy<Value = TestFailure> {
    ("[a-z_]+::[a-z_]+", "[A-Za-z ]+").prop_map(|(name, msg)| TestFailure::new(name, msg))
}

fn arb_failure_categories() -> impl Strategy<Value = HashMap<FailureCategory, Vec<TestFailure>>> {
    prop::collection::vec(
        (arb_failure_category(), prop::collection::vec(arb_test_failure(), 0..5)),
        0..5,
    )
    .prop_map(|pairs| {
        let mut map = HashMap::new();
        for (cat, failures) in pairs {
            map.entry(cat).or_insert_with(Vec::new).extend(failures);
        }
        map
    })
}

fn arb_framework_test_result() -> impl Strategy<Value = FrameworkTestResult> {
    (arb_framework_info(), arb_test_counts(), arb_failure_categories(), 1u64..3600).prop_map(
        |(framework, (passed, failed, skipped, errors), failure_categories, duration_secs)| {
            FrameworkTestResult {
                framework,
                total_tests: passed + failed + skipped + errors,
                passed,
                failed,
                skipped,
                errors,
                failure_categories,
                duration: Duration::from_secs(duration_secs),
                timestamp: chrono::Utc::now(),
                raw_output: None,
            }
        },
    )
}

fn arb_dx_py_version() -> impl Strategy<Value = String> {
    (0u32..2, 1u32..20, 0u32..50)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // =========================================================================
    // Property 9: Compatibility Matrix Completeness
    // *For any* set of framework test results, the generated Compatibility_Matrix
    // should contain: (a) accurate pass/fail counts, (b) correct failure
    // categorization, (c) valid markdown output with version information,
    // (d) correct regression detection when compared to previous results.
    // **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**
    // =========================================================================

    /// Property 9a: Matrix contains accurate pass/fail counts
    #[test]
    fn prop_matrix_accurate_counts(
        results in prop::collection::vec(arb_framework_test_result(), 1..10),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results.clone());

        // Verify total counts match
        let expected_total: usize = results.iter().map(|r| r.total_tests).sum();
        let expected_passed: usize = results.iter().map(|r| r.passed).sum();

        let json = matrix.generate_json();
        let frameworks = json["frameworks"].as_array().unwrap();

        let actual_total: usize = frameworks.iter()
            .map(|f| f["total"].as_u64().unwrap() as usize)
            .sum();
        let actual_passed: usize = frameworks.iter()
            .map(|f| f["passed"].as_u64().unwrap() as usize)
            .sum();

        prop_assert_eq!(actual_total, expected_total, "Total test count mismatch");
        prop_assert_eq!(actual_passed, expected_passed, "Passed test count mismatch");
    }

    /// Property 9b: Matrix correctly categorizes failures
    #[test]
    fn prop_matrix_correct_failure_categorization(
        results in prop::collection::vec(arb_framework_test_result(), 1..5),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results.clone());

        // Count expected failures by category
        let mut expected_counts: HashMap<FailureCategory, usize> = HashMap::new();
        for result in &results {
            for (cat, failures) in &result.failure_categories {
                *expected_counts.entry(*cat).or_insert(0) += failures.len();
            }
        }

        // Get actual failure summary
        let actual_counts = matrix.failure_summary();

        // Verify counts match
        for (cat, expected) in &expected_counts {
            let actual = actual_counts.get(cat).copied().unwrap_or(0);
            prop_assert_eq!(actual, *expected,
                "Failure count mismatch for category {:?}", cat);
        }
    }

    /// Property 9c: Markdown output contains version information
    #[test]
    fn prop_matrix_markdown_contains_version(
        results in prop::collection::vec(arb_framework_test_result(), 1..5),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results);

        let markdown = matrix.generate_markdown();

        // Must contain version
        prop_assert!(markdown.contains(&version),
            "Markdown should contain DX-Py version");

        // Must contain header
        prop_assert!(markdown.contains("# DX-Py Compatibility Matrix"),
            "Markdown should contain header");

        // Must contain table structure
        prop_assert!(markdown.contains("| Framework |"),
            "Markdown should contain framework table");
    }

    /// Property 9d: Regression detection is correct
    #[test]
    fn prop_regression_detection_correct(
        prev_results in prop::collection::vec(arb_framework_test_result(), 1..5),
        curr_results in prop::collection::vec(arb_framework_test_result(), 1..5),
        prev_version in arb_dx_py_version(),
        curr_version in arb_dx_py_version()
    ) {
        let previous = CompatibilitySnapshot::new(&prev_version, prev_results.clone());
        let current = CompatibilitySnapshot::new(&curr_version, curr_results.clone());

        let detector = RegressionDetector::new();
        let report = detector.compare(&previous, &current);

        // Verify version information
        prop_assert_eq!(report.previous_version, prev_version);
        prop_assert_eq!(report.current_version, curr_version);

        // Verify regression count matches actual regressions
        let actual_regressions = report.changes.iter()
            .filter(|c| c.change_type == regression::ChangeType::Regression)
            .count();
        prop_assert_eq!(report.regression_count, actual_regressions);

        // Verify improvement count matches actual improvements
        let actual_improvements = report.changes.iter()
            .filter(|c| c.change_type == regression::ChangeType::Improvement)
            .count();
        prop_assert_eq!(report.improvement_count, actual_improvements);
    }

    /// Property 9e: Overall score is within valid range
    #[test]
    fn prop_overall_score_valid_range(
        results in prop::collection::vec(arb_framework_test_result(), 0..10),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results);

        let score = matrix.overall_score();

        prop_assert!(score >= 0.0, "Score should be >= 0");
        prop_assert!(score <= 100.0, "Score should be <= 100");
    }

    /// Property 9f: Pass rate calculation is correct
    #[test]
    fn prop_pass_rate_calculation(
        passed in 0usize..1000,
        failed in 0usize..100,
        skipped in 0usize..50,
        errors in 0usize..20
    ) {
        let total = passed + failed + skipped + errors;
        let result = FrameworkTestResult {
            framework: FrameworkInfo::new("Test", "1.0"),
            total_tests: total,
            passed,
            failed,
            skipped,
            errors,
            failure_categories: HashMap::new(),
            duration: Duration::from_secs(1),
            timestamp: chrono::Utc::now(),
            raw_output: None,
        };

        let rate = result.pass_rate();

        if total == 0 {
            prop_assert_eq!(rate, 0.0, "Empty result should have 0% pass rate");
        } else {
            let expected = passed as f64 / total as f64;
            prop_assert!((rate - expected).abs() < 0.0001,
                "Pass rate calculation incorrect: expected {}, got {}", expected, rate);
        }
    }

    /// Property 9g: JSON output is valid and complete
    #[test]
    fn prop_json_output_complete(
        results in prop::collection::vec(arb_framework_test_result(), 1..5),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results.clone());

        let json = matrix.generate_json();

        // Must have required fields
        prop_assert!(json.get("dx_py_version").is_some(), "JSON must have dx_py_version");
        prop_assert!(json.get("overall_score").is_some(), "JSON must have overall_score");
        prop_assert!(json.get("frameworks").is_some(), "JSON must have frameworks");
        prop_assert!(json.get("generated_at").is_some(), "JSON must have generated_at");

        // Framework count must match
        let frameworks = json["frameworks"].as_array().unwrap();
        prop_assert_eq!(frameworks.len(), results.len(),
            "JSON framework count should match input");
    }

    /// Property 9h: Snapshot preserves all data
    #[test]
    fn prop_snapshot_preserves_data(
        results in prop::collection::vec(arb_framework_test_result(), 1..5),
        version in arb_dx_py_version()
    ) {
        let mut matrix = CompatibilityMatrix::new(&version);
        matrix.add_results(results.clone());

        let snapshot = matrix.create_snapshot();

        prop_assert_eq!(snapshot.dx_py_version, version);
        prop_assert_eq!(snapshot.results.len(), results.len());

        // Verify pass rates are preserved
        for (original, snapshotted) in results.iter().zip(snapshot.results.iter()) {
            prop_assert!((original.pass_rate() - snapshotted.pass_rate()).abs() < 0.0001,
                "Snapshot should preserve pass rates");
        }
    }
}

// Additional unit tests for edge cases

#[test]
fn test_empty_matrix() {
    let matrix = CompatibilityMatrix::new("0.1.0");

    assert_eq!(matrix.overall_score(), 0.0);
    assert_eq!(matrix.overall_pass_rate(), 0.0);

    let md = matrix.generate_markdown();
    assert!(md.contains("DX-Py Compatibility Matrix"));
}

#[test]
fn test_failure_categorizer_all_categories() {
    let _categorizer = FailureCategorizer::new();

    // Test each category has a description
    let categories = [
        FailureCategory::CExtensionLoad,
        FailureCategory::MissingApi,
        FailureCategory::AsyncBehavior,
        FailureCategory::ImportError,
        FailureCategory::RuntimeError,
        FailureCategory::TypeMismatch,
        FailureCategory::MemoryError,
        FailureCategory::AssertionError,
        FailureCategory::Timeout,
        FailureCategory::Unknown,
    ];

    for cat in categories {
        assert!(!cat.description().is_empty(), "Category {:?} should have a description", cat);
    }
}

#[test]
fn test_regression_report_markdown() {
    let prev = CompatibilitySnapshot::new(
        "0.1.0",
        vec![FrameworkTestResult {
            framework: FrameworkInfo::new("Django", "4.2"),
            total_tests: 100,
            passed: 80,
            failed: 20,
            skipped: 0,
            errors: 0,
            failure_categories: HashMap::new(),
            duration: Duration::from_secs(10),
            timestamp: chrono::Utc::now(),
            raw_output: None,
        }],
    );

    let curr = CompatibilitySnapshot::new(
        "0.2.0",
        vec![FrameworkTestResult {
            framework: FrameworkInfo::new("Django", "4.2"),
            total_tests: 100,
            passed: 95,
            failed: 5,
            skipped: 0,
            errors: 0,
            failure_categories: HashMap::new(),
            duration: Duration::from_secs(10),
            timestamp: chrono::Utc::now(),
            raw_output: None,
        }],
    );

    let detector = RegressionDetector::new();
    let report = detector.compare(&prev, &curr);
    let md = report.generate_markdown();

    assert!(md.contains("Regression Report"));
    assert!(md.contains("0.1.0"));
    assert!(md.contains("0.2.0"));
    assert!(md.contains("Django"));
}

// =========================================================================
// Property 11: Benchmark Metrics Validity
// *For any* framework benchmark run, the Framework_Validator should produce
// metrics that include: (a) valid timing measurements, (b) CPython comparison
// on identical workloads, (c) appropriate metrics for the framework type
// (latency/throughput for web, operation time for data).
// **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
// =========================================================================

fn arb_duration_ms() -> impl Strategy<Value = f64> {
    // Generate realistic benchmark durations (0.1ms to 10 seconds)
    0.1f64..10000.0
}

fn arb_memory_mb() -> impl Strategy<Value = Option<f64>> {
    prop_oneof![Just(None), (1.0f64..1000.0).prop_map(Some),]
}

fn arb_throughput() -> impl Strategy<Value = Option<f64>> {
    prop_oneof![Just(None), (1.0f64..100000.0).prop_map(Some),]
}

fn arb_benchmark_metrics() -> impl Strategy<Value = BenchmarkMetrics> {
    (arb_duration_ms(), arb_memory_mb(), arb_throughput()).prop_map(
        |(duration, memory, throughput)| {
            let mut metrics = BenchmarkMetrics::new(duration);
            if let Some(mem) = memory {
                metrics = metrics.with_memory(mem);
            }
            if let Some(tp) = throughput {
                metrics = metrics.with_throughput(tp);
            }
            metrics
        },
    )
}

fn arb_benchmark_framework() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Django".to_string()),
        Just("Flask".to_string()),
        Just("FastAPI".to_string()),
        Just("NumPy".to_string()),
        Just("Pandas".to_string()),
    ]
}

fn arb_workload() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("request_latency".to_string()),
        Just("orm_operations".to_string()),
        Just("template_rendering".to_string()),
        Just("array_creation".to_string()),
        Just("arithmetic_operations".to_string()),
        Just("linear_algebra".to_string()),
        Just("dataframe_creation".to_string()),
        Just("groupby_operations".to_string()),
        Just("io_operations".to_string()),
    ]
}

fn arb_real_world_benchmark() -> impl Strategy<Value = RealWorldBenchmark> {
    (
        arb_benchmark_framework(),
        arb_workload(),
        arb_benchmark_metrics(),
        arb_benchmark_metrics(),
    )
        .prop_map(|(framework, workload, dxpy, cpython)| {
            RealWorldBenchmark::new(framework, workload, dxpy, cpython)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11a: Benchmark metrics have valid timing measurements
    #[test]
    fn prop_benchmark_valid_timing(
        benchmark in arb_real_world_benchmark()
    ) {
        let validation = validate_benchmark_metrics(&benchmark);

        // If both durations are positive, timing should be valid
        if benchmark.dxpy_result.duration_ms > 0.0 && benchmark.cpython_result.duration_ms > 0.0 {
            prop_assert!(validation.has_timing,
                "Benchmark with positive durations should have valid timing");
        }
    }

    /// Property 11b: Benchmark includes CPython comparison
    #[test]
    fn prop_benchmark_has_cpython_comparison(
        benchmark in arb_real_world_benchmark()
    ) {
        let validation = validate_benchmark_metrics(&benchmark);

        // All benchmarks should have CPython comparison
        prop_assert!(validation.has_cpython_comparison,
            "All benchmarks should include CPython comparison");

        // Speedup should be calculated
        prop_assert!(benchmark.speedup > 0.0,
            "Speedup should be positive");
    }

    /// Property 11c: Speedup calculation is correct
    #[test]
    fn prop_speedup_calculation_correct(
        dxpy_duration in 0.1f64..10000.0,
        cpython_duration in 0.1f64..10000.0
    ) {
        let dxpy = BenchmarkMetrics::new(dxpy_duration);
        let cpython = BenchmarkMetrics::new(cpython_duration);

        let benchmark = RealWorldBenchmark::new("Test", "test_workload", dxpy, cpython);

        let expected_speedup = cpython_duration / dxpy_duration;
        prop_assert!((benchmark.speedup - expected_speedup).abs() < 0.0001,
            "Speedup calculation incorrect: expected {}, got {}",
            expected_speedup, benchmark.speedup);
    }

    /// Property 11d: is_dxpy_faster is consistent with speedup
    #[test]
    fn prop_is_faster_consistent(
        benchmark in arb_real_world_benchmark()
    ) {
        let is_faster = benchmark.is_dxpy_faster();

        if benchmark.speedup > 1.0 {
            prop_assert!(is_faster,
                "DX-Py should be marked faster when speedup > 1.0");
        } else {
            prop_assert!(!is_faster,
                "DX-Py should not be marked faster when speedup <= 1.0");
        }
    }

    /// Property 11e: Speedup percentage is consistent with speedup
    #[test]
    fn prop_speedup_percentage_consistent(
        benchmark in arb_real_world_benchmark()
    ) {
        let percentage = benchmark.speedup_percentage();
        let expected = (benchmark.speedup - 1.0) * 100.0;

        prop_assert!((percentage - expected).abs() < 0.0001,
            "Speedup percentage incorrect: expected {}, got {}",
            expected, percentage);
    }

    /// Property 11f: Memory ratio is calculated correctly when both have memory
    #[test]
    fn prop_memory_ratio_correct(
        dxpy_memory in 1.0f64..1000.0,
        cpython_memory in 1.0f64..1000.0
    ) {
        let dxpy = BenchmarkMetrics::new(100.0).with_memory(dxpy_memory);
        let cpython = BenchmarkMetrics::new(100.0).with_memory(cpython_memory);

        let benchmark = RealWorldBenchmark::new("Test", "test", dxpy, cpython);

        let expected_ratio = dxpy_memory / cpython_memory;
        prop_assert!(benchmark.memory_ratio.is_some(),
            "Memory ratio should be present when both have memory");
        prop_assert!((benchmark.memory_ratio.unwrap() - expected_ratio).abs() < 0.0001,
            "Memory ratio incorrect");
    }

    /// Property 11g: Throughput ratio is calculated correctly when both have throughput
    #[test]
    fn prop_throughput_ratio_correct(
        dxpy_throughput in 1.0f64..100000.0,
        cpython_throughput in 1.0f64..100000.0
    ) {
        let dxpy = BenchmarkMetrics::new(100.0).with_throughput(dxpy_throughput);
        let cpython = BenchmarkMetrics::new(100.0).with_throughput(cpython_throughput);

        let benchmark = RealWorldBenchmark::new("Test", "test", dxpy, cpython);

        let expected_ratio = dxpy_throughput / cpython_throughput;
        prop_assert!(benchmark.throughput_ratio.is_some(),
            "Throughput ratio should be present when both have throughput");
        prop_assert!((benchmark.throughput_ratio.unwrap() - expected_ratio).abs() < 0.0001,
            "Throughput ratio incorrect");
    }

    /// Property 11h: Benchmark config validation works correctly
    #[test]
    fn prop_config_validation(
        warmup in 0u32..10,
        measurements in 0u32..100,
        timeout_secs in 0u64..3600
    ) {
        let config = BenchmarkConfig {
            warmup_iterations: warmup,
            measurement_iterations: measurements,
            timeout: std::time::Duration::from_secs(timeout_secs),
            ..Default::default()
        };

        let result = config.validate();

        if measurements == 0 {
            prop_assert!(result.is_err(),
                "Config with 0 measurements should be invalid");
        } else if timeout_secs == 0 {
            prop_assert!(result.is_err(),
                "Config with 0 timeout should be invalid");
        } else {
            prop_assert!(result.is_ok(),
                "Config with valid values should be valid");
        }
    }

    /// Property 11i: Report generator produces valid output
    #[test]
    fn prop_report_generator_valid(
        benchmarks in prop::collection::vec(arb_real_world_benchmark(), 1..10)
    ) {
        let mut generator = BenchmarkReportGenerator::new();
        for benchmark in &benchmarks {
            generator.add_result(benchmark.clone());
        }

        let markdown = generator.generate_markdown();
        let json = generator.generate_json();

        // Markdown should contain header
        prop_assert!(markdown.contains("# Real-World Benchmark Report"),
            "Markdown should contain header");

        // Markdown should contain summary
        prop_assert!(markdown.contains("## Summary"),
            "Markdown should contain summary");

        // JSON should be valid
        let parsed: Result<Vec<RealWorldBenchmark>, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "JSON should be valid");

        // JSON should have correct count
        let parsed = parsed.unwrap();
        prop_assert_eq!(parsed.len(), benchmarks.len(),
            "JSON should contain all benchmarks");
    }

    /// Property 11j: Validation detects invalid metrics
    #[test]
    fn prop_validation_detects_invalid(
        valid_duration in 0.1f64..10000.0
    ) {
        // Test with invalid DX-Py duration
        let invalid_dxpy = BenchmarkMetrics::new(-1.0);
        let valid_cpython = BenchmarkMetrics::new(valid_duration);
        let benchmark = RealWorldBenchmark::new("Test", "test", invalid_dxpy, valid_cpython);

        let validation = validate_benchmark_metrics(&benchmark);
        prop_assert!(!validation.is_valid,
            "Benchmark with negative duration should be invalid");
        prop_assert!(!validation.has_timing,
            "Benchmark with negative duration should not have valid timing");
    }
}

// Additional unit tests for benchmark module

#[test]
fn test_benchmark_config_defaults() {
    let config = BenchmarkConfig::default();
    assert_eq!(config.warmup_iterations, 3);
    assert_eq!(config.measurement_iterations, 10);
    assert!(config.measure_memory);
    assert!(config.measure_throughput);
}

#[test]
fn test_benchmark_config_builder() {
    let config = BenchmarkConfig::new()
        .with_warmup(5)
        .with_measurements(20)
        .with_timeout(std::time::Duration::from_secs(600))
        .with_dxpy_interpreter("./dx-py")
        .with_cpython_interpreter("python3.11");

    assert_eq!(config.warmup_iterations, 5);
    assert_eq!(config.measurement_iterations, 20);
    assert_eq!(config.timeout, std::time::Duration::from_secs(600));
    assert_eq!(config.dxpy_interpreter, "./dx-py");
    assert_eq!(config.cpython_interpreter, "python3.11");
}

#[test]
fn test_benchmark_metrics_custom() {
    let metrics = BenchmarkMetrics::new(100.0)
        .with_memory(50.0)
        .with_throughput(1000.0)
        .with_custom("iterations", 1000.0)
        .with_custom("cache_hits", 950.0);

    assert_eq!(metrics.duration_ms, 100.0);
    assert_eq!(metrics.memory_mb, Some(50.0));
    assert_eq!(metrics.throughput, Some(1000.0));
    assert_eq!(metrics.custom_metrics.get("iterations"), Some(&1000.0));
    assert_eq!(metrics.custom_metrics.get("cache_hits"), Some(&950.0));
}

#[test]
fn test_real_world_benchmark_faster() {
    let dxpy = BenchmarkMetrics::new(80.0);
    let cpython = BenchmarkMetrics::new(100.0);

    let benchmark = RealWorldBenchmark::new("Django", "request_latency", dxpy, cpython);

    assert!(benchmark.is_dxpy_faster());
    assert_eq!(benchmark.speedup, 1.25);
    assert_eq!(benchmark.speedup_percentage(), 25.0);
}

#[test]
fn test_real_world_benchmark_slower() {
    let dxpy = BenchmarkMetrics::new(150.0);
    let cpython = BenchmarkMetrics::new(100.0);

    let benchmark = RealWorldBenchmark::new("NumPy", "array_ops", dxpy, cpython);

    assert!(!benchmark.is_dxpy_faster());
    assert!(benchmark.speedup < 1.0);
    assert!(benchmark.speedup_percentage() < 0.0);
}

#[test]
fn test_benchmark_validation_web_framework() {
    let dxpy = BenchmarkMetrics::new(100.0);
    let cpython = BenchmarkMetrics::new(100.0);

    let benchmark = RealWorldBenchmark::new("Django", "request_latency", dxpy, cpython);
    let validation = validate_benchmark_metrics(&benchmark);

    assert!(validation.is_valid);
    assert!(validation.has_timing);
    assert!(validation.has_cpython_comparison);
    // Should warn about missing throughput for web framework request benchmark
    assert!(!validation.warnings.is_empty());
}

#[test]
fn test_benchmark_validation_data_framework() {
    let dxpy = BenchmarkMetrics::new(100.0);
    let cpython = BenchmarkMetrics::new(100.0);

    let benchmark = RealWorldBenchmark::new("NumPy", "array_creation", dxpy, cpython);
    let validation = validate_benchmark_metrics(&benchmark);

    assert!(validation.is_valid);
    assert!(validation.has_timing);
    assert!(validation.has_cpython_comparison);
    // Data frameworks don't need throughput for array operations
    assert!(validation.warnings.is_empty());
}

#[test]
fn test_report_generator_empty() {
    let generator = BenchmarkReportGenerator::new();

    let markdown = generator.generate_markdown();
    assert!(markdown.contains("# Real-World Benchmark Report"));

    let json = generator.generate_json();
    assert_eq!(json, "[]");
}

#[test]
fn test_report_generator_multiple_frameworks() {
    let mut generator = BenchmarkReportGenerator::new();

    generator.add_result(RealWorldBenchmark::new(
        "Django",
        "request_latency",
        BenchmarkMetrics::new(80.0),
        BenchmarkMetrics::new(100.0),
    ));

    generator.add_result(RealWorldBenchmark::new(
        "NumPy",
        "array_creation",
        BenchmarkMetrics::new(50.0),
        BenchmarkMetrics::new(60.0),
    ));

    let markdown = generator.generate_markdown();
    assert!(markdown.contains("Django"));
    assert!(markdown.contains("NumPy"));
    assert!(markdown.contains("request_latency"));
    assert!(markdown.contains("array_creation"));
}

// =========================================================================
// Property 12: Graceful Degradation Informativeness
// *For any* incompatibility encountered (extension load failure, missing API,
// partial framework support), the error or warning message should contain:
// (a) the specific incompatibility, (b) actionable information (workaround
// or alternative if available).
// **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5**
// =========================================================================

fn arb_extension_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("numpy".to_string()),
        Just("pandas".to_string()),
        Just("tensorflow".to_string()),
        Just("torch".to_string()),
        Just("scipy".to_string()),
        "[a-z][a-z0-9_]{2,15}".prop_map(|s| s),
    ]
}

fn arb_path() -> impl Strategy<Value = std::path::PathBuf> {
    prop_oneof![
        Just(std::path::PathBuf::from("/usr/lib/python3.11")),
        Just(std::path::PathBuf::from("/home/user/.local/lib")),
        Just(std::path::PathBuf::from("C:\\Python311\\Lib")),
        "[a-z/]{5,30}".prop_map(std::path::PathBuf::from),
    ]
}

fn arb_version_string() -> impl Strategy<Value = String> {
    (3u32..4, 8u32..13).prop_map(|(major, minor)| format!("{}.{}", major, minor))
}

fn arb_api_function() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("PyArg_ParseTuple".to_string()),
        Just("Py_BuildValue".to_string()),
        Just("PyObject_GetAttr".to_string()),
        Just("PyDict_SetItem".to_string()),
        Just("PyList_Append".to_string()),
        "Py[A-Z][a-zA-Z_]{5,20}".prop_map(|s| s),
    ]
}

fn arb_failure_reason() -> impl Strategy<Value = FailureReason> {
    prop_oneof![
        prop::collection::vec(arb_path(), 1..5).prop_map(|paths| {
            FailureReason::NotFound {
                searched_paths: paths,
            }
        }),
        (arb_version_string(), arb_version_string()).prop_map(|(expected, found)| {
            FailureReason::AbiMismatch {
                expected_version: expected,
                found_version: found,
            }
        }),
        prop::collection::vec(arb_extension_name(), 1..5)
            .prop_map(|deps| { FailureReason::MissingDependencies { dependencies: deps } }),
        prop_oneof![
            Just("linux-x86_64".to_string()),
            Just("win32".to_string()),
            Just("darwin-arm64".to_string()),
        ]
        .prop_map(|platform| { FailureReason::UnsupportedPlatform { platform } }),
        prop::collection::vec(arb_api_function(), 1..5)
            .prop_map(|funcs| { FailureReason::MissingApiFunctions { functions: funcs } }),
        "[A-Za-z ]{10,50}".prop_map(|reason| { FailureReason::InitializationFailed { reason } }),
        Just(FailureReason::PermissionDenied),
        Just(FailureReason::CorruptedFile),
        "[A-Za-z ]{10,50}".prop_map(|details| { FailureReason::Unknown { details } }),
    ]
}

fn arb_extension_failure_info() -> impl Strategy<Value = ExtensionFailureInfo> {
    (arb_extension_name(), arb_failure_reason())
        .prop_map(|(name, reason)| ExtensionFailureInfo::new(name, reason))
}

fn arb_support_level() -> impl Strategy<Value = SupportLevel> {
    prop_oneof![
        Just(SupportLevel::Full),
        Just(SupportLevel::Partial),
        Just(SupportLevel::None),
    ]
}

fn arb_feature_compatibility() -> impl Strategy<Value = FeatureCompatibility> {
    (
        "[A-Za-z]{3,15}".prop_map(|s| s),
        any::<bool>(),
        arb_support_level(),
        prop::option::of("[A-Za-z ]{10,50}".prop_map(|s| s)),
    )
        .prop_map(|(name, supported, level, notes)| {
            let mut feature = FeatureCompatibility::new(name, supported);
            feature.support_level = level;
            feature.notes = notes;
            feature
        })
}

fn arb_issue_severity() -> impl Strategy<Value = IssueSeverity> {
    prop_oneof![
        Just(IssueSeverity::Critical),
        Just(IssueSeverity::High),
        Just(IssueSeverity::Medium),
        Just(IssueSeverity::Low),
    ]
}

fn arb_known_issue() -> impl Strategy<Value = KnownIssue> {
    (
        "[A-Za-z ]{5,30}".prop_map(|s| s),
        "[A-Za-z ]{20,100}".prop_map(|s| s),
        arb_issue_severity(),
    )
        .prop_map(|(title, description, severity)| KnownIssue::new(title, description, severity))
}

fn arb_partial_compatibility_report() -> impl Strategy<Value = PartialCompatibilityReport> {
    (
        arb_benchmark_framework(),
        arb_version(),
        prop::collection::vec(arb_feature_compatibility(), 1..10),
        prop::collection::vec(arb_known_issue(), 0..5),
    )
        .prop_map(|(framework, version, features, issues)| {
            PartialCompatibilityReport::new(framework, version)
                .with_features(features)
                .with_known_issues(issues)
        })
}

#[allow(dead_code)]
fn arb_incompatibility_type() -> impl Strategy<Value = IncompatibilityType> {
    prop_oneof![
        Just(IncompatibilityType::CExtension),
        Just(IncompatibilityType::MissingApi),
        Just(IncompatibilityType::AsyncBehavior),
        Just(IncompatibilityType::Platform),
        Just(IncompatibilityType::Other),
    ]
}

#[allow(dead_code)]
fn arb_incompatibility_info() -> impl Strategy<Value = IncompatibilityInfo> {
    (
        "[A-Za-z ]{5,30}".prop_map(|s| s),
        "[A-Za-z ]{20,100}".prop_map(|s| s),
        arb_incompatibility_type(),
    )
        .prop_map(|(title, description, itype)| IncompatibilityInfo::new(title, description, itype))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12a: Extension failure info contains specific incompatibility
    #[test]
    fn prop_failure_info_contains_specifics(
        info in arb_extension_failure_info()
    ) {
        let detailed = info.detailed_message();

        // Must contain extension name
        prop_assert!(detailed.contains(&info.extension_name),
            "Detailed message should contain extension name");

        // Must contain error message
        prop_assert!(detailed.contains("Reason:"),
            "Detailed message should contain reason");
    }

    /// Property 12b: Extension failure info provides workarounds
    #[test]
    fn prop_failure_info_has_workarounds(
        info in arb_extension_failure_info()
    ) {
        // All failure types should have at least one workaround
        prop_assert!(!info.workarounds.is_empty(),
            "Failure info should have at least one workaround");

        // Each workaround should have a description
        for workaround in &info.workarounds {
            prop_assert!(!workaround.description.is_empty(),
                "Workaround should have a description");
        }
    }

    /// Property 12c: Error messages are specific to failure type
    #[test]
    fn prop_error_message_specific_to_type(
        name in arb_extension_name(),
        reason in arb_failure_reason()
    ) {
        let message = reason.to_error_message(&name);

        // Message should contain extension name
        prop_assert!(message.contains(&name),
            "Error message should contain extension name");

        // Message should be specific to failure type
        match &reason {
            FailureReason::NotFound { .. } => {
                prop_assert!(message.contains("not found") || message.contains("Not found"),
                    "NotFound error should mention 'not found'");
            }
            FailureReason::AbiMismatch { expected_version, found_version } => {
                prop_assert!(message.contains(expected_version) && message.contains(found_version),
                    "AbiMismatch error should mention both versions");
            }
            FailureReason::MissingDependencies { dependencies } => {
                for dep in dependencies {
                    prop_assert!(message.contains(dep),
                        "MissingDependencies error should list dependencies");
                }
            }
            FailureReason::UnsupportedPlatform { platform } => {
                prop_assert!(message.contains(platform),
                    "UnsupportedPlatform error should mention platform");
            }
            FailureReason::MissingApiFunctions { functions } => {
                for func in functions {
                    prop_assert!(message.contains(func),
                        "MissingApiFunctions error should list functions");
                }
            }
            _ => {}
        }
    }

    /// Property 12d: Partial compatibility report contains feature info
    #[test]
    fn prop_partial_report_contains_features(
        report in arb_partial_compatibility_report()
    ) {
        let markdown = report.generate_markdown();

        // Should contain framework name
        prop_assert!(markdown.contains(&report.framework),
            "Report should contain framework name");

        // Should contain version
        prop_assert!(markdown.contains(&report.version),
            "Report should contain version");

        // Should contain feature table
        prop_assert!(markdown.contains("| Feature |"),
            "Report should contain feature table");

        // Should contain each feature name
        for feature in &report.features {
            prop_assert!(markdown.contains(&feature.name),
                "Report should contain feature name: {}", feature.name);
        }
    }

    /// Property 12e: Support level is correctly determined
    #[test]
    fn prop_support_level_correct(
        features in prop::collection::vec(arb_feature_compatibility(), 1..10)
    ) {
        let report = PartialCompatibilityReport::new("Test", "1.0")
            .with_features(features.clone());

        let all_full = features.iter().all(|f| f.support_level == SupportLevel::Full);
        let any_supported = features.iter().any(|f| f.supported);

        if all_full {
            prop_assert_eq!(report.overall_support, SupportLevel::Full,
                "All full features should result in Full support");
        } else if any_supported {
            prop_assert_eq!(report.overall_support, SupportLevel::Partial,
                "Mixed support should result in Partial support");
        } else {
            prop_assert_eq!(report.overall_support, SupportLevel::None,
                "No supported features should result in None support");
        }
    }

    /// Property 12f: Compatibility checker detects known incompatibilities
    #[test]
    fn prop_checker_detects_incompatibilities(
        imports in prop::collection::vec(arb_extension_name(), 1..10)
    ) {
        let checker = CompatibilityChecker::new();
        let result = checker.check_imports(&imports);

        // If any import has known incompatibilities, result should not be compatible
        let has_known_incompatible = imports.iter()
            .any(|i| checker.has_incompatibilities(i));

        if has_known_incompatible {
            prop_assert!(!result.is_compatible,
                "Result should be incompatible when known incompatibilities exist");
            prop_assert!(!result.incompatibilities.is_empty(),
                "Incompatibilities should be listed");
        }
    }

    /// Property 12g: Compatibility check report is informative
    #[test]
    fn prop_check_report_informative(
        imports in prop::collection::vec(arb_extension_name(), 1..10)
    ) {
        let checker = CompatibilityChecker::new();
        let result = checker.check_imports(&imports);
        let report = result.generate_report();

        // Report should not be empty
        prop_assert!(!report.is_empty(),
            "Report should not be empty");

        // If compatible, should say so
        if result.is_compatible && result.warnings.is_empty() {
            prop_assert!(report.contains("compatible") || report.contains("Compatible"),
                "Compatible result should mention compatibility");
        }

        // If incompatible, should list issues
        if !result.is_compatible {
            prop_assert!(report.contains("Incompatibilities") || report.contains("incompatibilities"),
                "Incompatible result should list incompatibilities");
        }
    }

    /// Property 12h: Workarounds have valid confidence levels
    #[test]
    fn prop_workaround_confidence_valid(
        confidence in 0.0f64..2.0
    ) {
        let workaround = Workaround::new("Test workaround", None)
            .with_confidence(confidence);

        prop_assert!(workaround.confidence >= 0.0,
            "Confidence should be >= 0");
        prop_assert!(workaround.confidence <= 1.0,
            "Confidence should be <= 1");
    }

    /// Property 12i: Documentation links are provided
    #[test]
    fn prop_documentation_links_provided(
        reason in arb_failure_reason()
    ) {
        let links = reason.documentation_links();

        prop_assert!(!links.is_empty(),
            "Documentation links should be provided");

        for link in &links {
            prop_assert!(link.starts_with("http"),
                "Documentation links should be URLs");
        }
    }

    /// Property 12j: Import extraction works correctly
    #[test]
    fn prop_import_extraction(
        module_name in "[a-z][a-z0-9_]{2,15}"
    ) {
        let checker = CompatibilityChecker::new();

        // Test various import formats
        let code1 = format!("import {}", module_name);
        let imports1 = checker.extract_imports(&code1);
        prop_assert!(imports1.contains(&module_name),
            "Should extract 'import x' format");

        let code2 = format!("from {} import something", module_name);
        let imports2 = checker.extract_imports(&code2);
        prop_assert!(imports2.contains(&module_name),
            "Should extract 'from x import y' format");
    }
}

// Additional unit tests for degradation module

#[test]
fn test_extension_failure_detailed_message() {
    let info = ExtensionFailureInfo::new(
        "numpy",
        FailureReason::AbiMismatch {
            expected_version: "3.11".to_string(),
            found_version: "3.10".to_string(),
        },
    )
    .with_path("/usr/lib/python3.11/site-packages/numpy");

    let msg = info.detailed_message();
    assert!(msg.contains("numpy"));
    assert!(msg.contains("ABI"));
    assert!(msg.contains("3.11"));
    assert!(msg.contains("3.10"));
    assert!(msg.contains("Workarounds"));
}

#[test]
fn test_failure_reason_not_found() {
    let reason = FailureReason::NotFound {
        searched_paths: vec![
            std::path::PathBuf::from("/usr/lib"),
            std::path::PathBuf::from("/home/user/.local"),
        ],
    };

    let msg = reason.to_error_message("test_ext");
    assert!(msg.contains("not found"));
    assert!(msg.contains("/usr/lib"));
    assert!(msg.contains("/home/user/.local"));

    let workarounds = reason.suggested_workarounds("test_ext");
    assert!(!workarounds.is_empty());
    assert!(workarounds.iter().any(|w| w.command.is_some()));
}

#[test]
fn test_partial_compatibility_report_markdown() {
    let report = PartialCompatibilityReport::new("Django", "4.2")
        .with_features(vec![
            FeatureCompatibility::new("Core", true),
            FeatureCompatibility::partial("ORM", "Complex queries limited"),
            FeatureCompatibility::new("Admin", false),
        ])
        .with_known_issues(vec![KnownIssue::new(
            "Memory leak",
            "Memory leak in large queries",
            IssueSeverity::Medium,
        )
        .with_workaround("Use pagination")])
        .with_recommendations(vec!["Use chunked queries for large datasets".to_string()]);

    let md = report.generate_markdown();
    assert!(md.contains("Django"));
    assert!(md.contains("4.2"));
    assert!(md.contains("Core"));
    assert!(md.contains("ORM"));
    assert!(md.contains("Admin"));
    assert!(md.contains("Memory leak"));
    assert!(md.contains("Recommendations"));
}

#[test]
fn test_compatibility_checker_scan_file() {
    let checker = CompatibilityChecker::new();
    let code = r#"
import numpy as np
import pandas
from tensorflow import keras
import os
"#;

    let result = checker.scan_file(code);

    // numpy and pandas should be supported
    assert_eq!(result.support_levels.get("numpy"), Some(&SupportLevel::Full));
    assert_eq!(result.support_levels.get("pandas"), Some(&SupportLevel::Full));

    // tensorflow should have incompatibilities
    assert!(!result.is_compatible);
    assert!(result.incompatibilities.contains_key("tensorflow"));
}

#[test]
fn test_incompatibility_info() {
    let info = IncompatibilityInfo::new(
        "Custom C extension",
        "Uses internal CPython structures",
        IncompatibilityType::CExtension,
    )
    .with_severity(IssueSeverity::Critical)
    .with_workaround("Use pure Python implementation");

    assert_eq!(info.severity, IssueSeverity::Critical);
    assert!(info.workaround.is_some());
    assert_eq!(info.incompatibility_type, IncompatibilityType::CExtension);
}

#[test]
fn test_support_level_display() {
    assert_eq!(SupportLevel::Full.as_str(), "Full");
    assert_eq!(SupportLevel::Partial.as_str(), "Partial");
    assert_eq!(SupportLevel::None.as_str(), "None");

    assert_eq!(SupportLevel::Full.emoji(), "✅");
    assert_eq!(SupportLevel::Partial.emoji(), "⚠️");
    assert_eq!(SupportLevel::None.emoji(), "❌");
}

#[test]
fn test_issue_severity_display() {
    assert_eq!(format!("{}", IssueSeverity::Critical), "Critical");
    assert_eq!(format!("{}", IssueSeverity::High), "High");
    assert_eq!(format!("{}", IssueSeverity::Medium), "Medium");
    assert_eq!(format!("{}", IssueSeverity::Low), "Low");
}
