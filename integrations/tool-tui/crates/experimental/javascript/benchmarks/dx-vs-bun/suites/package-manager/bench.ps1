# Package Manager Benchmark Suite
# Compares DX package manager vs Bun install performance

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
    Clean up installed packages and caches for cold install benchmark.
.PARAMETER ProjectPath
    Path to the project directory.
.PARAMETER ClearCache
    Whether to clear package manager caches.
#>
function Clear-InstallArtifacts {
    param(
        [string]$ProjectPath,
        [bool]$ClearCache = $false
    )
    
    # Remove node_modules
    $nodeModules = Join-Path $ProjectPath "node_modules"
    if (Test-Path $nodeModules) {
        Remove-Item -Recurse -Force $nodeModules -ErrorAction SilentlyContinue
    }
    
    # Remove lock files
    Remove-Item -Path (Join-Path $ProjectPath "package-lock.json") -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $ProjectPath "bun.lockb") -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $ProjectPath "dx.lock") -ErrorAction SilentlyContinue
    
    if ($ClearCache) {
        # Clear Bun cache
        $bunCache = Join-Path $env:USERPROFILE ".bun"
        if (Test-Path $bunCache) {
            Remove-Item -Recurse -Force (Join-Path $bunCache "install") -ErrorAction SilentlyContinue
        }
        
        # Clear DX cache
        $dxCache = Join-Path $env:USERPROFILE ".dx-cache"
        if (Test-Path $dxCache) {
            Remove-Item -Recurse -Force $dxCache -ErrorAction SilentlyContinue
        }
        
        # Clear npm cache (in case DX uses it)
        $npmCache = Join-Path $env:APPDATA "npm-cache"
        if (Test-Path $npmCache) {
            Remove-Item -Recurse -Force $npmCache -ErrorAction SilentlyContinue
        }
    }
}


<#
.SYNOPSIS
    Measure package installation time.
.PARAMETER Runtime
    Path to the runtime/package manager executable.
.PARAMETER ProjectPath
    Path to the project to install.
.PARAMETER IsBun
    Whether this is Bun (affects install command).
.PARAMETER Runs
    Number of benchmark runs.
.PARAMETER ClearCache
    Whether to clear caches between runs (cold install).
#>
function Measure-InstallTime {
    param(
        [string]$Runtime,
        [string]$ProjectPath,
        [bool]$IsBun = $false,
        [int]$Runs,
        [bool]$ClearCache = $false
    )
    
    $times = @()
    $installCmd = if ($IsBun) { "install" } else { "install" }
    
    for ($i = 0; $i -lt $Runs; $i++) {
        # Clean up before each run
        Clear-InstallArtifacts -ProjectPath $ProjectPath -ClearCache $ClearCache
        
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            $pinfo.Arguments = $installCmd
            $pinfo.WorkingDirectory = $ProjectPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit()
            
            $sw.Stop()
            
            if ($process.ExitCode -eq 0) {
                $times += $sw.Elapsed.TotalMilliseconds
            } else {
                $stderr = $process.StandardError.ReadToEnd()
                Write-Warning "Install failed (run $i): $stderr"
            }
        }
        catch {
            Write-Warning "Install run $i failed: $_"
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
    Run all package manager benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. Cold Install - Small Project
    Write-Host "  [1/4] Cold Install (Small Project)..." -NoNewline
    $dxColdSmall = Measure-InstallTime -Runtime $DxPath -ProjectPath "$FixturesPath/small-project" -IsBun $false -Runs $Runs -ClearCache $true
    $bunColdSmall = Measure-InstallTime -Runtime $BunPath -ProjectPath "$FixturesPath/small-project" -IsBun $true -Runs $Runs -ClearCache $true
    
    if ($dxColdSmall -and $bunColdSmall) {
        $comparison = Compare-Results -ResultA $dxColdSmall -ResultB $bunColdSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Install (Small)"
            unit = "ms"
            dx = $dxColdSmall
            bun = $bunColdSmall
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 2. Warm Install - Small Project
    Write-Host "  [2/4] Warm Install (Small Project)..." -NoNewline
    $dxWarmSmall = Measure-InstallTime -Runtime $DxPath -ProjectPath "$FixturesPath/small-project" -IsBun $false -Runs $Runs -ClearCache $false
    $bunWarmSmall = Measure-InstallTime -Runtime $BunPath -ProjectPath "$FixturesPath/small-project" -IsBun $true -Runs $Runs -ClearCache $false
    
    if ($dxWarmSmall -and $bunWarmSmall) {
        $comparison = Compare-Results -ResultA $dxWarmSmall -ResultB $bunWarmSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Warm Install (Small)"
            unit = "ms"
            dx = $dxWarmSmall
            bun = $bunWarmSmall
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 3. Cold Install - Large Project
    Write-Host "  [3/4] Cold Install (Large Project)..." -NoNewline
    $dxColdLarge = Measure-InstallTime -Runtime $DxPath -ProjectPath "$FixturesPath/large-project" -IsBun $false -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $true
    $bunColdLarge = Measure-InstallTime -Runtime $BunPath -ProjectPath "$FixturesPath/large-project" -IsBun $true -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $true
    
    if ($dxColdLarge -and $bunColdLarge) {
        $comparison = Compare-Results -ResultA $dxColdLarge -ResultB $bunColdLarge -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Install (Large)"
            unit = "ms"
            dx = $dxColdLarge
            bun = $bunColdLarge
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Warm Install - Large Project
    Write-Host "  [4/4] Warm Install (Large Project)..." -NoNewline
    $dxWarmLarge = Measure-InstallTime -Runtime $DxPath -ProjectPath "$FixturesPath/large-project" -IsBun $false -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $false
    $bunWarmLarge = Measure-InstallTime -Runtime $BunPath -ProjectPath "$FixturesPath/large-project" -IsBun $true -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $false
    
    if ($dxWarmLarge -and $bunWarmLarge) {
        $comparison = Compare-Results -ResultA $dxWarmLarge -ResultB $bunWarmLarge -LowerIsBetter $true
        $benchmarks += @{
            name = "Warm Install (Large)"
            unit = "ms"
            dx = $dxWarmLarge
            bun = $bunWarmLarge
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
        name = "Package Manager"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
