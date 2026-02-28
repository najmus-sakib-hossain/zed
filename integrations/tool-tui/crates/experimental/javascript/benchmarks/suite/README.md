
# DX JavaScript Runtime Benchmark Suite

A comprehensive benchmark suite for measuring and comparing DX JavaScript Runtime performance against Node.js and Bun.

## Overview

This benchmark suite measures: -Cold Start Time: Time to start the runtime and execute a minimal script -Memory Usage: Heap and RSS memory consumption -Throughput: Operations per second for various workloads -Comparison: Side-by-side comparison with Node.js and Bun

## Requirements

- DX JavaScript Runtime (built from source or installed)
- Node.js 18+ (for comparison)
- Bun (optional, for comparison)
- Python 3.8+ (for result analysis)

## Running Benchmarks

```bash


# Run all benchmarks


./benchmarks/suite/run.sh


# Run specific benchmark category


./benchmarks/suite/run.sh --category startup ./benchmarks/suite/run.sh --category memory ./benchmarks/suite/run.sh --category throughput


# Run with comparison


./benchmarks/suite/run.sh --compare


# Generate report


./benchmarks/suite/run.sh --report ```


## Benchmark Categories



### 1. Startup Time


- Cold start (no cache)
- Warm start (with cache)
- TypeScript compilation overhead


### 2. Memory Usage


- Baseline memory footprint
- Memory under load
- Memory growth over time


### 3. Throughput


- JSON parsing/serialization
- Array operations
- Object manipulation
- String processing
- Async operations
- HTTP server requests/second


## Output


Results are saved to `benchmarks/suite/results/` in JSON format with: -Timestamp -Platform information -Raw measurements -Statistical analysis (mean, median, std dev, confidence intervals)
