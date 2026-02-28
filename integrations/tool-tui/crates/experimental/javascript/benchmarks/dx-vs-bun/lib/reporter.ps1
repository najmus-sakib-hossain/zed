# Reporter Library for DX vs Bun Benchmarks
# Generates reports in multiple formats (Markdown, JSON, ASCII charts)

<#
.SYNOPSIS
    Generate a markdown report from benchmark results.
.PARAMETER Results
    Hashtable containing benchmark results.
.PARAMETER OutputPath
    Path to write the markdown file.
#>
function New-MarkdownReport {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$Results,
        [Parameter(Mandatory = $true)]
        [string]$OutputPath
    )
    
    $sb = [System.Text.StringBuilder]::new()
    
    # Header
    [void]$sb.AppendLine("# DX vs Bun Benchmark Results")
    [void]$sb.AppendLine("")
    [void]$sb.AppendLine("Generated: $($Results.timestamp)")
    [void]$sb.AppendLine("")
    
    # System Information
    [void]$sb.AppendLine("## System Information")
    [void]$sb.AppendLine("")
    [void]$sb.AppendLine("| Property | Value |")
    [void]$sb.AppendLine("|----------|-------|")
    [void]$sb.AppendLine("| OS | $($Results.system.os) |")
    [void]$sb.AppendLine("| Platform | $($Results.system.platform) |")
    [void]$sb.AppendLine("| CPU | $($Results.system.cpu) |")
    [void]$sb.AppendLine("| Cores | $($Results.system.cores) |")
    [void]$sb.AppendLine("| Memory | $($Results.system.memory) |")
    [void]$sb.AppendLine("| DX Version | $($Results.system.dxVersion) |")
    [void]$sb.AppendLine("| Bun Version | $($Results.system.bunVersion) |")
    [void]$sb.AppendLine("")
    
    # Summary
    [void]$sb.AppendLine("## Summary")
    [void]$sb.AppendLine("")
    $summary = $Results.summary
    [void]$sb.AppendLine("- **Total Benchmarks**: $($summary.totalBenchmarks)")
    [void]$sb.AppendLine("- **DX Wins**: $($summary.dxWins)")
    [void]$sb.AppendLine("- **Bun Wins**: $($summary.bunWins)")
    [void]$sb.AppendLine("- **Ties**: $($summary.ties)")
    [void]$sb.AppendLine("- **Overall Winner**: **$($summary.overallWinner.ToUpper())**")
    [void]$sb.AppendLine("")
    
    # Category Summary
    if ($summary.categories -and $summary.categories.Count -gt 0) {
        [void]$sb.AppendLine("### Category Results")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("| Category | Winner | Speedup |")
        [void]$sb.AppendLine("|----------|--------|---------|")
        foreach ($cat in $summary.categories) {
            $speedupStr = if ($cat.speedup -gt 1) { "$($cat.speedup)x" } else { "-" }
            [void]$sb.AppendLine("| $($cat.name) | $($cat.winner) | $speedupStr |")
        }
        [void]$sb.AppendLine("")
    }
    
    # Detailed Results by Suite
    [void]$sb.AppendLine("## Detailed Results")
    [void]$sb.AppendLine("")
    
    foreach ($suite in $Results.suites) {
        [void]$sb.AppendLine("### $($suite.name)")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("| Benchmark | DX | Bun | Winner | Speedup |")
        [void]$sb.AppendLine("|-----------|-----|-----|--------|---------|")
        
        foreach ($bench in $suite.benchmarks) {
            $dxValue = Format-Measurement -Value $bench.dx.stats.median -Unit $bench.unit
            $bunValue = Format-Measurement -Value $bench.bun.stats.median -Unit $bench.unit
            $winner = $bench.winner
            $speedup = if ($bench.speedup -gt 1) { "$($bench.speedup)x" } else { "-" }
            $sig = if ($bench.isSignificant) { "" } else { " (ns)" }
            
            [void]$sb.AppendLine("| $($bench.name) | $dxValue | $bunValue | $winner$sig | $speedup |")
        }
        [void]$sb.AppendLine("")
        
        # ASCII chart for this suite
        $chart = New-AsciiChart -Data $suite.benchmarks -Title $suite.name
        [void]$sb.AppendLine("``````")
        [void]$sb.AppendLine($chart)
        [void]$sb.AppendLine("``````")
        [void]$sb.AppendLine("")
    }
    
    # Methodology
    [void]$sb.AppendLine("## Methodology")
    [void]$sb.AppendLine("")
    [void]$sb.AppendLine("- Each benchmark was run multiple times with warmup runs excluded")
    [void]$sb.AppendLine("- Results show median values to reduce impact of outliers")
    [void]$sb.AppendLine("- Statistical significance determined using 95% confidence intervals")
    [void]$sb.AppendLine("- (ns) indicates result is not statistically significant")
    [void]$sb.AppendLine("")
    
    # Recommendations
    if ($Results.summary) {
        $recommendations = Get-Recommendations -Summary $Results.summary
        if ($recommendations.Count -gt 0) {
            [void]$sb.AppendLine("## Recommendations")
            [void]$sb.AppendLine("")
            foreach ($rec in $recommendations) {
                [void]$sb.AppendLine("- $rec")
            }
            [void]$sb.AppendLine("")
        }
    }
    
    # Write to file
    $sb.ToString() | Out-File -FilePath $OutputPath -Encoding utf8
    
    return $OutputPath
}

