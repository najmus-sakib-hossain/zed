
# DX JavaScript Runtime

A fast, modern JavaScript/TypeScript runtime built with Rust and Cranelift JIT.

## Installation

```bash
npm install -g dx-js ```


## Usage


```bash

# Run a JavaScript file

dx-js script.js

# Run a TypeScript file (no compilation step needed)

dx-js script.ts

# Show help

dx-js --help

# Show version

dx-js --version ```

## Features

- Fast Startup: Near-instant cold start times
- TypeScript Support: Run TypeScript files directly without compilation
- Node.js Compatibility: Compatible with most Node.js APIs
- Modern JavaScript: Full ES2020+ support including BigInt, dynamic import, etc.
- JIT Compilation: Cranelift-based JIT for optimal performance

## Supported Platforms

+----------+--------------+--------+
| Platform | Architecture | Status |
+==========+==============+========+
| Linux    | x86          | 64     |
+----------+--------------+--------+



## Manual Installation

If npm installation fails, you can download binaries directly from GitHub Releases.

### Linux/macOS

```bash


# Download and extract


tar -xzf dx-js-<platform>.tar.gz


# Make executable


chmod +x dx-js


# Move to PATH


sudo mv dx-js /usr/local/bin/ ```


### Windows


- Download the `.zip` file
- Extract `dx-js.exe`
- Add the directory to your PATH


## Verifying Downloads


All releases include SHA256 checksums. To verify:
```bash

# Linux

sha256sum -c dx-js-linux-x86_64.tar.gz.sha256

# macOS

shasum -a 256 -c dx-js-macos-arm64.tar.gz.sha256 ```

## License

MIT
