
# DX Benchmarks

⚠️ Early Development Notice (v0.0.1) DX-JS is in early development. These benchmarks are preliminary and may not reflect final performance. Results vary significantly by hardware, workload, and configuration. Performance measurements for the DX JavaScript toolchain.

## Runtime Benchmark Results

+-----------+-------+-----------+-------+------------+
| Benchmark | DX-JS | Bun       | 1.3.5 | Difference |
+===========+=======+===========+=======+============+
| Hello     | World | (startup) | 80ms  | 96ms       |
+-----------+-------+-----------+-------+------------+



## Running Benchmarks

### DX vs Bun Comparison

```bash
cd benchmarks/dx-vs-bun bash run-bench.sh ```


### Quick Start


```bash

# Run all benchmarks (5 runs each)

./benchmarks/run-benchmarks.sh

# Run with comparison to npm/bun

./benchmarks/run-benchmarks.sh -c

# Custom number of runs

./benchmarks/run-benchmarks.sh -r 10

# Save results to custom file

./benchmarks/run-benchmarks.sh -o my-results.json ```

### Windows (PowerShell)

```powershell


# Run all benchmarks


.\benchmarks\run-benchmarks.ps1


# With comparison


.\benchmarks\run-benchmarks.ps1 -Compare


# Custom runs


.\benchmarks\run-benchmarks.ps1 -Runs 10 ```


## Benchmark Categories



### Runtime Benchmarks


+--------+-------------+
| Metric | Description |
+========+=============+
| Cold   | start       |
+--------+-------------+


### Package Manager Benchmarks


+--------+-------------+
| Metric | Description |
+========+=============+
| Cold   | install     |
+--------+-------------+


### Bundler Benchmarks


+--------+-------------+
| Metric | Description |
+========+=============+
| Cold   | bundle      |
+--------+-------------+


## Sample Results


Results from a typical development machine (varies by hardware):


### Runtime


+-----------+--------+
| Operation | Time   |
+===========+========+
| Simple    | script |
+-----------+--------+


### Package Manager


+-----------+---------+
| Operation | Time    |
+===========+=========+
| Warm      | install |
+-----------+---------+


### Bundler


+-----------+--------+
| Operation | Time   |
+===========+========+
| Small     | bundle |
+-----------+--------+


### Test Runner


+--------+-------+
| Metric | Value |
+========+=======+
| 50     | tests |
+--------+-------+


## Methodology



### Measurement


- Each benchmark runs multiple times (default: 5)
- Results report median, min, max, and mean
- Warm-up runs are excluded
- System load is minimized during benchmarks


### Environment


- Release builds only (`cargo build
- -release`)
- No debug assertions
- Native CPU optimizations enabled
- Disk cache warmed for warm benchmarks


### Comparison


When comparing with other tools: -Same test files used -Same machine and conditions -Multiple runs to account for variance -Results may vary by workload


## Interpreting Results



### What Affects Performance


- Cold vs Warm: First run includes compilation/caching overhead
- File Size: Larger files take longer to parse and compile
- Complexity: More complex code requires more optimization
- I/O: Network and disk speed affect package installation
- CPU: JIT compilation benefits from faster CPUs


### Caveats


- Benchmarks measure specific workloads
- Real-world performance varies
- Micro-benchmarks may not reflect actual usage
- Always benchmark your specific use case


## Running Your Own Benchmarks



### Runtime


```bash

# With timing

DX_DEBUG=1 ./target/release/dx-js your-script.js

# Multiple runs

for i in {1..10}; do time ./target/release/dx-js your-script.js done ```

### Package Manager

```bash


# Cold install


rm -rf node_modules dx.lock time ./target/release/dx install


# Warm install


rm -rf node_modules time ./target/release/dx install ```


### Bundler


```bash

# Cold bundle

rm -rf .dx-cache time ./target/release/dx-bundle bundle src/index.js -o dist/bundle.js

# Warm bundle

time ./target/release/dx-bundle bundle src/index.js -o dist/bundle.js ```

## CI Integration

The benchmark suite can be integrated into CI:
```yaml


# GitHub Actions example


- name: Run benchmarks
run: | cargo build --release
./benchmarks/run-benchmarks.sh -r 3 -o benchmark-results.json
- name: Upload results
uses: actions/upload-artifact@v3 with:
name: benchmark-results path: benchmarks/benchmark-results.json ```


## Notes


- Results are machine-specific
- Run benchmarks on consistent hardware for comparisons
- Consider variance when comparing small differences
- Focus on trends rather than absolute numbers
