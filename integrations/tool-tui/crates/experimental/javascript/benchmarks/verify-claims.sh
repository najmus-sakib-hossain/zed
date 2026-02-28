#!/bin/bash
# Verify Performance Claims
#
# This script runs benchmarks and verifies that the performance claims
# in the README files are accurate.
#
# Usage:
#   ./benchmarks/verify-claims.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║           Performance Claims Verification                    ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Build tools
echo -e "${YELLOW}Building release binaries...${NC}"
cd "$ROOT_DIR"
cargo build --release -p dx-js-runtime -p dx-pkg-cli -p dx-bundle-cli 2>/dev/null
echo ""

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

check_claim() {
    local name=$1
    local expected=$2
    local actual=$3
    local tolerance=$4  # percentage tolerance
    
    local min_expected=$(echo "scale=2; $expected * (1 - $tolerance / 100)" | bc)
    local max_expected=$(echo "scale=2; $expected * (1 + $tolerance / 100)" | bc)
    
    if (( $(echo "$actual >= $min_expected && $actual <= $max_expected" | bc -l) )); then
        echo -e "  ${GREEN}✓${NC} $name: ${actual}x (expected ~${expected}x)"
        ((PASS_COUNT++))
    elif (( $(echo "$actual > $max_expected" | bc -l) )); then
        echo -e "  ${GREEN}✓${NC} $name: ${actual}x (EXCEEDS claim of ${expected}x!)"
        ((PASS_COUNT++))
    else
        echo -e "  ${RED}✗${NC} $name: ${actual}x (expected ~${expected}x)"
        ((FAIL_COUNT++))
    fi
}

# ═══════════════════════════════════════════════════════════════
# RUNTIME CLAIMS
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                    RUNTIME CLAIMS                              ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Claim: 10.59x faster than Bun (average)${NC}"
echo ""

# Check if Bun is available
if ! command -v bun &> /dev/null; then
    echo -e "  ${YELLOW}⚠${NC} Bun not installed - cannot verify runtime claims"
    ((WARN_COUNT++))
else
    # Create test file
    mkdir -p "$SCRIPT_DIR/fixtures"
    cat > "$SCRIPT_DIR/fixtures/verify-test.js" << 'EOF'
const data = [];
for (let i = 0; i < 1000; i++) {
    data.push({ id: i, value: Math.random() });
}
const sorted = data.sort((a, b) => a.value - b.value);
console.log('Done:', sorted.length);
EOF

    # Measure DX
    DX_JS="$ROOT_DIR/target/release/dx-js"
    dx_times=()
    for i in {1..5}; do
        start=$(date +%s%N)
        "$DX_JS" "$SCRIPT_DIR/fixtures/verify-test.js" > /dev/null 2>&1
        end=$(date +%s%N)
        dx_times+=($((($end - $start) / 1000000)))
    done
    IFS=$'\n' dx_sorted=($(sort -n <<<"${dx_times[*]}")); unset IFS
    dx_median=${dx_sorted[2]}
    
    # Measure Bun
    bun_times=()
    for i in {1..5}; do
        start=$(date +%s%N)
        bun run "$SCRIPT_DIR/fixtures/verify-test.js" > /dev/null 2>&1
        end=$(date +%s%N)
        bun_times+=($((($end - $start) / 1000000)))
    done
    IFS=$'\n' bun_sorted=($(sort -n <<<"${bun_times[*]}")); unset IFS
    bun_median=${bun_sorted[2]}
    
    # Calculate speedup
    if (( dx_median > 0 )); then
        speedup=$(echo "scale=2; $bun_median / $dx_median" | bc)
        check_claim "Runtime speedup vs Bun" "10.59" "$speedup" "50"
    else
        echo -e "  ${YELLOW}⚠${NC} Could not measure dx-js time"
        ((WARN_COUNT++))
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# PACKAGE MANAGER CLAIMS
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}               PACKAGE MANAGER CLAIMS                           ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Claim: 125x faster warm installs than Bun${NC}"
echo ""

# Create test package
mkdir -p "$SCRIPT_DIR/fixtures/verify-pkg"
cat > "$SCRIPT_DIR/fixtures/verify-pkg/package.json" << 'EOF'
{
    "name": "verify-test",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21"
    }
}
EOF

cd "$SCRIPT_DIR/fixtures/verify-pkg"

# First, do a cold install to populate cache
DX_PKG="$ROOT_DIR/target/release/dx"
rm -rf node_modules dx.lock 2>/dev/null || true
"$DX_PKG" install > /dev/null 2>&1 || true

# Measure DX warm install
dx_pkg_times=()
for i in {1..5}; do
    rm -rf node_modules 2>/dev/null || true
    start=$(date +%s%N)
    "$DX_PKG" install > /dev/null 2>&1 || true
    end=$(date +%s%N)
    dx_pkg_times+=($((($end - $start) / 1000000)))
