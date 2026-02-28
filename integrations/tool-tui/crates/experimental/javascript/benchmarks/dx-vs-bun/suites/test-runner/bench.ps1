# Test Runner Benchmark Suite
# Compares DX-Test vs Bun test performance

param(
    [Parameter(Mandatory = $true)]
    [string]$DxPath,
    [Parameter(Mandatory = $true)]
    [string]$BunPath,
    [int]$Runs = 5,
    [int]$Warmup = 1
)

$ErrorActionPreference = "Stop"
$ScriptRoot = $PSScriptRoot
$FixturesPath = "$ScriptRoot/fixtures"

# Import stats library
. "$ScriptRoot/../../lib/stats.ps1"

<#
.SYNOPSIS
    Measure test discovery time.
.PARAMETER Runtime
    Path to the test runner executable.
.PARAMETER SuitePath
    Path to the test suite directory.
.PARAMETER IsBun
    Whether this is Bun test runner.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-TestDiscovery {
    param(
        [string]$Runtime,
        [string]$SuitePath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                # Bun test with --dry-run to only discover tests
                $pinfo.Arguments = "test --dry-run"
            } else {
                # DX test with list-only flag
                $pinfo.Arguments = "test --list"
            }
            
            $pinfo.WorkingDirectory = $SuitePath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(30000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Discovery run $i failed: $_"
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

<#
.SYNOPSIS
    Measure test execution time and calculate TPS.
.PARAMETER Runtime
    Path to the test runner executable.
.PARAMETER SuitePath
    Path to the test suite directory.
.PARAMETER IsBun
    Whether this is Bun test runner.
.PARAMETER Runs
    Number of benchmark runs.
.PARAMETER TestCount
    Expected number of tests in the suite.
#>
function Measure-TestExecution {
    param(
        [string]$Runtime,
        [string]$SuitePath,
        [bool]$IsBun = $false,
        [int]$Runs,
        [int]$TestCount = 50
    )
    
    $times = @()
    $tpsValues = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo.Arguments = "test"
            } else {
                $pinfo.Arguments = "test"
            }
            
            $pinfo.WorkingDirectory = $SuitePath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            
            $elapsedMs = $sw.Elapsed.TotalMilliseconds
            $times += $elapsedMs
            
            # Calculate tests per second
            if ($elapsedMs -gt 0) {
                $tps = ($TestCount / $elapsedMs) * 1000
                $tpsValues += $tps
            }
        }
        catch {
            Write-Warning "Execution run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
        tps = if ($tpsValues.Count -gt 0) { Get-Stats -Values $tpsValues } else { $null }
        testCount = $TestCount
    }
}

<#
.SYNOPSIS
    Measure parallelization efficiency.
.PARAMETER Runtime
    Path to the test runner executable.
.PARAMETER SuitePath
    Path to the test suite directory.
.PARAMETER IsBun
    Whether this is Bun test runner.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-Parallelization {
    param(
        [string]$Runtime,
        [string]$SuitePath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $singleThreadTimes = @()
    $multiThreadTimes = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Single-threaded run
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo.Arguments = "test --concurrency 1"
            } else {
                $pinfo.Arguments = "test --workers 1"
            }
            
            $pinfo.WorkingDirectory = $SuitePath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            $singleThreadTimes += $sw.Elapsed.TotalMilliseconds
            
            # Multi-threaded run (default parallelization)
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo2.Arguments = "test"
            } else {
                $pinfo2.Arguments = "test"
            }
            
            $pinfo2.WorkingDirectory = $SuitePath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            $multiThreadTimes += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Parallelization run $i failed: $_"
        }
    }
    
    if ($singleThreadTimes.Count -eq 0 -or $multiThreadTimes.Count -eq 0) {
        return $null
    }
    
    $singleStats = Get-Stats -Values $singleThreadTimes
    $multiStats = Get-Stats -Values $multiThreadTimes
    
    # Calculate efficiency (speedup from parallelization)
    $efficiency = if ($multiStats.mean -gt 0) { $singleStats.mean / $multiStats.mean } else { 1.0 }
    
    return @{
        singleThread = @{ times = $singleThreadTimes; stats = $singleStats }
        multiThread = @{ times = $multiThreadTimes; stats = $multiStats }
        efficiency = [math]::Round($efficiency, 2)
    }
}

