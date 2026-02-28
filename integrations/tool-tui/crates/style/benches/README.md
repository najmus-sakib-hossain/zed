
# dx-style Benchmarks

This directory contains performance benchmarks for the dx-style crate.

## Running Benchmarks

```bash


# Run all benchmarks


cargo bench -p dx-style


# Run specific benchmark file


cargo bench -p dx-style --bench binary_styles_benchmark cargo bench -p dx-style --bench style_benchmark


# Run without plots (faster)


cargo bench -p dx-style -- --noplot


# Run specific benchmark group


cargo bench -p dx-style -- "Class Lookup"
```

## Benchmark Files

### `binary_styles_benchmark.rs`

+-----------+---------+-------------+--------+
| Benchmark | Group   | Description | Target |
+===========+=========+=============+========+
| Class     | Lookup: | Single      | Single |
+-----------+---------+-------------+--------+



### `style_benchmark.rs`

+-----------+---------+-------------+
| Benchmark | Group   | Description |
+===========+=========+=============+
| html      | parsing | Parse       |
+-----------+---------+-------------+



## Latest Results

Results from running on Windows x86_64:

### Class Lookup Performance

+-----------+-------+--------+
| Operation | Time  | Status |
+===========+=======+========+
| Single    | class | lookup |
+-----------+-------+--------+



### HTML Extraction Performance

+------+------+------------+
| Size | Time | Throughput |
+======+======+============+
| 1kb  | <    | 50µs       |
+------+------+------------+



## Reproducing Results

- Ensure you have a release build:
```bash
cargo build --release -p dx-style ```
- Run benchmarks:
```bash
cargo bench -p dx-style ```
- Results are saved in `target/criterion/` with HTML reports.

## Performance Claims Validation

+------------+-----------+---------------------+
| Claim      | Benchmark | Validated           |
+============+===========+=====================+
| "Sub-20µs  | class     | additions/removals" |
+------------+-----------+---------------------+



## Notes

- Benchmarks use Criterion.rs for statistical analysis
- Each benchmark runs 100 samples by default
- Results may vary based on hardware and system load
- Run benchmarks on a quiet system for consistent results
