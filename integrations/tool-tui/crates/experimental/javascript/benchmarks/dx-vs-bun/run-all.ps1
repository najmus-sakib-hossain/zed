# DX vs Bun Benchmark Suite - Main Runner
# Orchestrates all benchmark suites and generates reports

param(
    [int]$Runs = 10,              # Number of runs per benchmark
    [int]$Warmup = 3,             # Warmup runs (excluded from results)
    [switch]$SkipBuild,           # Skip building DX tools
    [string]$Suite,               # Run specific suite only
    [string]$Output = "results"   # Output directory
)

$ErrorActionPreference = "Stop"
$ScriptRoot = $PSScriptRoot
$WorkspaceRoot = (Get-Item "$ScriptRoot/../..").FullName

# Import libraries
. "$ScriptRoot/lib/stats.ps1"
. "$ScriptRoot/lib/reporter.ps1"

<#
.SYNOPSIS
    Get system information for the benchmark report.
#>
function Get-SystemInfo {
    $os = [System.Environment]::OSVersion.VersionString
    $platform = $PSVersionTable.Platform
    if (-not $platform) { $platform = "Win32NT" }
    
    # Get CPU info
    $cpu = "Unknown"
    try {
        if ($IsWindows -or (-not $IsLinux -and -not $IsMacOS)) {
            $cpuInfo = Get-CimInstance -ClassName Win32_Processor -ErrorAction SilentlyContinue
            if ($cpuInfo) { $cpu = $cpuInfo.Name }
        } else {
            $cpu = (cat /proc/cpuinfo 2>/dev/null | grep "model name" | head -1 | cut -d: -f2).Trim()
        }
    } catch { }
    
    # Get core count
    $cores = [Environment]::ProcessorCount
    
    # Get memory
    $memory = "Unknown"
    try {
        if ($IsWindows -or (-not $IsLinux -and -not $IsMacOS)) {
            $memInfo = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction SilentlyContinue
            if ($memInfo) { 
                $memGB = [math]::Round($memInfo.TotalPhysicalMemory / 1GB, 1)
                $memory = "$memGB GB"
            }
        } else {
            $memKB = (cat /proc/meminfo 2>/dev/null | grep MemTotal | awk '{print $2}')
            $memGB = [math]::Round($memKB / 1024 / 1024, 1)
            $memory = "$memGB GB"
        }
    } catch { }
    
    # Get DX version
    $dxVersion = "Unknown"
    $dxPath = Get-DxPath
    if ($dxPath -and (Test-Path $dxPath)) {
        try {
            $dxVersion = & $dxPath --version 2>&1 | Select-Object -First 1
        } catch { }
    }
    
    # Get Bun version
    $bunVersion = "Unknown"
    $bunPath = Get-BunPath
    if ($bunPath) {
        try {
            $bunVersion = & $bunPath --version 2>&1 | Select-Object -First 1
        } catch { }
    }
    
    return @{
        os = $os
        platform = $platform
        cpu = $cpu
        cores = $cores
        memory = $memory
        dxVersion = $dxVersion
        bunVersion = $bunVersion
    }
}

<#
.SYNOPSIS
    Get the path to the DX runtime executable.
#>
function Get-DxPath {
    $possiblePaths = @(
        "$WorkspaceRoot/runtime/target/release/dx-js.exe",
        "$WorkspaceRoot/runtime/target/release/dx-js",
        "$WorkspaceRoot/target/release/dx-js.exe",
        "$WorkspaceRoot/target/release/dx-js"
    )
    
    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    return $null
}

<#
.SYNOPSIS
    Get the path to the Bun executable.
#>
function Get-BunPath {
    $bunCmd = Get-Command "bun" -ErrorAction SilentlyContinue
    if ($bunCmd) {
        return $bunCmd.Source
    }
    return $null
}

<#
.SYNOPSIS
    Check prerequisites for running benchmarks.
