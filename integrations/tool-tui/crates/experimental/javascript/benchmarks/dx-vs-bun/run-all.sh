#!/bin/bash
# DX vs Bun Benchmark Suite - Main Runner (Unix)
# Orchestrates all benchmark suites and generates reports

set -e

# Default parameters
RUNS=10
WARMUP=3
SKIP_BUILD=false
SUITE=""
OUTPUT="results"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--runs)
            RUNS="$2"
            shift 2
            ;;
        -w|--warmup)
            WARMUP="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        -s|--suite)
            SUITE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  -r, --runs N       Number of runs per benchmark (default: 10)"
            echo "  -w, --warmup N     Warmup runs excluded from results (default: 3)"
            echo "  --skip-build       Skip building DX tools"
            echo "  -s, --suite NAME   Run specific suite only"
            echo "  -o, --output DIR   Output directory (default: results)"
            echo "  -h, --help         Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Source libraries
source "$SCRIPT_DIR/lib/stats.sh" 2>/dev/null || true

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo_color() {
    local color=$1
    shift
    echo -e "${color}$*${NC}"
}

# Check system load before benchmarks
check_system_load() {
    local threshold=${1:-50}
    local cpu_usage=0
    
    if [[ -f /proc/stat ]]; then
        # Linux: Calculate CPU usage from /proc/stat
        local cpu1=($(head -1 /proc/stat | awk '{print $2,$3,$4,$5}'))
        sleep 1
        local cpu2=($(head -1 /proc/stat | awk '{print $2,$3,$4,$5}'))
        
        local idle1=${cpu1[3]}
        local idle2=${cpu2[3]}
        local total1=$((${cpu1[0]} + ${cpu1[1]} + ${cpu1[2]} + ${cpu1[3]}))
        local total2=$((${cpu2[0]} + ${cpu2[1]} + ${cpu2[2]} + ${cpu2[3]}))
        
        local idle_diff=$((idle2 - idle1))
        local total_diff=$((total2 - total1))
        
        if [[ $total_diff -gt 0 ]]; then
            cpu_usage=$((100 * (total_diff - idle_diff) / total_diff))
        fi
    elif command -v top &> /dev/null; then
        # macOS/BSD: Use top
        cpu_usage=$(top -l 1 | grep "CPU usage" | awk '{print $3}' | tr -d '%' | cut -d. -f1 2>/dev/null || echo "0")
    fi
    
    if [[ $cpu_usage -gt $threshold ]]; then
        echo_color "$YELLOW" "Warning: High CPU usage detected (${cpu_usage}%). Results may be affected."
        echo_color "$YELLOW" "Consider closing other applications for accurate benchmarks."
        return 1
    fi
    
    return 0
}


# Get DX path
get_dx_path() {
    local paths=(
        "$WORKSPACE_ROOT/runtime/target/release/dx-js"
        "$WORKSPACE_ROOT/target/release/dx-js"
    )
    
    for path in "${paths[@]}"; do
        if [[ -x "$path" ]]; then
            echo "$path"
            return 0
        fi
    done
    
    return 1
}

# Get Bun path
get_bun_path() {
    if command -v bun &> /dev/null; then
        command -v bun
        return 0
    fi
    return 1
}

