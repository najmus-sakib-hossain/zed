# End-to-End Workflow Benchmark Suite
# Compares DX vs Bun for complete development workflows

param(
    [Parameter(Mandatory = $true)]
    [string]$DxPath,
    [Parameter(Mandatory = $true)]
    [string]$BunPath,
    [int]$Runs = 3,
    [int]$Warmup = 1
)

$ErrorActionPreference = "Stop"
$ScriptRoot = $PSScriptRoot
$FixturesPath = "$ScriptRoot/fixtures"

# Import stats library
. "$ScriptRoot/../../lib/stats.ps1"

<#
.SYNOPSIS
    Measure fresh project setup time (install + first build).
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER ProjectPath
    Path to the project directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-FreshProjectSetup {
    param(
        [string]$Runtime,
        [string]$ProjectPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    $memoryUsage = @()
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Clean up before each run
            $nodeModules = "$ProjectPath/node_modules"
            $lockFile = if ($IsBun) { "$ProjectPath/bun.lockb" } else { "$ProjectPath/dx.lock" }
            
            if (Test-Path $nodeModules) { Remove-Item -Recurse -Force $nodeModules }
            if (Test-Path $lockFile) { Remove-Item -Force $lockFile }
            
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            # Step 1: Install dependencies
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            $pinfo.Arguments = "install"
            $pinfo.WorkingDirectory = $ProjectPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            
            # Step 2: Run build
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            $pinfo2.Arguments = "run build"
            $pinfo2.WorkingDirectory = $ProjectPath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(60000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
            
            # Measure memory (approximate via node_modules size)
            if (Test-Path $nodeModules) {
                $size = (Get-ChildItem -Recurse $nodeModules | Measure-Object -Property Length -Sum).Sum / 1MB
                $memoryUsage += $size
            }
        }
        catch {
            Write-Warning "Fresh setup run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
        memoryMB = if ($memoryUsage.Count -gt 0) { Get-Stats -Values $memoryUsage } else { $null }
    }
}


<#
.SYNOPSIS
    Measure development iteration time (file change → test → rebuild).
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER ProjectPath
    Path to the project directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-DevIteration {
    param(
        [string]$Runtime,
        [string]$ProjectPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    
    # Ensure dependencies are installed first
    $nodeModules = "$ProjectPath/node_modules"
    if (-not (Test-Path $nodeModules)) {
        $pinfo = New-Object System.Diagnostics.ProcessStartInfo
        $pinfo.FileName = $Runtime
        $pinfo.Arguments = "install"
        $pinfo.WorkingDirectory = $ProjectPath
        $pinfo.RedirectStandardOutput = $true
        $pinfo.RedirectStandardError = $true
        $pinfo.UseShellExecute = $false
        $pinfo.CreateNoWindow = $true
        
        $process = New-Object System.Diagnostics.Process
        $process.StartInfo = $pinfo
        $process.Start() | Out-Null
        $process.WaitForExit(120000) | Out-Null
    }
    
    $testFile = "$ProjectPath/src/utils.js"
    $originalContent = $null
    if (Test-Path $testFile) {
        $originalContent = Get-Content $testFile -Raw
    }
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Simulate file change
            if ($originalContent) {
                $newContent = $originalContent + "`n// Change iteration $i at $(Get-Date)"
                Set-Content -Path $testFile -Value $newContent
            }
            
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            # Step 1: Run tests
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            $pinfo.Arguments = "run test"
            $pinfo.WorkingDirectory = $ProjectPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(60000) | Out-Null
            
            # Step 2: Run build
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            $pinfo2.Arguments = "run build"
            $pinfo2.WorkingDirectory = $ProjectPath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(60000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Dev iteration run $i failed: $_"
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
    Measure CI pipeline simulation (install → build → test → bundle).
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER ProjectPath
    Path to the project directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-CIPipeline {
    param(
        [string]$Runtime,
        [string]$ProjectPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    $stepTimes = @{
        install = @()
        build = @()
        test = @()
        lint = @()
    }
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Clean up before each run
            $nodeModules = "$ProjectPath/node_modules"
            $lockFile = if ($IsBun) { "$ProjectPath/bun.lockb" } else { "$ProjectPath/dx.lock" }
            
            if (Test-Path $nodeModules) { Remove-Item -Recurse -Force $nodeModules }
            if (Test-Path $lockFile) { Remove-Item -Force $lockFile }
            
            $totalSw = [System.Diagnostics.Stopwatch]::StartNew()
            
            # Step 1: Install
            $stepSw = [System.Diagnostics.Stopwatch]::StartNew()
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            $pinfo.Arguments = "install"
            $pinfo.WorkingDirectory = $ProjectPath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            $stepSw.Stop()
            $stepTimes.install += $stepSw.Elapsed.TotalMilliseconds
            
            # Step 2: Build
            $stepSw = [System.Diagnostics.Stopwatch]::StartNew()
            $pinfo2 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo2.FileName = $Runtime
            $pinfo2.Arguments = "run build"
            $pinfo2.WorkingDirectory = $ProjectPath
            $pinfo2.RedirectStandardOutput = $true
            $pinfo2.RedirectStandardError = $true
            $pinfo2.UseShellExecute = $false
            $pinfo2.CreateNoWindow = $true
            
            $process2 = New-Object System.Diagnostics.Process
            $process2.StartInfo = $pinfo2
            $process2.Start() | Out-Null
            $process2.WaitForExit(60000) | Out-Null
            $stepSw.Stop()
            $stepTimes.build += $stepSw.Elapsed.TotalMilliseconds
            
            # Step 3: Test
            $stepSw = [System.Diagnostics.Stopwatch]::StartNew()
            $pinfo3 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo3.FileName = $Runtime
            $pinfo3.Arguments = "run test"
            $pinfo3.WorkingDirectory = $ProjectPath
            $pinfo3.RedirectStandardOutput = $true
            $pinfo3.RedirectStandardError = $true
            $pinfo3.UseShellExecute = $false
            $pinfo3.CreateNoWindow = $true
            
            $process3 = New-Object System.Diagnostics.Process
            $process3.StartInfo = $pinfo3
            $process3.Start() | Out-Null
            $process3.WaitForExit(60000) | Out-Null
            $stepSw.Stop()
            $stepTimes.test += $stepSw.Elapsed.TotalMilliseconds
            
            # Step 4: Lint
            $stepSw = [System.Diagnostics.Stopwatch]::StartNew()
            $pinfo4 = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo4.FileName = $Runtime
            $pinfo4.Arguments = "run lint"
            $pinfo4.WorkingDirectory = $ProjectPath
            $pinfo4.RedirectStandardOutput = $true
            $pinfo4.RedirectStandardError = $true
            $pinfo4.UseShellExecute = $false
            $pinfo4.CreateNoWindow = $true
            
            $process4 = New-Object System.Diagnostics.Process
            $process4.StartInfo = $pinfo4
            $process4.Start() | Out-Null
            $process4.WaitForExit(60000) | Out-Null
            $stepSw.Stop()
            $stepTimes.lint += $stepSw.Elapsed.TotalMilliseconds
            
            $totalSw.Stop()
            $times += $totalSw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "CI pipeline run $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        return $null
    }
    
    return @{
        times = $times
        stats = Get-Stats -Values $times
        stepTimes = @{
            install = if ($stepTimes.install.Count -gt 0) { Get-Stats -Values $stepTimes.install } else { $null }
            build = if ($stepTimes.build.Count -gt 0) { Get-Stats -Values $stepTimes.build } else { $null }
            test = if ($stepTimes.test.Count -gt 0) { Get-Stats -Values $stepTimes.test } else { $null }
            lint = if ($stepTimes.lint.Count -gt 0) { Get-Stats -Values $stepTimes.lint } else { $null }
        }
    }
}


<#
.SYNOPSIS
    Measure monorepo affected build time.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER MonorepoPath
    Path to the monorepo directory.
.PARAMETER IsBun
    Whether this is Bun.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Measure-MonorepoAffectedBuild {
    param(
        [string]$Runtime,
        [string]$MonorepoPath,
        [bool]$IsBun = $false,
        [int]$Runs
    )
    
    $times = @()
    
    # Use the project-manager monorepo fixtures
    $monorepoFixture = "$ScriptRoot/../project-manager/fixtures/monorepo-50"
    if (-not (Test-Path $monorepoFixture)) {
        Write-Warning "Monorepo fixture not found at $monorepoFixture"
        return $null
    }
    
    $testFile = "$monorepoFixture/packages/pkg-001/src/index.js"
    $originalContent = $null
    if (Test-Path $testFile) {
        $originalContent = Get-Content $testFile -Raw
    }
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            # Simulate file change
            if ($originalContent) {
                $newContent = $originalContent + "`n// Affected build change $i at $(Get-Date)"
                Set-Content -Path $testFile -Value $newContent
            }
            
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            
            if ($IsBun) {
                # Bun filter for affected packages
                $pinfo.Arguments = "run --filter '...[HEAD]' build"
            } else {
                # DX affected build
                $pinfo.Arguments = "run build --affected"
            }
            
            $pinfo.WorkingDirectory = $monorepoFixture
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            $process.WaitForExit(120000) | Out-Null
            
            $sw.Stop()
            $times += $sw.Elapsed.TotalMilliseconds
        }
        catch {
            Write-Warning "Affected build run $i failed: $_"
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
    Run all workflow benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    
    # 1. Fresh Project Setup
    Write-Host "  [1/5] Fresh Project Setup..." -NoNewline
    $dxFresh = Measure-FreshProjectSetup -Runtime $DxPath -ProjectPath "$FixturesPath/fresh-project" -IsBun $false -Runs $Runs
    $bunFresh = Measure-FreshProjectSetup -Runtime $BunPath -ProjectPath "$FixturesPath/fresh-project" -IsBun $true -Runs $Runs
    
    if ($dxFresh -and $bunFresh) {
        $comparison = Compare-Results -ResultA $dxFresh -ResultB $bunFresh -LowerIsBetter $true
        $benchmarks += @{
            name = "Fresh Project Setup"
            unit = "ms"
            dx = $dxFresh
            bun = $bunFresh
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 2. Development Iteration
    Write-Host "  [2/5] Development Iteration..." -NoNewline
    $dxDev = Measure-DevIteration -Runtime $DxPath -ProjectPath "$FixturesPath/dev-iteration" -IsBun $false -Runs $Runs
    $bunDev = Measure-DevIteration -Runtime $BunPath -ProjectPath "$FixturesPath/dev-iteration" -IsBun $true -Runs $Runs
    
    if ($dxDev -and $bunDev) {
        $comparison = Compare-Results -ResultA $dxDev -ResultB $bunDev -LowerIsBetter $true
        $benchmarks += @{
            name = "Development Iteration"
            unit = "ms"
            dx = $dxDev
            bun = $bunDev
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 3. CI Pipeline Simulation
    Write-Host "  [3/5] CI Pipeline Simulation..." -NoNewline
    $dxCI = Measure-CIPipeline -Runtime $DxPath -ProjectPath "$FixturesPath/fresh-project" -IsBun $false -Runs ([math]::Max(1, $Runs - 1))
    $bunCI = Measure-CIPipeline -Runtime $BunPath -ProjectPath "$FixturesPath/fresh-project" -IsBun $true -Runs ([math]::Max(1, $Runs - 1))
    
    if ($dxCI -and $bunCI) {
        $comparison = Compare-Results -ResultA $dxCI -ResultB $bunCI -LowerIsBetter $true
        $benchmarks += @{
            name = "CI Pipeline (Total)"
            unit = "ms"
            dx = $dxCI
            bun = $bunCI
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
            extra = @{
                dxSteps = $dxCI.stepTimes
                bunSteps = $bunCI.stepTimes
            }
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Failed" -ForegroundColor Red
    }
    
    # 4. Monorepo Affected Build
    Write-Host "  [4/5] Monorepo Affected Build..." -NoNewline
    $dxAffected = Measure-MonorepoAffectedBuild -Runtime $DxPath -MonorepoPath "" -IsBun $false -Runs $Runs
    $bunAffected = Measure-MonorepoAffectedBuild -Runtime $BunPath -MonorepoPath "" -IsBun $true -Runs $Runs
    
    if ($dxAffected -and $bunAffected) {
        $comparison = Compare-Results -ResultA $dxAffected -ResultB $bunAffected -LowerIsBetter $true
        $benchmarks += @{
            name = "Monorepo Affected Build"
            unit = "ms"
            dx = $dxAffected
            bun = $bunAffected
            winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
            speedup = $comparison.speedup
            isSignificant = $comparison.isSignificant
        }
        Write-Host " Done" -ForegroundColor Green
    } else {
        Write-Host " Skipped (fixture not found)" -ForegroundColor Yellow
    }
    
    # 5. Calculate cumulative time savings
    Write-Host "  [5/5] Calculating Time Savings..." -NoNewline
    $dxTotal = 0
    $bunTotal = 0
    
    foreach ($bench in $benchmarks) {
        if ($bench.dx -and $bench.dx.stats -and $bench.bun -and $bench.bun.stats) {
            $dxTotal += $bench.dx.stats.mean
            $bunTotal += $bench.bun.stats.mean
        }
    }
    
    if ($dxTotal -gt 0 -and $bunTotal -gt 0) {
        $timeSavings = [math]::Abs($dxTotal - $bunTotal)
        $savingsPercent = ($timeSavings / [math]::Max($dxTotal, $bunTotal)) * 100
        $fasterTool = if ($dxTotal -lt $bunTotal) { "dx" } else { "bun" }
        
        $benchmarks += @{
            name = "Cumulative Time Savings"
            unit = "ms"
            dx = @{ stats = @{ mean = $dxTotal; median = $dxTotal } }
            bun = @{ stats = @{ mean = $bunTotal; median = $bunTotal } }
            winner = $fasterTool
            speedup = [math]::Round([math]::Max($dxTotal, $bunTotal) / [math]::Min($dxTotal, $bunTotal), 2)
            isSignificant = $savingsPercent -gt 5
            extra = @{
                timeSavingsMs = [math]::Round($timeSavings, 2)
                savingsPercent = [math]::Round($savingsPercent, 1)
            }
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
        name = "End-to-End Workflows"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
