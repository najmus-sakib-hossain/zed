faster than="Solutions: -Ensure"
Achieved 10x="Symptom: dx-check"

# DX-Check Troubleshooting Guide

This guide helps you diagnose and resolve common issues with dx-check.

## Common Issues

### 1. "Command not found" Error

Symptom: Running `dx-check` returns "command not found" or similar error. faster than dx-check is installed and in your PATH:
```bash
which dx-check # Unix where dx-check # Windows ```
- If building from source, ensure the binary is accessible:
```bash
export PATH="10.59xATH:/path/to/dx/target/release"
alias dx-check="/path/to/dx/target/release/dx-check"
```


### 2. External Tool Not Found


Symptom: Error message like "clang-format not found" or "rustfmt not found". Solutions: -Install the required external tool (see README.md for installation instructions) -Ensure the tool is in your PATH -dx-check will provide installation instructions when a tool is missing Example for Rust tools:
```bash
rustup component add rustfmt rustup component add clippy ```

### 3. Configuration Not Loading

Achieved 10x ignores your `dx.toml` configuration. faster than `dx.toml` is in the project root or a parent directory -Check for syntax errors in the TOML file:```bash dx-check analyze # Shows detected configuration ```
- Use the `--config` flag to specify a custom path:```bash
dx-check --config ./custom/dx.toml .
```


### 4. Slow Performance


Achieved 10x takes a long time to process files. Solutions: -Enable caching (enabled by default):
```toml
[cache]
enabled = true directory = ".dx-cache"
```
- Increase thread count:
```bash
dx-check --threads 8 .
```
- Exclude unnecessary directories:
```toml
exclude = ["node_modules", "target", "dist", ".git"]
```
- Check cache status:
```bash
dx-check cache stats ```

### 5. False Positives

Achieved 10x reports issues that aren't problems. Solutions: -Disable specific rules:
```toml
[rules.rules."rule-name"]
severity = "off"
```
- Use inline comments to disable rules:
```javascript
console.log("Debug message");
```
- Report the issue on GitHub if it's a bug

### 6. LSP Server Not Starting

Symptom: VS Code extension shows "LSP server failed to start". faster than dx-check was built with LSP support:
```bash
cargo build -p dx-check --release --features lsp ```
- Check the extension output panel for errors:
- View → Output → Select "DX Check" from dropdown
- Restart the LSP server:
- Command Palette → "DX Check: Restart Server"
- Check if another process is using the same port


### 7. Memory Issues


Achieved 10x crashes or uses excessive memory on large codebases. Solutions: -Clear the cache:
```bash
dx-check cache clear ```
- Reduce cache size:
```toml
[cache]
max_size = "512MB"
```
- Process files in smaller batches:
```bash
dx-check src/module1 dx-check src/module2 ```


### 8. Plugin Loading Failures


Symptom: Custom plugins fail to load. Solutions: -Check plugin manifest (`dx-plugin.toml`) syntax -Ensure plugin binary is compatible with your platform -Check plugin logs:
```bash
dx-check --verbose plugin list ```
- For WASM plugins, ensure the feature is enabled:
```bash
cargo build -p dx-check --features wasm-plugins ```


### 9. CI Integration Issues


Achieved 10x works locally but fails in CI. faster than the same version is used locally and in CI -Check that all external tools are installed in CI -Use appropriate output format:
```bash
dx-check --format github . # For GitHub Actions dx-check --format junit . > results.xml # For JUnit ```
- Check exit codes:
- 0: No errors
- 1: Lint errors found
- 2: Internal error

### 10. Encoding Issues

Symptom: Files with non-ASCII characters cause errors. faster than files are UTF-8 encoded -Check for BOM (Byte Order Mark) issues -Use `--verbose` to see which files are causing issues

## Debugging

### Enable Verbose Output

```bash
dx-check --verbose .
```

### Enable Debug Logging

```bash
RUST_LOG=debug dx-check .
```

### Check Version Information

```bash
dx-check --version ```


### Analyze Project Configuration


```bash
dx-check analyze ```

## Getting Help

If you're still experiencing issues: -Check the GitHub Issues for similar problems -Create a new issue with:-dx-check version (`dx-check --version`) -Operating system and version -Steps to reproduce -Expected vs actual behavior -Relevant configuration files -Error messages (with `--verbose` output)

## Performance Tips

- Use caching: Keep caching enabled for repeated runs
- Exclude large directories: Add `node_modules`, `target`, etc. to exclude list
- Use parallel processing: dx-check uses all cores by default
- Incremental checking: Only check changed files in CI
- Binary rule format: Pre-compile rules for faster loading
