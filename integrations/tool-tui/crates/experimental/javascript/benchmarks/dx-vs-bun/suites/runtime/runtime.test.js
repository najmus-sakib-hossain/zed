/**
 * Property-Based Tests for Runtime Benchmarks
 * Feature: dx-bun-benchmarks
 * Uses fast-check for property-based testing
 */

const fc = require('fast-check');
const { getStats, compareResults } = require('../../lib/stats');

describe('Runtime Benchmark Property Tests', () => {
    describe('Property 4: Speedup Calculation Correctness', () => {
        test('speedup ratio is max/min for time-based metrics', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 10, max: 100, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    fc.array(fc.double({ min: 200, max: 500, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    (fastTimes, slowTimes) => {
                        const result = compareResults({ times: fastTimes }, { times: slowTimes }, true);
                        const statsA = getStats(fastTimes);
                        const statsB = getStats(slowTimes);
                        const expectedSpeedup = Math.max(statsA.mean, statsB.mean) / Math.min(statsA.mean, statsB.mean);
                        return Math.abs(result.speedup - Math.round(expectedSpeedup * 100) / 100) < 0.1;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('speedup is always >= 1', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    (timesA, timesB) => {
                        const result = compareResults({ times: timesA }, { times: timesB });
                        return result.speedup >= 1;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('speedup calculation is symmetric', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    (timesA, timesB) => {
                        const resultAB = compareResults({ times: timesA }, { times: timesB });
                        const resultBA = compareResults({ times: timesB }, { times: timesA });
                        return Math.abs(resultAB.speedup - resultBA.speedup) < 0.01;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });
});