<#
.SYNOPSIS
    Format a measurement value with appropriate unit.
.PARAMETER Value
    The numeric value.
.PARAMETER Unit
    The unit of measurement.
#>
function Format-Measurement {
    param(
        [double]$Value,
        [string]$Unit
    )
    
    switch ($Unit) {
        "ms" { return "$([math]::Round($Value, 2)) ms" }
        "µs" { return "$([math]::Round($Value, 2)) µs" }
        "ops/s" { return "$([math]::Round($Value, 0)) ops/s" }
        "MB" { return "$([math]::Round($Value, 2)) MB" }
        "tests/s" { return "$([math]::Round($Value, 0)) tests/s" }
        default { return "$([math]::Round($Value, 2)) $Unit" }
    }
}

<#
.SYNOPSIS
    Generate a JSON report from benchmark results.
.PARAMETER Results
    Hashtable containing benchmark results.
.PARAMETER OutputPath
    Path to write the JSON file.
#>
function New-JsonReport {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$Results,
        [Parameter(Mandatory = $true)]
        [string]$OutputPath
    )
    
    # Ensure all required fields are present
    $output = @{
        name = if ($Results.name) { $Results.name } else { "DX vs Bun Benchmarks" }
        timestamp = if ($Results.timestamp) { $Results.timestamp } else { (Get-Date -Format "o") }
        system = if ($Results.system) { $Results.system } else { @{} }
        suites = if ($Results.suites) { $Results.suites } else { @() }
        summary = if ($Results.summary) { $Results.summary } else { @{} }
    }
    
    $json = $output | ConvertTo-Json -Depth 10
    $json | Out-File -FilePath $OutputPath -Encoding utf8
    
    return $OutputPath
}

<#
.SYNOPSIS
    Generate an ASCII bar chart for visual comparison.
.PARAMETER Data
    Array of benchmark comparisons.
.PARAMETER Title
    Chart title.
.PARAMETER Width
    Chart width in characters (default 60).
#>
function New-AsciiChart {
    param(
        [Parameter(Mandatory = $true)]
        [array]$Data,
        [string]$Title = "Benchmark Comparison",
        [int]$Width = 60
    )
    
    $sb = [System.Text.StringBuilder]::new()
    
    [void]$sb.AppendLine($Title)
    [void]$sb.AppendLine("=" * $Title.Length)
    [void]$sb.AppendLine("")
    
    # Find max value for scaling
    $maxValue = 0
    foreach ($item in $Data) {
        $dxVal = $item.dx.stats.median
        $bunVal = $item.bun.stats.median
        if ($dxVal -gt $maxValue) { $maxValue = $dxVal }
        if ($bunVal -gt $maxValue) { $maxValue = $bunVal }
    }
    
    if ($maxValue -eq 0) { $maxValue = 1 }
    
    $barWidth = $Width - 25  # Leave room for labels
    
    foreach ($item in $Data) {
        $name = $item.name
        if ($name.Length -gt 15) { $name = $name.Substring(0, 12) + "..." }
        $name = $name.PadRight(15)
        
        $dxVal = $item.dx.stats.median
        $bunVal = $item.bun.stats.median
        
        $dxBarLen = [math]::Max(1, [math]::Round(($dxVal / $maxValue) * $barWidth))
        $bunBarLen = [math]::Max(1, [math]::Round(($bunVal / $maxValue) * $barWidth))
        
        $dxBar = "█" * $dxBarLen
        $bunBar = "▓" * $bunBarLen
        
        [void]$sb.AppendLine("$name DX  |$dxBar $([math]::Round($dxVal, 1))")
        [void]$sb.AppendLine("$(" " * 15) Bun |$bunBar $([math]::Round($bunVal, 1))")
        [void]$sb.AppendLine("")
    }
    
    [void]$sb.AppendLine("Legend: █ = DX, ▓ = Bun")
    
    return $sb.ToString()
}

<#
.SYNOPSIS
    Create a summary from suite results.
.PARAMETER Suites
    Array of suite results.
