
# Benchmark Results and Methodology

This document describes the benchmarking methodology and results for dx-www.

## Methodology

### Environment

Benchmarks are run on: -CI Environment: GitHub Actions `ubuntu-latest` runners -Local Development: Results may vary based on hardware

### Tools

- Criterion.rs: Statistical benchmarking library
- cargo bench: Standard Rust benchmarking command

### Configuration

```toml
[profile.bench]
opt-level = 3 lto = true codegen-units = 1 ```


### Statistical Approach


- Minimum 100 iterations per benchmark
- Outlier detection and removal
- Confidence intervals reported (95%)
- Comparison against baseline when available


## Benchmark Categories



### 1. Parser Benchmarks (`benches/parser_benchmarks.rs`)


+-----------+--------+-----------+--------+------------+
| Benchmark | Mean   | Std       | Dev    | Throughput |
+===========+========+===========+========+============+
| parse     | simple | component | ~50μs  | ±2μs       |
+-----------+--------+-----------+--------+------------+


### 2. SSR Benchmarks (`benches/ssr_benchmarks.rs`)


+-----------+--------+------+---------+------------+
| Benchmark | Mean   | Std  | Dev     | Throughput |
+===========+========+======+=========+============+
| render    | simple | page | ~100μs  | ±5μs       |
+-----------+--------+------+---------+------------+


### 3. Delta Benchmarks (`benches/delta_benchmarks.rs`)


+-----------+-------+-------+--------+------------+
| Benchmark | Mean  | Std   | Dev    | Throughput |
+===========+=======+=======+========+============+
| compute   | small | delta | ~10μs  | ±1μs       |
+-----------+-------+-------+--------+------------+


### 4. HTIP Benchmarks (`benches/htip_benchmarks.rs`)


+-----------+----------+-------+----------+------------+
| Benchmark | Mean     | Std   | Dev      | Throughput |
+===========+==========+=======+==========+============+
| encode    | template | ~5μs  | ±0.5μs   | ~200k      |
+-----------+----------+-------+----------+------------+


## Running Benchmarks



### Full Benchmark Suite


```bash
cargo bench --workspace ```

### Specific Benchmark

```bash
cargo bench --bench parser_benchmarks ```


### With Baseline Comparison


```bash

# Save baseline

cargo bench -- --save-baseline main

# Compare against baseline

cargo bench -- --baseline main ```

### Generate HTML Report

```bash
cargo bench -- --plotting-backend plotters


# Results in target/criterion/report/index.html


```

## CI Integration

Benchmarks run automatically on: -Push to `main` branch -Results uploaded as artifacts

### Viewing CI Results

- Go to Actions tab
- Select "CI" workflow
- Download "benchmark-results" artifact
- Open `target/criterion/report/index.html`

## Performance Targets

+--------+------------+---------+
| Metric | Target     | Current |
+========+============+=========+
| Parse  | throughput | >10k    |
+--------+------------+---------+



## Optimization History

### v1.0.0

- Baseline performance established
- Static regex compilation (10% parser improvement)
- Zero-copy HTIP parsing (30% protocol improvement)

### Future Optimizations

- SIMD-accelerated parsing (planned)
- Arena allocation for AST (planned)
- Parallel SSR (planned)

## Reproducing Results

To reproduce benchmark results locally:
```bash


# Ensure consistent environment


export RUSTFLAGS="-C target-cpu=native"


# Run with high precision


cargo bench -- --measurement-time 10


# Compare multiple runs


for i in {1..5}; do cargo bench -- --save-baseline "run-$i"
done ```


## Notes


- Results are hardware-dependent
- CI results provide consistent baseline
- Local results may be faster with native CPU optimizations
- Memory benchmarks not included (use `cargo bench
- -features memory-profiling`)
