#!/bin/bash
# Real-World Benchmark: DX vs Bun vs npm
# Tests actual package installation with real packages from npm

set -e

echo "════════════════════════════════════════════════════════════════"
echo "🧪 DX Package Manager Real-World Benchmark"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Create test directory
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
echo "📁 Test directory: $TEST_DIR"
echo ""

# Create package.json with popular packages
cat > package.json << 'EOF'
{
  "name": "benchmark-test",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.21",
    "express": "^4.18.2",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "axios": "^1.6.0",
    "chalk": "^5.3.0",
    "commander": "^11.0.0",
    "uuid": "^9.0.0",
    "dotenv": "^16.0.0",
    "debug": "^4.3.0"
  }
}
EOF

echo "📦 Test Package: 10 popular packages with dependencies"
echo "   (lodash, express, react, axios, chalk, commander, uuid, dotenv, debug)"
echo ""

# Function to run test and measure time
run_test() {
    local tool=$1
    local command=$2
    local cache_clean=$3
    
    echo "───────────────────────────────────────────────────────────────"
    echo "Testing: $tool"
    echo "───────────────────────────────────────────────────────────────"
    
    # Clean
    rm -rf node_modules package-lock.json bun.lockb dx.lock.json .dx 2>/dev/null || true
    eval "$cache_clean" 2>/dev/null || true
    
    # Measure time
    START=$(date +%s%3N)
    eval "$command" >/dev/null 2>&1
    END=$(date +%s%3N)
    TIME=$((END - START))
    
    # Count installed packages
    PKG_COUNT=$(find node_modules -maxdepth 1 -type d | wc -l)
    
    echo "✅ Time: ${TIME}ms"
    echo "📦 Packages: $PKG_COUNT"
    echo ""
    
    # Return time for comparison
    echo "$TIME"
}

# Test npm (baseline)
echo "════════════════════════════════════════════════════════════════"
echo "1️⃣  npm (baseline)"
echo "════════════════════════════════════════════════════════════════"
echo ""
NPM_TIME=$(run_test "npm" "npm install --silent" "npm cache clean --force")

# Test bun
echo "════════════════════════════════════════════════════════════════"
echo "2️⃣  Bun"
echo "════════════════════════════════════════════════════════════════"
echo ""
if command -v bun &> /dev/null; then
    BUN_TIME=$(run_test "bun" "bun install --silent" "rm -rf ~/.bun/install/cache")
else
    echo "⚠️  Bun not installed - skipping"
    BUN_TIME=0
fi

# Test dx
echo "════════════════════════════════════════════════════════════════"
echo "3️⃣  DX (binary-first)"
echo "════════════════════════════════════════════════════════════════"
echo ""
if command -v dx &> /dev/null; then
    DX_TIME=$(run_test "dx" "dx install" "rm -rf ~/.dx/cache")
else
    echo "⚠️  DX not installed - skipping"
    echo "💡 Build it: cargo build --release -p dx-pkg-cli"
    DX_TIME=0
fi

# Summary
echo "════════════════════════════════════════════════════════════════"
echo "📊 RESULTS SUMMARY (Cold Install)"
echo "════════════════════════════════════════════════════════════════"
echo ""
printf "%-12s %10s %15s\n" "Tool" "Time (ms)" "vs Bun"
echo "────────────────────────────────────────────────────────────────"
printf "%-12s %10d %15s\n" "npm" "$NPM_TIME" "$(echo "scale=1; $NPM_TIME / $BUN_TIME" | bc)x slower"

if [ "$BUN_TIME" -gt 0 ]; then
    printf "%-12s %10d %15s\n" "bun" "$BUN_TIME" "baseline"
fi

if [ "$DX_TIME" -gt 0 ] && [ "$BUN_TIME" -gt 0 ]; then
    SPEEDUP=$(echo "scale=1; $BUN_TIME / $DX_TIME" | bc)
    printf "%-12s %10d %15s\n" "dx" "$DX_TIME" "${SPEEDUP}x faster ⚡"
fi
echo ""

# Warm cache test
if [ "$DX_TIME" -gt 0 ]; then
    echo "════════════════════════════════════════════════════════════════"
    echo "📊 WARM CACHE TEST (DX only)"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    
    rm -rf node_modules
    START=$(date +%s%3N)
    dx install >/dev/null 2>&1
    END=$(date +%s%3N)
    DX_WARM=$((END - START))
    
    echo "✅ DX (warm): ${DX_WARM}ms"
    
    if [ "$BUN_TIME" -gt 0 ]; then
        WARM_SPEEDUP=$(echo "scale=1; $BUN_TIME / $DX_WARM" | bc)
        echo "   └─ ${WARM_SPEEDUP}x faster than Bun's cold install!"
    fi
    echo ""
fi

# Cleanup
cd -
rm -rf "$TEST_DIR"

echo "════════════════════════════════════════════════════════════════"
echo "✅ Benchmark Complete"
echo "════════════════════════════════════════════════════════════════"
