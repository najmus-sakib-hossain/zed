# Project Manager Benchmark Suite
# Compares DX-Project vs Bun workspace performance

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
    Measure workspace discovery time.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-WorkspaceDiscovery {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
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
                # Bun workspace list
                $pinfo.Arguments = "pm ls"
            } else {
                # DX project workspace list
                $pinfo.Arguments = "workspace list"
            }
            
            $pinfo.WorkingDirectory = $MonorepoPath
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
    Measure task graph construction time.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-TaskGraphConstruction {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
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
                # Bun doesn't have explicit task graph, use dry-run
                $pinfo.Arguments = "run --filter '*' build --dry-run"
            } else {
                # DX project task graph
                $pinfo.Arguments = "run build --graph --dry-run"
            }
            
            $pinfo.WorkingDirectory = $MonorepoPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(60000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Task graph run $i failed: $_"
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
    Measure affected package detection time.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-AffectedDetection {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    
    # Create a temporary change to detect affected packages
    $testFile = "$MonorepoPath/packages/pkg-001/src/index.js"
    $originalContent = $null
    
    if (Test-Path $testFile) {
        $originalContent = Get-Content $testFile -Raw
    }
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Simulate a file change by touching the file
            if ($originalContent) {
                $newContent = $originalContent + "`n// Change $i at $(Get-Date)"
                Set-Content -Path $testFile -Value $newContent
            }
            
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                # Bun filter for affected packages
                $pinfo.Arguments = "run --filter '...[HEAD]' build --dry-run"
            } else {
                # DX project affected detection
                $pinfo.Arguments = "affected --base HEAD~1"
            }
            
            $pinfo.WorkingDirectory = $MonorepoPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(60000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Affected detection run $i failed: $_"
        }
    }
    
    # Restore original file
    if ($originalContent) {
        Set-Content -Path $testFile -Value $originalContent
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
    Measure cache hit vs miss performance.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-CachePerformance {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $coldTimes = @()
    $warmTimes = @()
    
    # Cache directories
    $dxCacheDir = "$MonorepoPath/.dx-cache"
    $bunCacheDir = "$MonorepoPath/.bun-cache"
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Cold run - clear cache first
            if (Test-Path $dxCacheDir) { Remove-Item -Recurse -Force $dxCacheDir }
            if (Test-Path $bunCacheDir) { Remove-Item -Recurse -Force $bunCacheDir }
            
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo.Arguments = "run --filter '*' build"
            } else {
                $pinfo.Arguments = "run build"
            }
            
            $pinfo.WorkingDirectory = $MonorepoPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            $coldTimes += $sw.Elapsed.TotalMilliseconds
            
            # Warm run - cache should be populated
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo2.Arguments = "run --filter '*' build"
            } else {
                $pinfo2.Arguments = "run build"
            }
            
            $pinfo2.WorkingDirectory = $MonorepoPath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            $warmTimes += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Cache performance run $i failed: $_"
        }
    }
    
    if ($coldTimes.Count -eq 0 -or $warmTimes.Count -eq 0) {
        return $null
    }
    
    $coldStats = Get-Stats -Values $coldTimes
    $warmStats = Get-Stats -Values $warmTimes
    
    # Calculate cache effectiveness (speedup from cache)
    $cacheSpeedup = if ($warmStats.mean -gt 0) { $coldStats.mean / $warmStats.mean } else { 1.0 }
    
    return @{
        cold = @{ times = $coldTimes; stats = $coldStats }
        warm = @{ times = $warmTimes; stats = $warmStats }
        cacheSpeedup = [math]::Round($cacheSpeedup, 2)
    }
}

