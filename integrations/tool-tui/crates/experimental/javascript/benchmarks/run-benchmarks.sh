#!/bin/bash
# DX JavaScript Tooling Benchmark Suite
# Cross-platform benchmark script (Bash)
#
# Usage:
#   ./benchmarks/run-benchmarks.sh [-r RUNS] [-c] [-o OUTPUT]
#
# Options:
#   -r RUNS    Number of benchmark runs (default: 5)
#   -c         Compare with npm/bun
#   -o OUTPUT  Output file for results (default: benchmark-results.json)
#
# Requirements:
#   - Rust toolchain (cargo)
#   - Node.js (for npm comparison)
#   - Bun (optional, for comparison)

set -e

# Default values
RUNS=5
COMPARE=false
OUTPUT="benchmark-results.json"

# Parse arguments
while getopts "r:co:" opt; do
    case $opt in
        r) RUNS=$OPTARG ;;
        c) COMPARE=true ;;
        o) OUTPUT=$OPTARG ;;
        *) echo "Usage: $0 [-r RUNS] [-c] [-o OUTPUT]"; exit 1 ;;
    esac
done

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║         DX JavaScript Tooling Benchmark Suite                ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"

# Create fixtures directory
mkdir -p "$FIXTURES_DIR"

# Helper function to measure execution time in milliseconds
measure_ms() {
    local start=$(date +%s%N)
    "$@" > /dev/null 2>&1
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

# Helper function to run benchmark and get statistics
run_benchmark() {
    local name=$1
    shift
    local cmd=("$@")
    local times=()
    
    echo -n "  Running $name..."
    
    for ((i=1; i<=RUNS; i++)); do
        local ms=$(measure_ms "${cmd[@]}")
        times+=($ms)
        echo -n "."
    done
    
    # Sort times
    IFS=$'\n' sorted=($(sort -n <<<"${times[*]}")); unset IFS
    
    # Calculate statistics
    local min=${sorted[0]}
    local max=${sorted[-1]}
    local median=${sorted[$((RUNS/2))]}
    local sum=0
    for t in "${times[@]}"; do
        sum=$((sum + t))
    done
    local mean=$((sum / RUNS))
    
    echo -e " ${GREEN}${median}ms (median)${NC}"
    
    # Store results (global variables for simplicity)
    eval "${name}_min=$min"
    eval "${name}_max=$max"
    eval "${name}_median=$median"
    eval "${name}_mean=$mean"
}

# Build release binaries
echo -e "${YELLOW}Building release binaries...${NC}"
cd "$ROOT_DIR"

echo "  Building dx-js-runtime..."
cargo build --release -p dx-js-runtime 2>/dev/null

echo "  Building dx-pkg-cli..."
cargo build --release -p dx-pkg-cli 2>/dev/null

echo "  Building dx-bundle-cli..."
cargo build --release -p dx-bundle-cli 2>/dev/null

echo ""

# Create test fixtures
cat > "$FIXTURES_DIR/test.js" << 'EOF'
// Benchmark test file
const data = [];
for (let i = 0; i < 1000; i++) {
    data.push({ id: i, value: Math.random() });
}
const sorted = data.sort((a, b) => a.value - b.value);
console.log('Processed', sorted.length, 'items');
EOF

cat > "$FIXTURES_DIR/test.ts" << 'EOF'
// TypeScript benchmark test file
interface Item {
    id: number;
    value: number;
}

const data: Item[] = [];
for (let i = 0; i < 1000; i++) {
    data.push({ id: i, value: Math.random() });
}
const sorted = data.sort((a, b) => a.value - b.value);
console.log('Processed', sorted.length, 'items');
EOF

mkdir -p "$FIXTURES_DIR/pkg-test"
cat > "$FIXTURES_DIR/pkg-test/package.json" << 'EOF'
{
    "name": "benchmark-test",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21"
    }
}
EOF

# Determine binary extension
DX_JS="$ROOT_DIR/target/release/dx-js"
DX_PKG="$ROOT_DIR/target/release/dx"
DX_BUNDLE="$ROOT_DIR/target/release/dx-bundle"

echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                    RUNTIME BENCHMARKS                          ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Runtime benchmarks
run_benchmark "runtime_js" "$DX_JS" "$FIXTURES_DIR/test.js"
run_benchmark "runtime_ts" "$DX_JS" "$FIXTURES_DIR/test.ts"

