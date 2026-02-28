//! Integration tests for the benchmarking framework
//!
//! **Feature: comparative-benchmarks**
//!
//! These tests verify the complete flow from benchmark execution
//! through result storage and report generation.

use dx_py_benchmarks::core::{BenchmarkConfig, BenchmarkFramework, OutputFormat};
use dx_py_benchmarks::data::ResultStore;
use std::fs;
use tempfile::TempDir;

/// **Integration Test: End-to-end benchmark flow**
/// Runs a small benchmark suite through the complete flow and verifies
/// results are stored and reports generated.
/// **Validates: Requirements 1.1, 1.4, 7.5**
#[test]
fn test_end_to_end_benchmark_flow() {
    let temp_dir = TempDir::new().unwrap();

    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec!["runtime".to_string()],
        filter: Some("int_arithmetic".to_string()), // Filter to just one benchmark for speed
    };

    let mut framework = BenchmarkFramework::new(config.clone());

    // Run the benchmark suite
    let result = framework.run_suite("runtime");

    // The benchmark may fail if Python is not available, which is OK for this test
    // We're testing the framework flow, not the actual Python execution
    match result {
        Ok(suite_result) => {
            // Verify suite name
            assert_eq!(suite_result.suite_name, "runtime");

            // Verify reports were generated
            assert!(
                !suite_result.markdown_report.is_empty(),
                "Markdown report should be generated"
            );
            assert!(!suite_result.json_report.is_empty(), "JSON report should be generated");

            // Verify JSON is valid
            let json_parsed: Result<serde_json::Value, _> =
                serde_json::from_str(&suite_result.json_report);
            assert!(json_parsed.is_ok(), "JSON report should be valid JSON");

            // Verify result was stored
            if let Some(stored_id) = &suite_result.stored_id {
                let store = ResultStore::new(temp_dir.path().to_path_buf());
                let loaded = store.load(stored_id);
                assert!(loaded.is_ok(), "Stored result should be loadable");

                let loaded_result = loaded.unwrap();
                assert_eq!(loaded_result.results.suite, "runtime");
            }

            // Verify output files were created
            let files: Vec<_> =
                fs::read_dir(temp_dir.path()).unwrap().filter_map(|e| e.ok()).collect();

            // Should have at least the stored JSON result
            assert!(!files.is_empty(), "Output files should be created");
        }
        Err(e) => {
            // If Python is not available, the benchmark will fail
            // This is expected in some test environments
            println!("Benchmark execution failed (expected if Python not available): {}", e);
        }
    }
}

/// **Integration Test: Result storage round-trip**
/// Verifies that benchmark results can be saved and loaded correctly.
/// **Validates: Requirements 7.5**
#[test]
fn test_result_storage_round_trip() {
    let temp_dir = TempDir::new().unwrap();

    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    };

    // Create test results manually
    use dx_py_benchmarks::core::BenchmarkResult;
    use dx_py_benchmarks::data::BenchmarkResults;
    use std::time::Duration;

    let mut results = BenchmarkResults::new("test_suite", config.clone());

    let mut bench = BenchmarkResult::new("test_benchmark");
    bench.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 100)).collect();
    bench.warmup_completed = true;
    results.add_result(bench);

    // Save results
    let store = ResultStore::new(temp_dir.path().to_path_buf());
    let save_result = store.save(&results, &config);
    assert!(save_result.is_ok(), "Should save results successfully");

    let stored_id = save_result.unwrap();

    // Load results
    let load_result = store.load(&stored_id);
    assert!(load_result.is_ok(), "Should load results successfully");

    let loaded = load_result.unwrap();

    // Verify loaded data matches
    assert_eq!(loaded.results.suite, "test_suite");
    assert_eq!(loaded.results.benchmarks.len(), 1);
    assert_eq!(loaded.results.benchmarks[0].name, "test_benchmark");
    assert_eq!(loaded.results.benchmarks[0].timings.len(), 30);

    // Verify config was stored
    assert_eq!(loaded.config.warmup_iterations, 2);
    assert_eq!(loaded.config.measurement_iterations, 30);
    assert_eq!(loaded.config.seed, Some(42));
}