<#
.SYNOPSIS
    Measure parallel task execution efficiency.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-ParallelExecution {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $serialTimes = @()
    $parallelTimes = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Serial execution
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo.Arguments = "run --filter '*' --concurrency 1 build"
            } else {
                $pinfo.Arguments = "run build --concurrency 1"
            }
            
            $pinfo.WorkingDirectory = $MonorepoPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(180000) | Out-Null
            
            $sw.Stop()
            $serialTimes += $sw.Elapsed.TotalMilliseconds
            
            # Parallel execution (default)
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo2.Arguments = "run --filter '*' build"
            } else {
                $pinfo2.Arguments = "run build"
            }
            
            $pinfo2.WorkingDirectory = $MonorepoPath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(180000) | Out-Null
            
            $sw.Stop()
            $parallelTimes += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Parallel execution run $i failed: $_"
        }
    }
    
    if ($serialTimes.Count -eq 0 -or $parallelTimes.Count -eq 0) {
        return $null
    }
    
    $serialStats = Get-Stats -Values $serialTimes
    $parallelStats = Get-Stats -Values $parallelTimes
    
    # Calculate parallelization efficiency
    $efficiency = if ($parallelStats.mean -gt 0) { $serialStats.mean / $parallelStats.mean } else { 1.0 }
    
    return @{
        serial = @{ times = $serialTimes; stats = $serialStats }
        parallel = @{ times = $parallelTimes; stats = $parallelStats }
        efficiency = [math]::Round($efficiency, 2)
    }
}


