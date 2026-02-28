# Runtime Benchmark Suite
# Compares DX-JS runtime vs Bun runtime performance

param(
    [Parameter(Mandatory = $true)]
    [string]$DxPath,
    [Parameter(Mandatory = $true)]
    [string]$BunPath,
    [int]$Runs = 10,
    [int]$Warmup = 3
)

$ErrorActionPreference = "Stop"
$ScriptRoot = $PSScriptRoot
$FixturesPath = "$ScriptRoot/fixtures"

# Import stats library
. "$ScriptRoot/../../lib/stats.ps1"

function Invoke-RuntimeBenchmark {
    param(
        [string]$Runtime,
        [string]$Script,
        [int]$Runs,
        [int]$Warmup
    )
    
    $times = @()
    $totalRuns = $Runs + $Warmup
    
    for ($i = 0; $i -lt $totalRuns; $i++) {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            & $Runtime $Script 2>&1 | Out-Null
            $sw.Stop()
            
            if ($i -ge $Warmup) {
                $times += $sw.Elapsed.TotalMilliseconds
            }
        }
        catch {
            Write-Warning "Run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
    }
}

function Measure-StartupTime {
    param(
        [string]$Runtime,
        [string]$Script,
        [int]$Runs,
        [bool]$ClearCache = $false
    )
    
    $times = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        if ($ClearCache) {
            $cacheDir = [System.IO.Path]::GetTempPath()
            Get-ChildItem -Path $cacheDir -Filter "bun-*" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
            Get-ChildItem -Path $cacheDir -Filter "dx-*" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
        }
        
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            & $Runtime $Script 2>&1 | Out-Null
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds * 1000
        }
        catch {
            Write-Warning "Startup run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
    }
}

function Measure-MemoryUsage {
    param(
        [string]$Runtime,
        [string]$Script,
        [int]$Runs
    )
    
    $memoryValues = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            $process = Start-Process -FilePath $Runtime -ArgumentList $Script -PassThru -NoNewWindow -RedirectStandardOutput "NUL" -RedirectStandardError "NUL"
            Start-Sleep -Milliseconds 100
            
            $peakMem = 0
            while (-not $process.HasExited) {
                try {
                    $process.Refresh()
                    if ($process.PeakWorkingSet64 -gt $peakMem) {
                        $peakMem = $process.PeakWorkingSet64
                    }
                }
                catch { }
                Start-Sleep -Milliseconds 10
            }
            
            $process.WaitForExit()
            $memoryValues += $peakMem / 1024 / 1024
        }
        catch {
            Write-Warning "Memory run $i failed: $_"
        }
    }
    
    if ($memoryValues.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $memoryValues
        stats = Get-Stats -Values $memoryValues
    }
}