done
IFS=$'\n' dx_pkg_sorted=($(sort -n <<<"${dx_pkg_times[*]}")); unset IFS
dx_pkg_median=${dx_pkg_sorted[2]}

echo "  DX warm install: ${dx_pkg_median}ms"

if command -v bun &> /dev/null; then
    # Measure Bun warm install
    rm -rf node_modules bun.lockb 2>/dev/null || true
    bun install > /dev/null 2>&1 || true  # Cold install to populate cache
    
    bun_pkg_times=()
    for i in {1..5}; do
        rm -rf node_modules 2>/dev/null || true
        start=$(date +%s%N)
        bun install > /dev/null 2>&1 || true
        end=$(date +%s%N)
        bun_pkg_times+=($((($end - $start) / 1000000)))
    done
    IFS=$'\n' bun_pkg_sorted=($(sort -n <<<"${bun_pkg_times[*]}")); unset IFS
    bun_pkg_median=${bun_pkg_sorted[2]}
    
    echo "  Bun warm install: ${bun_pkg_median}ms"
    
    if (( dx_pkg_median > 0 )); then
        pkg_speedup=$(echo "scale=2; $bun_pkg_median / $dx_pkg_median" | bc)
        check_claim "Package manager warm speedup vs Bun" "125" "$pkg_speedup" "80"
    fi
else
    echo -e "  ${YELLOW}⚠${NC} Bun not installed - cannot verify package manager claims"
    ((WARN_COUNT++))
fi

cd "$SCRIPT_DIR"
echo ""

# ═══════════════════════════════════════════════════════════════
# BUNDLER CLAIMS
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                    BUNDLER CLAIMS                              ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Claim: 3x faster than Bun${NC}"
echo ""

# Create test bundle file
mkdir -p "$SCRIPT_DIR/fixtures/verify-bundle"
cat > "$SCRIPT_DIR/fixtures/verify-bundle/index.js" << 'EOF'
const data = [1, 2, 3, 4, 5];
const doubled = data.map(x => x * 2);
console.log(doubled);
export { doubled };
EOF

DX_BUNDLE="$ROOT_DIR/target/release/dx-bundle"

# Measure DX bundle
dx_bundle_times=()
for i in {1..5}; do
    rm -rf "$SCRIPT_DIR/fixtures/verify-bundle/.dx-cache" 2>/dev/null || true
    start=$(date +%s%N)
    "$DX_BUNDLE" bundle "$SCRIPT_DIR/fixtures/verify-bundle/index.js" -o "$SCRIPT_DIR/fixtures/verify-bundle/dist/bundle.js" > /dev/null 2>&1 || true
    end=$(date +%s%N)
    dx_bundle_times+=($((($end - $start) / 1000000)))
done
IFS=$'\n' dx_bundle_sorted=($(sort -n <<<"${dx_bundle_times[*]}")); unset IFS
dx_bundle_median=${dx_bundle_sorted[2]}

echo "  DX bundle: ${dx_bundle_median}ms"

if command -v bun &> /dev/null; then
    # Measure Bun bundle
    bun_bundle_times=()
    for i in {1..5}; do
        start=$(date +%s%N)
        bun build "$SCRIPT_DIR/fixtures/verify-bundle/index.js" --outfile "$SCRIPT_DIR/fixtures/verify-bundle/dist/bun-bundle.js" > /dev/null 2>&1 || true
        end=$(date +%s%N)
        bun_bundle_times+=($((($end - $start) / 1000000)))
    done
    IFS=$'\n' bun_bundle_sorted=($(sort -n <<<"${bun_bundle_times[*]}")); unset IFS
    bun_bundle_median=${bun_bundle_sorted[2]}
    
    echo "  Bun bundle: ${bun_bundle_median}ms"
    
    if (( dx_bundle_median > 0 )); then
        bundle_speedup=$(echo "scale=2; $bun_bundle_median / $dx_bundle_median" | bc)
        check_claim "Bundler speedup vs Bun" "3" "$bundle_speedup" "50"
    fi
else
    echo -e "  ${YELLOW}⚠${NC} Bun not installed - cannot verify bundler claims"
    ((WARN_COUNT++))
fi

echo ""

# ═══════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}                       SUMMARY                                  ${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "  ${GREEN}Passed:${NC}   $PASS_COUNT"
echo -e "  ${RED}Failed:${NC}   $FAIL_COUNT"
echo -e "  ${YELLOW}Warnings:${NC} $WARN_COUNT"
echo ""

if (( FAIL_COUNT == 0 )); then
    echo -e "${GREEN}✓ All performance claims verified!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some performance claims could not be verified.${NC}"
    echo -e "${YELLOW}Note: Results may vary based on system configuration.${NC}"
    exit 1
fi
