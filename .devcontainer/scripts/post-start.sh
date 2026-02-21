#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/zed

if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

export CARGO_TARGET_DIR=/tmp/zed-target
mkdir -p /tmp/zed-xdg/cache /tmp/zed-xdg/config /tmp/zed-xdg/data /tmp/zed-user

DISPLAY_NUMBER=":1"

ensure_vnc_stack() {
  if ! pgrep -f "Xvfb ${DISPLAY_NUMBER}" >/dev/null 2>&1; then
    nohup Xvfb "${DISPLAY_NUMBER}" -screen 0 1440x900x24 -ac >/tmp/zed-xvfb.log 2>&1 &
    sleep 1
  fi

  if ! ss -ltn | grep -q ':5900 '; then
    x11vnc -display "${DISPLAY_NUMBER}" -forever -shared -rfbport 5900 -localhost -nopw -bg -o /tmp/zed-x11vnc.log
    sleep 1
  fi

  if ! ss -ltn | grep -q ':6080 '; then
    nohup /usr/share/novnc/utils/novnc_proxy --listen 6080 --vnc localhost:5900 >/tmp/zed-novnc.log 2>&1 &
    sleep 1
  fi
}

ensure_vnc_stack

if [ ! -x "$CARGO_TARGET_DIR/debug/zed" ]; then
  cargo build -p zed
fi

if pgrep -f "$CARGO_TARGET_DIR/debug/zed --user-data-dir /tmp/zed-user" >/dev/null 2>&1; then
  echo "Zed is already running in this Codespace."
else
  nohup env \
    DISPLAY="${DISPLAY_NUMBER}" \
    XDG_CACHE_HOME=/tmp/zed-xdg/cache \
    XDG_CONFIG_HOME=/tmp/zed-xdg/config \
    XDG_DATA_HOME=/tmp/zed-xdg/data \
    ZED_ALLOW_EMULATED_GPU=1 \
    "$CARGO_TARGET_DIR/debug/zed" --user-data-dir /tmp/zed-user \
    >/tmp/zed-gui.log 2>&1 &
  echo "Started Zed GUI. Logs: /tmp/zed-gui.log"
fi

if [ -n "${CODESPACE_NAME:-}" ] && [ -n "${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN:-}" ]; then
  desktop_url="https://${CODESPACE_NAME}-6080.${GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN}/vnc.html?autoconnect=true&resize=scale"
  echo "Open desktop UI: ${desktop_url}"

  if [ -n "${BROWSER:-}" ] && command -v "$BROWSER" >/dev/null 2>&1; then
    "$BROWSER" "$desktop_url" >/dev/null 2>&1 || true
    echo "Attempted browser auto-open with \$BROWSER."
  fi
fi
