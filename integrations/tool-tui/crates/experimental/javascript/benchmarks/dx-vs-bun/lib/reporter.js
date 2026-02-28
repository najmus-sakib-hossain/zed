/**
 * Reporter Library for DX vs Bun Benchmarks
 * JavaScript implementation for testing and cross-platform use
 */

/**
 * Format a measurement value with appropriate unit.
 * @param {number} value - The numeric value
 * @param {string} unit - The unit of measurement
 * @returns {string} Formatted string
 */
function formatMeasurement(value, unit) {
    switch (unit) {
        case 'ms': return `${value.toFixed(2)} ms`;
        case 'µs': return `${value.toFixed(2)} µs`;
        case 'ops/s': return `${Math.round(value)} ops/s`;
        case 'MB': return `${value.toFixed(2)} MB`;
        case 'tests/s': return `${Math.round(value)} tests/s`;
        default: return `${value.toFixed(2)} ${unit}`;
    }
}

/**
 * Generate a markdown report from benchmark results.
 * @param {Object} results - Benchmark results object
 * @returns {string} Markdown content
 */
function newMarkdownReport(results) {
    const lines = [];

    // Header
    lines.push('# DX vs Bun Benchmark Results');
    lines.push('');
    lines.push(`Generated: ${results.timestamp}`);
    lines.push('');

    // System Information
    lines.push('## System Information');
    lines.push('');
    lines.push('| Property | Value |');
    lines.push('|----------|-------|');
    lines.push(`| OS | ${results.system?.os || 'N/A'} |`);
    lines.push(`| Platform | ${results.system?.platform || 'N/A'} |`);
    lines.push(`| CPU | ${results.system?.cpu || 'N/A'} |`);
    lines.push(`| Cores | ${results.system?.cores || 'N/A'} |`);
    lines.push(`| Memory | ${results.system?.memory || 'N/A'} |`);
    lines.push(`| DX Version | ${results.system?.dxVersion || 'N/A'} |`);
    lines.push(`| Bun Version | ${results.system?.bunVersion || 'N/A'} |`);
    lines.push('');

    // Summary
    lines.push('## Summary');
    lines.push('');
    const summary = results.summary || {};
    lines.push(`- **Total Benchmarks**: ${summary.totalBenchmarks || 0}`);
    lines.push(`- **DX Wins**: ${summary.dxWins || 0}`);
    lines.push(`- **Bun Wins**: ${summary.bunWins || 0}`);
    lines.push(`- **Ties**: ${summary.ties || 0}`);
    lines.push(`- **Overall Winner**: **${(summary.overallWinner || 'N/A').toUpperCase()}**`);
    lines.push('');

    // Category Summary
    if (summary.categories && summary.categories.length > 0) {
        lines.push('### Category Results');
        lines.push('');
        lines.push('| Category | Winner | Speedup |');
        lines.push('|----------|--------|---------|');
        for (const cat of summary.categories) {
            const speedupStr = cat.speedup > 1 ? `${cat.speedup}x` : '-';
            lines.push(`| ${cat.name} | ${cat.winner} | ${speedupStr} |`);
        }
        lines.push('');
    }

    // Detailed Results by Suite
    lines.push('## Detailed Results');
    lines.push('');

    for (const suite of (results.suites || [])) {
        lines.push(`### ${suite.name}`);
        lines.push('');
        lines.push('| Benchmark | DX | Bun | Winner | Speedup |');
        lines.push('|-----------|-----|-----|--------|---------|');

        for (const bench of (suite.benchmarks || [])) {
            const dxValue = formatMeasurement(bench.dx?.stats?.median || 0, bench.unit);
            const bunValue = formatMeasurement(bench.bun?.stats?.median || 0, bench.unit);
            const winner = bench.winner || 'tie';
            const speedup = bench.speedup > 1 ? `${bench.speedup}x` : '-';
            const sig = bench.isSignificant ? '' : ' (ns)';

            lines.push(`| ${bench.name} | ${dxValue} | ${bunValue} | ${winner}${sig} | ${speedup} |`);
        }
        lines.push('');
    }

    // Methodology
    lines.push('## Methodology');
    lines.push('');
    lines.push('- Each benchmark was run multiple times with warmup runs excluded');
    lines.push('- Results show median values to reduce impact of outliers');
    lines.push('- Statistical significance determined using 95% confidence intervals');
    lines.push('- (ns) indicates result is not statistically significant');
    lines.push('');

    return lines.join('\n');
}

/**
 * Generate a JSON report from benchmark results.
 * @param {Object} results - Benchmark results object
 * @returns {string} JSON string
 */
