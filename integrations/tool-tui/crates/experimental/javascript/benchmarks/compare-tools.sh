#!/bin/bash
# DX vs npm/yarn/pnpm/Bun Comparison Benchmark
#
# This script compares DX tools against popular alternatives:
# - Runtime: dx-js vs Bun vs Node.js
# - Package Manager: dx vs npm vs yarn vs pnpm vs Bun
# - Bundler: dx-bundle vs Bun vs esbuild
#
# Usage:
#   ./benchmarks/compare-tools.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

RUNS=5
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║       DX vs Industry Tools - Comparison Benchmark            ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check available tools
echo -e "${YELLOW}Checking available tools...${NC}"
TOOLS_AVAILABLE=""

if command -v node &> /dev/null; then
    echo "  ✓ Node.js $(node --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE node"
fi

if command -v npm &> /dev/null; then
    echo "  ✓ npm $(npm --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE npm"
fi

if command -v yarn &> /dev/null; then
    echo "  ✓ yarn $(yarn --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE yarn"
fi

if command -v pnpm &> /dev/null; then
    echo "  ✓ pnpm $(pnpm --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE pnpm"
fi

if command -v bun &> /dev/null; then
    echo "  ✓ Bun $(bun --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE bun"
fi

if command -v esbuild &> /dev/null; then
    echo "  ✓ esbuild $(esbuild --version)"
    TOOLS_AVAILABLE="$TOOLS_AVAILABLE esbuild"
fi

echo ""

# Build DX tools
echo -e "${YELLOW}Building DX tools...${NC}"
cd "$ROOT_DIR"
cargo build --release -p dx-js-runtime -p dx-pkg-cli -p dx-bundle-cli 2>/dev/null
echo "  ✓ DX tools built"
echo ""

DX_JS="$ROOT_DIR/target/release/dx-js"
DX_PKG="$ROOT_DIR/target/release/dx"
DX_BUNDLE="$ROOT_DIR/target/release/dx-bundle"

# Create fixtures
mkdir -p "$FIXTURES_DIR"

# Test file for runtime comparison
cat > "$FIXTURES_DIR/runtime-test.js" << 'EOF'
// Runtime benchmark - compute-intensive task
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

const start = Date.now();
const result = fibonacci(35);
const elapsed = Date.now() - start;
console.log(`fibonacci(35) = ${result} in ${elapsed}ms`);
EOF

# Package.json for package manager comparison
mkdir -p "$FIXTURES_DIR/pkg-compare"
cat > "$FIXTURES_DIR/pkg-compare/package.json" << 'EOF'
{
    "name": "benchmark-compare",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21",
        "axios": "^1.6.0",
        "express": "^4.18.0"
    }
}
EOF

# Helper function
measure_ms() {
    local start=$(date +%s%N)
    "$@" > /dev/null 2>&1
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

benchmark() {
    local name=$1
    shift
    local times=()
    
    for ((i=1; i<=RUNS; i++)); do
        local ms=$(measure_ms "$@")
        times+=($ms)
    done
    
    IFS=$'\n' sorted=($(sort -n <<<"${times[*]}")); unset IFS
    echo ${sorted[$((RUNS/2))]}
}

# ═══════════════════════════════════════════════════════════════
# RUNTIME COMPARISON
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                    RUNTIME COMPARISON                          ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BOLD}Test: fibonacci(35) computation${NC}"
echo ""

printf "%-20s %10s %10s\n" "Tool" "Time (ms)" "Speedup"
printf "%-20s %10s %10s\n" "----" "---------" "-------"

# DX Runtime
dx_time=$(benchmark "dx-js" "$DX_JS" "$FIXTURES_DIR/runtime-test.js")
printf "%-20s %10s %10s\n" "dx-js" "${dx_time}ms" "baseline"

# Node.js
if [[ $TOOLS_AVAILABLE == *"node"* ]]; then
    node_time=$(benchmark "node" node "$FIXTURES_DIR/runtime-test.js")
    speedup=$(echo "scale=2; $node_time / $dx_time" | bc)
    printf "%-20s %10s %10s\n" "Node.js" "${node_time}ms" "${speedup}x slower"
