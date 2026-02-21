#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/zed

if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

if ! command -v cargo-watch >/dev/null 2>&1; then
  echo "Installing cargo-watch (one-time)..."
  cargo install cargo-watch
fi

export CARGO_TARGET_DIR=/tmp/zed-target
export DISPLAY=:1
export WGPU_BACKEND=gl
export LIBGL_ALWAYS_SOFTWARE=1
export ZED_ALLOW_EMULATED_GPU=1
export XDG_CACHE_HOME=/tmp/zed-xdg/cache
export XDG_CONFIG_HOME=/tmp/zed-xdg/config
export XDG_DATA_HOME=/tmp/zed-xdg/data

mkdir -p /tmp/zed-xdg/cache /tmp/zed-xdg/config /tmp/zed-xdg/data /tmp/zed-user

bash .devcontainer/scripts/post-start.sh

pkill -f '/tmp/zed-target/debug/zed --user-data-dir /tmp/zed-user' || true

echo "Starting live preview loop (incremental compile + auto-restart)..."
echo "Stop with Ctrl+C"

cargo watch \
  -w crates \
  -w assets \
  -w extensions \
  -w Cargo.toml \
  -w Cargo.lock \
  -i /tmp \
  -x 'run -p zed -- --user-data-dir /tmp/zed-user'
