
# DX Serializer Benchmark Methodology

This document describes how to reproduce the token efficiency benchmarks for DX Serializer.

## Overview

DX Serializer benchmarks measure token efficiency across multiple LLM tokenizers by comparing the token count of JSON input against DX Serializer output.

## Prerequisites

- Build the CLI:
```bash
cargo build --release --package dx-cli ```
- Verify the CLI is available:
```bash
./target/release/dx-cli --version ```

## Running Benchmarks

### Quick Start

```bash


# Run DX Serializer benchmark only


dx sr benchmark essence/


# Run both DX Markdown and DX Serializer benchmarks


./scripts/benchmark.sh -v


# On Windows


scripts\benchmark.bat -v ```


### Benchmark Script Options


```bash
./scripts/benchmark.sh [OPTIONS]
Options:
- b,
- -build Build the CLI before running benchmarks
- v,
- -verbose Show detailed output
- o,
- -output DIR Output directory for results (default: .dx/benchmarks)
- f,
- -format FMT Output format: text, json, markdown (default: text)
- h,
- -help Show this help message
```


## Source Data


+------------------+-------------+-----------+
| File             | Description | Size      |
+==================+=============+===========+
| `analytics.json` | Web         | analytics |
+------------------+-------------+-----------+


### Adding Custom Test Data


To benchmark your own data: -Place JSON files in `essence/datasets/` -Run the benchmark script -Results will include your files


## Tokenizers Used


+-----------+--------+
| Tokenizer | Models |
+===========+========+
| `cl100k   | base`  |
+-----------+--------+


## Benchmark Process


- Load JSON file: Read the source JSON file
- Parse JSON: Parse into internal representation
- Convert to DX: Serialize to DX LLM format
- Tokenize both: Count tokens for JSON and DX using each tokenizer
- Calculate savings: `(json_tokens
- dx_tokens) / json_tokens * 100`


## Output Format


Benchmark results are saved to `.dx/benchmarks/` with timestamps: @tree:.dx/benchmarks[]


### Sample Output


```markdown

## Summary by Model

+--------+-----+----------+-----+----+-----+---------+
| Model  | Avg | Original | Avg | DX | Avg | Savings |
+========+=====+==========+=====+====+=====+=========+
| GPT-4o | 5   | 496      | 1   | o1 | 5   | 496     |
+--------+-----+----------+-----+----+-----+---------+
```


## Reproducing Published Results


To reproduce the results shown in the README:
```bash



# 1. Clone the repository

git clone <repository-url> cd dx

# 2. Build the CLI

cargo build --release --package dx-cli

# 3. Run benchmarks with verbose output

./scripts/benchmark.sh -v

# 4. View results

cat .dx/benchmarks/dxs_benchmark_*.md ```

## Comparison Methodology

When comparing DX Serializer to other formats: -JSON: Standard `JSON.stringify()` output -JSON compact: `JSON.stringify()` without whitespace -YAML: Standard YAML serialization -XML: Standard XML serialization -TOON: TOON format serialization All formats are tokenized using the same tokenizers for fair comparison.

## Performance Benchmarks

For runtime performance benchmarks (not token efficiency), dx-serializer includes a comprehensive benchmark suite using Criterion.

### Running Benchmarks

```bash


# Run all benchmarks


cargo bench -p dx-serializer


# Run the comprehensive benchmark suite


cargo bench -p dx-serializer --bench comprehensive


# Run specific benchmark groups


cargo bench -p dx-serializer -- parse_machine cargo bench -p dx-serializer -- parse_llm cargo bench -p dx-serializer -- serialize_machine cargo bench -p dx-serializer -- serialize_llm cargo bench -p dx-serializer -- roundtrip_machine cargo bench -p dx-serializer -- roundtrip_llm ```


### Benchmark Categories


+----------+-------------+-----------+--------+
| Category | Description | Functions | Tested |
+==========+=============+===========+========+
| `parse   | machine`    | Parse     | binary |
+----------+-------------+-----------+--------+


### Input Sizes


+-------+---------------+-------+------+
| Size  | Bytes/Entries | Use   | Case |
+=======+===============+=======+======+
| Small | ~100          | bytes | /    |
+-------+---------------+-------+------+


### Benchmark Data Generation


The benchmarks use realistic data generators: -Machine format: Table-based data with typed columns (id, name, email, score, active) -LLM format: Key-value pairs in Dx Serializer syntax with mixed types (strings, numbers, booleans)


### Performance Metrics


+-----------+--------+-------+
| Operation | Metric | Notes |
+===========+========+=======+
| Parse     | (LLM)  | Time  |
+-----------+--------+-------+


### Sample Results


Results will vary based on hardware. Run benchmarks on your target hardware for accurate measurements.
```

# Example output format (actual numbers depend on hardware)

parse_machine/small time: [X.XX µs Y.YY µs Z.ZZ µs]
parse_machine/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
parse_machine/large time: [X.XX ms Y.YY ms Z.ZZ ms]
parse_llm/small time: [X.XX µs Y.YY µs Z.ZZ µs]
parse_llm/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
parse_llm/large time: [X.XX ms Y.YY ms Z.ZZ ms]
serialize_machine/small time: [X.XX µs Y.YY µs Z.ZZ µs]
serialize_machine/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
serialize_machine/large time: [X.XX ms Y.YY ms Z.ZZ ms]
serialize_llm/small time: [X.XX µs Y.YY µs Z.ZZ µs]
serialize_llm/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
serialize_llm/large time: [X.XX ms Y.YY ms Z.ZZ ms]
roundtrip_machine/small time: [X.XX µs Y.YY µs Z.ZZ µs]
roundtrip_machine/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
roundtrip_machine/large time: [X.XX ms Y.YY ms Z.ZZ ms]
roundtrip_llm/small time: [X.XX µs Y.YY µs Z.ZZ µs]
roundtrip_llm/medium time: [X.XX µs Y.YY µs Z.ZZ µs]
roundtrip_llm/large time: [X.XX ms Y.YY ms Z.ZZ ms]
```


### Interpreting Results


- Throughput: Criterion reports throughput in bytes/second for parsing benchmarks
- Time: Three values shown are [lower bound, estimate, upper bound] at 95% confidence
- Comparison: Run benchmarks before and after changes to detect regressions
- Sample size: Large inputs use reduced sample size (20) for faster benchmark runs


### Benchmark Reports


Criterion generates HTML reports in `target/criterion/`:
```bash

# After running benchmarks, open the report

open target/criterion/report/index.html # macOS xdg-open target/criterion/report/index.html # Linux start target/criterion/report/index.html # Windows ```
Reports include: -Performance over time (if run multiple times) -Statistical analysis with confidence intervals -Comparison with previous runs -Throughput graphs for different input sizes

## Notes

- Token counts may vary slightly between tokenizer versions
- Savings percentages are calculated per-file, then averaged
- Machine format benchmarks measure raw performance, not token efficiency
- All benchmarks run on the same hardware for consistency
