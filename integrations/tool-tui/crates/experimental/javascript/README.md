⚠️ Pre-Alpha Software (v0.0.1) DX-JS is in early development. Many features are experimental, incomplete, or not yet implemented. APIs may change without notice. Not recommended for production use. Known Limitations: -BigInt: Not implemented -dynamic import(): Not implemented -fs.watch(): Not implemented -Many Node.js APIs are stubs or partial implementations

## Overview

DX is a complete JavaScript/TypeScript development toolchain that combines a package manager, runtime, bundler, and test runner into a single, cohesive experience. Built from the ground up in Rust for maximum performance, DX aims to deliver significant speed improvements over existing tools while maintaining full compatibility with the npm ecosystem.

## Features

### Package Manager (`dx`)

- O(1) cached installs
- Near-instant installs from warm cache
- npm-compatible
- Works with existing `package.json` and lockfiles
- Workspace support
- Monorepo-ready with workspace protocols
- Security auditing
- Built-in vulnerability scanning
- Lifecycle scripts
- Full support for pre/post install hooks

### Runtime (`dx-js`)

- TypeScript-first
- Run `.ts` files directly without compilation
- ES Modules & CommonJS
- Full support for both module systems
- Node.js compatible
- Drop-in replacement for most Node.js scripts
- Built-in REPL
- Interactive JavaScript/TypeScript shell
- Fast startup
- Optimized cold start performance

### Bundler (`dx-bundle`)

- SIMD-accelerated scanning
- Ultra-fast import/export detection
- Tree shaking
- Dead code elimination for smaller bundles
- Code splitting
- Automatic chunk optimization
- Source maps
- Full debugging support
- Multiple formats
- ESM, CJS, IIFE, UMD output

### Test Runner (`dx-test`)

- Parallel execution
- Utilize all CPU cores
- Jest-compatible API
- Familiar `test`, `expect`, `describe`
- Snapshot testing
- Built-in snapshot support
- Code coverage
- HTML, JSON, LCOV reports
- Watch mode
- Instant feedback during development

## Installation

### From Source

```bash


# Clone the repository


git clone https://github.com/user/dx-js.git cd dx-js


# Build all components


cargo build --release


# Binaries are in target/release/


```

### Pre-built Binaries

Pre-built binaries are not yet available. Build from source for now.

## Quick Start

### Package Manager

```bash


# Initialize a new project


dx init


# Add dependencies


dx add lodash dx add typescript --dev


# Install all dependencies


dx install


# Run scripts


dx run build dx run test ```


### Runtime


```bash

# Run JavaScript

dx-js app.js

# Run TypeScript directly

dx-js app.ts

# Start REPL

dx-js

# Run with debugging

dx-js --inspect app.ts ```

### Bundler

```bash


# Bundle for production


dx-bundle bundle src/index.ts -o dist/bundle.js --minify


# Development with watch mode


dx-bundle bundle src/index.ts -o dist/bundle.js --watch


# Different output formats


dx-bundle bundle src/index.ts -o dist/bundle.cjs --format cjs ```


### Test Runner


```bash

# Run all tests

dx-test

# Watch mode

dx-test --watch

# With coverage

dx-test --coverage

# Filter tests

dx-test "auth"
```


## Benchmarks


Performance benchmarks are available in the benchmarks (./benchmarks) directory. Results vary based on hardware, OS, and workload characteristics.


### Benchmark Methodology


- All benchmarks are reproducible using scripts in `benchmarks/`
- Results are averaged over 10 runs with standard deviation reported
- Warm cache and cold cache scenarios are tested separately
- Hardware specifications are documented with each benchmark run


### Running Benchmarks


```bash

# Run all benchmarks

cd benchmarks ./run-benchmarks.sh

# Compare against other tools

./compare-tools.sh ```
See docs/BENCHMARKS.md (docs/BENCHMARKS.md) for detailed methodology and results. Test Environment: -CPU: AMD Ryzen 9 5900X -RAM: 32GB DDR4 -OS: Ubuntu 22.04 -Rust: 1.75.0 +-------------+------------------------+--------------------+-----------------------------+
| Benchmark   | DX-JS                  | Node.js 20         | Notes                       |
+=============+========================+====================+=============================+
| Hello World | ~80ms                  | ~100ms             | Cold start Fibonacci (n=40) |
+-------------+------------------------+--------------------+-----------------------------+



## Architecture

@tree:dx-js[]

## Platform Support

+----------+--------------+--------+
| Platform | Architecture | Status |
+==========+==============+========+
| Linux    | x86          | 64     |
+----------+--------------+--------+



## Documentation

- Getting Started Guide (docs/GETTING_STARTED.md)
- Installation and basic usage
- API Reference (docs/API_REFERENCE.md)
- Complete API documentation
- Compatibility Matrix (docs/COMPATIBILITY.md)
- Node.js API support status
- Migration Guide (docs/MIGRATION.md)
- Migrating from Node.js/npm/Jest
- Benchmark Details (docs/BENCHMARKS.md)
- Performance methodology

## Contributing

We welcome contributions! See our contributing guidelines for details.
```bash


# Run tests


cargo test --workspace


# Run clippy


cargo clippy --workspace


# Format code


cargo fmt --all ```


## License


Licensed under either of: -Apache License, Version 2.0 (LICENSE-APACHE (LICENSE-APACHE) or //www.apache.org/licenses/LICENSE-2.0) -MIT License (LICENSE-MIT (LICENSE-MIT) or //opensource.org/licenses/MIT) at your option.


## Acknowledgments


DX builds on the shoulders of giants: -OXC - Fast JavaScript parser -SWC - Speedy web compiler -Bun - Inspiration for all-in-one tooling