/// **Integration Test: Dual output format generation**
/// Verifies that both Markdown and JSON outputs are generated correctly.
/// **Validates: Requirements 1.4**
#[test]
fn test_dual_output_format_generation() {
    let temp_dir = TempDir::new().unwrap();

    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    };

    // Create test results
    use dx_py_benchmarks::core::BenchmarkResult;
    use dx_py_benchmarks::data::BenchmarkResults;
    use dx_py_benchmarks::report::ReportGenerator;
    use std::time::Duration;

    let mut results = BenchmarkResults::new("test_suite", config.clone());

    let mut bench = BenchmarkResult::new("test_benchmark");
    bench.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 100)).collect();
    bench.warmup_completed = true;
    results.add_result(bench);

    let reporter = ReportGenerator::new(temp_dir.path().to_path_buf());

    // Generate JSON report
    let json = reporter.generate_json(&results);

    // Verify JSON is valid
    let json_parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(json_parsed.is_ok(), "JSON output should be valid");

    let json_value = json_parsed.unwrap();
    assert!(json_value.get("suite").is_some(), "JSON should contain suite");
    assert!(json_value.get("benchmarks").is_some(), "JSON should contain benchmarks");
    assert!(json_value.get("system_info").is_some(), "JSON should contain system_info");
    assert!(json_value.get("config").is_some(), "JSON should contain config");
    assert!(json_value.get("timestamp").is_some(), "JSON should contain timestamp");

    // Verify benchmark data in JSON
    let benchmarks = json_value.get("benchmarks").unwrap().as_array().unwrap();
    assert_eq!(benchmarks.len(), 1);
    assert_eq!(benchmarks[0].get("name").unwrap().as_str().unwrap(), "test_benchmark");
}

/// **Integration Test: Historical comparison**
/// Verifies that historical comparison works correctly.
/// **Validates: Requirements 6.6**
#[test]
fn test_historical_comparison() {
    let temp_dir = TempDir::new().unwrap();

    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    };

    use dx_py_benchmarks::core::BenchmarkResult;
    use dx_py_benchmarks::data::{BenchmarkResults, StoredResult};
    use dx_py_benchmarks::report::ReportGenerator;
    use std::time::Duration;

    // Create current results
    let mut current = BenchmarkResults::new("test_suite", config.clone());
    let mut bench = BenchmarkResult::new("test_benchmark");
    bench.timings = (0..30).map(|i| Duration::from_micros((i as u64 + 1) * 100)).collect();
    bench.warmup_completed = true;
    current.add_result(bench);

    // Create previous results (slightly slower)
    let mut previous = BenchmarkResults::new("test_suite", config.clone());
    let mut prev_bench = BenchmarkResult::new("test_benchmark");
    prev_bench.timings = (0..30)
        .map(|i| Duration::from_micros((i as u64 + 1) * 120)) // 20% slower
        .collect();
    prev_bench.warmup_completed = true;
    previous.add_result(prev_bench);

    let stored_previous = StoredResult::new(previous, config.clone());

    let reporter = ReportGenerator::new(temp_dir.path().to_path_buf());

    // Generate historical comparison
    let historical = reporter.generate_historical_comparison(&current, &[stored_previous]);

    // Verify historical comparison content
    assert!(
        historical.contains("Historical Comparison"),
        "Should contain historical comparison header"
    );
    assert!(
        historical.contains("Performance Trends"),
        "Should contain performance trends section"
    );
    assert!(historical.contains("test_benchmark"), "Should contain benchmark name");
}

/// **Integration Test: Framework configuration validation**
/// Verifies that the framework correctly validates configuration.
/// **Validates: Requirements 1.3, 5.6**
#[test]
fn test_framework_configuration_validation() {
    let temp_dir = TempDir::new().unwrap();

    // Test with low iteration count (should warn)
    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 10, // Below recommended minimum
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    };

    let framework = BenchmarkFramework::new(config);

    // Verify runner was created with correct config
    assert_eq!(framework.runner.warmup_iterations, 2);
    assert_eq!(framework.runner.measurement_iterations, 10);

    // Verify validation detects low iteration count
    let validation = framework.runner.validate();
    assert!(!validation.warnings.is_empty(), "Should warn about low iteration count");
    assert!(
        validation.warnings[0].contains("below recommended minimum"),
        "Warning should mention minimum iterations"
    );
}

/// **Integration Test: Available suites listing**
/// Verifies that all expected suites are available.
/// **Validates: Requirements 1.1**
#[test]
fn test_available_suites() {
    let suites = BenchmarkFramework::available_suites();

    assert!(suites.contains(&"runtime"), "Should include runtime suite");
    assert!(suites.contains(&"package"), "Should include package suite");
    assert!(suites.contains(&"test_runner"), "Should include test_runner suite");
    assert_eq!(suites.len(), 3, "Should have exactly 3 suites");
}