#>
function Test-Prerequisites {
    $errors = @()
    
    # Check DX tools
    $dxPath = Get-DxPath
    if (-not $dxPath) {
        $errors += "DX runtime not found. Build with: cargo build --release -p dx-js-runtime"
    }
    
    # Check Bun
    $bunPath = Get-BunPath
    if (-not $bunPath) {
        $errors += "Bun not installed. Install from: https://bun.sh"
    }
    
    if ($errors.Count -gt 0) {
        Write-Warning "Prerequisites not met:"
        foreach ($err in $errors) {
            Write-Warning "  - $err"
        }
        return $false
    }
    
    Write-Host "Prerequisites check passed" -ForegroundColor Green
    Write-Host "  DX: $dxPath"
    Write-Host "  Bun: $bunPath"
    return $true
}

<#
.SYNOPSIS
    Build DX tools in release mode.
#>
function Build-DxTools {
    Write-Host "Building DX tools in release mode..." -ForegroundColor Cyan
    
    Push-Location $WorkspaceRoot
    try {
        # Build runtime
        Write-Host "  Building dx-js-runtime..."
        $result = cargo build --release -p dx-js-runtime 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Failed to build dx-js-runtime"
            Write-Warning $result
            return $false
        }
        
        Write-Host "DX tools built successfully" -ForegroundColor Green
        return $true
    }
    finally {
        Pop-Location
    }
}

<#
.SYNOPSIS
    Check system load before running benchmarks.
#>
function Test-SystemLoad {
    try {
        if ($IsWindows -or (-not $IsLinux -and -not $IsMacOS)) {
            $cpu = (Get-Counter '\Processor(_Total)\% Processor Time' -ErrorAction SilentlyContinue).CounterSamples.CookedValue
            if ($cpu -gt 50) {
                Write-Warning "High CPU usage detected ($([math]::Round($cpu))%). Results may be affected."
                return $false
            }
        }
    } catch {
        # Ignore errors in load detection
    }
    return $true
}

<#
.SYNOPSIS
    Run a single benchmark with warmup and multiple iterations.
#>
function Invoke-SingleBenchmark {
    param(
        [string]$Name,
        [scriptblock]$ScriptBlock,
        [int]$Runs = 10,
        [int]$Warmup = 3
    )
    
    $times = @()
    $totalRuns = $Runs + $Warmup
    
    Write-Host "    Running $Name ($totalRuns iterations, $Warmup warmup)..." -NoNewline
    
    for ($i = 0; $i -lt $totalRuns; $i++) {
        try {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            & $ScriptBlock | Out-Null
            $sw.Stop()
            
            # Only record after warmup
            if ($i -ge $Warmup) {
                $times += $sw.Elapsed.TotalMilliseconds
            }
        }
        catch {
            Write-Warning "Iteration $i failed: $_"
        }
    }
    
    if ($times.Count -eq 0) {
        Write-Host " FAILED" -ForegroundColor Red
        return $null
    }
    
    $stats = Get-Stats -Values $times
    Write-Host " $([math]::Round($stats.median, 2))ms (median)" -ForegroundColor Green
    
    return @{
        times = $times
        stats = $stats
    }
}

<#
.SYNOPSIS
    Run a benchmark suite script.
#>
function Invoke-BenchmarkSuite {
    param(
        [string]$SuiteName,
        [string]$SuitePath,
        [int]$Runs,
        [int]$Warmup
    )
    
    Write-Host ""
    Write-Host "Running $SuiteName benchmarks..." -ForegroundColor Cyan
    
    if (-not (Test-Path $SuitePath)) {
        Write-Warning "Suite script not found: $SuitePath"
        return @{
            name = $SuiteName
            benchmarks = @()
            winner = "tie"
            totalSpeedup = 1.0
            error = "Suite script not found"
        }
    }
    
    try {
        $result = & $SuitePath -DxPath (Get-DxPath) -BunPath (Get-BunPath) -Runs $Runs -Warmup $Warmup
        return $result
    }
    catch {
        Write-Warning "Suite $SuiteName failed: $_"
        return @{
            name = $SuiteName
            benchmarks = @()
            winner = "tie"
            totalSpeedup = 1.0
            error = $_.Exception.Message
        }
    }
}

