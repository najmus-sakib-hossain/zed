# Statistics Library for DX vs Bun Benchmarks
# Provides statistical analysis functions for benchmark measurements

<#
.SYNOPSIS
    Calculate comprehensive statistics from an array of measurements.
.PARAMETER Values
    Array of numeric measurements.
.OUTPUTS
    Hashtable with min, max, mean, median, stddev, p95, p99, and count.
#>
function Get-Stats {
    param(
        [Parameter(Mandatory = $true)]
        [double[]]$Values
    )
    
    if ($Values.Count -eq 0) {
        return @{
            min = 0
            max = 0
            mean = 0
            median = 0
            stddev = 0
            p95 = 0
            p99 = 0
            count = 0
        }
    }
    
    $sorted = $Values | Sort-Object
    $count = $Values.Count
    
    # Basic stats
    $min = $sorted[0]
    $max = $sorted[$count - 1]
    $sum = ($Values | Measure-Object -Sum).Sum
    $mean = $sum / $count
    
    # Median
    if ($count % 2 -eq 0) {
        $median = ($sorted[$count / 2 - 1] + $sorted[$count / 2]) / 2
    } else {
        $median = $sorted[[math]::Floor($count / 2)]
    }
    
    # Standard deviation
    $sumSquaredDiff = 0
    foreach ($v in $Values) {
        $sumSquaredDiff += [math]::Pow($v - $mean, 2)
    }
    $variance = if ($count -gt 1) { $sumSquaredDiff / ($count - 1) } else { 0 }
    $stddev = [math]::Sqrt($variance)
    
    # Percentiles
    $p95Index = [math]::Ceiling(0.95 * $count) - 1
    $p99Index = [math]::Ceiling(0.99 * $count) - 1
    $p95 = $sorted[[math]::Min($p95Index, $count - 1)]
    $p99 = $sorted[[math]::Min($p99Index, $count - 1)]
    
    return @{
        min = $min
        max = $max
        mean = $mean
        median = $median
        stddev = $stddev
        p95 = $p95
        p99 = $p99
        count = $count
    }
}

<#
.SYNOPSIS
    Remove outliers from measurements using the IQR method.
.PARAMETER Values
    Array of numeric measurements.
.PARAMETER Factor
    IQR multiplier for outlier detection (default 1.5).
.OUTPUTS
    Array with outliers removed.
#>
function Remove-Outliers {
    param(
        [Parameter(Mandatory = $true)]
        [double[]]$Values,
        [double]$Factor = 1.5
    )
    
    if ($Values.Count -lt 4) {
        return $Values
    }
    
    $sorted = $Values | Sort-Object
    $count = $Values.Count
    
    # Calculate Q1 and Q3
    $q1Index = [math]::Floor($count * 0.25)
    $q3Index = [math]::Floor($count * 0.75)
    
    $q1 = $sorted[$q1Index]
    $q3 = $sorted[$q3Index]
    $iqr = $q3 - $q1
    
    $lowerBound = $q1 - ($Factor * $iqr)
    $upperBound = $q3 + ($Factor * $iqr)
    
    return $Values | Where-Object { $_ -ge $lowerBound -and $_ -le $upperBound }
}

<#
.SYNOPSIS
    Calculate confidence interval for measurements.
.PARAMETER Values
    Array of numeric measurements.
.PARAMETER Confidence
    Confidence level (default 0.95 for 95%).
.OUTPUTS
    Hashtable with lower, upper bounds and margin of error.
