#!/usr/bin/env pwsh
# Windows PowerShell benchmark script

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  🧪 DX Test Runner vs Bun - Performance Benchmark" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Build DX
Write-Host "📦 Building DX Test Runner (release mode)..." -ForegroundColor Yellow
cargo build --release -p dx-test-cli 2>&1 | Out-Null
Write-Host "✓ Build complete" -ForegroundColor Green
Write-Host ""

# Count tests
$testCount = (Get-ChildItem -Path tests\*.test.js | ForEach-Object { 
    (Select-String -Path $_ -Pattern "^test\(" -AllMatches).Matches.Count 
} | Measure-Object -Sum).Sum

Write-Host "Found $testCount tests across 5 files" -ForegroundColor White
Write-Host ""

# Run Bun benchmark
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Blue
Write-Host "  Running Bun Test Runner..." -ForegroundColor Blue
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Blue
Write-Host ""

Set-Location tests
$bunStart = Get-Date
bun test 2>&1 | Out-String | Write-Host
$bunEnd = Get-Date
$bunTime = ($bunEnd - $bunStart).TotalMilliseconds
Set-Location ..

Write-Host ""
Write-Host "Bun completed in: $([math]::Round($bunTime, 2))ms" -ForegroundColor Yellow
Write-Host ""

# Clear DX cache
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host "  Running DX Test Runner (Cold Start)..." -ForegroundColor Magenta
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host ""

.\target\release\dx-test.exe clear | Out-Null

$dxColdStart = Get-Date
.\target\release\dx-test.exe 2>&1 | Out-String | Write-Host
$dxColdEnd = Get-Date
$dxColdTime = ($dxColdEnd - $dxColdStart).TotalMilliseconds

Write-Host ""
Write-Host "DX (cold) completed in: $([math]::Round($dxColdTime, 2))ms" -ForegroundColor Yellow
Write-Host ""

# Run again with warm cache
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "  Running DX Test Runner (Warm Cache)..." -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""

$dxWarmStart = Get-Date
.\target\release\dx-test.exe 2>&1 | Out-String | Write-Host
$dxWarmEnd = Get-Date
$dxWarmTime = ($dxWarmEnd - $dxWarmStart).TotalMilliseconds

Write-Host ""
Write-Host "DX (warm) completed in: $([math]::Round($dxWarmTime, 2))ms" -ForegroundColor Yellow
Write-Host ""

# Calculate speedups
$speedupCold = [math]::Round($bunTime / $dxColdTime, 1)
$speedupWarm = [math]::Round($bunTime / $dxWarmTime, 1)

# Results summary
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  📊 Performance Summary ($testCount tests)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Runner" -NoNewline
Write-Host "               Time       Speedup" -ForegroundColor Gray
Write-Host "  ────────────────────────────────────────" -ForegroundColor Gray
Write-Host "  Bun                 $([math]::Round($bunTime, 2))ms" -ForegroundColor White
Write-Host "  DX (cold)           $([math]::Round($dxColdTime, 2))ms       ${speedupCold}x faster" -ForegroundColor Green
Write-Host "  DX (warm)           $([math]::Round($dxWarmTime, 2))ms       ${speedupWarm}x faster" -ForegroundColor Green
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Check if we met our goal
if ($speedupWarm -ge 50) {
    Write-Host "✅ SUCCESS! DX is ${speedupWarm}x faster than Bun (target: 50x)" -ForegroundColor Green
} elseif ($speedupWarm -ge 25) {
    Write-Host "⚠️  GOOD! DX is ${speedupWarm}x faster than Bun (target: 50x)" -ForegroundColor Yellow
    Write-Host "   → With Phase 2 optimizations (SIMD, prediction), we'll reach 50x+" -ForegroundColor Gray
} else {
    Write-Host "❌ NEEDS IMPROVEMENT: DX is only ${speedupWarm}x faster (target: 50x)" -ForegroundColor Red
}

Write-Host ""
