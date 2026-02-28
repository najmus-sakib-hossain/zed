#!/bin/bash
#
# DX vs Bun Professional Benchmark Suite
# ======================================
# Comprehensive performance comparison between DX and Bun runtimes
#

set -e

# Configuration
RUNS=10
DX_BIN="./runtime/target/release/dx-js.exe"
BUN_BIN="/c/Users/Computer/.bun/bin/bun"
FIXTURES_DIR="benchmarks/dx-vs-bun/suites/runtime/fixtures"
RESULTS_DIR="benchmarks/dx-vs-bun/results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Print header
print_header() {
    echo ""
    echo -e "${BOLD}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}║          DX vs Bun - Professional Benchmark Suite                ║${NC}"
    echo -e "${BOLD}╠══════════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${BOLD}║  Date: $(date '+%Y-%m-%d %H:%M:%S')                                      ║${NC}"
    echo -e "${BOLD}║  Runs per test: ${RUNS}                                                 ║${NC}"
    echo -e "${BOLD}╚══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Get version info
get_versions() {
    echo -e "${CYAN}Runtime Versions:${NC}"
    echo -n "  DX:  "
    $DX_BIN --version 2>/dev/null || echo "unknown"
    echo -n "  Bun: "
    $BUN_BIN --version 2>/dev/null || echo "unknown"
    echo ""
}

# Run benchmark and return average time in ms
run_benchmark() {
    local cmd="$1"
    local file="$2"
    local total=0
    local times=()
    
    for i in $(seq 1 $RUNS); do
        # Use time command and extract real time
        local start=$(date +%s%N)
        $cmd "$file" > /dev/null 2>&1
        local end=$(date +%s%N)
        local elapsed=$(( (end - start) / 1000000 ))
        times+=($elapsed)
        total=$((total + elapsed))
    done
    
    # Calculate average
    local avg=$((total / RUNS))
    
    # Calculate min and max
    local min=${times[0]}
    local max=${times[0]}
    for t in "${times[@]}"; do
        ((t < min)) && min=$t
        ((t > max)) && max=$t
    done
    
    echo "$avg $min $max"
}

# Run a single test
run_test() {
    local name="$1"
    local file="$2"
    
    echo -e "${YELLOW}Testing: ${name}${NC}"
    
    # Run DX benchmark
    echo -n "  DX:  "
    local dx_result=$(run_benchmark "$DX_BIN" "$file")
    local dx_avg=$(echo $dx_result | cut -d' ' -f1)
    local dx_min=$(echo $dx_result | cut -d' ' -f2)
    local dx_max=$(echo $dx_result | cut -d' ' -f3)
    echo "${dx_avg}ms (min: ${dx_min}ms, max: ${dx_max}ms)"
    
    # Run Bun benchmark
    echo -n "  Bun: "
    local bun_result=$(run_benchmark "$BUN_BIN" "$file")
    local bun_avg=$(echo $bun_result | cut -d' ' -f1)
    local bun_min=$(echo $bun_result | cut -d' ' -f2)
    local bun_max=$(echo $bun_result | cut -d' ' -f3)
    echo "${bun_avg}ms (min: ${bun_min}ms, max: ${bun_max}ms)"
    
    # Calculate speedup
    if [ $dx_avg -gt 0 ]; then
        local speedup=$(echo "scale=2; $bun_avg / $dx_avg" | bc)
        if (( $(echo "$speedup > 1" | bc -l) )); then
            echo -e "  ${GREEN}→ DX is ${speedup}x faster${NC}"
        else
            local inverse=$(echo "scale=2; $dx_avg / $bun_avg" | bc)
            echo -e "  ${RED}→ Bun is ${inverse}x faster${NC}"
        fi
    fi
    
    echo ""
    
    # Store results
    echo "$name,$dx_avg,$dx_min,$dx_max,$bun_avg,$bun_min,$bun_max" >> "$RESULTS_DIR/benchmark_raw.csv"
}

# Main benchmark suite
main() {
    print_header
    get_versions
    
    # Create results directory
    mkdir -p "$RESULTS_DIR"
    
    # Initialize CSV
    echo "test,dx_avg,dx_min,dx_max,bun_avg,bun_min,bun_max" > "$RESULTS_DIR/benchmark_raw.csv"
    
    echo -e "${BOLD}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}                        BENCHMARK RESULTS                          ${NC}"
    echo -e "${BOLD}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""
    
    # Run all tests
    run_test "Hello World (JS)" "$FIXTURES_DIR/hello.js"
    run_test "Hello World (TS)" "$FIXTURES_DIR/hello.ts"
    run_test "Fibonacci (CPU)" "$FIXTURES_DIR/fibonacci.js"
    run_test "JSON Parse" "$FIXTURES_DIR/json-parse.js"
    run_test "Memory Stress" "$FIXTURES_DIR/memory-stress.js"
    run_test "Async Concurrent" "$FIXTURES_DIR/async-concurrent.js"
    
    echo -e "${BOLD}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}Benchmark complete! Results saved to $RESULTS_DIR${NC}"
}

main "$@"
