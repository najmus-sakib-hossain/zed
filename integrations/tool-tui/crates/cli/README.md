
# dx-cli

(LICENSE) The unified command-line interface for the DX ecosystem.

## Overview

`dx-cli` provides a single entry point for all DX tools, including the web framework compiler, font utilities, code generation, and more. It orchestrates the various DX crates into a cohesive developer experience.

## Features

- Unified CLI for all DX tools
- Project scaffolding and initialization
- Development server with hot reload
- Build and optimization commands
- Code generation utilities
- Template registry and management
- Structured logging with verbose/quiet modes
- Graceful shutdown handling

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
dx-cli = "0.1.0"
```
Or install globally:
```bash
cargo install dx-cli ```

## Usage

```bash
dx new my-project dx dev dx build --release dx gen run component --param name=MyComponent dx font search "Roboto"
dx style ./src/styles ./dist/styles.css ```

## Global Flags

t:0(Flag,Description)[]

## Subcommands

+--------------+-------------+
| Command      | Description |
+==============+=============+
| `new`        | Create      |
+--------------+-------------+
| `markdown`   | Process     |
+--------------+-------------+
| `serializer` | Convert     |
+--------------+-------------+
| `token`      | Count       |
+--------------+-------------+
| `forge`      | Control     |
+--------------+-------------+
| `gen`        | Generate    |
+--------------+-------------+
| `security`   | Scan        |
+--------------+-------------+
| `dcp`        | Run         |
+--------------+-------------+
| `www`        | Web         |
+--------------+-------------+


## Forge Daemon (`dx forge`)

Control the Forge daemon for background processing.
```bash
dx forge start dx forge status dx forge stop dx forge logs --lines 100 ```

## Code Generation (`dx gen`)

The `dx gen` command provides powerful template-based code generation.

### Generate from Template

```bash
dx gen run component --param name=Counter --param with_state=true dx gen run component --param name=Counter --output src/components/counter.rs dx gen run component --param name=Counter --dry-run dx gen run component --param name=Counter --force ```

### List Available Templates

```bash
dx gen list dx gen list --category rust dx gen list --format json ```

### Scaffold Multi-File Projects

```bash
dx gen scaffold rust-crate --param name=my_lib --param description="My library"
dx gen scaffold rust-crate --param name=my_lib --target ./libs ```

## Sandbox Management (`dx sandbox`)

The `dx sandbox` command provides isolated execution environments for AI-generated code and untrusted operations.

### Create Sandbox (Opens Shell Automatically)

```bash

# Create sandbox - automatically opens interactive shell

dx sandbox create my-env


# With Docker backend and custom settings

dx sandbox create my-env --backend docker --memory 512 --network


# Skip shell opening (just create)

dx sandbox create my-env --no-shell
```

### Interactive Shell

```bash

# Open shell in existing sandbox

dx sandbox shell my-env


# Specify shell type

dx sandbox shell my-env --shell bash
```

### Execute Commands

```bash
dx sandbox run my-env -- echo "Hello from sandbox"
dx sandbox run my-env -- cargo build
```

### File Operations

```bash
dx sandbox copy-in my-env ./script.sh /workspace/script.sh
dx sandbox run my-env -- sh /workspace/script.sh
dx sandbox copy-out my-env /workspace/output.txt ./output.txt
```

### List and Manage

```bash
dx sandbox list
dx sandbox info my-env
dx sandbox destroy my-env
dx sandbox destroy-all
```

## Security Scanning (`dx security`)

Scan code for security vulnerabilities.
```bash
dx security scan dx security scan ./src dx security scan --format json dx security audit ```

## DCP Server (`dx dcp`)

Run the DX Communication Protocol server.
```bash
dx dcp serve dx dcp serve --port 9000 dx dcp validate message.json ```

## WWW Framework (`dx www`)

Web framework commands for building applications.
```bash
dx www component Button dx www route /users dx www build dx www dev ```

## Configuration

Create a `dx.toml` in your project root:
```toml
[project]
name = "my-app"
version = "0.1.0"
[build]
target = "web"
optimize = true
[dev]
port = 3000 hot_reload = true
[generator]
template_paths = [".dx/templates", "~/.dx/templates"]
output_dir = "src/generated"
enable_patching = true
[generator.watch]
enabled = false debounce_ms = 500 patterns = ["src/**/*.schema.json"]
```

## Environment Variables

+----------+-------------+---------+
| Variable | Description | Default |
+==========+=============+=========+
| `DX      | LOG         | LEVEL`  |
+----------+-------------+---------+

## Output Formats

Several commands support different output formats:
```bash
dx gen list --format table dx gen list --format json dx gen list --format simple ```

## Troubleshooting

### Daemon Connection Issues

If you're having trouble connecting to the Forge daemon:
```bash
dx forge status dx forge stop dx forge start dx forge logs --lines 50 ```

### Port Conflicts

If default ports are in use, configure alternatives via environment variables:
```bash
export DX_FORGE_PORT=9880 export DX_DCP_PORT=9010 dx forge start ```

### Verbose Debugging

Enable verbose output to diagnose issues:
```bash
dx --verbose forge start DX_LOG_LEVEL=debug dx forge start ```

### Cache Issues

Clear the cache if you encounter stale data:
```bash
dx cache clean dx cache info ```

## License

This project is dual-licensed under MIT OR Apache-2.0.