<#
.SYNOPSIS
    Run all project manager benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. Workspace Discovery - 10 packages
    Write-Host "  [1/10] Workspace Discovery (10 packages)..." -NoNewline
    $dxDisc10 = Measure-WorkspaceDiscovery -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $false -Runs $Runs
    $bunDisc10 = Measure-WorkspaceDiscovery -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $true -Runs $Runs
    
    if ($dxDisc10 -and $bunDisc10) {
        $comparison = Compare-Results -ResultA $dxDisc10 -ResultB $bunDisc10 -LowerIsBetter $true
        $benchmarks += @{
            name = "Workspace Discovery (10 packages)"
            unit = "ms"
            dx = $dxDisc10
            bun = $bunDisc10
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 2. Workspace Discovery - 50 packages
    Write-Host "  [2/10] Workspace Discovery (50 packages)..." -NoNewline
    $dxDisc50 = Measure-WorkspaceDiscovery -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $false -Runs $Runs
    $bunDisc50 = Measure-WorkspaceDiscovery -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $true -Runs $Runs
    
    if ($dxDisc50 -and $bunDisc50) {
        $comparison = Compare-Results -ResultA $dxDisc50 -ResultB $bunDisc50 -LowerIsBetter $true
        $benchmarks += @{
            name = "Workspace Discovery (50 packages)"
            unit = "ms"
            dx = $dxDisc50
            bun = $bunDisc50
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 3. Workspace Discovery - 100 packages
    Write-Host "  [3/10] Workspace Discovery (100 packages)..." -NoNewline
    $dxDisc100 = Measure-WorkspaceDiscovery -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-100" -IsBun $false -Runs $Runs
    $bunDisc100 = Measure-WorkspaceDiscovery -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-100" -IsBun $true -Runs $Runs
    
    if ($dxDisc100 -and $bunDisc100) {
        $comparison = Compare-Results -ResultA $dxDisc100 -ResultB $bunDisc100 -LowerIsBetter $true
        $benchmarks += @{
            name = "Workspace Discovery (100 packages)"
            unit = "ms"
            dx = $dxDisc100
            bun = $bunDisc100
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Task Graph Construction - 10 packages
    Write-Host "  [4/10] Task Graph Construction (10 packages)..." -NoNewline
    $dxGraph10 = Measure-TaskGraphConstruction -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $false -Runs $Runs
    $bunGraph10 = Measure-TaskGraphConstruction -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $true -Runs $Runs
    
    if ($dxGraph10 -and $bunGraph10) {
        $comparison = Compare-Results -ResultA $dxGraph10 -ResultB $bunGraph10 -LowerIsBetter $true
        $benchmarks += @{
            name = "Task Graph Construction (10 packages)"
            unit = "ms"
            dx = $dxGraph10
            bun = $bunGraph10
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 5. Task Graph Construction - 50 packages
    Write-Host "  [5/10] Task Graph Construction (50 packages)..." -NoNewline
    $dxGraph50 = Measure-TaskGraphConstruction -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $false -Runs $Runs
    $bunGraph50 = Measure-TaskGraphConstruction -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $true -Runs $Runs
    
    if ($dxGraph50 -and $bunGraph50) {
        $comparison = Compare-Results -ResultA $dxGraph50 -ResultB $bunGraph50 -LowerIsBetter $true
        $benchmarks += @{
            name = "Task Graph Construction (50 packages)"
            unit = "ms"
            dx = $dxGraph50
            bun = $bunGraph50
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 6. Affected Detection - 50 packages
    Write-Host "  [6/10] Affected Detection (50 packages)..." -NoNewline
    $dxAffected = Measure-AffectedDetection -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $false -Runs $Runs
    $bunAffected = Measure-AffectedDetection -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $true -Runs $Runs
    
    if ($dxAffected -and $bunAffected) {
        $comparison = Compare-Results -ResultA $dxAffected -ResultB $bunAffected -LowerIsBetter $true
        $benchmarks += @{
            name = "Affected Detection (50 packages)"
            unit = "ms"
            dx = $dxAffected
            bun = $bunAffected
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 7. Cache Performance - Cold vs Warm (10 packages)
    Write-Host "  [7/10] Cache Performance (10 packages)..." -NoNewline
    $dxCache10 = Measure-CachePerformance -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $false -Runs ([math]::Max(1, $Runs - 2))
    $bunCache10 = Measure-CachePerformance -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-10" -IsBun $true -Runs ([math]::Max(1, $Runs - 2))
    
    if ($dxCache10 -and $bunCache10) {
        # Compare warm (cached) performance
        $comparison = Compare-Results -ResultA $dxCache10.warm -ResultB $bunCache10.warm -LowerIsBetter $true
        $benchmarks += @{
            name = "Cache Hit Performance (10 packages)"
            unit = "ms"
            dx = $dxCache10.warm
            bun = $bunCache10.warm
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
            extra = @{
                dxCacheSpeedup = $dxCache10.cacheSpeedup
                bunCacheSpeedup = $bunCache10.cacheSpeedup
            }
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 8. Cache Miss Performance (Cold) - 10 packages
    Write-Host "  [8/10] Cache Miss Performance (10 packages)..." -NoNewline
    if ($dxCache10 -and $bunCache10) {
        $comparison = Compare-Results -ResultA $dxCache10.cold -ResultB $bunCache10.cold -LowerIsBetter $true
        $benchmarks += @{
            name = "Cache Miss Performance (10 packages)"
            unit = "ms"
            dx = $dxCache10.cold
            bun = $bunCache10.cold
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Skipped" -ForegroundColor Yellow
    }
    
    # 9. Parallel Execution Efficiency - 50 packages
    Write-Host "  [9/10] Parallel Execution (50 packages)..." -NoNewline
    $dxParallel = Measure-ParallelExecution -Runtime $DxPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $false -Runs ([math]::Max(1, $Runs - 2))
    $bunParallel = Measure-ParallelExecution -Runtime $BunPath -MonorepoPath "$FixturesPath/monorepo-50" -IsBun $true -Runs ([math]::Max(1, $Runs - 2))
    
    if ($dxParallel -and $bunParallel) {
        # Compare parallel execution time
        $comparison = Compare-Results -ResultA $dxParallel.parallel -ResultB $bunParallel.parallel -LowerIsBetter $true
        $benchmarks += @{
            name = "Parallel Task Execution (50 packages)"
            unit = "ms"
            dx = $dxParallel.parallel
            bun = $bunParallel.parallel
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
            extra = @{
                dxEfficiency = $dxParallel.efficiency
                bunEfficiency = $bunParallel.efficiency
            }
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 10. Parallelization Efficiency Comparison
    Write-Host "  [10/10] Parallelization Efficiency..." -NoNewline
    if ($dxParallel -and $bunParallel) {
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
        Write-Host " Skipped" -ForegroundColor Yellow
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
        name = "Project Manager"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