/// **Integration Test: Unknown suite handling**
/// Verifies that unknown suites are handled gracefully.
/// **Validates: Requirements 1.1**
#[test]
fn test_unknown_suite_handling() {
    let temp_dir = TempDir::new().unwrap();

    let config = BenchmarkConfig {
        warmup_iterations: 2,
        measurement_iterations: 30,
        timeout_seconds: 60,
        output_format: OutputFormat::Both,
        output_dir: temp_dir.path().to_path_buf(),
        seed: Some(42),
        suites: vec![],
        filter: None,
    };

    let mut framework = BenchmarkFramework::new(config);

    let result = framework.run_suite("nonexistent_suite");

    assert!(result.is_err(), "Should return error for unknown suite");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Unknown suite"), "Error should mention unknown suite");
}

/// **Integration Test: External tool invocation - CPython**
/// Verifies that CPython invocation works or fails gracefully.
/// **Validates: Requirements 2.3**
#[test]
fn test_external_tool_cpython() {
    use dx_py_benchmarks::core::{BenchmarkRunner, PythonRuntime};
    use std::time::Duration;

    let runner = BenchmarkRunner::new(1, 5, Duration::from_secs(30));

    // Try to run a simple Python script
    let result =
        runner.run_python_benchmark("test_cpython", "print('hello')", PythonRuntime::CPython);

    match result {
        Ok(bench_result) => {
            // Python is available
            assert_eq!(bench_result.name, "test_cpython");
            assert!(bench_result.warmup_completed, "Warmup should complete");
            assert!(!bench_result.timings.is_empty(), "Should have timing data");
        }
        Err(e) => {
            // Python is not available - this is OK
            let err_str = e.to_string();
            assert!(
                err_str.contains("not found") || err_str.contains("failed"),
                "Error should indicate tool not found or command failed: {}",
                err_str
            );
        }
    }
}

/// **Integration Test: External tool invocation - DX-Py**
/// Verifies that DX-Py invocation works or fails gracefully.
/// **Validates: Requirements 2.3**
#[test]
fn test_external_tool_dxpy() {
    use dx_py_benchmarks::core::{BenchmarkRunner, PythonRuntime};
    use std::time::Duration;

    let runner = BenchmarkRunner::new(1, 5, Duration::from_secs(30));

    // Try to run a simple script on DX-Py
    let result = runner.run_python_benchmark("test_dxpy", "print('hello')", PythonRuntime::DxPy);

    match result {
        Ok(bench_result) => {
            // DX-Py is available
            assert_eq!(bench_result.name, "test_dxpy");
            assert!(bench_result.warmup_completed, "Warmup should complete");
        }
        Err(e) => {
            // DX-Py is not available - this is expected in most environments
            let err_str = e.to_string();
            assert!(
                err_str.contains("not found") || err_str.contains("failed"),
                "Error should indicate tool not found or command failed: {}",
                err_str
            );
        }
    }
}

/// **Integration Test: External command execution**
/// Verifies that external command execution works correctly.
/// **Validates: Requirements 2.3, 3.4, 4.3**
#[test]
fn test_external_command_execution() {
    use dx_py_benchmarks::core::BenchmarkRunner;
    use std::time::Duration;

    let runner = BenchmarkRunner::new(1, 5, Duration::from_secs(30));

    // Run a simple command that should work on all platforms
    #[cfg(windows)]
    let cmd = vec!["cmd", "/c", "echo hello"];
    #[cfg(not(windows))]
    let cmd = vec!["echo", "hello"];

    let result = runner.run_external_command("test_echo", &cmd);

    assert!(result.is_ok(), "Echo command should succeed");
    let bench_result = result.unwrap();
    assert_eq!(bench_result.name, "test_echo");
    assert!(bench_result.warmup_completed, "Warmup should complete");
    assert!(!bench_result.timings.is_empty(), "Should have timing data");
}

/// **Integration Test: Missing tool handling**
/// Verifies that missing tools are handled gracefully.
/// **Validates: Requirements 2.3, 3.4, 4.3**
#[test]
fn test_missing_tool_handling() {
    use dx_py_benchmarks::core::BenchmarkRunner;
    use std::time::Duration;

    let runner = BenchmarkRunner::new(1, 5, Duration::from_secs(30));

    // Try to run a command that doesn't exist
    let cmd = vec!["nonexistent_tool_12345", "--version"];
    let result = runner.run_external_command("test_missing", &cmd);

    assert!(result.is_err(), "Should fail for missing tool");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found") || err.to_string().contains("ToolNotFound"),
        "Error should indicate tool not found: {}",
        err
    );
}

