# DX JavaScript Runtime Benchmark Suite Runner (PowerShell)
# Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6

param(
    [int]$Runs = 10,
    [switch]$Compare,
    [string]$Category = "all",
    [switch]$Report
)

$ErrorActionPreference = "Stop"

# Configuration
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$WorkloadsDir = Join-Path $ScriptDir "workloads"
$ResultsDir = Join-Path $ScriptDir "results"

# Detect DX binary
$DxBin = Join-Path $RootDir "runtime\target\release\dx-js.exe"

if (-not (Test-Path $DxBin)) {
    Write-Host "Building DX runtime..." -ForegroundColor Yellow
    Push-Location $RootDir
    cargo build --manifest-path runtime/Cargo.toml --release
    Pop-Location
}

# Create results directory
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$ResultFile = Join-Path $ResultsDir "benchmark_$Timestamp.json"

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║          DX JavaScript Runtime Benchmark Suite                   ║" -ForegroundColor Cyan
Write-Host "╠══════════════════════════════════════════════════════════════════╣" -ForegroundColor Cyan
Write-Host "║  Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')                                      ║" -ForegroundColor Cyan
Write-Host "║  Runs: $Runs | Category: $Category | Compare: $Compare                   ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# Function to measure cold start time
function Measure-ColdStart {
    param(
        [string]$Runtime,
        [string]$Script
    )
    
    $times = @()
    
    for ($i = 1; $i -le $Runs; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & $Runtime $Script 2>&1 | Out-Null
        $sw.Stop()
        $times += $sw.ElapsedMilliseconds
    }
    
    $sorted = $times | Sort-Object
    $min = $sorted[0]
    $max = $sorted[-1]
    $median = $sorted[[math]::Floor($sorted.Count / 2)]
    $mean = ($times | Measure-Object -Average).Average
    
    # Calculate standard deviation
    $sqSum = 0
    foreach ($t in $times) {
        $diff = $t - $mean
        $sqSum += $diff * $diff
    }
    $variance = $sqSum / $times.Count
    $stdDev = [math]::Sqrt($variance)
    
    return @{
        Min = $min
        Max = $max
        Median = $median
        Mean = [math]::Round($mean, 2)
        StdDev = [math]::Round($stdDev, 2)
    }
}

# Function to run throughput benchmark
function Run-Throughput {
    param(
        [string]$Runtime,
        [string]$Script
    )
    
    $output = & $Runtime $Script 2>&1
    return $output
}

# Initialize results
$results = @{
    timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platform = "Windows"
    arch = $env:PROCESSOR_ARCHITECTURE
    runs = $Runs
    results = @{}
}

# Run startup benchmarks
if ($Category -eq "all" -or $Category -eq "startup") {
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host "                      STARTUP BENCHMARKS                           " -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "Cold Start (JavaScript):" -ForegroundColor Yellow
    Write-Host -NoNewline "  DX:   "
    $dxJsResult = Measure-ColdStart -Runtime $DxBin -Script (Join-Path $WorkloadsDir "startup.js")
    Write-Host "$($dxJsResult.Median)ms (median)"
    
    if ($Compare) {
        $nodePath = Get-Command node -ErrorAction SilentlyContinue
        if ($nodePath) {
            Write-Host -NoNewline "  Node: "
            $nodeJsResult = Measure-ColdStart -Runtime "node" -Script (Join-Path $WorkloadsDir "startup.js")
            Write-Host "$($nodeJsResult.Median)ms (median)"
        }
        
        $bunPath = Get-Command bun -ErrorAction SilentlyContinue
        if ($bunPath) {
            Write-Host -NoNewline "  Bun:  "
            $bunJsResult = Measure-ColdStart -Runtime "bun" -Script (Join-Path $WorkloadsDir "startup.js")
            Write-Host "$($bunJsResult.Median)ms (median)"
        }
    }
    
    Write-Host ""
    Write-Host "Cold Start (TypeScript):" -ForegroundColor Yellow
    Write-Host -NoNewline "  DX:   "
    $dxTsResult = Measure-ColdStart -Runtime $DxBin -Script (Join-Path $WorkloadsDir "startup.ts")
    Write-Host "$($dxTsResult.Median)ms (median)"
    
    Write-Host ""
    
    $results.results.startup = @{
        js = @{ dx = $dxJsResult }
        ts = @{ dx = $dxTsResult }
    }
}

# Run throughput benchmarks
if ($Category -eq "all" -or $Category -eq "throughput") {
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host "                     THROUGHPUT BENCHMARKS                         " -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "JSON Throughput:" -ForegroundColor Yellow
    Write-Host "  DX:"
    $dxJson = Run-Throughput -Runtime $DxBin -Script (Join-Path $WorkloadsDir "json-throughput.js")
    Write-Host "    $dxJson"
    
    Write-Host ""
    Write-Host "Array Throughput:" -ForegroundColor Yellow
    Write-Host "  DX:"
    $dxArray = Run-Throughput -Runtime $DxBin -Script (Join-Path $WorkloadsDir "array-throughput.js")
    Write-Host "    $dxArray"
    
    Write-Host ""
    Write-Host "Async Throughput:" -ForegroundColor Yellow
    Write-Host "  DX:"
    $dxAsync = Run-Throughput -Runtime $DxBin -Script (Join-Path $WorkloadsDir "async-throughput.js")
    Write-Host "    $dxAsync"
    
    Write-Host ""
    Write-Host "Fibonacci (CPU):" -ForegroundColor Yellow
    Write-Host "  DX:"
    $dxFib = Run-Throughput -Runtime $DxBin -Script (Join-Path $WorkloadsDir "fibonacci.js")
    Write-Host "    $dxFib"
    
    Write-Host ""
    
    $results.results.throughput = @{
        json = $dxJson
        array = $dxArray
        async = $dxAsync
        fibonacci = $dxFib
    }
}

# Run memory benchmarks
if ($Category -eq "all" -or $Category -eq "memory") {
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host "                       MEMORY BENCHMARKS                           " -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "Memory Baseline:" -ForegroundColor Yellow
    Write-Host "  DX:"
    $dxMem = Run-Throughput -Runtime $DxBin -Script (Join-Path $WorkloadsDir "memory-baseline.js")
    Write-Host "    $dxMem"
    
    Write-Host ""
    
    $results.results.memory = @{
        baseline = $dxMem
    }
}

# Save results
$results | ConvertTo-Json -Depth 10 | Out-File -FilePath $ResultFile -Encoding UTF8

Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "Benchmark complete! Results saved to: $ResultFile" -ForegroundColor Green
Write-Host ""
