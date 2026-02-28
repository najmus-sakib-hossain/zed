# Bundler Benchmark Suite
# Compares DX bundler vs Bun bundler performance

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
    Clean up bundle output and caches.
.PARAMETER ProjectPath
    Path to the project directory.
.PARAMETER ClearCache
    Whether to clear bundler caches.
#>
function Clear-BundleArtifacts {
    param(
        [string]$ProjectPath,
        [bool]$ClearCache = $false
    )
    
    # Remove output directories
    $distDir = Join-Path $ProjectPath "dist"
    if (Test-Path $distDir) {
        Remove-Item -Recurse -Force $distDir -ErrorAction SilentlyContinue
    }
    
    $outDir = Join-Path $ProjectPath "out"
    if (Test-Path $outDir) {
        Remove-Item -Recurse -Force $outDir -ErrorAction SilentlyContinue
    }
    
    # Remove bundle files
    Get-ChildItem -Path $ProjectPath -Filter "*.bundle.js" -ErrorAction SilentlyContinue | Remove-Item -Force
    
    if ($ClearCache) {
        # Clear Bun cache
        $bunCache = Join-Path $env:USERPROFILE ".bun"
        if (Test-Path (Join-Path $bunCache "cache")) {
            Remove-Item -Recurse -Force (Join-Path $bunCache "cache") -ErrorAction SilentlyContinue
        }
        
        # Clear DX bundler cache
        $dxCache = Join-Path $ProjectPath ".dx-cache"
        if (Test-Path $dxCache) {
            Remove-Item -Recurse -Force $dxCache -ErrorAction SilentlyContinue
        }
    }
}


<#
.SYNOPSIS
    Measure bundle time for a project.
.PARAMETER Runtime
    Path to the runtime/bundler executable.
.PARAMETER ProjectPath
    Path to the project to bundle.
.PARAMETER EntryPoint
    Entry point file relative to project.
.PARAMETER OutputFile
    Output bundle file path.
.PARAMETER IsBun
    Whether this is Bun (affects bundle command).
.PARAMETER Runs
    Number of benchmark runs.
.PARAMETER ClearCache
    Whether to clear caches between runs (cold bundle).
#>
function Measure-BundleTime {
    param(
        [string]$Runtime,
        [string]$ProjectPath,
        [string]$EntryPoint = "src/index.js",
        [string]$OutputFile = "dist/bundle.js",
        [bool]$IsBun = $false,
        [int]$Runs,
        [bool]$ClearCache = $false
    )
    
    $times = @()
    $bundleSizes = @()
    $entryPath = Join-Path $ProjectPath $EntryPoint
    $outPath = Join-Path $ProjectPath $OutputFile
    
    for ($i = 0; $i -lt $Runs; $i++) {
        # Clean up before each run
        Clear-BundleArtifacts -ProjectPath $ProjectPath -ClearCache $ClearCache
        
        # Ensure output directory exists
        $outDir = Split-Path $outPath -Parent
        if (-not (Test-Path $outDir)) {
            New-Item -ItemType Directory -Path $outDir -Force | Out-Null
        }
        
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                $pinfo.Arguments = "build `"$entryPath`" --outfile `"$outPath`""
            } else {
                # DX bundler command
                $pinfo.Arguments = "bundle `"$entryPath`" --output `"$outPath`""
            }
            
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
            
            if ($process.ExitCode -eq 0 -and (Test-Path $outPath)) {
                $times += $sw.Elapsed.TotalMilliseconds
                $bundleSizes += (Get-Item $outPath).Length
            } else {
                $stderr = $process.StandardError.ReadToEnd()
                Write-Warning "Bundle failed (run $i): $stderr"
            }
        }
        catch {
            Write-Warning "Bundle run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
        bundleSize = if ($bundleSizes.Count -gt 0) { ($bundleSizes | Measure-Object -Average).Average } else { 0 }
    }
}