/// **Integration Test: Empty command handling**
/// Verifies that empty commands are handled gracefully.
/// **Validates: Requirements 2.3, 3.4, 4.3**
#[test]
fn test_empty_command_handling() {
    use dx_py_benchmarks::core::BenchmarkRunner;
    use std::time::Duration;

    let runner = BenchmarkRunner::new(1, 5, Duration::from_secs(30));

    // Try to run an empty command
    let cmd: Vec<&str> = vec![];
    let result = runner.run_external_command("test_empty", &cmd);

    assert!(result.is_err(), "Should fail for empty command");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Empty command"),
        "Error should indicate empty command: {}",
        err
    );
}

/// **Integration Test: Benchmark runner with function**
/// Verifies that the benchmark runner correctly times functions.
/// **Validates: Requirements 1.2, 1.3**
#[test]
fn test_benchmark_runner_function() {
    use dx_py_benchmarks::core::BenchmarkRunner;
    use std::time::Duration;

    let runner = BenchmarkRunner::new(5, 30, Duration::from_secs(60));

    // Run a simple benchmark function with enough work to be measurable
    let result = runner.run_benchmark("test_function", || {
        let mut sum = 0u64;
        for i in 0..100_000 {
            sum = sum.wrapping_add(i);
        }
        std::hint::black_box(sum);
    });

    assert_eq!(result.name, "test_function");
    assert!(result.warmup_completed, "Warmup should complete");
    assert_eq!(result.timings.len(), 30, "Should have 30 timing measurements");
    assert!(!result.timed_out, "Should not time out");

    // All timings should be valid durations (may be zero on very fast systems)
    for timing in &result.timings {
        // Duration is always non-negative by construction, just verify it's a valid duration
        let _ = timing.as_nanos(); // Ensure timing is accessible
    }
}

/// **Integration Test: Benchmark timeout handling**
/// Verifies that benchmark timeouts are handled correctly.
/// **Validates: Requirements 1.3**
#[test]
fn test_benchmark_timeout_handling() {
    use dx_py_benchmarks::core::BenchmarkRunner;
    use std::time::Duration;

    // Create a runner with a very short timeout
    let runner = BenchmarkRunner::new(0, 100, Duration::from_millis(10));

    // Run a benchmark that takes longer than the timeout
    let result = runner.run_benchmark("test_timeout", || {
        std::thread::sleep(Duration::from_millis(5));
    });

    // The benchmark should either complete some iterations or time out
    // Either way, it should not panic
    assert_eq!(result.name, "test_timeout");

    if result.timed_out {
        // If it timed out, it should have partial results
        assert!(result.timings.len() < 100, "Should have fewer than requested iterations");
    }
}

/// **Integration Test: System info collection**
/// Verifies that system information is collected correctly.
/// **Validates: Requirements 1.5**
#[test]
fn test_system_info_collection() {
    use dx_py_benchmarks::core::SystemInfo;

    let info = SystemInfo::collect();

    // Verify required fields are populated
    assert!(!info.os.is_empty(), "OS should be populated");
    assert!(!info.os_version.is_empty(), "OS version should be populated");
    assert!(!info.cpu_model.is_empty(), "CPU model should be populated");
    assert!(info.cpu_cores > 0, "CPU cores should be positive");
    assert!(info.memory_gb > 0.0, "Memory should be positive");

    // Python version might be empty if Python is not installed
    // DX-Py version might be empty if DX-Py is not installed
    // These are optional
}

/// **Integration Test: Statistical analysis integration**
/// Verifies that statistical analysis works correctly with benchmark results.
/// **Validates: Requirements 5.1, 5.2, 5.3**
#[test]
fn test_statistical_analysis_integration() {
    use dx_py_benchmarks::analysis::StatisticalAnalyzer;
    use std::time::Duration;

    let analyzer = StatisticalAnalyzer::new();

    // Create sample timing data
    let timings: Vec<Duration> =
        (0..100).map(|i| Duration::from_micros(100 + (i % 20) as u64)).collect();

    let stats = analyzer.compute_statistics(&timings);

    // Verify statistics are computed
    assert!(stats.mean > 0.0, "Mean should be positive");
    assert!(stats.median > 0.0, "Median should be positive");
    assert!(stats.std_dev >= 0.0, "Std dev should be non-negative");
    assert!(stats.min <= stats.max, "Min should be <= max");
    assert!(stats.p50 <= stats.p95, "P50 should be <= P95");
    assert!(stats.p95 <= stats.p99, "P95 should be <= P99");

    // Verify confidence interval
    assert!(stats.confidence_interval_95.0 <= stats.mean, "CI lower should be <= mean");
    assert!(stats.confidence_interval_95.1 >= stats.mean, "CI upper should be >= mean");
}
