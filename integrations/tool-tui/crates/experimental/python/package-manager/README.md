
# DX-Py Package Manager

A Python package manager written in Rust.

## Status: In Development

The package manager is actively being developed with a focus on correctness.

## Features

- Add packages to pyproject.toml
- Version constraint support (==, >=, <, etc.)
- Dev dependencies (--dev flag)
- Format preservation using toml_edit
- PubGrub-based dependency resolution

## Installation

```bash
cd package-manager cargo build --release


# Binary at target/release/dx-py


```

## Quick Start

```bash


# Add dependencies


dx-py add requests numpy pandas dx-py add --dev pytest black


# Add with version constraint


dx-py add "requests>=2.28"
```

## Commands

+---------+-------------+
| Command | Description |
+=========+=============+
| `add`   | Add         |
+---------+-------------+



## Architecture

@tree:package-manager[]

## Implemented Features

### Add Command

- Modifies pyproject.toml correctly
- Preserves file formatting
- Supports version constraints (==, >=, <, etc.)
- Supports
- -dev flag for dev dependencies
- Validates package names
- Prints success/error messages

### Dependency Resolution

- PubGrub-based resolver
- Conflict detection
- Circular dependency detection
- Prerelease handling
- Extras support

### Configuration

```toml


# pyproject.toml


[project]
name = "my-project"
version = "0.1.0"
dependencies = ["requests>=2.28"]
[project.optional-dependencies]
dev = ["pytest", "black"]
```

## Known Limitations

- Package installation from PyPI is partial
- Lock file generation is basic
- Some edge cases in dependency resolution
- Virtual environment creation is limited

## Test Coverage

~200+ tests covering: -Add command functionality -Version constraint parsing -pyproject.toml modification -Format preservation -Dependency resolution

## Testing

```bash


# Run all tests


cargo test --workspace


# Run property-based tests


cargo test --workspace -- proptest


# Run integration tests (requires network)


cargo test --test integration_tests -- --ignored ```


## License


MIT OR Apache-2.0
