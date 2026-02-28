#!/usr/bin/env bash
# Benchmark comparison script

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 DX Test Runner vs Bun - Performance Comparison"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Build DX test runner
echo "📦 Building DX Test Runner..."
cargo build --release -p dx-test-cli
echo "✓ Build complete"
echo

# Count tests
TEST_COUNT=$(find tests -name "*.test.js" -exec grep -c "^test(" {} \; | awk '{s+=$1} END {print s}')
echo "Found $TEST_COUNT tests"
echo

# Run Bun benchmark
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Running Bun Test Runner..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd tests
BUN_START=$(date +%s%N)
bun test 2>&1 | tee ../bun-results.txt
BUN_END=$(date +%s%N)
BUN_TIME=$(echo "scale=2; ($BUN_END - $BUN_START) / 1000000" | bc)
cd ..

echo
echo "Bun completed in: ${BUN_TIME}ms"
echo

# Clear DX cache for fair comparison
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Running DX Test Runner (Cold Start)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

./target/release/dx-test clear > /dev/null

DX_START=$(date +%s%N)
./target/release/dx-test --verbose | tee dx-cold-results.txt
DX_END=$(date +%s%N)
DX_COLD_TIME=$(echo "scale=2; ($DX_END - $DX_START) / 1000000" | bc)

echo
echo "DX (cold) completed in: ${DX_COLD_TIME}ms"
echo

# Run again with warm cache
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Running DX Test Runner (Warm Cache)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

DX_WARM_START=$(date +%s%N)
./target/release/dx-test --verbose | tee dx-warm-results.txt
DX_WARM_END=$(date +%s%N)
DX_WARM_TIME=$(echo "scale=2; ($DX_WARM_END - $DX_WARM_START) / 1000000" | bc)

echo
echo "DX (warm) completed in: ${DX_WARM_TIME}ms"
echo

# Calculate speedups
SPEEDUP_COLD=$(echo "scale=1; $BUN_TIME / $DX_COLD_TIME" | bc)
SPEEDUP_WARM=$(echo "scale=1; $BUN_TIME / $DX_WARM_TIME" | bc)

# Results summary
echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Performance Summary ($TEST_COUNT tests)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
printf "%-20s %10s\n" "Runner" "Time"
echo "──────────────────────────────────────────────────"
printf "%-20s %10.2fms\n" "Bun" "$BUN_TIME"
printf "%-20s %10.2fms  (${SPEEDUP_COLD}x faster)\n" "DX (cold)" "$DX_COLD_TIME"
printf "%-20s %10.2fms  (${SPEEDUP_WARM}x faster)\n" "DX (warm)" "$DX_WARM_TIME"
echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check if we met our goal
if (( $(echo "$SPEEDUP_WARM >= 50" | bc -l) )); then
    echo "✅ SUCCESS! DX is ${SPEEDUP_WARM}x faster than Bun (target: 50x)"
elif (( $(echo "$SPEEDUP_WARM >= 25" | bc -l) )); then
    echo "⚠️  GOOD! DX is ${SPEEDUP_WARM}x faster than Bun (target: 50x)"
else
    echo "❌ NEEDS IMPROVEMENT: DX is only ${SPEEDUP_WARM}x faster (target: 50x)"
fi

echo