# Check prerequisites
check_prerequisites() {
    local errors=()
    
    if ! DX_PATH=$(get_dx_path); then
        errors+=("DX runtime not found. Build with: cargo build --release -p dx-js-runtime")
    fi
    
    if ! BUN_PATH=$(get_bun_path); then
        errors+=("Bun not installed. Install from: https://bun.sh")
    fi
    
    if [[ ${#errors[@]} -gt 0 ]]; then
        echo_color "$YELLOW" "Prerequisites not met:"
        for err in "${errors[@]}"; do
            echo_color "$YELLOW" "  - $err"
        done
        return 1
    fi
    
    echo_color "$GREEN" "Prerequisites check passed"
    echo "  DX: $DX_PATH"
    echo "  Bun: $BUN_PATH"
    return 0
}

# Build DX tools
build_dx_tools() {
    echo_color "$CYAN" "Building DX tools in release mode..."
    
    pushd "$WORKSPACE_ROOT" > /dev/null
    
    echo "  Building dx-js-runtime..."
    if ! cargo build --release -p dx-js-runtime 2>&1; then
        echo_color "$YELLOW" "Failed to build dx-js-runtime"
        popd > /dev/null
        return 1
    fi
    
    popd > /dev/null
    echo_color "$GREEN" "DX tools built successfully"
    return 0
}

# Get system info
get_system_info() {
    local os=$(uname -s)
    local platform=$(uname -m)
    local cpu="Unknown"
    local cores=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "Unknown")
    local memory="Unknown"
    
    # Get CPU info
    if [[ -f /proc/cpuinfo ]]; then
        cpu=$(grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)
    elif [[ "$os" == "Darwin" ]]; then
        cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Unknown")
    fi
    
    # Get memory
    if [[ -f /proc/meminfo ]]; then
        local mem_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
        memory="$((mem_kb / 1024 / 1024)) GB"
    elif [[ "$os" == "Darwin" ]]; then
        local mem_bytes=$(sysctl -n hw.memsize 2>/dev/null)
        if [[ -n "$mem_bytes" ]]; then
            memory="$((mem_bytes / 1024 / 1024 / 1024)) GB"
        fi
    fi
    
    # Get versions
    local dx_version="Unknown"
    local bun_version="Unknown"
    
    if DX_PATH=$(get_dx_path); then
        dx_version=$("$DX_PATH" --version 2>&1 | head -1) || dx_version="Unknown"
    fi
    
    if BUN_PATH=$(get_bun_path); then
        bun_version=$("$BUN_PATH" --version 2>&1 | head -1) || bun_version="Unknown"
    fi
    
    cat << EOF
{
    "os": "$os",
    "platform": "$platform",
    "cpu": "$cpu",
    "cores": $cores,
    "memory": "$memory",
    "dxVersion": "$dx_version",
    "bunVersion": "$bun_version"
}
EOF
}


# Run benchmark suite
run_suite() {
    local suite_name=$1
    local suite_path=$2
    
    echo ""
    echo_color "$CYAN" "Running $suite_name benchmarks..."
    
    if [[ ! -f "$suite_path" ]]; then
        echo_color "$YELLOW" "Suite script not found: $suite_path"
        return 1
    fi
    
    # For bash suites, run directly
    if [[ "$suite_path" == *.sh ]]; then
        bash "$suite_path" --dx-path "$DX_PATH" --bun-path "$BUN_PATH" --runs "$RUNS" --warmup "$WARMUP"
    else
        # For PowerShell suites on Unix, use pwsh if available
        if command -v pwsh &> /dev/null; then
            pwsh -File "$suite_path" -DxPath "$DX_PATH" -BunPath "$BUN_PATH" -Runs "$RUNS" -Warmup "$WARMUP"
        else
            echo_color "$YELLOW" "PowerShell (pwsh) not found. Skipping $suite_name"
            return 1
        fi
    fi
}

# Main function
main() {
    echo_color "$CYAN" "========================================"
    echo_color "$CYAN" "  DX vs Bun Benchmark Suite"
    echo_color "$CYAN" "========================================"
    echo ""
    
    # Check system load
    check_system_load 50 || true
    
    # Build DX if needed
    if [[ "$SKIP_BUILD" != "true" ]]; then
        if ! build_dx_tools; then
            echo_color "$RED" "Failed to build DX tools. Use --skip-build to skip."
            exit 1
        fi
    fi
    
    # Check prerequisites
    if ! check_prerequisites; then
        echo_color "$RED" "Prerequisites not met. Please install missing tools."
        exit 1
    fi
    
    # Get paths
    DX_PATH=$(get_dx_path)
    BUN_PATH=$(get_bun_path)
    
    # Collect system info
    echo ""
    echo_color "$CYAN" "Collecting system information..."
    SYSTEM_INFO=$(get_system_info)
    
    # Define suites
    declare -A SUITES=(
        ["Runtime"]="$SCRIPT_DIR/suites/runtime/bench.ps1"
        ["Package Manager"]="$SCRIPT_DIR/suites/package-manager/bench.ps1"
        ["Bundler"]="$SCRIPT_DIR/suites/bundler/bench.ps1"
        ["Test Runner"]="$SCRIPT_DIR/suites/test-runner/bench.ps1"
        ["Project Manager"]="$SCRIPT_DIR/suites/project-manager/bench.ps1"
        ["Compatibility"]="$SCRIPT_DIR/suites/compatibility/bench.ps1"
    )
    
    # Ensure output directory exists
    OUTPUT_DIR="$SCRIPT_DIR/$OUTPUT"
    mkdir -p "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR/history"
    
    # Run suites
    local dx_wins=0
    local bun_wins=0
    local ties=0
    local total=0
    
    for suite_name in "${!SUITES[@]}"; do
        # Filter by suite name if specified
        if [[ -n "$SUITE" && "$suite_name" != *"$SUITE"* ]]; then
            continue
        fi
        
        suite_path="${SUITES[$suite_name]}"
        if run_suite "$suite_name" "$suite_path"; then
            ((total++)) || true
        fi
    done
    
    # Generate timestamp
    TIMESTAMP=$(date -Iseconds)
    
    # Create basic JSON output
    cat > "$OUTPUT_DIR/latest.json" << EOF
{
    "name": "DX vs Bun Benchmarks",
    "timestamp": "$TIMESTAMP",
    "system": $SYSTEM_INFO,
    "suites": [],
    "summary": {
        "totalBenchmarks": $total,
        "dxWins": $dx_wins,
        "bunWins": $bun_wins,
        "ties": $ties,
        "overallWinner": "pending"
    }
}
EOF
    
    # Copy to history
    cp "$OUTPUT_DIR/latest.json" "$OUTPUT_DIR/history/$(date +%Y-%m-%d_%H-%M-%S).json"
    
    # Print summary
    echo ""
    echo_color "$CYAN" "========================================"
    echo_color "$CYAN" "  Summary"
    echo_color "$CYAN" "========================================"
    echo ""
    echo "Results saved to: $OUTPUT_DIR/latest.json"
    echo ""
    echo_color "$YELLOW" "Note: For full benchmark results, run on Windows with PowerShell"
    echo_color "$YELLOW" "or install PowerShell Core (pwsh) on Unix systems."
    echo ""
}

# Run main
main "$@"
