
# Design Document: Comparative Benchmarks

## Overview

This design describes a comprehensive benchmarking framework for comparing DX-Py components against industry-standard tools. The framework is implemented in Rust for maximum performance and reliability, with Python bindings for executing Python-based benchmarks. The architecture follows a modular design with clear separation between benchmark execution, statistical analysis, and report generation.

## Architecture

@flow:TD[]

## Components and Interfaces

### BenchmarkFramework

The central orchestrator that coordinates benchmark execution, analysis, and reporting.
```rust
pub struct BenchmarkFramework { config: BenchmarkConfig, runner: BenchmarkRunner, analyzer: StatisticalAnalyzer, reporter: ReportGenerator, result_store: ResultStore, }
impl BenchmarkFramework { pub fn new(config: BenchmarkConfig) -> Self;
pub fn run_suite(&self, suite: &str) -> Result<BenchmarkResults, BenchmarkError>;
pub fn run_all(&self) -> Result<Vec<BenchmarkResults>, BenchmarkError>;
pub fn compare_with_baseline(&self, results: &BenchmarkResults) -> ComparisonReport;
pub fn reproduce(&self, run_id: &str) -> Result<BenchmarkResults, BenchmarkError>;
}
```

### BenchmarkRunner

Executes individual benchmarks with proper isolation and timing.
```rust
pub struct BenchmarkRunner { warmup_iterations: u32, measurement_iterations: u32, timeout: Duration, }
pub struct BenchmarkResult { pub name: String, pub timings: Vec<Duration>, pub memory_samples: Vec<usize>, pub metadata: BenchmarkMetadata, }
impl BenchmarkRunner { pub fn run_benchmark<F>(&self, name: &str, f: F) -> BenchmarkResult where F: Fn() -> ();
pub fn run_external_command(&self, name: &str, cmd: &[&str]) -> BenchmarkResult;
pub fn run_python_benchmark(&self, name: &str, script: &str, runtime: PythonRuntime) -> BenchmarkResult;
}
pub enum PythonRuntime { CPython, DxPy, }
```

### StatisticalAnalyzer

Computes statistical metrics and performs significance testing.
```rust
pub struct StatisticalAnalyzer;
pub struct Statistics { pub mean: f64, pub median: f64, pub std_dev: f64, pub min: f64, pub max: f64, pub p50: f64, pub p95: f64, pub p99: f64, pub confidence_interval_95: (f64, f64), pub coefficient_of_variation: f64, pub outliers: Vec<usize>, }
pub struct ComparisonResult { pub baseline_stats: Statistics, pub subject_stats: Statistics, pub speedup: f64, pub speedup_ci: (f64, f64), pub is_significant: bool, pub p_value: f64, }
impl StatisticalAnalyzer { pub fn compute_statistics(&self, timings: &[Duration]) -> Statistics;
pub fn compare(&self, baseline: &[Duration], subject: &[Duration]) -> ComparisonResult;
pub fn detect_outliers(&self, timings: &[Duration]) -> Vec<usize>;
pub fn welch_t_test(&self, a: &[f64], b: &[f64]) -> (f64, f64); // (t_statistic, p_value)
}
```

### ReportGenerator

Produces human-readable and machine-readable reports.
```rust
pub struct ReportGenerator { output_dir: PathBuf, }
impl ReportGenerator { pub fn generate_markdown(&self, results: &BenchmarkResults, comparison: &ComparisonReport) -> String;
pub fn generate_json(&self, results: &BenchmarkResults) -> String;
pub fn generate_historical_comparison(&self, current: &BenchmarkResults, previous: &[BenchmarkResults]) -> String;
}
```

### Benchmark Suites

#### RuntimeSuite

```rust
pub struct RuntimeSuite { data_generator: TestDataGenerator, }
impl RuntimeSuite { // Micro-benchmarks pub fn bench_int_arithmetic(&self) -> BenchmarkSpec;
pub fn bench_string_operations(&self) -> BenchmarkSpec;
pub fn bench_list_operations(&self) -> BenchmarkSpec;
pub fn bench_dict_operations(&self) -> BenchmarkSpec;
// Macro-benchmarks pub fn bench_json_parsing(&self) -> BenchmarkSpec;
pub fn bench_file_io(&self) -> BenchmarkSpec;
pub fn bench_http_handling(&self) -> BenchmarkSpec;
// Startup and memory pub fn bench_cold_startup(&self) -> BenchmarkSpec;
pub fn bench_memory_usage(&self) -> BenchmarkSpec;
}
pub struct BenchmarkSpec { pub name: String, pub cpython_code: String, pub dxpy_code: String, pub setup_code: Option<String>, pub teardown_code: Option<String>, }
```

#### PackageSuite

