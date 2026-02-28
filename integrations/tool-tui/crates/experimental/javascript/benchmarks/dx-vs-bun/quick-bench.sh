#!/bin/bash
# Quick DX vs Bun Benchmark Comparison

DX_PATH="../../runtime/target/release/dx-js.exe"
BUN_PATH="bun"
FIXTURES="suites/runtime/fixtures"

echo "========================================"
echo "  DX vs Bun Quick Benchmark"
echo "========================================"
echo ""

# Function to measure time
measure() {
    local name=$1
    local cmd=$2
    local runs=5
    local total=0
    
    for i in $(seq 1 $runs); do
        start=$(date +%s%N)
        eval "$cmd" > /dev/null 2>&1
        end=$(date +%s%N)
        elapsed=$(( (end - start) / 1000000 ))
        total=$((total + elapsed))
    done
    
    avg=$((total / runs))
    echo "$avg"
}

# Function to calculate speedup (integer approximation)
calc_speedup() {
    local slower=$1
    local faster=$2
    local speedup=$((slower * 100 / faster))
    local whole=$((speedup / 100))
    local frac=$((speedup % 100))
    echo "${whole}.${frac}"
}

echo "Running benchmarks (5 runs each, averaged)..."
echo ""

# Hello World - Startup Time
echo "--- Hello World (Startup Time) ---"
dx_hello=$(measure "DX Hello" "$DX_PATH $FIXTURES/hello.js")
bun_hello=$(measure "Bun Hello" "$BUN_PATH $FIXTURES/hello.js")
echo "DX:  ${dx_hello}ms"
echo "Bun: ${bun_hello}ms"
if [ $dx_hello -lt $bun_hello ]; then
    speedup=$(calc_speedup $bun_hello $dx_hello)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_hello $bun_hello)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

# Fibonacci - CPU Intensive
echo "--- Fibonacci (CPU Intensive) ---"
dx_fib=$(measure "DX Fib" "$DX_PATH $FIXTURES/fibonacci.js")
bun_fib=$(measure "Bun Fib" "$BUN_PATH $FIXTURES/fibonacci.js")
echo "DX:  ${dx_fib}ms"
echo "Bun: ${bun_fib}ms"
if [ $dx_fib -lt $bun_fib ]; then
    speedup=$(calc_speedup $bun_fib $dx_fib)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_fib $bun_fib)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

# JSON Parse
echo "--- JSON Parse ---"
dx_json=$(measure "DX JSON" "$DX_PATH $FIXTURES/json-parse.js")
bun_json=$(measure "Bun JSON" "$BUN_PATH $FIXTURES/json-parse.js")
echo "DX:  ${dx_json}ms"
echo "Bun: ${bun_json}ms"
if [ $dx_json -lt $bun_json ]; then
    speedup=$(calc_speedup $bun_json $dx_json)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_json $bun_json)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

# Memory Stress
echo "--- Memory Stress ---"
dx_mem=$(measure "DX Memory" "$DX_PATH $FIXTURES/memory-stress.js")
bun_mem=$(measure "Bun Memory" "$BUN_PATH $FIXTURES/memory-stress.js")
echo "DX:  ${dx_mem}ms"
echo "Bun: ${bun_mem}ms"
if [ $dx_mem -lt $bun_mem ]; then
    speedup=$(calc_speedup $bun_mem $dx_mem)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_mem $bun_mem)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

# Async Concurrent
echo "--- Async Concurrent ---"
dx_async=$(measure "DX Async" "$DX_PATH $FIXTURES/async-concurrent.js")
bun_async=$(measure "Bun Async" "$BUN_PATH $FIXTURES/async-concurrent.js")
echo "DX:  ${dx_async}ms"
echo "Bun: ${bun_async}ms"
if [ $dx_async -lt $bun_async ]; then
    speedup=$(calc_speedup $bun_async $dx_async)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_async $bun_async)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

# TypeScript
echo "--- TypeScript (hello.ts) ---"
dx_ts=$(measure "DX TS" "$DX_PATH $FIXTURES/hello.ts")
bun_ts=$(measure "Bun TS" "$BUN_PATH $FIXTURES/hello.ts")
echo "DX:  ${dx_ts}ms"
echo "Bun: ${bun_ts}ms"
if [ $dx_ts -lt $bun_ts ]; then
    speedup=$(calc_speedup $bun_ts $dx_ts)
    echo "Winner: DX (${speedup}x faster)"
else
    speedup=$(calc_speedup $dx_ts $bun_ts)
    echo "Winner: Bun (${speedup}x faster)"
fi
echo ""

echo "========================================"
echo "  Summary"
echo "========================================"
echo ""

# Calculate totals
dx_total=$((dx_hello + dx_fib + dx_json + dx_mem + dx_async + dx_ts))
bun_total=$((bun_hello + bun_fib + bun_json + bun_mem + bun_async + bun_ts))

echo "Total DX time:  ${dx_total}ms"
echo "Total Bun time: ${bun_total}ms"
echo ""

if [ $dx_total -lt $bun_total ]; then
    overall=$(calc_speedup $bun_total $dx_total)
    echo "Overall Winner: DX (${overall}x faster overall)"
else
    overall=$(calc_speedup $dx_total $bun_total)
    echo "Overall Winner: Bun (${overall}x faster overall)"
fi

echo ""
echo "========================================"
echo "  Detailed Results"
echo "========================================"
echo ""
echo "| Benchmark        | DX (ms) | Bun (ms) | Winner | Speedup |"
echo "|------------------|---------|----------|--------|---------|"

print_row() {
    local name=$1
    local dx=$2
    local bun=$3
    if [ $dx -lt $bun ]; then
        speedup=$(calc_speedup $bun $dx)
        printf "| %-16s | %7d | %8d | DX     | %5sx  |\n" "$name" "$dx" "$bun" "$speedup"
    else
        speedup=$(calc_speedup $dx $bun)
        printf "| %-16s | %7d | %8d | Bun    | %5sx  |\n" "$name" "$dx" "$bun" "$speedup"
    fi
}

print_row "Hello World" $dx_hello $bun_hello
print_row "Fibonacci" $dx_fib $bun_fib
print_row "JSON Parse" $dx_json $bun_json
print_row "Memory Stress" $dx_mem $bun_mem
print_row "Async Concurrent" $dx_async $bun_async
print_row "TypeScript" $dx_ts $bun_ts
echo "|------------------|---------|----------|--------|---------|"
print_row "TOTAL" $dx_total $bun_total