<#
.SYNOPSIS
    Main entry point for running all benchmarks.
#>
function Start-Benchmarks {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  DX vs Bun Benchmark Suite" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    
    # Check system load
    Test-SystemLoad | Out-Null
    
    # Build DX if needed
    if (-not $SkipBuild) {
        if (-not (Build-DxTools)) {
            Write-Error "Failed to build DX tools. Use -SkipBuild to skip."
            return
        }
    }
    
    # Check prerequisites
    if (-not (Test-Prerequisites)) {
        Write-Error "Prerequisites not met. Please install missing tools."
        return
    }
    
    # Collect system info
    Write-Host ""
    Write-Host "Collecting system information..." -ForegroundColor Cyan
    $systemInfo = Get-SystemInfo
    
    # Define suites
    $suites = @(
        @{ name = "Runtime"; path = "$ScriptRoot/suites/runtime/bench.ps1" },
        @{ name = "Package Manager"; path = "$ScriptRoot/suites/package-manager/bench.ps1" },
        @{ name = "Bundler"; path = "$ScriptRoot/suites/bundler/bench.ps1" },
        @{ name = "Test Runner"; path = "$ScriptRoot/suites/test-runner/bench.ps1" },
        @{ name = "Project Manager"; path = "$ScriptRoot/suites/project-manager/bench.ps1" },
        @{ name = "Compatibility"; path = "$ScriptRoot/suites/compatibility/bench.ps1" }
    )
    
    # Filter to specific suite if requested
    if ($Suite) {
        $suites = $suites | Where-Object { $_.name -like "*$Suite*" }
        if ($suites.Count -eq 0) {
            Write-Error "No suite matching '$Suite' found."
            return
        }
    }
    
    # Run each suite
    $suiteResults = @()
    foreach ($suite in $suites) {
        $result = Invoke-BenchmarkSuite -SuiteName $suite.name -SuitePath $suite.path -Runs $Runs -Warmup $Warmup
        $suiteResults += $result
    }
    
    # Generate summary
    Write-Host ""
    Write-Host "Generating summary..." -ForegroundColor Cyan
    $summary = New-Summary -Suites $suiteResults
    
    # Compile results
    $results = @{
        name = "DX vs Bun Benchmarks"
        timestamp = (Get-Date -Format "o")
        system = $systemInfo
        suites = $suiteResults
        summary = $summary
    }
    
    # Ensure output directory exists
    $outputDir = "$ScriptRoot/$Output"
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    
    # Generate reports
    Write-Host ""
    Write-Host "Generating reports..." -ForegroundColor Cyan
    
    $jsonPath = New-JsonReport -Results $results -OutputPath "$outputDir/latest.json"
    Write-Host "  JSON: $jsonPath"
    
    $mdPath = New-MarkdownReport -Results $results -OutputPath "$ScriptRoot/reports/RESULTS.md"
    Write-Host "  Markdown: $mdPath"
    
    # Save to history
    $historyPath = "$outputDir/history/$(Get-Date -Format 'yyyy-MM-dd_HH-mm-ss').json"
    if (-not (Test-Path "$outputDir/history")) {
        New-Item -ItemType Directory -Path "$outputDir/history" -Force | Out-Null
    }
    Copy-Item $jsonPath $historyPath
    
    # Print summary
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  Summary" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Total Benchmarks: $($summary.totalBenchmarks)"
    Write-Host "DX Wins: $($summary.dxWins)" -ForegroundColor $(if ($summary.dxWins -gt $summary.bunWins) { "Green" } else { "White" })
    Write-Host "Bun Wins: $($summary.bunWins)" -ForegroundColor $(if ($summary.bunWins -gt $summary.dxWins) { "Green" } else { "White" })
    Write-Host "Ties: $($summary.ties)"
    Write-Host ""
    Write-Host "Overall Winner: $($summary.overallWinner.ToUpper())" -ForegroundColor Yellow
    Write-Host ""
    
    return $results
}

# Run if executed directly
if ($MyInvocation.InvocationName -ne '.') {
    Start-Benchmarks
}
