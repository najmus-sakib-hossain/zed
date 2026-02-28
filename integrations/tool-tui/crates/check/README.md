# dx-check

Fast JavaScript/TypeScript linter built in Rust.

## What Works

### Linting (check command)
- ✅ JavaScript (.js, .mjs, .cjs)
- ✅ TypeScript (.ts, .tsx)
- ✅ JSX (.jsx)
- ✅ HTML (.html)
- ✅ Built-in rules (no-console, no-var, no-debugger, no-eval, etc.)
- ✅ Fast performance (3000+ files/sec)

### Formatting (format command)
- ✅ Python (.py) - via ruff/black
- ✅ Rust (.rs) - via rustfmt
- ✅ TOML (.toml)
- ✅ Markdown (.md)
- ✅ CSS (.css)
- ✅ JSON (.json)
- ✅ YAML (.yaml, .yml)
- ⚠️ Go (.go) - requires gofmt installed

### Multi-language Linting (lint command)
- ✅ Python - via ruff (requires ruff installed)
- ⚠️ Rust - via clippy (requires cargo installed)
- ⚠️ Go - via gofmt (requires go installed)
- ⚠️ Markdown - via rumdl (requires rumdl installed)

### Other Features
- ✅ Directory scanning
- ✅ Multiple output formats (pretty, JSON, compact, GitHub, JUnit)
- ✅ Parallel processing
- ✅ Caching

## What Doesn't Work

- ❌ `--fix` flag (disabled - spans incorrect)
- ❌ `--score` command (stubbed)
- ❌ `--test` command (stubbed)
- ❌ `--coverage` flag (not implemented)
- ❌ Plugin system (install/search not implemented)

## Usage

```bash
# Lint JavaScript/TypeScript
dx-check check src/

# Format Python
dx-check format src/ --write

# Multi-language lint (requires external tools)
dx-check lint src/

# Check specific file
dx-check check file.js

# JSON output
dx-check check src/ --format json
```

## Status

Working JavaScript/TypeScript linter with multi-language formatting support via external tools.