function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. CPU Benchmark - Fibonacci
    Write-Host "  [1/6] CPU Benchmark (Fibonacci)..." -NoNewline
    $dxFib = Invoke-RuntimeBenchmark -Runtime $DxPath -Script "$FixturesPath/fibonacci.js" -Runs $Runs -Warmup $Warmup
    $bunFib = Invoke-RuntimeBenchmark -Runtime $BunPath -Script "$FixturesPath/fibonacci.js" -Runs $Runs -Warmup $Warmup
    
    if ($dxFib -and $bunFib) {
        $comparison = Compare-Results -ResultA $dxFib -ResultB $bunFib -LowerIsBetter $true
        $benchmarks += @{
            name = "Fibonacci (CPU)"
            unit = "ms"
            dx = $dxFib
            bun = $bunFib
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 2. JSON Parsing Benchmark
    Write-Host "  [2/6] JSON Parsing..." -NoNewline
    $dxJson = Invoke-RuntimeBenchmark -Runtime $DxPath -Script "$FixturesPath/json-parse.js" -Runs $Runs -Warmup $Warmup
    $bunJson = Invoke-RuntimeBenchmark -Runtime $BunPath -Script "$FixturesPath/json-parse.js" -Runs $Runs -Warmup $Warmup
    
    if ($dxJson -and $bunJson) {
        $comparison = Compare-Results -ResultA $dxJson -ResultB $bunJson -LowerIsBetter $true
        $benchmarks += @{
            name = "JSON Parse/Stringify"
            unit = "ms"
            dx = $dxJson
            bun = $bunJson
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 3. Cold Startup Time
    Write-Host "  [3/6] Cold Startup Time..." -NoNewline
    $dxCold = Measure-StartupTime -Runtime $DxPath -Script "$FixturesPath/hello.js" -Runs $Runs -ClearCache $true
    $bunCold = Measure-StartupTime -Runtime $BunPath -Script "$FixturesPath/hello.js" -Runs $Runs -ClearCache $true
    
    if ($dxCold -and $bunCold) {
        $comparison = Compare-Results -ResultA $dxCold -ResultB $bunCold -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Startup"
            unit = "us"
            dx = $dxCold
            bun = $bunCold
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Warm Startup Time
    Write-Host "  [4/6] Warm Startup Time..." -NoNewline
    $dxWarm = Measure-StartupTime -Runtime $DxPath -Script "$FixturesPath/hello.js" -Runs $Runs -ClearCache $false
    $bunWarm = Measure-StartupTime -Runtime $BunPath -Script "$FixturesPath/hello.js" -Runs $Runs -ClearCache $false
    
    if ($dxWarm -and $bunWarm) {
        $comparison = Compare-Results -ResultA $dxWarm -ResultB $bunWarm -LowerIsBetter $true
        $benchmarks += @{
            name = "Warm Startup"
            unit = "us"
            dx = $dxWarm
            bun = $bunWarm
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 5. Memory Usage
    Write-Host "  [5/6] Memory Usage..." -NoNewline
    $dxMem = Measure-MemoryUsage -Runtime $DxPath -Script "$FixturesPath/memory-stress.js" -Runs ([math]::Min($Runs, 5))
    $bunMem = Measure-MemoryUsage -Runtime $BunPath -Script "$FixturesPath/memory-stress.js" -Runs ([math]::Min($Runs, 5))
    
    if ($dxMem -and $bunMem) {
        $comparison = Compare-Results -ResultA $dxMem -ResultB $bunMem -LowerIsBetter $true
        $benchmarks += @{
            name = "Memory Usage"
            unit = "MB"
            dx = $dxMem
            bun = $bunMem
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 6. Async Concurrency
    Write-Host "  [6/6] Async Concurrency..." -NoNewline
    $dxAsync = Invoke-RuntimeBenchmark -Runtime $DxPath -Script "$FixturesPath/async-concurrent.js" -Runs $Runs -Warmup $Warmup
    $bunAsync = Invoke-RuntimeBenchmark -Runtime $BunPath -Script "$FixturesPath/async-concurrent.js" -Runs $Runs -Warmup $Warmup
    
    if ($dxAsync -and $bunAsync) {
        $comparison = Compare-Results -ResultA $dxAsync -ResultB $bunAsync -LowerIsBetter $true
        $benchmarks += @{
            name = "Async Concurrency"
            unit = "ms"
            dx = $dxAsync
            bun = $bunAsync
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # Calculate suite winner
    $dxWins = ($benchmarks | Where-Object { $_.winner -eq "dx" }).Count
    $bunWins = ($benchmarks | Where-Object { $_.winner -eq "bun" }).Count
    
    $suiteWinner = "tie"
    if ($dxWins -gt $bunWins) { $suiteWinner = "dx" }
    elseif ($bunWins -gt $dxWins) { $suiteWinner = "bun" }
    
    $avgSpeedup = 1.0
    $speedups = $benchmarks | Where-Object { $_.speedup -gt 1 } | ForEach-Object { $_.speedup }
    if ($speedups.Count -gt 0) {
        $avgSpeedup = ($speedups | Measure-Object -Average).Average
    }
    
    return @{
        name = "Runtime"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