```rust
pub struct PackageSuite { test_projects: Vec<TestProject>, }
pub struct TestProject { pub name: String, pub pyproject_toml: String, pub dependency_count: usize, }
impl PackageSuite { pub fn bench_resolution_small(&self) -> BenchmarkSpec;
pub fn bench_resolution_medium(&self) -> BenchmarkSpec;
pub fn bench_resolution_large(&self) -> BenchmarkSpec;
pub fn bench_install_cold_cache(&self) -> BenchmarkSpec;
pub fn bench_install_warm_cache(&self) -> BenchmarkSpec;
pub fn bench_lock_generation(&self) -> BenchmarkSpec;
pub fn bench_venv_creation(&self) -> BenchmarkSpec;
pub fn bench_real_world_flask(&self) -> BenchmarkSpec;
pub fn bench_real_world_django(&self) -> BenchmarkSpec;
}
```

#### TestRunnerSuite

```rust
pub struct TestRunnerSuite { test_generator: TestDataGenerator, }
impl TestRunnerSuite { pub fn bench_discovery_small(&self) -> BenchmarkSpec; // 10 tests pub fn bench_discovery_medium(&self) -> BenchmarkSpec; // 100 tests pub fn bench_discovery_large(&self) -> BenchmarkSpec; // 1000 tests pub fn bench_execution_simple(&self) -> BenchmarkSpec;
pub fn bench_execution_fixtures(&self) -> BenchmarkSpec;
pub fn bench_execution_parametrized(&self) -> BenchmarkSpec;
pub fn bench_execution_async(&self) -> BenchmarkSpec;
pub fn bench_parallel_execution(&self) -> BenchmarkSpec;
}
```

### TestDataGenerator

```rust
pub struct TestDataGenerator { seed: u64, }
impl TestDataGenerator { pub fn new(seed: u64) -> Self;
// Test file generation pub fn generate_test_files(&self, count: usize, pattern: TestPattern) -> Vec<TestFile>;
// Project generation pub fn generate_project(&self, deps: usize) -> TestProject;
// Data generation pub fn generate_json_data(&self, size: DataSize) -> String;
pub fn generate_string_data(&self, size: DataSize) -> String;
}
pub enum TestPattern { SimpleFunctions, Classes, Fixtures, Async, Parametrized, Mixed, }
pub enum DataSize { Small, // ~1KB Medium, // ~100KB Large, // ~10MB }
```

### ResultStore

```rust
pub struct ResultStore { storage_path: PathBuf, }
pub struct StoredResult { pub id: String, pub timestamp: DateTime<Utc>, pub config: BenchmarkConfig, pub system_info: SystemInfo, pub results: BenchmarkResults, }
impl ResultStore { pub fn save(&self, results: &BenchmarkResults, config: &BenchmarkConfig) -> Result<String, Error>;
pub fn load(&self, id: &str) -> Result<StoredResult, Error>;
pub fn list_recent(&self, count: usize) -> Vec<StoredResult>;
pub fn get_historical(&self, suite: &str, count: usize) -> Vec<StoredResult>;
}
```

## Data Models

### BenchmarkConfig

```rust
pub struct BenchmarkConfig { pub warmup_iterations: u32, pub measurement_iterations: u32, pub timeout_seconds: u64, pub output_format: OutputFormat, pub output_dir: PathBuf, pub seed: Option<u64>, pub suites: Vec<String>, pub filter: Option<String>, }
pub enum OutputFormat { Markdown, Json, Both, }
```

### SystemInfo

```rust
pub struct SystemInfo { pub os: String, pub os_version: String, pub cpu_model: String, pub cpu_cores: usize, pub memory_gb: f64, pub python_version: String, pub dxpy_version: String, pub uv_version: Option<String>, pub pytest_version: Option<String>, }
```

### BenchmarkResults