fi

# Bun
if [[ $TOOLS_AVAILABLE == *"bun"* ]]; then
    bun_time=$(benchmark "bun" bun run "$FIXTURES_DIR/runtime-test.js")
    if (( bun_time > dx_time )); then
        speedup=$(echo "scale=2; $bun_time / $dx_time" | bc)
        printf "%-20s %10s %10s\n" "Bun" "${bun_time}ms" "${speedup}x slower"
    else
        speedup=$(echo "scale=2; $dx_time / $bun_time" | bc)
        printf "%-20s %10s %10s\n" "Bun" "${bun_time}ms" "${speedup}x faster"
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# PACKAGE MANAGER COMPARISON
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}               PACKAGE MANAGER COMPARISON                       ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BOLD}Test: Install lodash + axios + express (cold)${NC}"
echo ""

cd "$FIXTURES_DIR/pkg-compare"

printf "%-20s %10s %10s\n" "Tool" "Time (ms)" "Speedup"
printf "%-20s %10s %10s\n" "----" "---------" "-------"

# DX Package Manager
dx_pkg_time=$(benchmark "dx" bash -c "rm -rf node_modules dx.lock 2>/dev/null; $DX_PKG install")
printf "%-20s %10s %10s\n" "dx" "${dx_pkg_time}ms" "baseline"

# npm
if [[ $TOOLS_AVAILABLE == *"npm"* ]]; then
    npm_time=$(benchmark "npm" bash -c "rm -rf node_modules package-lock.json 2>/dev/null; npm install --silent")
    speedup=$(echo "scale=2; $npm_time / $dx_pkg_time" | bc)
    printf "%-20s %10s %10s\n" "npm" "${npm_time}ms" "${speedup}x slower"
fi

# yarn
if [[ $TOOLS_AVAILABLE == *"yarn"* ]]; then
    yarn_time=$(benchmark "yarn" bash -c "rm -rf node_modules yarn.lock 2>/dev/null; yarn install --silent")
    speedup=$(echo "scale=2; $yarn_time / $dx_pkg_time" | bc)
    printf "%-20s %10s %10s\n" "yarn" "${yarn_time}ms" "${speedup}x slower"
fi

# pnpm
if [[ $TOOLS_AVAILABLE == *"pnpm"* ]]; then
    pnpm_time=$(benchmark "pnpm" bash -c "rm -rf node_modules pnpm-lock.yaml 2>/dev/null; pnpm install --silent")
    speedup=$(echo "scale=2; $pnpm_time / $dx_pkg_time" | bc)
    printf "%-20s %10s %10s\n" "pnpm" "${pnpm_time}ms" "${speedup}x slower"
fi

# Bun
if [[ $TOOLS_AVAILABLE == *"bun"* ]]; then
    bun_pkg_time=$(benchmark "bun" bash -c "rm -rf node_modules bun.lockb 2>/dev/null; bun install")
    if (( bun_pkg_time > dx_pkg_time )); then
        speedup=$(echo "scale=2; $bun_pkg_time / $dx_pkg_time" | bc)
        printf "%-20s %10s %10s\n" "Bun" "${bun_pkg_time}ms" "${speedup}x slower"
    else
        speedup=$(echo "scale=2; $dx_pkg_time / $bun_pkg_time" | bc)
        printf "%-20s %10s %10s\n" "Bun" "${bun_pkg_time}ms" "${speedup}x faster"
    fi
fi

cd "$SCRIPT_DIR"
echo ""

# ═══════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                       SUMMARY                                  ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}DX JavaScript Tooling Performance:${NC}"
echo ""
echo "  Runtime:         ${dx_time}ms (fibonacci benchmark)"
echo "  Package Manager: ${dx_pkg_time}ms (cold install)"
echo ""
echo -e "${YELLOW}Note: Results may vary based on system load, network conditions,${NC}"
echo -e "${YELLOW}and cache state. Run multiple times for accurate comparison.${NC}"
echo ""