# Compare with Bun if available
if $COMPARE && command -v bun &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Comparison with Bun:${NC}"
    run_benchmark "bun_js" bun run "$FIXTURES_DIR/test.js"
    run_benchmark "bun_ts" bun run "$FIXTURES_DIR/test.ts"
    
    js_speedup=$(echo "scale=2; $bun_js_median / $runtime_js_median" | bc)
    ts_speedup=$(echo "scale=2; $bun_ts_median / $runtime_ts_median" | bc)
    
    echo ""
    echo -e "  ${GREEN}JavaScript speedup: ${js_speedup}x faster than Bun${NC}"
    echo -e "  ${GREEN}TypeScript speedup: ${ts_speedup}x faster than Bun${NC}"
fi

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                 PACKAGE MANAGER BENCHMARKS                     ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

cd "$FIXTURES_DIR/pkg-test"

# Clean up
rm -rf node_modules dx.lock 2>/dev/null || true

# Package manager benchmarks
run_benchmark "pkg_cold" bash -c "rm -rf node_modules dx.lock 2>/dev/null; $DX_PKG install"
run_benchmark "pkg_warm" bash -c "rm -rf node_modules 2>/dev/null; $DX_PKG install"

# Compare with npm if available
if $COMPARE && command -v npm &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Comparison with npm:${NC}"
    
    # Fewer runs for npm (it's slow)
    NPM_RUNS=$((RUNS < 3 ? RUNS : 3))
    RUNS=$NPM_RUNS run_benchmark "npm_cold" bash -c "rm -rf node_modules package-lock.json 2>/dev/null; npm install --silent"
    
    npm_speedup=$(echo "scale=2; $npm_cold_median / $pkg_cold_median" | bc)
    echo ""
    echo -e "  ${GREEN}Cold install speedup: ${npm_speedup}x faster than npm${NC}"
fi

cd "$SCRIPT_DIR"

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                    BUNDLER BENCHMARKS                          ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create bundle test file
cat > "$FIXTURES_DIR/bundle-test.js" << 'EOF'
import { sortBy } from 'lodash';

const data = [
    { name: 'Alice', age: 30 },
    { name: 'Bob', age: 25 },
    { name: 'Charlie', age: 35 }
];

const sorted = sortBy(data, 'age');
console.log(sorted);
EOF

mkdir -p "$FIXTURES_DIR/dist"

# Bundler benchmarks
run_benchmark "bundle_cold" bash -c "rm -rf $FIXTURES_DIR/.dx-cache 2>/dev/null; $DX_BUNDLE bundle $FIXTURES_DIR/bundle-test.js -o $FIXTURES_DIR/dist/bundle.js"
run_benchmark "bundle_warm" "$DX_BUNDLE" bundle "$FIXTURES_DIR/bundle-test.js" -o "$FIXTURES_DIR/dist/bundle.js"

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                       SUMMARY                                  ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Runtime (median):${NC}"
echo "  JavaScript: ${runtime_js_median}ms"
echo "  TypeScript: ${runtime_ts_median}ms"
echo ""

echo -e "${YELLOW}Package Manager (median):${NC}"
echo "  Cold install: ${pkg_cold_median}ms"
echo "  Warm install: ${pkg_warm_median}ms"
echo ""

echo -e "${YELLOW}Bundler (median):${NC}"
echo "  Cold bundle: ${bundle_cold_median}ms"
echo "  Warm bundle: ${bundle_warm_median}ms"
echo ""

# Save results to JSON
cat > "$SCRIPT_DIR/$OUTPUT" << EOF
{
    "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "platform": "$(uname -s)",
    "runs": $RUNS,
    "results": {
        "runtime_js": {
            "min": $runtime_js_min,
            "max": $runtime_js_max,
            "median": $runtime_js_median,
            "mean": $runtime_js_mean
        },
        "runtime_ts": {
            "min": $runtime_ts_min,
            "max": $runtime_ts_max,
            "median": $runtime_ts_median,
            "mean": $runtime_ts_mean
        },
        "pkg_cold": {
            "min": $pkg_cold_min,
            "max": $pkg_cold_max,
            "median": $pkg_cold_median,
            "mean": $pkg_cold_mean
        },
        "pkg_warm": {
            "min": $pkg_warm_min,
            "max": $pkg_warm_max,
            "median": $pkg_warm_median,
            "mean": $pkg_warm_mean
        },
        "bundle_cold": {
            "min": $bundle_cold_min,
            "max": $bundle_cold_max,
            "median": $bundle_cold_median,
            "mean": $bundle_cold_mean
        },
        "bundle_warm": {
            "min": $bundle_warm_min,
            "max": $bundle_warm_max,
            "median": $bundle_warm_median,
            "mean": $bundle_warm_mean
        }
    }
}
EOF

echo -e "${GREEN}Results saved to: $OUTPUT${NC}"