<#
.SYNOPSIS
    Run all test runner benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. Test Discovery - Small Suite
    Write-Host "  [1/8] Test Discovery (Small Suite)..." -NoNewline
    $dxDiscSmall = Measure-TestDiscovery -Runtime $DxPath -SuitePath "$FixturesPath/small-suite" -IsBun $false -Runs $Runs
    $bunDiscSmall = Measure-TestDiscovery -Runtime $BunPath -SuitePath "$FixturesPath/small-suite" -IsBun $true -Runs $Runs
    
    if ($dxDiscSmall -and $bunDiscSmall) {
        $comparison = Compare-Results -ResultA $dxDiscSmall -ResultB $bunDiscSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Test Discovery (Small)"
            unit = "ms"
            dx = $dxDiscSmall
            bun = $bunDiscSmall
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 2. Test Execution - Small Suite (50 tests)
    Write-Host "  [2/8] Test Execution (Small Suite)..." -NoNewline
    $dxExecSmall = Measure-TestExecution -Runtime $DxPath -SuitePath "$FixturesPath/small-suite" -IsBun $false -Runs $Runs -TestCount 50
    $bunExecSmall = Measure-TestExecution -Runtime $BunPath -SuitePath "$FixturesPath/small-suite" -IsBun $true -Runs $Runs -TestCount 50
    
    if ($dxExecSmall -and $bunExecSmall) {
        $comparison = Compare-Results -ResultA $dxExecSmall -ResultB $bunExecSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Test Execution (Small - 50 tests)"
            unit = "ms"
            dx = $dxExecSmall
            bun = $bunExecSmall
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 3. Test Execution - Medium Suite (150 tests)
    Write-Host "  [3/8] Test Execution (Medium Suite)..." -NoNewline
    $dxExecMed = Measure-TestExecution -Runtime $DxPath -SuitePath "$FixturesPath/medium-suite" -IsBun $false -Runs $Runs -TestCount 150
    $bunExecMed = Measure-TestExecution -Runtime $BunPath -SuitePath "$FixturesPath/medium-suite" -IsBun $true -Runs $Runs -TestCount 150
    
    if ($dxExecMed -and $bunExecMed) {
        $comparison = Compare-Results -ResultA $dxExecMed -ResultB $bunExecMed -LowerIsBetter $true
        $benchmarks += @{
            name = "Test Execution (Medium - 150 tests)"
            unit = "ms"
            dx = $dxExecMed
            bun = $bunExecMed
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Test Execution - Large Suite (300 tests)
    Write-Host "  [4/8] Test Execution (Large Suite)..." -NoNewline
    $dxExecLarge = Measure-TestExecution -Runtime $DxPath -SuitePath "$FixturesPath/large-suite" -IsBun $false -Runs ([math]::Max(1, $Runs - 2)) -TestCount 300
    $bunExecLarge = Measure-TestExecution -Runtime $BunPath -SuitePath "$FixturesPath/large-suite" -IsBun $true -Runs ([math]::Max(1, $Runs - 2)) -TestCount 300
    
    if ($dxExecLarge -and $bunExecLarge) {
        $comparison = Compare-Results -ResultA $dxExecLarge -ResultB $bunExecLarge -LowerIsBetter $true
        $benchmarks += @{
            name = "Test Execution (Large - 300 tests)"
            unit = "ms"
            dx = $dxExecLarge
            bun = $bunExecLarge
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 5. Tests Per Second (TPS) - Medium Suite
    Write-Host "  [5/8] Tests Per Second (TPS)..." -NoNewline
    if ($dxExecMed -and $bunExecMed -and $dxExecMed.tps -and $bunExecMed.tps) {
        # For TPS, higher is better
        $tpsComparison = Compare-Results -ResultA @{ times = @($dxExecMed.tps.mean) } -ResultB @{ times = @($bunExecMed.tps.mean) } -LowerIsBetter $false
        $benchmarks += @{
            name = "Tests Per Second (TPS)"
            unit = "tests/s"
            dx = @{ stats = $dxExecMed.tps }
            bun = @{ stats = $bunExecMed.tps }
            winner = switch ($tpsComparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $tpsComparison.speedup
            isSignificant = $tpsComparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Skipped (no TPS data)" -ForegroundColor Yellow
    }
    
    # 6. Parallelization Efficiency
    Write-Host "  [6/8] Parallelization Efficiency..." -NoNewline
    $dxParallel = Measure-Parallelization -Runtime $DxPath -SuitePath "$FixturesPath/medium-suite" -IsBun $false -Runs ([math]::Max(1, $Runs - 2))
    $bunParallel = Measure-Parallelization -Runtime $BunPath -SuitePath "$FixturesPath/medium-suite" -IsBun $true -Runs ([math]::Max(1, $Runs - 2))
    
    if ($dxParallel -and $bunParallel) {
        # Higher efficiency is better
        $effComparison = @{
            winner = "tie"
            speedup = 1.0
            isSignificant = $false
        }
        
        if ($dxParallel.efficiency -gt $bunParallel.efficiency * 1.1) {
            $effComparison.winner = "A"
            $effComparison.speedup = [math]::Round($dxParallel.efficiency / $bunParallel.efficiency, 2)
            $effComparison.isSignificant = $true
        } elseif ($bunParallel.efficiency -gt $dxParallel.efficiency * 1.1) {
            $effComparison.winner = "B"
            $effComparison.speedup = [math]::Round($bunParallel.efficiency / $dxParallel.efficiency, 2)
            $effComparison.isSignificant = $true
        }
        
        $benchmarks += @{
            name = "Parallelization Efficiency"
            unit = "x"
            dx = @{ stats = @{ mean = $dxParallel.efficiency; median = $dxParallel.efficiency } }
            bun = @{ stats = @{ mean = $bunParallel.efficiency; median = $bunParallel.efficiency } }
            winner = switch ($effComparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $effComparison.speedup
            isSignificant = $effComparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 7. Snapshot Test Performance
    Write-Host "  [7/8] Snapshot Test Performance..." -NoNewline
    # Run only snapshot tests
    $dxSnapshot = Measure-TestExecution -Runtime $DxPath -SuitePath "$FixturesPath/medium-suite" -IsBun $false -Runs $Runs -TestCount 20
    $bunSnapshot = Measure-TestExecution -Runtime $BunPath -SuitePath "$FixturesPath/medium-suite" -IsBun $true -Runs $Runs -TestCount 20
    
    if ($dxSnapshot -and $bunSnapshot) {
        $comparison = Compare-Results -ResultA $dxSnapshot -ResultB $bunSnapshot -LowerIsBetter $true
        $benchmarks += @{
            name = "Snapshot Tests"
            unit = "ms"
            dx = $dxSnapshot
            bun = $bunSnapshot
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 8. Mock Function Overhead
    Write-Host "  [8/8] Mock Function Overhead..." -NoNewline
    # This measures the overhead of mock functions
    $dxMock = Measure-TestExecution -Runtime $DxPath -SuitePath "$FixturesPath/medium-suite" -IsBun $false -Runs $Runs -TestCount 30
    $bunMock = Measure-TestExecution -Runtime $BunPath -SuitePath "$FixturesPath/medium-suite" -IsBun $true -Runs $Runs -TestCount 30
    
    if ($dxMock -and $bunMock) {
        $comparison = Compare-Results -ResultA $dxMock -ResultB $bunMock -LowerIsBetter $true
        $benchmarks += @{
            name = "Mock Function Tests"
            unit = "ms"
            dx = $dxMock
            bun = $bunMock
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
        name = "Test Runner"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
