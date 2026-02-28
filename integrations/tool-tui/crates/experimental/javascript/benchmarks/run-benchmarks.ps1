# DX JavaScript Tooling Benchmark Suite
# Cross-platform benchmark script (PowerShell)
#
# Usage:
#   ./benchmarks/run-benchmarks.ps1 [-Runs 5] [-Compare] [-Output results.json]
#
# Requirements:
#   - Rust toolchain (cargo)
#   - Node.js (for npm comparison)
#   - Bun (optional, for comparison)

param(
    [int]$Runs = 5,
    [switch]$Compare,
    [string]$Output = "benchmark-results.json"
)

$ErrorActionPreference = "Stop"

Write-Host "╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         DX JavaScript Tooling Benchmark Suite                ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# Configuration
$BENCHMARK_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$ROOT_DIR = Split-Path -Parent $BENCHMARK_DIR
$RESULTS = @{
    timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    platform = [System.Environment]::OSVersion.Platform
    runs = $Runs
    results = @{}
}

# Helper function to measure execution time
function Measure-Command-Ms {
    param([scriptblock]$Command)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $Command | Out-Null
    $sw.Stop()
    return $sw.ElapsedMilliseconds
}

# Helper function to run multiple times and get statistics
function Get-BenchmarkStats {
    param(
        [string]$Name,
        [scriptblock]$Command,
        [int]$Runs
    )
    
    Write-Host "  Running $Name..." -NoNewline
    $times = @()
    
    for ($i = 1; $i -le $Runs; $i++) {
        $ms = Measure-Command-Ms -Command $Command
        $times += $ms
        Write-Host "." -NoNewline
    }
    
    $sorted = $times | Sort-Object
    $stats = @{
        min = $sorted[0]
        max = $sorted[-1]
        median = $sorted[[math]::Floor($Runs / 2)]
        mean = [math]::Round(($times | Measure-Object -Average).Average, 2)
        times = $times
    }
    
    Write-Host " $($stats.median)ms (median)" -ForegroundColor Green
    return $stats
}

# Build release binaries
Write-Host "Building release binaries..." -ForegroundColor Yellow
Push-Location $ROOT_DIR

# Build runtime
Write-Host "  Building dx-js-runtime..."
cargo build --release -p dx-js-runtime 2>&1 | Out-Null

# Build package manager
Write-Host "  Building dx-pkg-cli..."
cargo build --release -p dx-pkg-cli 2>&1 | Out-Null

# Build bundler
Write-Host "  Building dx-bundle-cli..."
cargo build --release -p dx-bundle-cli 2>&1 | Out-Null

Pop-Location
Write-Host ""

# Create test fixtures
$FIXTURES_DIR = Join-Path $BENCHMARK_DIR "fixtures"
if (-not (Test-Path $FIXTURES_DIR)) {
    New-Item -ItemType Directory -Path $FIXTURES_DIR | Out-Null
}

# Create test JavaScript file
$TEST_JS = @"
// Benchmark test file
const data = [];
for (let i = 0; i < 1000; i++) {
    data.push({ id: i, value: Math.random() });
}
const sorted = data.sort((a, b) => a.value - b.value);
console.log('Processed', sorted.length, 'items');
"@
Set-Content -Path (Join-Path $FIXTURES_DIR "test.js") -Value $TEST_JS

# Create test TypeScript file
$TEST_TS = @"
// TypeScript benchmark test file
interface Item {
    id: number;
    value: number;
}

const data: Item[] = [];
for (let i = 0; i < 1000; i++) {
    data.push({ id: i, value: Math.random() });
}
const sorted = data.sort((a, b) => a.value - b.value);
console.log('Processed', sorted.length, 'items');
"@
Set-Content -Path (Join-Path $FIXTURES_DIR "test.ts") -Value $TEST_TS

# Create test package.json
$TEST_PKG = @"
{
    "name": "benchmark-test",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21"
    }
}
"@
$PKG_DIR = Join-Path $FIXTURES_DIR "pkg-test"
if (-not (Test-Path $PKG_DIR)) {
    New-Item -ItemType Directory -Path $PKG_DIR | Out-Null
}
Set-Content -Path (Join-Path $PKG_DIR "package.json") -Value $TEST_PKG

Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "                    RUNTIME BENCHMARKS                          " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$DX_JS = Join-Path $ROOT_DIR "target/release/dx-js"
if ($IsWindows) { $DX_JS += ".exe" }

# Benchmark: JavaScript execution
$RESULTS.results["runtime_js"] = Get-BenchmarkStats -Name "dx-js (JavaScript)" -Runs $Runs -Command {
    & $DX_JS (Join-Path $FIXTURES_DIR "test.js")
}

# Benchmark: TypeScript execution
$RESULTS.results["runtime_ts"] = Get-BenchmarkStats -Name "dx-js (TypeScript)" -Runs $Runs -Command {
    & $DX_JS (Join-Path $FIXTURES_DIR "test.ts")
}

