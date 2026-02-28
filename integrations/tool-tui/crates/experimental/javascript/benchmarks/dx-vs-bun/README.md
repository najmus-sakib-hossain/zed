
# DX vs Bun Benchmark Suite

A benchmark suite comparing the DX JavaScript runtime against Bun.

## Latest Results

+-----------+-------+-----------+------------+
| Benchmark | DX-JS | Bun       | Difference |
+===========+=======+===========+============+
| Hello     | World | (startup) | 80ms       |
+-----------+-------+-----------+------------+



## Overview

This benchmark suite measures and compares performance across: -Runtime: JavaScript execution speed, startup time, memory usage -Package Manager: Install speed (cold/warm), dependency resolution -Bundler: Bundle time, output size, tree-shaking effectiveness -Test Runner: Test discovery, execution speed, parallelization -Project Manager: Workspace discovery, task graph, affected detection -Compatibility Layer: Node.js API performance (fs, path, crypto, etc.) -End-to-End Workflows: Real-world development scenarios

## Prerequisites

### Required

- DX Toolchain: Built from source in release mode
- Bun: Latest stable version (install)
- Rust/Cargo: For building DX tools

### Windows

- PowerShell 5.1+ (included with Windows)
- Or PowerShell Core 7+ (recommended)

### Unix (Linux/macOS)

- Bash 4.0+
- `bc` command (for calculations)
- PowerShell Core 7+ (optional, for full benchmark support)

## Quick Start

### Windows (PowerShell)

```powershell


# Run all benchmarks with defaults


./run-all.ps1


# Run with custom settings


./run-all.ps1 -Runs 20 -Warmup 5


# Run specific suite only


./run-all.ps1 -Suite "Runtime"


# Skip DX build (if already built)


./run-all.ps1 -SkipBuild ```


### Unix (Bash)


```bash

# Make executable

chmod +x run-all.sh

# Run all benchmarks

./run-all.sh

# Run with custom settings

./run-all.sh --runs 20 --warmup 5

# Run specific suite

./run-all.sh --suite "Runtime"

# Skip DX build

./run-all.sh --skip-build ```

## Command Line Options

+--------+------------+------+---------+-------------+
| Option | PowerShell | Bash | Default | Description |
+========+============+======+=========+=============+
| Runs   | `-Runs     | N`   | `--runs | N`          |
+--------+------------+------+---------+-------------+



## Output

### Generated Files

@tree:results[]

### JSON Output Format

```json
{ "name": "DX vs Bun Benchmarks", "timestamp": "2024-01-15T10:30:00Z", "system": { "os": "Windows 11", "cpu": "AMD Ryzen 9 5900X", "cores": 24, "memory": "32 GB", "dxVersion": "0.0.1", "bunVersion": "1.0.0"
}, "suites": [...], "summary": { "totalBenchmarks": 50, "dxWins": 30, "bunWins": 15, "ties": 5, "overallWinner": "dx"
}
}
```

## Benchmark Suites

### Runtime (`suites/runtime/`)

- Fibonacci: CPU-intensive recursive calculation
- JSON Parse: Large JSON parsing/serialization
- Startup (Cold): First execution without cache
- Startup (Warm): Subsequent execution with cache
- Memory Stress: Object allocation and GC

### Package Manager (`suites/package-manager/`)

- Cold Install (Small): 3 dependencies, no cache
- Warm Install (Small): 3 dependencies, with cache
- Cold Install (Large): 50+ dependencies, no cache
- Warm Install (Large): 50+ dependencies, with cache

### Bundler (`suites/bundler/`)

- Small Project: 5 files
- Medium Project: 50 files
- Large Project: 150 files
- Tree Shaking: Dead code elimination

### Test Runner (`suites/test-runner/`)

- Discovery: Test file discovery time
- Small Suite: 50 tests
- Medium Suite: 150 tests
- Large Suite: 300 tests
- Parallelization: Multi-core efficiency

### Project Manager (`suites/project-manager/`)

- Workspace Discovery: 10/50/100 packages
- Task Graph: Dependency graph construction
- Affected Detection: Change impact analysis
- Cache Performance: Hit vs miss comparison

### Compatibility (`suites/compatibility/`)

- fs: readFile, writeFile, readdir, stat
- path: join, resolve, parse, normalize
- crypto: sha256, randomBytes, randomUUID
- server creation, request handling
- EventEmitter: emit, on, off operations
- Buffer: alloc, from, toString, concat
- Web APIs: TextEncoder, URL, fetch

### Workflows (`suites/workflows/`)

- Fresh Setup: install + first build
- Dev Iteration: change → test → rebuild
- CI Pipeline: install → build → test → lint
- Monorepo Affected: selective build

## Interpreting Results

### Winner Determination

A winner is declared only when: -The difference exceeds the margin of error (95% CI) -The percentage difference is > 5% Results marked with (ns) are not statistically significant.

### Speedup Calculation

- For time-based metrics: `speedup = slower / faster`
- For throughput metrics: `speedup = faster / slower`
- A speedup of 2.0x means one tool is twice as fast

### Statistical Methods

- Median: Primary metric (reduces outlier impact)
- Mean: Secondary metric
- Std Dev: Measurement consistency
- P95/P99: Tail latency
- Confidence Interval: 95% using t-distribution

## Methodology

### Fairness Measures

- Equivalent Code: Same algorithms for both tools
- No Optimizations: No runtime-specific tricks
- Isolation: Each benchmark runs in separate process
- Warmup: Initial runs excluded from results
- Multiple Runs: Minimum 10 iterations per benchmark

### System Requirements

For accurate results: -Close other applications -Disable power saving modes -Use consistent hardware -Run multiple times to verify

## Troubleshooting

### DX Not Found

```
DX runtime not found. Build with: cargo build --release -p dx-js-runtime ```
Build DX tools first:
```bash
cd /path/to/dx cargo build --release -p dx-js-runtime ```

### Bun Not Found

```
Bun not installed. Install from: https://bun.sh ```
Install Bun:
```bash
curl -fsSL https://bun.sh/install | bash
```


### High CPU Warning


```
High CPU usage detected (75%). Results may be affected.
```
Close other applications and retry.


## Contributing


- Add new benchmarks to appropriate suite directory
- Follow existing patterns for measurement
- Include warmup and multiple runs
- Document what the benchmark measures


## License


MIT License - See repository root for details.