<#
.SYNOPSIS
    Run all bundler benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. Cold Bundle - Small Project
    Write-Host "  [1/6] Cold Bundle (Small Project)..." -NoNewline
    $dxColdSmall = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/small-project" -IsBun $false -Runs $Runs -ClearCache $true
    $bunColdSmall = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/small-project" -IsBun $true -Runs $Runs -ClearCache $true
    
    if ($dxColdSmall -and $bunColdSmall) {
        $comparison = Compare-Results -ResultA $dxColdSmall -ResultB $bunColdSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Bundle (Small)"
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
    
    # 2. Warm Bundle - Small Project
    Write-Host "  [2/6] Warm Bundle (Small Project)..." -NoNewline
    $dxWarmSmall = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/small-project" -IsBun $false -Runs $Runs -ClearCache $false
    $bunWarmSmall = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/small-project" -IsBun $true -Runs $Runs -ClearCache $false
    
    if ($dxWarmSmall -and $bunWarmSmall) {
        $comparison = Compare-Results -ResultA $dxWarmSmall -ResultB $bunWarmSmall -LowerIsBetter $true
        $benchmarks += @{
            name = "Warm Bundle (Small)"
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
    
    # 3. Cold Bundle - Medium Project
    Write-Host "  [3/6] Cold Bundle (Medium Project)..." -NoNewline
    $dxColdMed = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/medium-project" -IsBun $false -Runs $Runs -ClearCache $true
    $bunColdMed = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/medium-project" -IsBun $true -Runs $Runs -ClearCache $true
    
    if ($dxColdMed -and $bunColdMed) {
        $comparison = Compare-Results -ResultA $dxColdMed -ResultB $bunColdMed -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Bundle (Medium)"
            unit = "ms"
            dx = $dxColdMed
            bun = $bunColdMed
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Cold Bundle - Large Project
    Write-Host "  [4/6] Cold Bundle (Large Project)..." -NoNewline
    $dxColdLarge = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/large-project" -IsBun $false -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $true
    $bunColdLarge = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/large-project" -IsBun $true -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $true
    
    if ($dxColdLarge -and $bunColdLarge) {
        $comparison = Compare-Results -ResultA $dxColdLarge -ResultB $bunColdLarge -LowerIsBetter $true
        $benchmarks += @{
            name = "Cold Bundle (Large)"
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
    
    # 5. Warm Bundle - Large Project
    Write-Host "  [5/6] Warm Bundle (Large Project)..." -NoNewline
    $dxWarmLarge = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/large-project" -IsBun $false -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $false
    $bunWarmLarge = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/large-project" -IsBun $true -Runs ([math]::Max(1, $Runs - 2)) -ClearCache $false
    
    if ($dxWarmLarge -and $bunWarmLarge) {
        $comparison = Compare-Results -ResultA $dxWarmLarge -ResultB $bunWarmLarge -LowerIsBetter $true
        $benchmarks += @{
            name = "Warm Bundle (Large)"
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
    
    # 6. Tree-Shaking Test
    Write-Host "  [6/6] Tree-Shaking Effectiveness..." -NoNewline
    $dxTreeShake = Measure-BundleTime -Runtime $DxPath -ProjectPath "$FixturesPath/tree-shaking-test" -IsBun $false -Runs $Runs -ClearCache $true
    $bunTreeShake = Measure-BundleTime -Runtime $BunPath -ProjectPath "$FixturesPath/tree-shaking-test" -IsBun $true -Runs $Runs -ClearCache $true
    
    if ($dxTreeShake -and $bunTreeShake) {
        # For tree-shaking, smaller bundle size is better
        $sizeComparison = @{
            winner = "tie"
            speedup = 1.0
            isSignificant = $false
        }
        
        if ($dxTreeShake.bundleSize -lt $bunTreeShake.bundleSize * 0.95) {
            $sizeComparison.winner = "A"
            $sizeComparison.speedup = [math]::Round($bunTreeShake.bundleSize / $dxTreeShake.bundleSize, 2)
            $sizeComparison.isSignificant = $true
        } elseif ($bunTreeShake.bundleSize -lt $dxTreeShake.bundleSize * 0.95) {
            $sizeComparison.winner = "B"
            $sizeComparison.speedup = [math]::Round($dxTreeShake.bundleSize / $bunTreeShake.bundleSize, 2)
            $sizeComparison.isSignificant = $true
        }
        
        $benchmarks += @{
            name = "Tree-Shaking (Bundle Size)"
            unit = "bytes"
            dx = @{ bundleSize = $dxTreeShake.bundleSize; stats = @{ mean = $dxTreeShake.bundleSize } }
            bun = @{ bundleSize = $bunTreeShake.bundleSize; stats = @{ mean = $bunTreeShake.bundleSize } }
            winner = switch ($sizeComparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $sizeComparison.speedup
            isSignificant = $sizeComparison.isSignificant
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
        name = "Bundler"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
