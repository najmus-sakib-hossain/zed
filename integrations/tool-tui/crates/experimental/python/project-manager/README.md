
# DX-Py Project Manager

Python project lifecycle and workspace management.

## Status: In Development

The project manager handles Python project lifecycle operations.

## Overview

Provides: -Project initialization and scaffolding -Workspace management for monorepos -Python version management

## Features

### Project Initialization

```bash


# Create new project


dx-py init my-project


# Initialize in current directory


dx-py init .


# Specify Python version


dx-py init --python 3.12 ```


### Workspace Management


Support for monorepo-style workspaces:
```toml

# pyproject.toml

[tool.dx-py.workspace]
members = ["packages/*"]
```


### Python Version Management


- Automatic Python version detection
- Version pinning via `.python-version`
- Integration with package manager


## Architecture


@tree:project-manager[]


## Development


```bash

# Run tests

cargo test

# Build

cargo build --release ```

## See Also

- Package Manager (../package-manager/README.md)
- Main Documentation (../README.md)

## License

MIT OR Apache-2.0
