# Developing Zed in GitHub Codespaces

This document describes how we are currently developing and running the full Zed GUI inside a GitHub Codespace.

## What this setup does

- Installs Linux build dependencies for Zed.
- Installs Rust toolchains (via `rustup`) if missing.
- Builds the Zed editor binary (`cargo build -p zed`).
- Starts a browser desktop (via `desktop-lite` feature on port `6080`).
- Launches Zed automatically inside that desktop session.

## Where the automation lives

- Devcontainer config: `.devcontainer/devcontainer.json`
- First-time setup script: `.devcontainer/scripts/post-create.sh`
- Per-start launch script: `.devcontainer/scripts/post-start.sh`

## Normal developer flow

1. Open this repository in GitHub Codespaces.
2. Wait for container creation to finish (post-create will install deps and build Zed).
3. Open port `6080` in the Codespaces **Ports** panel (desktop-lite UI).
4. Open the forwarded URL for port `6080`.
5. Zed should already be running in the virtual desktop; if not, use a terminal in the desktop and run:
   - `/tmp/zed-target/debug/zed --user-data-dir /tmp/zed-user`

## Runtime paths used in Codespaces

To avoid disk pressure on `/workspaces`, this setup uses `/tmp` for heavy build/output and app data:

- Cargo target: `/tmp/zed-target`
- User data dir: `/tmp/zed-user`
- XDG cache/config/data: `/tmp/zed-xdg/{cache,config,data}`
- Log file: `/tmp/zed-gui.log`

## Helpful commands

From a normal Codespaces terminal:

```bash
# Rebuild Zed manually
source "$HOME/.cargo/env"
CARGO_TARGET_DIR=/tmp/zed-target cargo build -p zed

# Restart GUI launch
pkill -f '/tmp/zed-target/debug/zed --user-data-dir /tmp/zed-user' || true
XDG_CACHE_HOME=/tmp/zed-xdg/cache \
XDG_CONFIG_HOME=/tmp/zed-xdg/config \
XDG_DATA_HOME=/tmp/zed-xdg/data \
ZED_ALLOW_EMULATED_GPU=1 \
/tmp/zed-target/debug/zed --user-data-dir /tmp/zed-user
```

## Troubleshooting

- If `script/linux` fails due a third-party apt key issue (for example Yarn), remove/fix the broken apt source and rerun `script/linux`.
- If the desktop URL does not load, verify port `6080` is running and forwarded.
- If Zed exits on startup, inspect `/tmp/zed-gui.log`.
