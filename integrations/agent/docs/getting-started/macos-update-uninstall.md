# macOS Update and Uninstall Guide

This page documents supported update and uninstall procedures for DX on macOS (OS X).

Last verified: **February 22, 2026**.

## 1) Check current install method

```bash
which dx
dx --version
```

Typical locations:

- Homebrew: `/opt/homebrew/bin/dx` (Apple Silicon) or `/usr/local/bin/dx` (Intel)
- Cargo/bootstrap/manual: `~/.cargo/bin/dx`

If both exist, your shell `PATH` order decides which one runs.

## 2) Update on macOS

### A) Homebrew install

```bash
brew update
brew upgrade dx
dx --version
```

### B) Clone + bootstrap install

From your local repository checkout:

```bash
git pull --ff-only
./bootstrap.sh --prefer-prebuilt
dx --version
```

If you want source-only update:

```bash
git pull --ff-only
cargo install --path . --force --locked
dx --version
```

### C) Manual prebuilt binary install

Re-run your download/install flow with the latest release asset, then verify:

```bash
dx --version
```

## 3) Uninstall on macOS

### A) Stop and remove background service first

This prevents the daemon from continuing to run after binary removal.

```bash
dx service stop || true
dx service uninstall || true
```

Service artifacts removed by `service uninstall`:

- `~/Library/LaunchAgents/com.dx.daemon.plist`

### B) Remove the binary by install method

Homebrew:

```bash
brew uninstall dx
```

Cargo/bootstrap/manual (`~/.cargo/bin/dx`):

```bash
cargo uninstall dx || true
rm -f ~/.cargo/bin/dx
```

### C) Optional: remove local runtime data

Only run this if you want a full cleanup of config, auth profiles, logs, and workspace state.

```bash
rm -rf ~/.dx
```

## 4) Verify uninstall completed

```bash
command -v dx || echo "dx binary not found"
pgrep -fl dx || echo "No running dx process"
```

If `pgrep` still finds a process, stop it manually and re-check:

```bash
pkill -f dx
```

## Related docs

- [One-Click Bootstrap](../one-click-bootstrap.md)
- [Commands Reference](../commands-reference.md)
- [Troubleshooting](../troubleshooting.md)
