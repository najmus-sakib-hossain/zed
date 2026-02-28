
# Build Setup Guide for Windows

## Current Status

✅ Rust code compiles successfully! The only build error occurs when the binary is already running (file access denied).

## Quick Build Commands

### Release Build (Recommended)

```bash


# Stop any running forge-cli.exe first


taskkill /F /IM forge-cli.exe 2>nul


# Build release version


cargo build --release ```


### Debug Build


```bash
cargo build ```

### Build Output

- Release binary: `target/release/forge-cli.exe`
- Debug binary: `target/debug/forge-cli.exe`

## Build Warnings (Non-Critical)

The build shows some warnings that don't prevent compilation: -Unused variables in `detector.rs` (can be ignored or prefixed with `_`) -Unused imports in `cli.rs` (cosmetic) To auto-fix some warnings:
```bash
cargo fix --bin "forge-cli"
```

## If You Encounter libgit2 Build Errors

If you see errors about `libgit2-sys` or missing `pkg-config`, you have two options:

### Option 1: Kill Running Process (Easiest)

The most common issue is the binary being locked because it's running:
```bash
taskkill /F /IM forge-cli.exe cargo build --release ```


### Option 2: Install Build Dependencies (If Needed)


Only needed if you see actual compilation errors (not just file locks): Using Scoop (Recommended):
```powershell

# Install scoop if not already installed

Set-ExecutionPolicy RemoteSigned -Scope CurrentUser irm get.scoop.sh | iex

# Install pkg-config

scoop install pkg-config ```
Using Chocolatey:
```powershell


# Install chocolatey if not already installed


Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072 iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))


# Install pkg-config


choco install pkgconfiglite ```


## Verifying Build Success


After building, verify the binary works:
```bash

# Check version

./target/release/forge-cli.exe --version

# Run watcher

./target/release/forge-cli.exe watch ```

## VS Code Extension Build

The TypeScript extension builds separately:
```bash
cd vscode-forge npm install npm run compile ```


## Common Issues & Solutions



### Issue: "Access is denied" during build


Solution: The binary is running. Kill it first:
```bash
taskkill /F /IM forge-cli.exe ```

### Issue: Build warnings about unused variables

Solution: These are cosmetic. To fix:
```bash
cargo fix --bin "forge-cli"
cargo clippy --fix ```


### Issue: Extension not updating after build


Solution: Reload VS Code Extension Development Host: -Press `Ctrl+Shift+P` -Run "Developer: Reload Window"


## Environment Info


Your setup: -OS: Windows -Shell: bash.exe (Git Bash or similar) -Rust toolchain: stable-x86_64-pc-windows-msvc -VS Build Tools: Installed (MSVC compiler found)


## Performance Note


The Rust watcher now includes: -Per-file log throttling (5s window) -Daily log rotation -Debug-level verbose logs (set `RUST_LOG=debug` to see all) -Lazy cleanup to prevent memory growth Default behavior: Only shows warnings and errors in logs, keeping `forge.log` small.