```rust
pub struct BenchmarkResults { pub suite: String, pub benchmarks: Vec<BenchmarkResult>, pub system_info: SystemInfo, pub config: BenchmarkConfig, pub timestamp: DateTime<Utc>, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Warmup Precedes Measurement

For any benchmark execution with warmup_iterations > 0, the warmup phase SHALL complete before any measurement timing begins, and warmup timings SHALL NOT be included in the final statistics. Validates: Requirements 1.2

### Property 2: Iteration Count Respected

For any benchmark configuration with specified warmup_iterations W and measurement_iterations M, the benchmark runner SHALL execute exactly W warmup iterations followed by exactly M measurement iterations. Validates: Requirements 1.3

### Property 3: Dual Output Format Validity

For any completed benchmark run, the framework SHALL produce both valid Markdown (parseable as Markdown) and valid JSON (parseable as JSON) outputs containing equivalent benchmark data. Validates: Requirements 1.4

### Property 4: System Info Completeness

For any benchmark result, the system_info field SHALL contain non-empty values for: os, os_version, cpu_model, cpu_cores, memory_gb, and python_version. Validates: Requirements 1.5

### Property 5: Benchmark Equivalence Across Tools

For any comparative benchmark, the same benchmark specification (code, data, configuration) SHALL be used for both the baseline tool and the subject tool, ensuring fair comparison. Validates: Requirements 2.3, 3.4, 4.3

### Property 6: Statistics Computation Correctness

For any non-empty array of timing measurements, the Statistical_Analyzer SHALL compute mean, median, standard deviation, and percentiles (p50, p95, p99) such that: -mean equals the arithmetic average of all values -median equals the middle value (or average of two middle values) -p50 equals median -p95 is greater than or equal to p50 -p99 is greater than or equal to p95 Validates: Requirements 5.1

### Property 7: Confidence Interval Validity

For any array of 30+ timing measurements, the 95% confidence interval (lower, upper) SHALL satisfy: lower <= mean <= upper, and the interval width SHALL decrease as sample size increases. Validates: Requirements 5.2

### Property 8: Significance Testing Consistency

For any comparison between two measurement arrays, the Statistical_Analyzer SHALL compute a p-value between 0 and 1, and is_significant SHALL be true if and only if p_value < 0.05. Validates: Requirements 5.3

### Property 9: Outlier Detection Correctness

For any array of measurements, outliers detected using IQR method SHALL be values that fall below Q1 - 1.5IQR or above Q3 + 1.5IQR, where IQR = Q3 - Q1. Validates: Requirements 5.4

### Property 10: Variance Warning Threshold

For any array of measurements where coefficient_of_variation > 0.10 (10%), the Statistical_Analyzer SHALL flag the results as potentially unreliable. Validates: Requirements 5.5

### Property 11: Minimum Iterations Enforcement

For any benchmark run with fewer than 30 measurement iterations, the framework SHALL either reject the configuration or include a warning about statistical validity. Validates: Requirements 5.6

### Property 12: Report Content Completeness

For any generated Markdown report, it SHALL contain: a comparison table with benchmark names and timings, speedup factors for each benchmark, confidence intervals for speedups, clear indication when speedup < 1.0 (slower), and a methodology section. Validates: Requirements 6.1, 6.2, 6.3, 6.5

### Property 13: JSON Output Validity

For any generated JSON output, it SHALL be valid JSON that can be parsed and SHALL contain all benchmark results with their statistical metrics. Validates: Requirements 6.4

### Property 14: Historical Comparison Generation

For any benchmark run where previous results exist in the result store, the Report_Generator SHALL include a historical comparison section showing performance trends. Validates: Requirements 6.6

### Property 15: Metadata Recording Completeness

For any stored benchmark result, it SHALL include: all configuration parameters used, relevant environment variables, and a timestamp. Validates: Requirements 7.1, 7.3, 7.5

### Property 16: Deterministic Generation Round-Trip

For any seed value S and data generation parameters P, calling the generator twice with (S, P) SHALL produce identical output. Validates: Requirements 7.2, 8.3

### Property 17: Data Size Configurability

For any requested data size (Small, Medium, Large), the TestDataGenerator SHALL produce data within the expected size range (Small: ~1KB, Medium: ~100KB, Large: ~10MB) with tolerance of ±50%. Validates: Requirements 8.1

## Error Handling

### Configuration Errors

+-------------------------+--------+----------+
| Error                   | Cause  | Handling |
+=========================+========+==========+
| `InvalidIterationCount` | warmup | or       |
+-------------------------+--------+----------+



### Runtime Errors

+--------------------+-----------+----------+
| Error              | Cause     | Handling |
+====================+===========+==========+
| `BenchmarkTimeout` | Benchmark | exceeds  |
+--------------------+-----------+----------+



### Statistical Errors

+-----------------------+-------+----------+
| Error                 | Cause | Handling |
+=======================+=======+==========+
| `InsufficientSamples` | <     | 30       |
+-----------------------+-------+----------+



## Testing Strategy

### Unit Tests

Unit tests will verify specific examples and edge cases: -StatisticalAnalyzer tests -Known input/output pairs for mean, median, std dev -Edge cases: single value, two values, all same values -Outlier detection with known outliers -ReportGenerator tests -Markdown table formatting -JSON structure validation -Speedup calculation edge cases (0, infinity) -TestDataGenerator tests -Seed reproducibility -Size bounds verification

### Property-Based Tests

Property-based tests will use the `proptest` crate with minimum 100 iterations per property: -Property 1-2: Generate random iteration counts, verify execution order -Property 3: Generate benchmark results, verify both outputs parse correctly -Property 4: Generate results, verify all system info fields present -Property 5: Generate benchmark specs, verify equivalence -Property 6-7: Generate random timing arrays, verify statistical formulas -Property 8: Generate two timing arrays, verify p-value bounds -Property 9: Generate arrays with known outliers, verify detection -Property 10: Generate high-variance arrays, verify warning -Property 11: Generate configs with < 30 iterations, verify rejection/warning -Property 12-13: Generate results, verify report content -Property 14: Generate results with history, verify comparison -Property 15: Generate configs, verify metadata completeness -Property 16: Generate seeds, verify determinism -Property 17: Generate size requests, verify bounds

### Integration Tests

- End-to-end benchmark run: Execute a small benchmark suite, verify complete flow
- External tool integration: Verify CPython, UV, pytest invocation works
- Result storage round-trip: Save and load results, verify equivalence
- Reproduce command: Run benchmark, reproduce, verify similar results
