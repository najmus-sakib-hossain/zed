#!/bin/bash
# Statistics Library for DX vs Bun Benchmarks (Bash)
# Provides statistical analysis functions for benchmark measurements

# Calculate minimum value from array
calc_min() {
    local values=("$@")
    local min=${values[0]}
    for val in "${values[@]}"; do
        if (( $(echo "$val < $min" | bc -l) )); then
            min=$val
        fi
    done
    echo "$min"
}

# Calculate maximum value from array
calc_max() {
    local values=("$@")
    local max=${values[0]}
    for val in "${values[@]}"; do
        if (( $(echo "$val > $max" | bc -l) )); then
            max=$val
        fi
    done
    echo "$max"
}

# Calculate sum of array
calc_sum() {
    local values=("$@")
    local sum=0
    for val in "${values[@]}"; do
        sum=$(echo "$sum + $val" | bc -l)
    done
    echo "$sum"
}

# Calculate mean (average)
calc_mean() {
    local values=("$@")
    local count=${#values[@]}
    if [[ $count -eq 0 ]]; then
        echo "0"
        return
    fi
    local sum=$(calc_sum "${values[@]}")
    echo "scale=6; $sum / $count" | bc -l
}

# Calculate median
calc_median() {
    local values=("$@")
    local count=${#values[@]}
    if [[ $count -eq 0 ]]; then
        echo "0"
        return
    fi
    
    # Sort values
    IFS=$'\n' sorted=($(sort -n <<<"${values[*]}")); unset IFS
    
    local mid=$((count / 2))
    if (( count % 2 == 0 )); then
        # Even count: average of two middle values
        local v1=${sorted[$((mid - 1))]}
        local v2=${sorted[$mid]}
        echo "scale=6; ($v1 + $v2) / 2" | bc -l
    else
        # Odd count: middle value
        echo "${sorted[$mid]}"
    fi
}

# Calculate standard deviation
calc_stddev() {
    local values=("$@")
    local count=${#values[@]}
    if [[ $count -lt 2 ]]; then
        echo "0"
        return
    fi
    
    local mean=$(calc_mean "${values[@]}")
    local sum_sq_diff=0
    
    for val in "${values[@]}"; do
        local diff=$(echo "$val - $mean" | bc -l)
        local sq=$(echo "$diff * $diff" | bc -l)
        sum_sq_diff=$(echo "$sum_sq_diff + $sq" | bc -l)
    done
    
    local variance=$(echo "scale=6; $sum_sq_diff / ($count - 1)" | bc -l)
    echo "scale=6; sqrt($variance)" | bc -l
}


# Calculate percentile
calc_percentile() {
    local percentile=$1
    shift
    local values=("$@")
    local count=${#values[@]}
    
    if [[ $count -eq 0 ]]; then
        echo "0"
        return
    fi
    
    # Sort values
    IFS=$'\n' sorted=($(sort -n <<<"${values[*]}")); unset IFS
    
    local index=$(echo "scale=0; ($percentile * $count + 99) / 100 - 1" | bc)
    if (( index < 0 )); then index=0; fi
    if (( index >= count )); then index=$((count - 1)); fi
    
    echo "${sorted[$index]}"
}

# Calculate all statistics and output as JSON
get_stats() {
    local values=("$@")
    local count=${#values[@]}
    
    if [[ $count -eq 0 ]]; then
        echo '{"min":0,"max":0,"mean":0,"median":0,"stddev":0,"p95":0,"p99":0,"count":0}'
        return
    fi
    
    local min=$(calc_min "${values[@]}")
    local max=$(calc_max "${values[@]}")
    local mean=$(calc_mean "${values[@]}")
    local median=$(calc_median "${values[@]}")
    local stddev=$(calc_stddev "${values[@]}")
    local p95=$(calc_percentile 95 "${values[@]}")
    local p99=$(calc_percentile 99 "${values[@]}")
    
    cat << EOF
{
    "min": $min,
    "max": $max,
    "mean": $mean,
    "median": $median,
    "stddev": $stddev,
    "p95": $p95,
    "p99": $p99,
    "count": $count
}
EOF
}

# Remove outliers using IQR method
remove_outliers() {
    local factor=${1:-1.5}
    shift
    local values=("$@")
    local count=${#values[@]}
    
    if [[ $count -lt 4 ]]; then
        echo "${values[@]}"
        return
    fi
    
    # Sort values
    IFS=$'\n' sorted=($(sort -n <<<"${values[*]}")); unset IFS
    
    # Calculate Q1 and Q3
    local q1_idx=$((count / 4))
    local q3_idx=$((count * 3 / 4))
    local q1=${sorted[$q1_idx]}
    local q3=${sorted[$q3_idx]}
    local iqr=$(echo "$q3 - $q1" | bc -l)
    
    local lower=$(echo "$q1 - $factor * $iqr" | bc -l)
    local upper=$(echo "$q3 + $factor * $iqr" | bc -l)
    
    local result=()
    for val in "${values[@]}"; do
        if (( $(echo "$val >= $lower && $val <= $upper" | bc -l) )); then
            result+=("$val")
        fi
    done
    
    echo "${result[@]}"
}

# Calculate confidence interval
get_confidence_interval() {
    local confidence=${1:-0.95}
    shift
    local values=("$@")
    local count=${#values[@]}
    
    if [[ $count -lt 2 ]]; then
        local mean=${values[0]:-0}
        echo "{\"lower\": $mean, \"upper\": $mean, \"marginOfError\": 0, \"mean\": $mean}"
        return
    fi
    
    local mean=$(calc_mean "${values[@]}")
    local stddev=$(calc_stddev "${values[@]}")
    
    # T-value approximation for 95% confidence
    local t_value=1.96
    if (( count <= 5 )); then t_value=2.571
    elif (( count <= 10 )); then t_value=2.228
    elif (( count <= 20 )); then t_value=2.086
    elif (( count <= 30 )); then t_value=2.042
    fi
    
    local se=$(echo "scale=6; $stddev / sqrt($count)" | bc -l)
    local moe=$(echo "scale=6; $t_value * $se" | bc -l)
    local lower=$(echo "scale=6; $mean - $moe" | bc -l)
    local upper=$(echo "scale=6; $mean + $moe" | bc -l)
    
    cat << EOF
{
    "lower": $lower,
    "upper": $upper,
    "marginOfError": $moe,
    "mean": $mean
}
EOF
}

# Compare two result sets
compare_results() {
    local lower_is_better=${1:-true}
    shift
    local -n result_a=$1
    local -n result_b=$2
    
    local mean_a=$(calc_mean "${result_a[@]}")
    local mean_b=$(calc_mean "${result_b[@]}")
    
    local winner="tie"
    local speedup=1.0
    
    if [[ "$lower_is_better" == "true" ]]; then
        if (( $(echo "$mean_a < $mean_b" | bc -l) )); then
            winner="A"
            speedup=$(echo "scale=2; $mean_b / $mean_a" | bc -l)
        elif (( $(echo "$mean_b < $mean_a" | bc -l) )); then
            winner="B"
            speedup=$(echo "scale=2; $mean_a / $mean_b" | bc -l)
        fi
    else
        if (( $(echo "$mean_a > $mean_b" | bc -l) )); then
            winner="A"
            speedup=$(echo "scale=2; $mean_a / $mean_b" | bc -l)
        elif (( $(echo "$mean_b > $mean_a" | bc -l) )); then
            winner="B"
            speedup=$(echo "scale=2; $mean_b / $mean_a" | bc -l)
        fi
    fi
    
    cat << EOF
{
    "winner": "$winner",
    "speedup": $speedup,
    "meanA": $mean_a,
    "meanB": $mean_b
}
EOF
}

# Measure execution time of a command
measure_time() {
    local cmd="$1"
    local start=$(date +%s%N)
    eval "$cmd" > /dev/null 2>&1
    local end=$(date +%s%N)
    local elapsed=$(echo "scale=3; ($end - $start) / 1000000" | bc -l)
    echo "$elapsed"
}
