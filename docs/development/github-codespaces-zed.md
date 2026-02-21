# Developing Zed in GitHub Codespaces (GUI via noVNC)

This runbook documents the current working setup for running the full Zed GUI in a headless GitHub Codespace.

## Setup goals

- Build Zed once and reuse incremental artifacts.
- Run a browser-visible desktop session in Codespaces.
- Start Zed automatically and recover from common VNC/noVNC failures.

## Automation files

- Devcontainer configuration: `.devcontainer/devcontainer.json`
- First-time setup: `.devcontainer/scripts/post-create.sh`
- Per-start runtime (self-healing): `.devcontainer/scripts/post-start.sh`

## What the scripts do

### `post-create.sh`

- Installs Rust (if missing).
- Runs `./script/linux` dependencies.
- Builds Zed into `/tmp/zed-target`:

```bash
CARGO_TARGET_DIR=/tmp/zed-target cargo build -p zed
```

### `post-start.sh`

- Ensures GUI runtime stack is up:
   - `Xvfb :1`
   - `x11vnc` on `5900`
   - `novnc_proxy` / `websockify` on `6080`
   - `fluxbox`
- De-duplicates stale `websockify` processes for stable noVNC behavior.
- Launches Zed with software-rendering-safe settings for headless X11:

```bash
DISPLAY=:1
WGPU_BACKEND=gl
LIBGL_ALWAYS_SOFTWARE=1
ZED_ALLOW_EMULATED_GPU=1
```

- Verifies a visible Zed window exists and restarts Zed if process exists but no mapped window is found.
- Prints and auto-opens the correct forwarded Codespaces URL for port `6080`.

## Runtime paths

To avoid filling `/workspaces`, heavy artifacts and runtime data stay in `/tmp`:

- Cargo target dir: `/tmp/zed-target`
- Zed user data dir: `/tmp/zed-user`
- XDG dirs: `/tmp/zed-xdg/{cache,config,data}`
- Logs:
   - `/tmp/zed-gui.log`
   - `/tmp/zed-xvfb.log`
   - `/tmp/zed-x11vnc.log`
   - `/tmp/zed-novnc.log`
   - `/tmp/zed-fluxbox.log`

## Fast development loop

Recommended commands:

```bash
# Fast compile feedback (no relinked binary needed for each edit)
source "$HOME/.cargo/env"
CARGO_TARGET_DIR=/tmp/zed-target cargo watch -x 'check -p zed'

# Build binary only when needed to run changed code
CARGO_TARGET_DIR=/tmp/zed-target cargo build -p zed

# Live browser preview without full rebuild cycle
# - Uses incremental compilation
# - Re-runs Zed automatically after code changes
bash .devcontainer/scripts/live-preview.sh
```

### Live preview details

`live-preview.sh` keeps noVNC running and uses `cargo watch` to execute:

```bash
cargo run -p zed -- --user-data-dir /tmp/zed-user
```

on each change, so only incremental recompilation is done for modified crates instead of a full clean build.

## Verification checklist

Run this after startup if anything looks wrong:

```bash
ss -ltnp | grep -E ':(5900|6080)'
DISPLAY=:1 xwininfo -root -tree | grep -Ei 'dev\.zed|empty project'
ps -ef | grep -E 'Xvfb :1|x11vnc|websockify|fluxbox|/tmp/zed-target/debug/zed' | grep -v grep
```

Expected:

- Port `5900` listening (`x11vnc`)
- Port `6080` listening (`websockify`)
- Zed window present in X tree (`dev.zed.Zed-Dev` / `empty project`)

## Common failure modes and fixes

### 1) noVNC page opens but says “Failed to connect to server”

Symptoms:

- noVNC UI loads but cannot connect.

Root causes:

- `x11vnc` is not listening on `5900`.
- stale `websockify` processes.

Fix:

```bash
bash .devcontainer/scripts/post-start.sh
```

### 2) Browser shows black screen

Symptoms:

- Connection works, but desktop is black / no Zed window.

Root causes:

- Zed process running without a mapped window.
- renderer path incompatible with headless Vulkan setup.

Fix:

- `post-start.sh` now enforces `WGPU_BACKEND=gl` and `LIBGL_ALWAYS_SOFTWARE=1` and restarts Zed if no window is mapped.
- Re-run:

```bash
bash .devcontainer/scripts/post-start.sh
```

### 3) `localhost` URL doesn’t work in Codespaces Simple Browser

Use forwarded URL, not plain `localhost`:

```text
https://<codespace-name>-6080.app.github.dev/vnc.html?autoconnect=true&resize=scale
```

### 4) Apt dependency install fails due third-party key/repo

Typical issue:

- broken Yarn apt key/source.

Fix:

- Disable/fix broken apt source, then rerun:

```bash
./script/linux
```

### 5) Disk fills during builds

Notes:

- `/tmp` usually has far more space than `/workspaces` in Codespaces.

Fix:

- keep `CARGO_TARGET_DIR=/tmp/zed-target`
- avoid `cargo clean` unless absolutely necessary

## “Known good” restart sequence

If the GUI session becomes unstable:

```bash
bash .devcontainer/scripts/post-start.sh
```

This script is intended to be the single recovery entrypoint.