function newJsonReport(results) {
    const output = {
        name: results.name || 'DX vs Bun Benchmarks',
        timestamp: results.timestamp || new Date().toISOString(),
        system: results.system || {},
        suites: results.suites || [],
        summary: results.summary || {}
    };

    return JSON.stringify(output, null, 2);
}

/**
 * Generate an ASCII bar chart for visual comparison.
 * @param {Array} data - Array of benchmark comparisons
 * @param {string} title - Chart title
 * @param {number} width - Chart width in characters
 * @returns {string} ASCII chart
 */
function newAsciiChart(data, title = 'Benchmark Comparison', width = 60) {
    const lines = [];

    lines.push(title);
    lines.push('='.repeat(title.length));
    lines.push('');

    // Find max value for scaling
    let maxValue = 0;
    for (const item of data) {
        const dxVal = item.dx?.stats?.median || 0;
        const bunVal = item.bun?.stats?.median || 0;
        if (dxVal > maxValue) maxValue = dxVal;
        if (bunVal > maxValue) maxValue = bunVal;
    }

    if (maxValue === 0) maxValue = 1;

    const barWidth = width - 25;

    for (const item of data) {
        let name = item.name || 'Unknown';
        if (name.length > 15) name = name.substring(0, 12) + '...';
        name = name.padEnd(15);

        const dxVal = item.dx?.stats?.median || 0;
        const bunVal = item.bun?.stats?.median || 0;

        const dxBarLen = Math.max(1, Math.round((dxVal / maxValue) * barWidth));
        const bunBarLen = Math.max(1, Math.round((bunVal / maxValue) * barWidth));

        const dxBar = '█'.repeat(dxBarLen);
        const bunBar = '▓'.repeat(bunBarLen);

        lines.push(`${name} DX  |${dxBar} ${dxVal.toFixed(1)}`);
        lines.push(`${' '.repeat(15)} Bun |${bunBar} ${bunVal.toFixed(1)}`);
        lines.push('');
    }

    lines.push('Legend: █ = DX, ▓ = Bun');

    return lines.join('\n');
}

/**
 * Create a summary from suite results.
 * @param {Array} suites - Array of suite results
 * @returns {Object} Summary object
 */
function newSummary(suites) {
    let totalBenchmarks = 0;
    let dxWins = 0;
    let bunWins = 0;
    let ties = 0;
    const categories = [];

    for (const suite of suites) {
        let suiteDxWins = 0;
        let suiteBunWins = 0;
        let suiteTies = 0;
        const suiteSpeedups = [];

        for (const bench of (suite.benchmarks || [])) {
            totalBenchmarks++;
            switch (bench.winner) {
                case 'dx':
                    dxWins++;
                    suiteDxWins++;
                    suiteSpeedups.push(bench.speedup);
                    break;
                case 'bun':
                    bunWins++;
                    suiteBunWins++;
                    suiteSpeedups.push(bench.speedup);
                    break;
                default:
                    ties++;
                    suiteTies++;
            }
        }

        // Determine suite winner
        let suiteWinner = 'tie';
        if (suiteDxWins > suiteBunWins) suiteWinner = 'dx';
        else if (suiteBunWins > suiteDxWins) suiteWinner = 'bun';

        const avgSpeedup = suiteSpeedups.length > 0
            ? suiteSpeedups.reduce((a, b) => a + b, 0) / suiteSpeedups.length
            : 1.0;

        categories.push({
            name: suite.name,
            winner: suiteWinner,
            speedup: Math.round(avgSpeedup * 100) / 100
        });
    }

    // Determine overall winner
    let overallWinner = 'tie';
    if (dxWins > bunWins) overallWinner = 'dx';
    else if (bunWins > dxWins) overallWinner = 'bun';

    return {
        totalBenchmarks,
        dxWins,
        bunWins,
        ties,
        overallWinner,
        categories
    };
}

/**
 * Validate that a results object has all required fields for JSON output.
 * @param {Object} results - Results object to validate
 * @returns {Object} Validation result with isValid and missingFields
 */
function validateResults(results) {
    const requiredFields = ['name', 'timestamp', 'system', 'suites', 'summary'];
    const missingFields = [];

    for (const field of requiredFields) {
        if (results[field] === undefined) {
            missingFields.push(field);
        }
    }

    return {
        isValid: missingFields.length === 0,
        missingFields
    };
}

module.exports = {
    formatMeasurement,
    newMarkdownReport,
    newJsonReport,
    newAsciiChart,
    newSummary,
    validateResults
};
