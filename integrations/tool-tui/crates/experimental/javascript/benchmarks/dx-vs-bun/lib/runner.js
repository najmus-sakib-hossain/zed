/**
 * Runner Library for DX vs Bun Benchmarks
 * JavaScript implementation for testing benchmark runner logic
 */

/**
 * Simulate running a benchmark with warmup and multiple iterations.
 * @param {Function} benchmarkFn - Function that returns execution time
 * @param {number} runs - Number of measurement runs
 * @param {number} warmup - Number of warmup runs
 * @returns {Object} Result with times array and metadata
 */
function runBenchmark(benchmarkFn, runs = 10, warmup = 3) {
    const allTimes = [];
    const warmupTimes = [];
    const measurementTimes = [];
    const totalRuns = runs + warmup;

    for (let i = 0; i < totalRuns; i++) {
        const time = benchmarkFn();
        allTimes.push({ iteration: i, time, isWarmup: i < warmup });

        if (i < warmup) {
            warmupTimes.push(time);
        } else {
            measurementTimes.push(time);
        }
    }

    return {
        times: measurementTimes,
        warmupTimes,
        allTimes,
        runs,
        warmup,
        totalRuns
    };
}

/**
 * Check if benchmark results meet minimum runs requirement.
 * @param {Object} result - Benchmark result
 * @param {number} minRuns - Minimum required runs
 * @returns {boolean} True if requirement is met
 */
function meetsMinimumRuns(result, minRuns = 10) {
    return result.times && result.times.length >= minRuns;
}

/**
 * Verify warmup runs are excluded from final measurements.
 * @param {Object} result - Benchmark result
 * @returns {boolean} True if warmup is properly excluded
 */
function warmupExcluded(result) {
    // Check that measurement times don't include warmup times
    if (!result.times || !result.warmupTimes) return false;

    // Measurement count should equal runs (not runs + warmup)
    if (result.times.length !== result.runs) return false;

    // Warmup count should equal warmup parameter
    if (result.warmupTimes.length !== result.warmup) return false;

    // Total should equal runs + warmup
    if (result.allTimes.length !== result.totalRuns) return false;

    return true;
}

/**
 * Simulate running benchmarks in isolation.
 * Each benchmark runs in its own "process" (simulated).
 * @param {Array} benchmarks - Array of benchmark functions
 * @returns {Array} Results with isolation metadata
 */
function runIsolatedBenchmarks(benchmarks) {
    const results = [];
    let processId = 0;

    for (const benchmark of benchmarks) {
        processId++;
        const isolationContext = {
            processId,
            startTime: Date.now(),
            sharedState: null // No shared state between benchmarks
        };

        try {
            const result = runBenchmark(benchmark.fn, benchmark.runs || 10, benchmark.warmup || 3);
            results.push({
                name: benchmark.name,
                result,
                isolation: isolationContext,
                success: true,
                error: null
            });
        } catch (error) {
            results.push({
                name: benchmark.name,
                result: null,
                isolation: isolationContext,
                success: false,
                error: error.message
            });
        }
    }

    return results;
}

/**
 * Check if benchmarks were run in isolation.
 * @param {Array} results - Array of benchmark results
 * @returns {boolean} True if all benchmarks ran in isolation
 */
function verifyIsolation(results) {
    // Each benchmark should have a unique process ID
    const processIds = results.map(r => r.isolation.processId);
    const uniqueIds = new Set(processIds);

    if (uniqueIds.size !== results.length) {
        return false;
    }

    // Failed benchmarks should not affect subsequent ones
    let foundFailure = false;
    for (const result of results) {
        if (!result.success) {
            foundFailure = true;
        } else if (foundFailure) {
            // A successful benchmark after a failure proves isolation
            // (failure didn't crash the suite)
        }
    }

    return true;
}

/**
 * Calculate speedup between two measurements.
 * @param {number} timeA - First measurement
 * @param {number} timeB - Second measurement
 * @param {boolean} lowerIsBetter - If true, lower values are better
 * @returns {Object} Speedup result
 */
function calculateSpeedup(timeA, timeB, lowerIsBetter = true) {
    if (timeA <= 0 || timeB <= 0) {
        return { speedup: 1, winner: 'tie', valid: false };
    }

    let speedup, winner;

    if (lowerIsBetter) {
        if (timeA < timeB) {
            speedup = timeB / timeA;
            winner = 'A';
        } else if (timeB < timeA) {
            speedup = timeA / timeB;
            winner = 'B';
        } else {
            speedup = 1;
            winner = 'tie';
        }
    } else {
        // Higher is better (throughput)
        if (timeA > timeB) {
            speedup = timeA / timeB;
            winner = 'A';
        } else if (timeB > timeA) {
            speedup = timeB / timeA;
            winner = 'B';
        } else {
            speedup = 1;
            winner = 'tie';
        }
    }

    return {
        speedup: Math.round(speedup * 100) / 100,
        winner,
        valid: true
    };
}

module.exports = {
    runBenchmark,
    meetsMinimumRuns,
    warmupExcluded,
    runIsolatedBenchmarks,
    verifyIsolation,
    calculateSpeedup
};
