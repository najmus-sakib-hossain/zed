
# DX-Py Package Manager Performance

Performance details for the DX-Py package manager.

## Status: In Development

Performance benchmarks require output validation before claims can be made.

## Architecture

### Binary Lock File (DPL Format)

Memory-mapped binary lock files with hash table lookup:
```
Format: Binary with FNV-1a hashing Lookup: O(1) hash table ```


### Content-Addressable Store


Hash-based package storage:
```
Deduplication: Automatic across projects Sharing: Hard links on supported filesystems ```
Packages are stored once by content hash and shared across all projects.

### Layout Cache

Pre-built virtual environment layouts:
```
Operation: Single symlink/junction for cached layouts ```


## Internal Benchmarks


Criterion benchmarks for core operations:


### Project Hash Computation


+---------+-------+------+
| Package | Count | Time |
+=========+=======+======+
| 10      | ~1µs  | 50   |
+---------+-------+------+


### DPL Lock File Lookup


+---------+-------+------+
| Package | Count | Time |
+=========+=======+======+
| 10      | ~80ns | 100  |
+---------+-------+------+


### Package Store Operations


+----------------+--------+
| Operation      | Time   |
+================+========+
| `contains\(\)` | ~18µs  |
+----------------+--------+


## Running Benchmarks


```bash

# Run all criterion benchmarks

cargo bench --package dx-py-cli

# Run specific benchmarks

cargo bench --package dx-py-cli --bench layout_benchmarks ```

## Methodology

- Iterations: Multiple runs per benchmark
- Statistical Analysis: Mean and standard deviation

## Known Limitations

- External comparison benchmarks (vs UV) require output validation
- Performance claims should be verified with validated benchmarks
- Results may vary based on system configuration

## See Also

- CLI Reference (docs/CLI_REFERENCE.md)
- Benchmark Framework (../benchmarks/README.md)