#>
function New-Summary {
    param(
        [Parameter(Mandatory = $true)]
        [array]$Suites
    )
    
    $totalBenchmarks = 0
    $dxWins = 0
    $bunWins = 0
    $ties = 0
    $categories = @()
    
    foreach ($suite in $Suites) {
        $suiteDxWins = 0
        $suiteBunWins = 0
        $suiteTies = 0
        $suiteSpeedups = @()
        $suiteDxTotal = 0
        $suiteBunTotal = 0
        
        foreach ($bench in $suite.benchmarks) {
            $totalBenchmarks++
            switch ($bench.winner) {
                "dx" { $dxWins++; $suiteDxWins++; $suiteSpeedups += $bench.speedup }
                "bun" { $bunWins++; $suiteBunWins++; $suiteSpeedups += $bench.speedup }
                default { $ties++; $suiteTies++ }
            }
            
            # Accumulate totals for percentage calculation
            if ($bench.dx -and $bench.dx.stats -and $bench.bun -and $bench.bun.stats) {
                $suiteDxTotal += $bench.dx.stats.mean
                $suiteBunTotal += $bench.bun.stats.mean
            }
        }
        
        # Determine suite winner
        $suiteWinner = "tie"
        if ($suiteDxWins -gt $suiteBunWins) { $suiteWinner = "dx" }
        elseif ($suiteBunWins -gt $suiteDxWins) { $suiteWinner = "bun" }
        
        $avgSpeedup = if ($suiteSpeedups.Count -gt 0) {
            ($suiteSpeedups | Measure-Object -Average).Average
        } else { 1.0 }
        
        # Calculate percentage difference
        $percentDiff = 0
        if ($suiteDxTotal -gt 0 -and $suiteBunTotal -gt 0) {
            $diff = [math]::Abs($suiteDxTotal - $suiteBunTotal)
            $percentDiff = ($diff / [math]::Max($suiteDxTotal, $suiteBunTotal)) * 100
        }
        
        $categories += @{
            name = $suite.name
            winner = $suiteWinner
            speedup = [math]::Round($avgSpeedup, 2)
            dxWins = $suiteDxWins
            bunWins = $suiteBunWins
            ties = $suiteTies
            percentDiff = [math]::Round($percentDiff, 1)
        }
    }
    
    # Determine overall winner
    $overallWinner = "tie"
    if ($dxWins -gt $bunWins) { $overallWinner = "dx" }
    elseif ($bunWins -gt $dxWins) { $overallWinner = "bun" }
    
    # Calculate overall percentage
    $dxWinPercent = if ($totalBenchmarks -gt 0) { [math]::Round(($dxWins / $totalBenchmarks) * 100, 1) } else { 0 }
    $bunWinPercent = if ($totalBenchmarks -gt 0) { [math]::Round(($bunWins / $totalBenchmarks) * 100, 1) } else { 0 }
    
    return @{
        totalBenchmarks = $totalBenchmarks
        dxWins = $dxWins
        bunWins = $bunWins
        ties = $ties
        overallWinner = $overallWinner
        dxWinPercent = $dxWinPercent
        bunWinPercent = $bunWinPercent
        categories = $categories
    }
}

<#
.SYNOPSIS
    Generate recommendations based on benchmark results.
.PARAMETER Summary
    Summary hashtable from New-Summary.
#>
function Get-Recommendations {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$Summary
    )
    
    $recommendations = @()
    
    # Overall recommendation
    if ($Summary.overallWinner -eq "dx") {
        $recommendations += "**Overall**: DX is the faster toolchain, winning $($Summary.dxWinPercent)% of benchmarks."
    } elseif ($Summary.overallWinner -eq "bun") {
        $recommendations += "**Overall**: Bun is the faster toolchain, winning $($Summary.bunWinPercent)% of benchmarks."
    } else {
        $recommendations += "**Overall**: Both toolchains perform similarly across benchmarks."
    }
    
    # Category-specific recommendations
    foreach ($cat in $Summary.categories) {
        $catName = $cat.name
        $winner = $cat.winner
        $speedup = $cat.speedup
        $percentDiff = $cat.percentDiff
        
        if ($winner -eq "dx" -and $speedup -gt 1.5) {
            $recommendations += "**$catName**: DX excels here with ${speedup}x speedup ($percentDiff% faster). Recommended for this use case."
        } elseif ($winner -eq "bun" -and $speedup -gt 1.5) {
            $recommendations += "**$catName**: Bun excels here with ${speedup}x speedup ($percentDiff% faster). Consider Bun for this use case."
        } elseif ($winner -eq "tie" -or $speedup -lt 1.2) {
            $recommendations += "**$catName**: Both tools perform similarly. Choose based on other factors."
        }
    }
    
    # Use case recommendations
    $runtimeCat = $Summary.categories | Where-Object { $_.name -eq "Runtime" }
    $pkgCat = $Summary.categories | Where-Object { $_.name -eq "Package Manager" }
    $testCat = $Summary.categories | Where-Object { $_.name -eq "Test Runner" }
    
    if ($runtimeCat -and $runtimeCat.winner -eq "dx" -and $runtimeCat.speedup -gt 1.3) {
        $recommendations += "**For compute-intensive tasks**: DX runtime offers better performance."
    }
    
    if ($pkgCat -and $pkgCat.winner -eq "dx" -and $pkgCat.speedup -gt 1.3) {
        $recommendations += "**For CI/CD pipelines**: DX package manager can reduce install times."
    }
    
    if ($testCat -and $testCat.winner -eq "dx" -and $testCat.speedup -gt 1.3) {
        $recommendations += "**For test-heavy workflows**: DX test runner provides faster feedback."
    }
    
    return $recommendations
}

# Export functions
Export-ModuleMember -Function New-MarkdownReport, New-JsonReport, New-AsciiChart, New-Summary, Format-Measurement, Get-Recommendations
