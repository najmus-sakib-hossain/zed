#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/zed

if ! command -v rustup >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- -y
fi

source "$HOME/.cargo/env"

# Some base images include a broken Yarn apt source key; disable that source if present.
if grep -R "dl.yarnpkg.com/debian" -n /etc/apt/sources.list /etc/apt/sources.list.d 2>/dev/null; then
  for file in /etc/apt/sources.list /etc/apt/sources.list.d/*.list; do
    [ -f "$file" ] || continue
    if grep -q "dl.yarnpkg.com/debian" "$file"; then
      sudo sed -i 's|^deb \(.*dl.yarnpkg.com/debian.*\)|# disabled by zed devcontainer: deb \1|' "$file"
    fi
  done
fi

./script/linux

export CARGO_TARGET_DIR=/tmp/zed-target
mkdir -p "$CARGO_TARGET_DIR"

cargo build -p zed

echo "Post-create completed: Zed built at $CARGO_TARGET_DIR/debug/zed"
