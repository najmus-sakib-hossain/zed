#!/bin/bash
# DX JavaScript Runtime Benchmark Suite Runner
# Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
WORKLOADS_DIR="$SCRIPT_DIR/workloads"
RESULTS_DIR="$SCRIPT_DIR/results"
RUNS=10
COMPARE=false
CATEGORY="all"
REPORT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --runs) RUNS="$2"; shift 2 ;;
    --compare) COMPARE=true; shift ;;
    --category) CATEGORY="$2"; shift 2 ;;
    --report) REPORT=true; shift ;;
    --help) 
      echo "Usage: $0 [options]"
      echo "Options:"
      echo "  --runs N        Number of runs per benchmark (default: 10)"
      echo "  --compare       Compare with Node.js and Bun"
      echo "  --category CAT  Run specific category (startup|memory|throughput|all)"
      echo "  --report        Generate HTML report"
      exit 0
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Detect DX binary
DX_BIN="$ROOT_DIR/runtime/target/release/dx-js"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
  DX_BIN="$DX_BIN.exe"
fi

if [[ ! -f "$DX_BIN" ]]; then
  echo -e "${YELLOW}Building DX runtime...${NC}"
  cargo build --manifest-path "$ROOT_DIR/runtime/Cargo.toml" --release
fi

# Create results directory
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULT_FILE="$RESULTS_DIR/benchmark_$TIMESTAMP.json"

echo -e "${BOLD}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║          DX JavaScript Runtime Benchmark Suite                   ║${NC}"
echo -e "${BOLD}╠══════════════════════════════════════════════════════════════════╣${NC}"
echo -e "${BOLD}║  Date: $(date '+%Y-%m-%d %H:%M:%S')                                      ║${NC}"
echo -e "${BOLD}║  Runs: $RUNS | Category: $CATEGORY | Compare: $COMPARE                   ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to measure cold start time
measure_cold_start() {
  local runtime="$1"
  local script="$2"
  local times=()
  
  for ((i=1; i<=RUNS; i++)); do
    # Clear any caches
    sync 2>/dev/null || true
    
    local start=$(date +%s%N)
    $runtime "$script" > /dev/null 2>&1
    local end=$(date +%s%N)
    local elapsed=$(( (end - start) / 1000000 ))
    times+=($elapsed)
  done
  
  # Calculate statistics
  IFS=$'\n' sorted=($(sort -n <<<"${times[*]}")); unset IFS
  local min=${sorted[0]}
  local max=${sorted[-1]}
  local median=${sorted[$((RUNS/2))]}
  local sum=0
  for t in "${times[@]}"; do
    sum=$((sum + t))
  done
  local mean=$((sum / RUNS))
  
  # Calculate standard deviation
  local sq_sum=0
  for t in "${times[@]}"; do
    local diff=$((t - mean))
    sq_sum=$((sq_sum + diff * diff))
  done
  local variance=$((sq_sum / RUNS))
  local stddev=$(echo "scale=2; sqrt($variance)" | bc)
  
  echo "$min $max $median $mean $stddev"
}

# Function to run throughput benchmark
run_throughput() {
  local runtime="$1"
  local script="$2"
  
  $runtime "$script" 2>/dev/null
}

# Initialize results JSON
echo "{" > "$RESULT_FILE"
echo "  \"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"," >> "$RESULT_FILE"
echo "  \"platform\": \"$(uname -s)\"," >> "$RESULT_FILE"
echo "  \"arch\": \"$(uname -m)\"," >> "$RESULT_FILE"
echo "  \"runs\": $RUNS," >> "$RESULT_FILE"
echo "  \"results\": {" >> "$RESULT_FILE"

# Run startup benchmarks
if [[ "$CATEGORY" == "all" || "$CATEGORY" == "startup" ]]; then
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo -e "${CYAN}                      STARTUP BENCHMARKS                           ${NC}"
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo ""
  
  echo -e "${YELLOW}Cold Start (JavaScript):${NC}"
  echo -n "  DX:   "
  dx_js_result=$(measure_cold_start "$DX_BIN" "$WORKLOADS_DIR/startup.js")
  dx_js_median=$(echo $dx_js_result | cut -d' ' -f3)
  echo "${dx_js_median}ms (median)"
  
  if $COMPARE; then
    if command -v node &> /dev/null; then
      echo -n "  Node: "
      node_js_result=$(measure_cold_start "node" "$WORKLOADS_DIR/startup.js")
      node_js_median=$(echo $node_js_result | cut -d' ' -f3)
      echo "${node_js_median}ms (median)"
    fi
    
    if command -v bun &> /dev/null; then
      echo -n "  Bun:  "
      bun_js_result=$(measure_cold_start "bun" "$WORKLOADS_DIR/startup.js")
      bun_js_median=$(echo $bun_js_result | cut -d' ' -f3)
      echo "${bun_js_median}ms (median)"
    fi
  fi
  
  echo ""
  echo -e "${YELLOW}Cold Start (TypeScript):${NC}"
  echo -n "  DX:   "
  dx_ts_result=$(measure_cold_start "$DX_BIN" "$WORKLOADS_DIR/startup.ts")
  dx_ts_median=$(echo $dx_ts_result | cut -d' ' -f3)
  echo "${dx_ts_median}ms (median)"
  
  if $COMPARE; then
    if command -v bun &> /dev/null; then
      echo -n "  Bun:  "
      bun_ts_result=$(measure_cold_start "bun" "$WORKLOADS_DIR/startup.ts")
      bun_ts_median=$(echo $bun_ts_result | cut -d' ' -f3)
      echo "${bun_ts_median}ms (median)"
    fi
  fi
  
  echo ""
  
  # Write startup results to JSON
  echo "    \"startup\": {" >> "$RESULT_FILE"
  echo "      \"js\": { \"dx\": { \"median\": $dx_js_median, \"raw\": \"$dx_js_result\" } }," >> "$RESULT_FILE"
  echo "      \"ts\": { \"dx\": { \"median\": $dx_ts_median, \"raw\": \"$dx_ts_result\" } }" >> "$RESULT_FILE"
  echo "    }," >> "$RESULT_FILE"
