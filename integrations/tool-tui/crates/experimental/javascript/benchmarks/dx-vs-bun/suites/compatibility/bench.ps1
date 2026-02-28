# Compatibility Layer Benchmark Suite
# Compares DX-Compat vs Bun Node.js/Web API compatibility performance

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
    Run a benchmark fixture and parse JSON results.
.PARAMETER Runtime
    Path to the runtime executable.
.PARAMETER FixturePath
    Path to the benchmark fixture file.
.PARAMETER Runs
    Number of benchmark runs.
#>
function Invoke-BenchmarkFixture {
    param(
        [string]$Runtime,
        [string]$FixturePath,
        [int]$Runs
    )
    
    $allResults = @{}
    
    for ($i = 0; $i -lt $Runs; $i++) {
        try {
            $pinfo = New-Object System.Diagnostics.ProcessStartInfo
            $pinfo.FileName = $Runtime
            $pinfo.Arguments = $FixturePath
            $pinfo.RedirectStandardOutput = $true
            $pinfo.RedirectStandardError = $true
            $pinfo.UseShellExecute = $false
            $pinfo.CreateNoWindow = $true
            
            $process = New-Object System.Diagnostics.Process
            $process.StartInfo = $pinfo
            $process.Start() | Out-Null
            
            $output = $process.StandardOutput.ReadToEnd()
            $process.WaitForExit(60000) | Out-Null
            
            # Parse JSON output
            $results = $output | ConvertFrom-Json
            
            foreach ($prop in $results.PSObject.Properties) {
                if (-not $allResults.ContainsKey($prop.Name)) {
                    $allResults[$prop.Name] = @()
                }
                $allResults[$prop.Name] += $prop.Value
            }
        }
        catch {
            Write-Warning "Benchmark run $i failed: $_"
        }
    }
    
    # Calculate stats for each metric
    $statsResults = @{}
    foreach ($key in $allResults.Keys) {
        $values = $allResults[$key]
        if ($values.Count -gt 0 -and $values[0] -ne -1) {
            $statsResults[$key] = @{
                times = $values
                stats = Get-Stats -Values $values
            }
        }
    }
    
    return $statsResults
}


<#
.SYNOPSIS
    Compare benchmark results for a specific metric.
.PARAMETER DxResults
    DX benchmark results hashtable.
.PARAMETER BunResults
    Bun benchmark results hashtable.
.PARAMETER MetricName
    Name of the metric to compare.
.PARAMETER DisplayName
    Display name for the benchmark.
#>
function Compare-BenchmarkMetric {
    param(
        [hashtable]$DxResults,
        [hashtable]$BunResults,
        [string]$MetricName,
        [string]$DisplayName
    )
    
    if (-not $DxResults.ContainsKey($MetricName) -or -not $BunResults.ContainsKey($MetricName)) {
        return $null
    }
    
    $dxData = $DxResults[$MetricName]
    $bunData = $BunResults[$MetricName]
    
    $comparison = Compare-Results -ResultA $dxData -ResultB $bunData -LowerIsBetter $true
    
    return @{
        name = $DisplayName
        unit = "ms"
        dx = $dxData
        bun = $bunData
        winner = switch ($comparison.winner) { "A" { "dx" } "B" { "bun" } default { "tie" } }
        speedup = $comparison.speedup
        isSignificant = $comparison.isSignificant
    }
}

<#
.SYNOPSIS
    Run all compatibility benchmarks.
