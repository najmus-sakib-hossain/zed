
# Design Document: DX-Py vs UV Benchmarking

## Overview

This design document specifies the architecture for a comprehensive benchmark suite that compares dx-py against uv, measuring performance across resolution, installation, and virtual environment operations. Results will be documented in README files with clear comparison tables.

## Architecture

@tree[]

## Components and Interfaces

### 1. Benchmark Runner

```rust
/// Main benchmark orchestrator pub struct BenchmarkRunner { dx_py_path: PathBuf, uv_path: Option<PathBuf>, output_dir: PathBuf, iterations: usize, }
impl BenchmarkRunner { pub fn new() -> Result<Self>;
pub fn detect_uv() -> Option<PathBuf>;
pub fn run_all(&self) -> BenchmarkResults;
pub fn run_resolution_benchmarks(&self) -> Vec<BenchmarkResult>;
pub fn run_installation_benchmarks(&self) -> Vec<BenchmarkResult>;
pub fn run_venv_benchmarks(&self) -> Vec<BenchmarkResult>;
}
```

### 2. Benchmark Scenarios

```rust
/// Test project configurations for benchmarks pub struct TestProject { pub name: String, pub dependencies: Vec<String>, pub category: ProjectCategory, }
pub enum ProjectCategory { Simple, // 5-10 deps: requests, click, rich Medium, // 20-50 deps: flask, django-rest-framework Complex, // 100+ deps: tensorflow, pandas ecosystem }
impl TestProject { pub fn simple() -> Self;
pub fn medium() -> Self;
pub fn complex() -> Self;
}
```

### 3. Timing and Results

```rust
/// Single benchmark measurement pub struct BenchmarkResult { pub tool: Tool, pub operation: Operation, pub scenario: String, pub cold_start_ms: Vec<f64>, pub warm_start_ms: Vec<f64>, pub mean_cold_ms: f64, pub mean_warm_ms: f64, pub std_dev_cold: f64, pub std_dev_warm: f64, }
pub enum Tool { DxPy, Uv, }
pub enum Operation { Resolution, Installation, VenvCreation, Download, }
/// Aggregated results for all benchmarks pub struct BenchmarkResults { pub results: Vec<BenchmarkResult>, pub system_info: SystemInfo, pub timestamp: DateTime<Utc>, }
impl BenchmarkResults { pub fn to_json(&self) -> String;
pub fn to_markdown_table(&self) -> String;
pub fn comparison_summary(&self) -> ComparisonSummary;
}
```

### 4. System Information

```rust
/// System specs for reproducibility pub struct SystemInfo { pub os: String, pub arch: String, pub cpu: String, pub cpu_cores: usize, pub memory_gb: f64, pub rust_version: String, pub dx_py_version: String, pub uv_version: Option<String>, }
impl SystemInfo { pub fn detect() -> Self;
}
```

### 5. Cache Management

```rust
/// Cache clearing for cold start benchmarks pub struct CacheManager { dx_py_cache: PathBuf, uv_cache: PathBuf, }
impl CacheManager { pub fn clear_dx_py_cache(&self) -> Result<()>;
pub fn clear_uv_cache(&self) -> Result<()>;
pub fn clear_all(&self) -> Result<()>;
}
```

## Data Models

### Benchmark Configuration

```rust
pub struct BenchmarkConfig { pub iterations: usize, // Default: 5 pub warmup_iterations: usize, // Default: 1 pub timeout_seconds: u64, // Default: 300 pub include_cold_start: bool, // Default: true pub include_warm_start: bool, // Default: true }
```

### Comparison Summary

```rust
pub struct ComparisonSummary { pub resolution_speedup: f64, // dx-py vs uv ratio pub installation_speedup: f64, pub venv_speedup: f64, pub overall_speedup: f64, }
```

## Benchmark Scenarios

### Resolution Benchmarks

+----------+--------------+----------+----------+
| Scenario | Dependencies | Example  | Packages |
+==========+==============+==========+==========+
| Simple   | 5-10         | requests | click    |
+----------+--------------+----------+----------+



### Installation Benchmarks

+----------+-------------+
| Scenario | Description |
+==========+=============+
| From     | Lock        |
+----------+-------------+



### Venv Benchmarks

+----------+-------------+
| Scenario | Description |
+==========+=============+
| Empty    | Venv        |
+----------+-------------+



## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system-, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees. Based on the prework analysis, the benchmarking feature primarily involves specific example scenarios rather than universal properties. The acceptance criteria are mostly about running specific benchmarks and producing output, which are better validated through integration tests. No testable properties were identified - all acceptance criteria are example-based tests that verify specific benchmark scenarios run correctly.

## Error Handling

### UV Not Found

When uv is not installed: -Display clear message: "uv not found in PATH" -Provide installation instructions -Continue with dx-py-only benchmarks -Mark uv results as "N/A" in output

### Benchmark Failures

- Timeout after 5 minutes per benchmark
- Retry failed benchmarks once
- Log detailed error information
- Continue with remaining benchmarks

### Cache Clearing Failures

- Log warning if cache cannot be cleared
- Skip cold start benchmark for that tool
- Continue with warm start benchmarks

## Testing Strategy

### Unit Tests

- JSON output parsing
- Markdown table generation
- System info detection
- Statistics calculations (mean, std dev)

### Integration Tests

- Run benchmark suite with mock projects
- Verify output format correctness
- Test UV detection logic
- Test cache clearing

### Manual Verification

- Run full benchmark suite on target platforms
- Verify results are reasonable
- Compare with manual timing measurements

## Output Format

### JSON Output

```json
{ "timestamp": "2024-12-26T10:00:00Z", "system_info": { "os": "Windows 11", "arch": "x86_64", "cpu": "AMD Ryzen 9 5900X", "cpu_cores": 12, "memory_gb": 32.0 }, "results": [ { "tool": "dx-py", "operation": "resolution", "scenario": "simple", "mean_cold_ms": 150.5, "mean_warm_ms": 45.2, "std_dev_cold": 12.3, "std_dev_warm": 5.1 }
]
}
```

### Markdown Table Output

```markdown


## Performance Comparison: dx-py vs uv


+------------+----------+-------+-------+
| Operation  | Scenario | dx-py | (cold |
+============+==========+=======+=======+
| Resolution | Simple   | 150ms | 320ms |
+------------+----------+-------+-------+
```
