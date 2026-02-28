
# DX-Py Compatibility

Compatibility layer for DX-Py.

## Status: In Development

This crate provides compatibility with existing Python tooling.

## Overview

Provides compatibility with: -UV configuration parsing and merging -pip/pipx command compatibility -Environment markers (PEP 508) -Virtual environment management -Platform detection (manylinux, musllinux)

## Features

### Configuration System

```rust
use dx_py_compability::config::{DxPyConfig, UvConfig};
// Load and merge configurations let uv_config = UvConfig::load("uv.toml")?;
let dx_config = DxPyConfig::load("pyproject.toml")?;
let merged = dx_config.merge_with_uv(&uv_config);
```

### Environment Markers

```rust
use dx_py_compability::markers::{MarkerEnvironment, evaluate};
let env = MarkerEnvironment::current();
let result = evaluate("python_version >= '3.8'", &env)?;
```

### Platform Detection

```rust
use dx_py_compability::platform::{detect_platform, ManylinuxVersion};
let platform = detect_platform();
let compatible = platform.is_manylinux_compatible(ManylinuxVersion::Manylinux2014);
```

## Architecture

@tree:compatibility[]

## Supported Markers

+---------+-------------+
| Marker  | Description |
+=========+=============+
| `python | version`    |
+---------+-------------+



## Platform Support

### Linux

- manylinux1 (glibc 2.5)
- manylinux2010 (glibc 2.12)
- manylinux2014 (glibc 2.17)
- manylinux_x_y (glibc x.y)
- musllinux_x_y (musl x.y)

### macOS

- macosx_x_y_arch
- Universal binaries

### Windows

- win32
- win_amd64
- win_arm64

## Testing

```bash


# Run all tests


cargo test


# Run property-based tests


cargo test -- proptest ```


## License


MIT OR Apache-2.0
