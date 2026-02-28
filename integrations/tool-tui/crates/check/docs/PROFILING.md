
# Profiling Guide

This document explains how to profile dx-check to identify performance bottlenecks.

## Prerequisites

Install profiling tools:
```bash
cargo install flamegraph cargo install cargo-instruments cargo install samply ```


## CPU Profiling



### Using Flamegraph (Linux/macOS)


```bash
cargo build -p dx-check --release cargo flamegraph -p dx-check -- check ./src ```

### Using samply (Cross-platform)

```bash
samply record cargo run -p dx-check --release -- check ./src ```


### Using cargo-instruments (macOS)


```bash
cargo instruments -t time -p dx-check -- check ./src cargo instruments -t Allocations -p dx-check -- check ./src ```

## Memory Profiling

### Using Valgrind (Linux)

```bash
cargo build -p dx-check --release valgrind --tool=massif ./target/release/dx-check check ./src ms_print massif.out.* ```


### Using heaptrack (Linux)


```bash
sudo apt install heaptrack heaptrack ./target/release/dx-check check ./src heaptrack_gui heaptrack.dx-check.*.gz ```

### Using DHAT (Cross-platform)

Add to Cargo.toml:
```toml
[profile.release]
debug = true
[dependencies]
dhat = { version = "0.3", optional = true }
[features]
dhat-heap = ["dhat"]
```
Then in main.rs:
```rust


#[cfg(feature ="dhat-heap")]#[global_allocator]static ALLOC:dhat::Alloc =dhat::Alloc;fn main(){#[cfg(feature ="dhat-heap")]let _profiler =dhat::Profiler::new_heap();}


```
Run:
```bash
cargo run -p dx-check --release --features dhat-heap -- check ./src ```


## Benchmark Profiling



### Using criterion with profiling


```bash
cargo bench -p dx-check -- --profile-time 10 cargo flamegraph --bench lint_benchmark -- --bench single_file ```

## Common Bottlenecks

### 1. File I/O

- Symptom: High time in `read_to_string`, `read_dir`
- Solution: Use memory-mapped files, parallel directory walking

### 2. Parsing

- Symptom: High time in parser functions
- Solution: Enable AST caching, use incremental parsing

### 3. Rule Execution

- Symptom: High time in rule check functions
- Solution: Optimize hot rules, use SIMD for pattern matching

### 4. Memory Allocation

- Symptom: High allocation count, fragmentation
- Solution: Use arena allocators, reduce cloning

### 5. Lock Contention

- Symptom: Threads waiting on locks
- Solution: Use lock-free data structures, reduce critical sections

## Profiling Checklist

- Baseline: Run benchmarks to establish baseline
- Profile: Identify hotspots with flamegraph
- Analyze: Understand why hotspots exist
- Optimize: Apply targeted optimizations
- Verify: Re-run benchmarks to confirm improvement
- Regression: Add benchmark to prevent regression

## Performance Targets

+-----------+--------+---------+
| Operation | Target | Current |
+===========+========+=========+
| Single    | file   | check   |
+-----------+--------+---------+



## Continuous Profiling

For CI integration, use criterion's baseline comparison:
```bash
cargo bench -p dx-check -- --save-baseline main cargo bench -p dx-check -- --baseline main cargo bench -p dx-check -- --baseline main --significance-level 0.05 ```


## Troubleshooting



### Flamegraph shows `[unknown]`


- Build with debug symbols: `RUSTFLAGS="-C debuginfo=2" cargo build
- release`


### Profile is too noisy


- Increase sample count: `cargo flamegraph
- iterations 1000`


### Memory profiler crashes


- Reduce input size or use sampling profiler
