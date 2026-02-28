/**
 * Property-Based Tests for Runner Library
 * Feature: dx-bun-benchmarks
 * 
 * Uses fast-check for property-based testing
 */

const fc = require('fast-check');
const {
    runBenchmark,
    meetsMinimumRuns,
    warmupExcluded,
    runIsolatedBenchmarks,
    verifyIsolation,
    calculateSpeedup
} = require('./runner');

describe('Runner Library Property Tests', () => {
    /**
     * Property 2: Minimum Runs Guarantee
     * For any benchmark result in the suite, the number of recorded measurements
     * SHALL be at least equal to the configured minimum runs (default 10).
     * **Validates: Requirements 1.4**
     */
    describe('Property 2: Minimum Runs Guarantee', () => {
        test('benchmark results contain at least minimum runs', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),  // runs
                    fc.integer({ min: 1, max: 10 }), // warmup
                    (runs, warmup) => {
                        // Create a simple benchmark function
                        const benchFn = () => Math.random() * 100;

                        const result = runBenchmark(benchFn, runs, warmup);

                        return meetsMinimumRuns(result, runs);
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('measurement count equals configured runs', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),
                    fc.integer({ min: 1, max: 10 }),
                    (runs, warmup) => {
                        const benchFn = () => Math.random() * 100;
                        const result = runBenchmark(benchFn, runs, warmup);

                        return result.times.length === runs;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('total iterations equals runs plus warmup', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),
                    fc.integer({ min: 1, max: 10 }),
                    (runs, warmup) => {
                        const benchFn = () => Math.random() * 100;
                        const result = runBenchmark(benchFn, runs, warmup);

                        return result.allTimes.length === runs + warmup;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Property 3: Warmup Exclusion
     * For any benchmark configured with W warmup runs and R total runs,
     * the final results SHALL contain exactly (R - W) measurements,
     * and warmup measurements SHALL NOT appear in the final statistics.
     * **Validates: Requirements 1.6**
     */
    describe('Property 3: Warmup Exclusion', () => {
        test('warmup runs are excluded from final measurements', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),
                    fc.integer({ min: 1, max: 10 }),
                    (runs, warmup) => {
                        const benchFn = () => Math.random() * 100;
                        const result = runBenchmark(benchFn, runs, warmup);

                        return warmupExcluded(result);
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('warmup times are tracked separately', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),
                    fc.integer({ min: 1, max: 10 }),
                    (runs, warmup) => {
                        const benchFn = () => Math.random() * 100;
                        const result = runBenchmark(benchFn, runs, warmup);

                        return result.warmupTimes.length === warmup;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('warmup iterations are marked correctly', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 5, max: 50 }),
                    fc.integer({ min: 1, max: 10 }),
                    (runs, warmup) => {
                        const benchFn = () => Math.random() * 100;
                        const result = runBenchmark(benchFn, runs, warmup);

                        // First 'warmup' iterations should be marked as warmup
                        const warmupMarked = result.allTimes
                            .slice(0, warmup)
                            .every(t => t.isWarmup === true);

                        // Remaining iterations should not be marked as warmup
                        const measurementMarked = result.allTimes
                            .slice(warmup)
                            .every(t => t.isWarmup === false);

                        return warmupMarked && measurementMarked;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Property 9: Benchmark Isolation
     * For any benchmark execution, each individual benchmark SHALL run in a
     * separate process to prevent state leakage, and the exit code of one
     * benchmark SHALL NOT affect the execution of subsequent benchmarks.
     * **Validates: Requirements 9.3, 7.6**
     */
    describe('Property 9: Benchmark Isolation', () => {
        test('each benchmark gets unique process ID', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 2, max: 10 }),
                    (numBenchmarks) => {
                        const benchmarks = Array.from({ length: numBenchmarks }, (_, i) => ({
                            name: `benchmark-${i}`,
                            fn: () => Math.random() * 100,
                            runs: 5,
                            warmup: 2
                        }));

                        const results = runIsolatedBenchmarks(benchmarks);

                        return verifyIsolation(results);
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('failed benchmark does not affect subsequent benchmarks', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 1, max: 5 }),  // position of failing benchmark
                    fc.integer({ min: 3, max: 8 }),  // total benchmarks
                    (failPos, total) => {
                        const actualFailPos = Math.min(failPos, total - 1);

                        const benchmarks = Array.from({ length: total }, (_, i) => ({
                            name: `benchmark-${i}`,
                            fn: i === actualFailPos
                                ? () => { throw new Error('Simulated failure'); }
                                : () => Math.random() * 100,
                            runs: 5,
                            warmup: 2
                        }));

                        const results = runIsolatedBenchmarks(benchmarks);

                        // Check that benchmarks after the failure still ran
                        const afterFailure = results.slice(actualFailPos + 1);
                        const allAfterRan = afterFailure.every(r =>
                            r.isolation.processId > 0
                        );

                        // Check that the failure was recorded
                        const failureRecorded = results[actualFailPos].success === false;

                        return allAfterRan && failureRecorded;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('no shared state between benchmarks', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 2, max: 10 }),
                    (numBenchmarks) => {
                        const benchmarks = Array.from({ length: numBenchmarks }, (_, i) => ({
                            name: `benchmark-${i}`,
                            fn: () => Math.random() * 100,
                            runs: 5,
                            warmup: 2
                        }));

                        const results = runIsolatedBenchmarks(benchmarks);

                        // All isolation contexts should have null shared state
                        return results.every(r => r.isolation.sharedState === null);
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Additional speedup calculation properties
     */
    describe('Speedup Calculation Properties', () => {
        test('speedup is always >= 1', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 0.1, max: 10000, noNaN: true }),
                    fc.double({ min: 0.1, max: 10000, noNaN: true }),
                    (timeA, timeB) => {
                        const result = calculateSpeedup(timeA, timeB);
                        return result.speedup >= 1;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('winner is correctly identified for lower-is-better', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 1, max: 100, noNaN: true }),
                    fc.double({ min: 200, max: 500, noNaN: true }),
                    (fastTime, slowTime) => {
                        const result = calculateSpeedup(fastTime, slowTime, true);
                        return result.winner === 'A'; // A is faster (lower)
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('winner is correctly identified for higher-is-better', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 200, max: 500, noNaN: true }),
                    fc.double({ min: 1, max: 100, noNaN: true }),
                    (highThroughput, lowThroughput) => {
                        const result = calculateSpeedup(highThroughput, lowThroughput, false);
                        return result.winner === 'A'; // A has higher throughput
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('equal times produce tie', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 0.1, max: 10000, noNaN: true }),
                    (time) => {
                        const result = calculateSpeedup(time, time);
                        return result.winner === 'tie' && result.speedup === 1;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('speedup calculation is symmetric', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 0.1, max: 10000, noNaN: true }),
                    fc.double({ min: 0.1, max: 10000, noNaN: true }),
                    (timeA, timeB) => {
                        const resultAB = calculateSpeedup(timeA, timeB);
                        const resultBA = calculateSpeedup(timeB, timeA);

                        // Speedup values should be the same
                        return Math.abs(resultAB.speedup - resultBA.speedup) < 0.01;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });
});