#>
function Invoke-AllBenchmarks {
    $benchmarks = @()
    $totalBenchmarks = 30
    $currentBenchmark = 0
    
    # ============================================
    # File System Benchmarks
    # ============================================
    Write-Host "  Running File System benchmarks..." -ForegroundColor Cyan
    
    $dxFs = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/fs-bench.js" -Runs $Runs
    $bunFs = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/fs-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] fs.readFileSync..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxFs -BunResults $bunFs -MetricName "readFileSync" -DisplayName "fs.readFileSync"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] fs.readFile (async)..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxFs -BunResults $bunFs -MetricName "readFileAsync" -DisplayName "fs.readFile (async)"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] fs.writeFileSync..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxFs -BunResults $bunFs -MetricName "writeFileSync" -DisplayName "fs.writeFileSync"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] fs.readdirSync..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxFs -BunResults $bunFs -MetricName "readdir" -DisplayName "fs.readdirSync"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] fs.statSync..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxFs -BunResults $bunFs -MetricName "stat" -DisplayName "fs.statSync"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    # ============================================
    # Path Module Benchmarks
    # ============================================
    Write-Host "  Running Path module benchmarks..." -ForegroundColor Cyan
    
    $dxPath = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/path-bench.js" -Runs $Runs
    $bunPath = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/path-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] path.join..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxPath -BunResults $bunPath -MetricName "join" -DisplayName "path.join"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] path.resolve..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxPath -BunResults $bunPath -MetricName "resolve" -DisplayName "path.resolve"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] path.parse..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxPath -BunResults $bunPath -MetricName "parse" -DisplayName "path.parse"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] path.basename..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxPath -BunResults $bunPath -MetricName "basename" -DisplayName "path.basename"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] path.normalize..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxPath -BunResults $bunPath -MetricName "normalize" -DisplayName "path.normalize"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }

    
    # ============================================
    # Crypto Module Benchmarks
    # ============================================
    Write-Host "  Running Crypto module benchmarks..." -ForegroundColor Cyan
    
    $dxCrypto = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/crypto-bench.js" -Runs $Runs
    $bunCrypto = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/crypto-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] crypto.sha256..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxCrypto -BunResults $bunCrypto -MetricName "sha256" -DisplayName "crypto.sha256"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] crypto.sha512..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxCrypto -BunResults $bunCrypto -MetricName "sha512" -DisplayName "crypto.sha512"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] crypto.randomBytes..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxCrypto -BunResults $bunCrypto -MetricName "randomBytes" -DisplayName "crypto.randomBytes"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] crypto.randomUUID..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxCrypto -BunResults $bunCrypto -MetricName "randomUUID" -DisplayName "crypto.randomUUID"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] crypto.hmac..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxCrypto -BunResults $bunCrypto -MetricName "hmac" -DisplayName "crypto.hmac"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    # ============================================
    # HTTP Module Benchmarks
    # ============================================
    Write-Host "  Running HTTP module benchmarks..." -ForegroundColor Cyan
    
    $dxHttp = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/http-bench.js" -Runs ([math]::Max(1, $Runs - 2))
    $bunHttp = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/http-bench.js" -Runs ([math]::Max(1, $Runs - 2))
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] http.createServer..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxHttp -BunResults $bunHttp -MetricName "serverCreation" -DisplayName "http.createServer"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    # ============================================
    # EventEmitter Benchmarks
    # ============================================
    Write-Host "  Running EventEmitter benchmarks..." -ForegroundColor Cyan
    
    $dxEvents = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/events-bench.js" -Runs $Runs
    $bunEvents = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/events-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] EventEmitter.emit (single)..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxEvents -BunResults $bunEvents -MetricName "emitSingle" -DisplayName "EventEmitter.emit (single)"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] EventEmitter.emit (multiple)..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxEvents -BunResults $bunEvents -MetricName "emitMultiple" -DisplayName "EventEmitter.emit (multiple)"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] EventEmitter.on/off..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxEvents -BunResults $bunEvents -MetricName "onOff" -DisplayName "EventEmitter.on/off"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] EventEmitter.once..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxEvents -BunResults $bunEvents -MetricName "once" -DisplayName "EventEmitter.once"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }

    
    # ============================================
    # Buffer Benchmarks
    # ============================================
    Write-Host "  Running Buffer benchmarks..." -ForegroundColor Cyan
    
    $dxBuffer = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/buffer-bench.js" -Runs $Runs
    $bunBuffer = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/buffer-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Buffer.alloc..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxBuffer -BunResults $bunBuffer -MetricName "alloc" -DisplayName "Buffer.alloc"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Buffer.from (string)..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxBuffer -BunResults $bunBuffer -MetricName "fromString" -DisplayName "Buffer.from (string)"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Buffer.toString..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxBuffer -BunResults $bunBuffer -MetricName "toString" -DisplayName "Buffer.toString"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Buffer.concat..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxBuffer -BunResults $bunBuffer -MetricName "concat" -DisplayName "Buffer.concat"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Buffer read/write..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxBuffer -BunResults $bunBuffer -MetricName "readWrite" -DisplayName "Buffer read/write"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    # ============================================
    # Web API Benchmarks
    # ============================================
    Write-Host "  Running Web API benchmarks..." -ForegroundColor Cyan
    
    $dxWebApi = Invoke-BenchmarkFixture -Runtime $DxPath -FixturePath "$FixturesPath/web-api-bench.js" -Runs $Runs
    $bunWebApi = Invoke-BenchmarkFixture -Runtime $BunPath -FixturePath "$FixturesPath/web-api-bench.js" -Runs $Runs
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] TextEncoder..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "textEncoder" -DisplayName "TextEncoder"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] TextDecoder..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "textDecoder" -DisplayName "TextDecoder"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] URL parsing..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "urlParse" -DisplayName "URL parsing"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] URLSearchParams..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "urlSearchParams" -DisplayName "URLSearchParams"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] Base64 (atob/btoa)..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "base64" -DisplayName "Base64 (atob/btoa)"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
    $currentBenchmark++
    Write-Host "  [$currentBenchmark/$totalBenchmarks] JSON parse/stringify..." -NoNewline
    $result = Compare-BenchmarkMetric -DxResults $dxWebApi -BunResults $bunWebApi -MetricName "json" -DisplayName "JSON parse/stringify"
    if ($result) { $benchmarks += $result; Write-Host " Done" -ForegroundColor Green } else { Write-Host " Skipped" -ForegroundColor Yellow }
    
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
        name = "Compatibility Layer"
        benchmarks = $benchmarks
        winner = $suiteWinner
        totalSpeedup = [math]::Round($avgSpeedup, 2)
    }
}

# Run benchmarks
Invoke-AllBenchmarks
