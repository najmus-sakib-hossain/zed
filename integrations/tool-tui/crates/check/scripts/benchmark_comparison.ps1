# Benchmark Comparison Script for dx-check
# Compares performance against ESLint, Biome, and other linters
#
# **Validates: Requirement 8.3 - Run benchmarks against ESLint, Biome, and other tools**
#
# Prerequisites:
# - Node.js and npm installed
# - ESLint installed globally: npm install -g eslint
# - Biome installed globally: npm install -g @biomejs/biome
# - dx-check built: cargo build -p dx-check --release

param(
    [string]$TestDir = "benchmark_test_files",
    [int]$FileCount = 100,
    [int]$Iterations = 5
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-ColorOutput Green "=== dx-check Benchmark Comparison ==="
Write-Output ""

# Create test directory
if (Test-Path $TestDir) {
    Remove-Item -Recurse -Force $TestDir
}
New-Item -ItemType Directory -Path $TestDir | Out-Null

# Generate test files
Write-Output "Generating $FileCount test files..."

$SampleCode = @"
import React, { useState, useEffect } from 'react';

export function Component({ prop1, prop2 }) {
    const [state, setState] = useState(null);
    
    useEffect(() => {
        console.log('mounted');
        return () => console.log('unmounted');
    }, []);
    
    if (state == null) {
        return <div>Loading...</div>;
    }
    
    return (
        <div className="container">
            <h1>{prop1}</h1>
            <p>{prop2}</p>
        </div>
    );
}
"@

for ($i = 0; $i -lt $FileCount; $i++) {
    $filename = Join-Path $TestDir "file_$i.jsx"
    Set-Content -Path $filename -Value $SampleCode
}

Write-Output "Generated $FileCount files in $TestDir"
Write-Output ""

# Initialize results
$Results = @{}

# Benchmark dx-check
Write-ColorOutput Cyan "Benchmarking dx-check..."
$dxCheckPath = "..\..\..\..\target\release\dx-check.exe"
if (-not (Test-Path $dxCheckPath)) {
    $dxCheckPath = "dx-check"
}

$dxCheckTimes = @()
for ($i = 0; $i -lt $Iterations; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $dxCheckPath $TestDir 2>&1 | Out-Null
    $sw.Stop()
    $dxCheckTimes += $sw.ElapsedMilliseconds
    Write-Output "  Iteration $($i + 1): $($sw.ElapsedMilliseconds)ms"
}
$Results["dx-check"] = ($dxCheckTimes | Measure-Object -Average).Average

# Benchmark ESLint (if available)
Write-ColorOutput Cyan "Benchmarking ESLint..."
$eslintAvailable = $null -ne (Get-Command "eslint" -ErrorAction SilentlyContinue)
if ($eslintAvailable) {
    # Create ESLint config
    $eslintConfig = @"
{
    "env": {
        "browser": true,
        "es2021": true
    },
    "extends": "eslint:recommended",
    "parserOptions": {
        "ecmaVersion": "latest",
        "sourceType": "module",
        "ecmaFeatures": {
            "jsx": true
        }
    },
    "rules": {
        "no-console": "warn",
        "eqeqeq": "error"
    }
}
"@
    Set-Content -Path (Join-Path $TestDir ".eslintrc.json") -Value $eslintConfig
    
    $eslintTimes = @()
    for ($i = 0; $i -lt $Iterations; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & eslint $TestDir --no-error-on-unmatched-pattern 2>&1 | Out-Null
        $sw.Stop()
        $eslintTimes += $sw.ElapsedMilliseconds
        Write-Output "  Iteration $($i + 1): $($sw.ElapsedMilliseconds)ms"
    }
    $Results["ESLint"] = ($eslintTimes | Measure-Object -Average).Average
} else {
    Write-Output "  ESLint not found, skipping..."
}

# Benchmark Biome (if available)
Write-ColorOutput Cyan "Benchmarking Biome..."
$biomeAvailable = $null -ne (Get-Command "biome" -ErrorAction SilentlyContinue)
if ($biomeAvailable) {
    $biomeTimes = @()
    for ($i = 0; $i -lt $Iterations; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & biome check $TestDir 2>&1 | Out-Null
        $sw.Stop()
        $biomeTimes += $sw.ElapsedMilliseconds
        Write-Output "  Iteration $($i + 1): $($sw.ElapsedMilliseconds)ms"
    }
    $Results["Biome"] = ($biomeTimes | Measure-Object -Average).Average
} else {
    Write-Output "  Biome not found, skipping..."
}

# Print results
Write-Output ""
Write-ColorOutput Green "=== Results ==="
Write-Output ""
Write-Output "Files: $FileCount"
Write-Output "Iterations: $Iterations"
Write-Output ""

$sortedResults = $Results.GetEnumerator() | Sort-Object Value
$fastest = $sortedResults[0].Value

Write-Output "Tool                 Avg Time (ms)    Relative Speed"
Write-Output "----                 -------------    --------------"

foreach ($result in $sortedResults) {
    $relative = if ($result.Value -gt 0) { [math]::Round($result.Value / $fastest, 2) } else { 1 }
    $relativeStr = if ($relative -eq 1) { "1.00x (fastest)" } else { "${relative}x slower" }
    Write-Output ("{0,-20} {1,13:N0}    {2}" -f $result.Key, $result.Value, $relativeStr)
}

# Calculate files per second
Write-Output ""
Write-ColorOutput Green "=== Throughput ==="
Write-Output ""

foreach ($result in $sortedResults) {
    $filesPerSec = if ($result.Value -gt 0) { [math]::Round($FileCount / ($result.Value / 1000), 0) } else { 0 }
    Write-Output ("{0,-20} {1,10:N0} files/sec" -f $result.Key, $filesPerSec)
}

# Cleanup
Write-Output ""
Write-Output "Cleaning up test files..."
Remove-Item -Recurse -Force $TestDir

Write-Output ""
Write-ColorOutput Green "Benchmark complete!"

# Save results to JSON
$jsonResults = @{
    "timestamp" = (Get-Date -Format "yyyy-MM-ddTHH:mm:ss")
    "file_count" = $FileCount
    "iterations" = $Iterations
    "results" = $Results
} | ConvertTo-Json

$resultsPath = "benchmark_results.json"
Set-Content -Path $resultsPath -Value $jsonResults
Write-Output "Results saved to $resultsPath"