# Compare with Bun if available
if ($Compare -and (Get-Command "bun" -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "Comparison with Bun:" -ForegroundColor Yellow
    
    $RESULTS.results["bun_js"] = Get-BenchmarkStats -Name "bun (JavaScript)" -Runs $Runs -Command {
        bun run (Join-Path $FIXTURES_DIR "test.js")
    }
    
    $RESULTS.results["bun_ts"] = Get-BenchmarkStats -Name "bun (TypeScript)" -Runs $Runs -Command {
        bun run (Join-Path $FIXTURES_DIR "test.ts")
    }
    
    # Calculate speedup
    $jsSpeedup = [math]::Round($RESULTS.results["bun_js"].median / $RESULTS.results["runtime_js"].median, 2)
    $tsSpeedup = [math]::Round($RESULTS.results["bun_ts"].median / $RESULTS.results["runtime_ts"].median, 2)
    
    Write-Host ""
    Write-Host "  JavaScript speedup: ${jsSpeedup}x faster than Bun" -ForegroundColor Green
    Write-Host "  TypeScript speedup: ${tsSpeedup}x faster than Bun" -ForegroundColor Green
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "                 PACKAGE MANAGER BENCHMARKS                     " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$DX_PKG = Join-Path $ROOT_DIR "target/release/dx"
if ($IsWindows) { $DX_PKG += ".exe" }

# Clean up before benchmark
Push-Location $PKG_DIR
if (Test-Path "node_modules") { Remove-Item -Recurse -Force "node_modules" }
if (Test-Path "dx.lock") { Remove-Item -Force "dx.lock" }

# Benchmark: Cold install
$RESULTS.results["pkg_cold"] = Get-BenchmarkStats -Name "dx install (cold)" -Runs $Runs -Command {
    if (Test-Path "node_modules") { Remove-Item -Recurse -Force "node_modules" }
    if (Test-Path "dx.lock") { Remove-Item -Force "dx.lock" }
    & $DX_PKG install
}

# Benchmark: Warm install
$RESULTS.results["pkg_warm"] = Get-BenchmarkStats -Name "dx install (warm)" -Runs $Runs -Command {
    if (Test-Path "node_modules") { Remove-Item -Recurse -Force "node_modules" }
    & $DX_PKG install
}

Pop-Location

# Compare with npm if available
if ($Compare -and (Get-Command "npm" -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "Comparison with npm:" -ForegroundColor Yellow
    
    Push-Location $PKG_DIR
    
    $RESULTS.results["npm_cold"] = Get-BenchmarkStats -Name "npm install (cold)" -Runs ([math]::Min($Runs, 3)) -Command {
        if (Test-Path "node_modules") { Remove-Item -Recurse -Force "node_modules" }
        if (Test-Path "package-lock.json") { Remove-Item -Force "package-lock.json" }
        npm install --silent
    }
    
    Pop-Location
    
    $npmSpeedup = [math]::Round($RESULTS.results["npm_cold"].median / $RESULTS.results["pkg_cold"].median, 2)
    Write-Host ""
    Write-Host "  Cold install speedup: ${npmSpeedup}x faster than npm" -ForegroundColor Green
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "                    BUNDLER BENCHMARKS                          " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$DX_BUNDLE = Join-Path $ROOT_DIR "target/release/dx-bundle"
if ($IsWindows) { $DX_BUNDLE += ".exe" }

# Create bundle test file
$BUNDLE_TEST = @"
import { sortBy } from 'lodash';

const data = [
    { name: 'Alice', age: 30 },
    { name: 'Bob', age: 25 },
    { name: 'Charlie', age: 35 }
];

const sorted = sortBy(data, 'age');
console.log(sorted);
"@
Set-Content -Path (Join-Path $FIXTURES_DIR "bundle-test.js") -Value $BUNDLE_TEST

# Benchmark: Bundle (cold)
$RESULTS.results["bundle_cold"] = Get-BenchmarkStats -Name "dx-bundle (cold)" -Runs $Runs -Command {
    if (Test-Path (Join-Path $FIXTURES_DIR ".dx-cache")) { 
        Remove-Item -Recurse -Force (Join-Path $FIXTURES_DIR ".dx-cache") 
    }
    & $DX_BUNDLE bundle (Join-Path $FIXTURES_DIR "bundle-test.js") -o (Join-Path $FIXTURES_DIR "dist/bundle.js")
}

# Benchmark: Bundle (warm)
$RESULTS.results["bundle_warm"] = Get-BenchmarkStats -Name "dx-bundle (warm)" -Runs $Runs -Command {
    & $DX_BUNDLE bundle (Join-Path $FIXTURES_DIR "bundle-test.js") -o (Join-Path $FIXTURES_DIR "dist/bundle.js")
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "                       SUMMARY                                  " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

Write-Host "Runtime (median):" -ForegroundColor Yellow
Write-Host "  JavaScript: $($RESULTS.results['runtime_js'].median)ms"
Write-Host "  TypeScript: $($RESULTS.results['runtime_ts'].median)ms"
Write-Host ""

Write-Host "Package Manager (median):" -ForegroundColor Yellow
Write-Host "  Cold install: $($RESULTS.results['pkg_cold'].median)ms"
Write-Host "  Warm install: $($RESULTS.results['pkg_warm'].median)ms"
Write-Host ""

Write-Host "Bundler (median):" -ForegroundColor Yellow
Write-Host "  Cold bundle: $($RESULTS.results['bundle_cold'].median)ms"
Write-Host "  Warm bundle: $($RESULTS.results['bundle_warm'].median)ms"
Write-Host ""

# Save results to JSON
$RESULTS | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $BENCHMARK_DIR $Output)
Write-Host "Results saved to: $Output" -ForegroundColor Green