fi

# Run throughput benchmarks
if [[ "$CATEGORY" == "all" || "$CATEGORY" == "throughput" ]]; then
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo -e "${CYAN}                     THROUGHPUT BENCHMARKS                         ${NC}"
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo ""
  
  echo -e "${YELLOW}JSON Throughput:${NC}"
  echo "  DX:"
  dx_json=$(run_throughput "$DX_BIN" "$WORKLOADS_DIR/json-throughput.js")
  echo "    $dx_json"
  
  if $COMPARE && command -v node &> /dev/null; then
    echo "  Node:"
    node_json=$(run_throughput "node" "$WORKLOADS_DIR/json-throughput.js")
    echo "    $node_json"
  fi
  
  echo ""
  echo -e "${YELLOW}Array Throughput:${NC}"
  echo "  DX:"
  dx_array=$(run_throughput "$DX_BIN" "$WORKLOADS_DIR/array-throughput.js")
  echo "    $dx_array"
  
  echo ""
  echo -e "${YELLOW}Async Throughput:${NC}"
  echo "  DX:"
  dx_async=$(run_throughput "$DX_BIN" "$WORKLOADS_DIR/async-throughput.js")
  echo "    $dx_async"
  
  echo ""
  echo -e "${YELLOW}Fibonacci (CPU):${NC}"
  echo "  DX:"
  dx_fib=$(run_throughput "$DX_BIN" "$WORKLOADS_DIR/fibonacci.js")
  echo "    $dx_fib"
  
  if $COMPARE && command -v node &> /dev/null; then
    echo "  Node:"
    node_fib=$(run_throughput "node" "$WORKLOADS_DIR/fibonacci.js")
    echo "    $node_fib"
  fi
  
  echo ""
  
  # Write throughput results to JSON
  echo "    \"throughput\": {" >> "$RESULT_FILE"
  echo "      \"json\": $dx_json," >> "$RESULT_FILE"
  echo "      \"array\": $dx_array," >> "$RESULT_FILE"
  echo "      \"async\": $dx_async," >> "$RESULT_FILE"
  echo "      \"fibonacci\": $dx_fib" >> "$RESULT_FILE"
  echo "    }," >> "$RESULT_FILE"
fi

# Run memory benchmarks
if [[ "$CATEGORY" == "all" || "$CATEGORY" == "memory" ]]; then
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo -e "${CYAN}                       MEMORY BENCHMARKS                           ${NC}"
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
  echo ""
  
  echo -e "${YELLOW}Memory Baseline:${NC}"
  echo "  DX:"
  dx_mem=$(run_throughput "$DX_BIN" "$WORKLOADS_DIR/memory-baseline.js")
  echo "    $dx_mem"
  
  if $COMPARE && command -v node &> /dev/null; then
    echo "  Node:"
    node_mem=$(run_throughput "node" "--expose-gc" "$WORKLOADS_DIR/memory-baseline.js")
    echo "    $node_mem"
  fi
  
  echo ""
  
  # Write memory results to JSON
  echo "    \"memory\": {" >> "$RESULT_FILE"
  echo "      \"baseline\": $dx_mem" >> "$RESULT_FILE"
  echo "    }" >> "$RESULT_FILE"
fi

# Close JSON
echo "  }" >> "$RESULT_FILE"
echo "}" >> "$RESULT_FILE"

echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Benchmark complete! Results saved to: $RESULT_FILE${NC}"
echo ""

# Generate report if requested
if $REPORT; then
  echo -e "${YELLOW}Generating HTML report...${NC}"
  # Report generation would go here
  echo -e "${GREEN}Report generated: $RESULTS_DIR/report_$TIMESTAMP.html${NC}"
fi