#>
function Get-ConfidenceInterval {
    param(
        [Parameter(Mandatory = $true)]
        [double[]]$Values,
        [double]$Confidence = 0.95
    )
    
    if ($Values.Count -lt 2) {
        $mean = if ($Values.Count -eq 1) { $Values[0] } else { 0 }
        return @{
            lower = $mean
            upper = $mean
            marginOfError = 0
            mean = $mean
        }
    }
    
    $stats = Get-Stats -Values $Values
    $n = $Values.Count
    
    # T-distribution critical values for common confidence levels
    # Using approximations for degrees of freedom
    $tValues = @{
        0.90 = @{ 5 = 2.015; 10 = 1.812; 20 = 1.725; 30 = 1.697; 100 = 1.660 }
        0.95 = @{ 5 = 2.571; 10 = 2.228; 20 = 2.086; 30 = 2.042; 100 = 1.984 }
        0.99 = @{ 5 = 4.032; 10 = 3.169; 20 = 2.845; 30 = 2.750; 100 = 2.626 }
    }
    
    # Get appropriate t-value based on degrees of freedom
    $df = $n - 1
    $confKey = [string]$Confidence
    if (-not $tValues.ContainsKey($confKey)) {
        $confKey = "0.95"
    }
    
    $tValue = 1.96  # Default to z-value for large samples
    if ($df -le 5) { $tValue = $tValues[$confKey][5] }
    elseif ($df -le 10) { $tValue = $tValues[$confKey][10] }
    elseif ($df -le 20) { $tValue = $tValues[$confKey][20] }
    elseif ($df -le 30) { $tValue = $tValues[$confKey][30] }
    else { $tValue = $tValues[$confKey][100] }
    
    $standardError = $stats.stddev / [math]::Sqrt($n)
    $marginOfError = $tValue * $standardError
    
    return @{
        lower = $stats.mean - $marginOfError
        upper = $stats.mean + $marginOfError
        marginOfError = $marginOfError
        mean = $stats.mean
    }
}

<#
.SYNOPSIS
    Compare two result sets and determine the winner.
.PARAMETER ResultA
    First measurement set (hashtable with 'times' array).
.PARAMETER ResultB
    Second measurement set (hashtable with 'times' array).
.PARAMETER LowerIsBetter
    If true, lower values win (for time measurements). Default true.
.PARAMETER Threshold
    Minimum percentage difference to declare a winner (default 0.05 = 5%).
.OUTPUTS
    Hashtable with winner, speedup ratio, and significance flag.
#>
function Compare-Results {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ResultA,
        [Parameter(Mandatory = $true)]
        [hashtable]$ResultB,
        [bool]$LowerIsBetter = $true,
        [double]$Threshold = 0.05
    )
    
    $statsA = Get-Stats -Values $ResultA.times
    $statsB = Get-Stats -Values $ResultB.times
    
    $ciA = Get-ConfidenceInterval -Values $ResultA.times
    $ciB = Get-ConfidenceInterval -Values $ResultB.times
    
    # Check for overlap in confidence intervals
    $overlaps = ($ciA.lower -le $ciB.upper) -and ($ciB.lower -le $ciA.upper)
    
    # Calculate speedup
    $meanA = $statsA.mean
    $meanB = $statsB.mean
    
    if ($meanA -eq 0 -or $meanB -eq 0) {
        return @{
            winner = "tie"
            speedup = 1.0
            isSignificant = $false
            statsA = $statsA
            statsB = $statsB
        }
    }
    
    # Determine winner based on metric type
    if ($LowerIsBetter) {
        if ($meanA -lt $meanB) {
            $winner = "A"
            $speedup = $meanB / $meanA
        } elseif ($meanB -lt $meanA) {
            $winner = "B"
            $speedup = $meanA / $meanB
        } else {
            $winner = "tie"
            $speedup = 1.0
        }
    } else {
        # Higher is better (throughput)
        if ($meanA -gt $meanB) {
            $winner = "A"
            $speedup = $meanA / $meanB
        } elseif ($meanB -gt $meanA) {
            $winner = "B"
            $speedup = $meanB / $meanA
        } else {
            $winner = "tie"
            $speedup = 1.0
        }
    }
    
    # Check if difference is significant
    $percentDiff = [math]::Abs($meanA - $meanB) / [math]::Max($meanA, $meanB)
    $isSignificant = (-not $overlaps) -and ($percentDiff -gt $Threshold)
    
    # If not significant, declare tie
    if (-not $isSignificant) {
        $winner = "tie"
    }
    
    return @{
        winner = $winner
        speedup = [math]::Round($speedup, 2)
        isSignificant = $isSignificant
        percentDiff = [math]::Round($percentDiff * 100, 1)
        statsA = $statsA
        statsB = $statsB
        ciA = $ciA
        ciB = $ciB
    }
}

# Export functions
Export-ModuleMember -Function Get-Stats, Remove-Outliers, Get-ConfidenceInterval, Compare-Results
