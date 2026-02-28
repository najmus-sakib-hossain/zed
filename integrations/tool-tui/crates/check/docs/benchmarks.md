
# dx-check Benchmark Methodology

This document describes the benchmark methodology used to measure dx-check performance.

## Running Benchmarks

```bash
cargo bench -p dx-check cargo bench -p dx-check -- simd_scanner cargo bench -p dx-check -- --save-baseline main ```


## Benchmark Categories



### 1. Parse and Lint (`parse_and_lint`)


Measures the time to parse and lint a single file. -parse_and_lint_simple: Clean JavaScript code (~1KB) -parse_and_lint_with_issues: Code with lint violations


### 2. SIMD Scanner (`simd_scanner`)


Measures the performance of the SIMD-accelerated pattern scanner. -scan_large_file: Scanning a large file (~100KB) -has_any_match_clean: Quick rejection of clean code -has_any_match_with_issues: Detection of patterns in code with issues


### 3. Scaling (`scaling`)


Measures how performance scales with file size. -Tests files from 1x to 1000x the base sample size -Reports throughput in bytes/second


### 4. Single File Check (`single_file_check`)


Measures check time for different file types. -javascript: Plain JavaScript files -typescript: TypeScript files with type annotations -jsx: React JSX files


### 5. Multi-File Check (`multi_file_check`)


Measures check time for multiple files. -Tests with 5, 10, 25, and 50 files -Includes parallel processing overhead


### 6. Rule Loading (`rule_loading`)


Measures initialization overhead. -create_registry: Time to create rule registry -create_checker: Time to create checker instance


### 7. Configuration Parsing (`config_parsing`)


Measures configuration handling overhead. -default_config: Creating default configuration -parse_toml: Parsing TOML configuration file


### 8. Diagnostics (`diagnostics`)


Measures diagnostic creation overhead. -create_diagnostic: Direct diagnostic creation -builder_pattern: Using DiagnosticBuilder -diagnostic_with_fix: Diagnostic with auto-fix


### 9. Fix Application (`fix_application`)


Measures fix application performance. -single_fix: Applying a single fix to source code


## Test Environment


For reproducible results, benchmarks should be run on: -CPU: Modern x86_64 with AVX2 support (for SIMD benchmarks) -Memory: At least 8GB RAM -OS: Linux, macOS, or Windows -Rust: Latest stable version


## Comparison Methodology


When comparing against other tools (ESLint, Biome): -Same codebase: Use identical source files -Same rules: Enable equivalent rules in each tool -Cold start: Measure first run (no caching) -Warm start: Measure subsequent runs (with caching) -Multiple runs: Average over 10+ runs


### ESLint Comparison


```bash
time npx eslint src/ --ext .js,.ts,.jsx,.tsx time dx-check check src/ ```

### Biome Comparison

```bash
time npx @biomejs/biome check src/ time dx-check check src/ ```


## Performance Targets


+--------+--------+-------+
| Metric | Target | Notes |
+========+========+=======+
| Single | file   | check |
+--------+--------+-------+


## Interpreting Results



### Throughput


Higher is better. Measured in: -bytes/second: For file processing -files/second: For multi-file operations


### Latency


Lower is better. Measured in: -microseconds (µs): For single operations -milliseconds (ms): For file operations


### Memory


Lower is better. Measured in: -bytes: For data structures -MB: For overall memory usage


## Continuous Benchmarking


Benchmarks are run on every PR to detect performance regressions: -Baseline: Benchmarks from `main` branch -PR: Benchmarks from PR branch -Comparison: Report any regressions >5%


## Known Limitations


- SIMD availability: SIMD benchmarks require AVX2 support
- Disk I/O: Multi-file benchmarks include disk I/O overhead
- Parallelism: Results vary based on CPU core count
- Caching: Warm benchmarks depend on OS file cache


## Contributing


To add new benchmarks: -Add benchmark function to `benches/lint_benchmark.rs` -Add to appropriate benchmark group -Document in this file -Run and verify results
