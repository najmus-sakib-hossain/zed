#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/zed

if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

export CARGO_TARGET_DIR=/tmp/zed-target
mkdir -p /tmp/zed-xdg/cache /tmp/zed-xdg/config /tmp/zed-xdg/data /tmp/zed-user

if [ ! -x "$CARGO_TARGET_DIR/debug/zed" ]; then
  cargo build -p zed
fi

if pgrep -f "$CARGO_TARGET_DIR/debug/zed --user-data-dir /tmp/zed-user" >/dev/null 2>&1; then
  echo "Zed is already running in this Codespace."
else
  nohup env \
    DISPLAY="${DISPLAY:-:1}" \
    XDG_CACHE_HOME=/tmp/zed-xdg/cache \
    XDG_CONFIG_HOME=/tmp/zed-xdg/config \
    XDG_DATA_HOME=/tmp/zed-xdg/data \
    ZED_ALLOW_EMULATED_GPU=1 \
    "$CARGO_TARGET_DIR/debug/zed" --user-data-dir /tmp/zed-user \
    >/tmp/zed-gui.log 2>&1 &
  echo "Started Zed GUI. Logs: /tmp/zed-gui.log"
fi

if [ -n "${CODESPACE_NAME:-}" ] && [ -n "${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN:-}" ]; then
  echo "Open desktop UI: https://${CODESPACE_NAME}-6080.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}/"
fi
