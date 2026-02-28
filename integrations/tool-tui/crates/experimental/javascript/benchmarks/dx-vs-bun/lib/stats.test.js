/**
 * Property-Based Tests for Statistics Library
 * Feature: dx-bun-benchmarks
 * 
 * Uses fast-check for property-based testing
 */

const fc = require('fast-check');
const { getStats, removeOutliers, getConfidenceInterval, compareResults } = require('./stats');

describe('Statistics Library Property Tests', () => {
    /**
     * Property 1: Statistics Calculation Correctness
     * For any array of benchmark measurements with at least 2 values,
     * the calculated statistics SHALL be mathematically correct.
     * **Validates: Requirements 1.5**
     */
    describe('Property 1: Statistics Calculation Correctness', () => {
        test('min <= all values <= max', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        return values.every(v => v >= stats.min && v <= stats.max);
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('mean equals sum divided by count', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        const expectedMean = values.reduce((a, b) => a + b, 0) / values.length;
                        return Math.abs(stats.mean - expectedMean) < 0.0001;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('median is middle value when sorted', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        const sorted = [...values].sort((a, b) => a - b);
                        const n = sorted.length;
                        let expectedMedian;
                        if (n % 2 === 0) {
                            expectedMedian = (sorted[n / 2 - 1] + sorted[n / 2]) / 2;
                        } else {
                            expectedMedian = sorted[Math.floor(n / 2)];
                        }
                        return Math.abs(stats.median - expectedMedian) < 0.0001;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('stddev is non-negative', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        return stats.stddev >= 0;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('count matches input length', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 1, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        return stats.count === values.length;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('percentiles are within range', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const stats = getStats(values);
                        return stats.p95 >= stats.min &&
                            stats.p95 <= stats.max &&
                            stats.p99 >= stats.min &&
                            stats.p99 <= stats.max &&
                            stats.p99 >= stats.p95;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Property 6: Confidence Interval Calculation
     * For any array of benchmark measurements, the confidence interval SHALL be
     * calculated using the formula: mean ± (t-value × stddev / √n)
     * **Validates: Requirements 9.5**
     */
    describe('Property 6: Confidence Interval Calculation', () => {
        test('confidence interval contains the mean', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const ci = getConfidenceInterval(values);
                        return ci.lower <= ci.mean && ci.mean <= ci.upper;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('margin of error is non-negative', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const ci = getConfidenceInterval(values);
                        return ci.marginOfError >= 0;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('interval is symmetric around mean', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 2, maxLength: 100 }),
                    (values) => {
                        const ci = getConfidenceInterval(values);
                        const lowerDiff = ci.mean - ci.lower;
                        const upperDiff = ci.upper - ci.mean;
                        return Math.abs(lowerDiff - upperDiff) < 0.0001;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('higher confidence level produces wider interval', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 10, maxLength: 50 }),
                    (values) => {
                        const ci90 = getConfidenceInterval(values, 0.90);
                        const ci95 = getConfidenceInterval(values, 0.95);
                        const ci99 = getConfidenceInterval(values, 0.99);
                        // Higher confidence should have wider or equal intervals
                        return ci95.marginOfError >= ci90.marginOfError * 0.99 &&
                            ci99.marginOfError >= ci95.marginOfError * 0.99;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('larger sample size reduces margin of error', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 50, max: 150, noNaN: true }),
                    fc.double({ min: 1, max: 20, noNaN: true }),
                    (baseMean, stddev) => {
                        // Generate samples of different sizes with similar distribution
                        const smallSample = Array.from({ length: 10 }, () => baseMean + (Math.random() - 0.5) * stddev);
                        const largeSample = Array.from({ length: 50 }, () => baseMean + (Math.random() - 0.5) * stddev);

                        const ciSmall = getConfidenceInterval(smallSample);
                        const ciLarge = getConfidenceInterval(largeSample);

                        // Larger sample should generally have smaller margin of error
                        // Allow some tolerance due to random variation
                        return ciLarge.marginOfError <= ciSmall.marginOfError * 1.5;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Additional property tests for outlier removal
     */
    describe('Outlier Removal Properties', () => {
        test('result is subset of input', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 4, maxLength: 100 }),
                    (values) => {
                        const filtered = removeOutliers(values);
                        return filtered.every(v => values.includes(v));
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('result length is at most input length', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 0.001, max: 10000, noNaN: true }), { minLength: 4, maxLength: 100 }),
                    (values) => {
                        const filtered = removeOutliers(values);
                        return filtered.length <= values.length;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('uniform data has no outliers removed', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 1, max: 100, noNaN: true }),
                    fc.integer({ min: 10, max: 50 }),
                    (value, count) => {
                        const values = Array(count).fill(value);
                        const filtered = removeOutliers(values);
                        return filtered.length === values.length;
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Compare Results Properties
     */
    describe('Compare Results Properties', () => {
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

        test('winner is one of A, B, or tie', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    (timesA, timesB) => {
                        const result = compareResults({ times: timesA }, { times: timesB });
                        return ['A', 'B', 'tie'].includes(result.winner);
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('identical inputs produce tie', () => {
            fc.assert(
                fc.property(
                    fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
                    (times) => {
                        const result = compareResults({ times }, { times });
                        return result.winner === 'tie' && result.speedup === 1;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('lower is better correctly identifies faster tool', () => {
            fc.assert(
                fc.property(
                    fc.double({ min: 10, max: 100, noNaN: true }),
                    fc.double({ min: 200, max: 500, noNaN: true }),
                    fc.integer({ min: 10, max: 20 }),
                    (fastBase, slowBase, count) => {
                        // Create clearly different distributions
                        const fastTimes = Array.from({ length: count }, () => fastBase + Math.random() * 5);
                        const slowTimes = Array.from({ length: count }, () => slowBase + Math.random() * 5);

                        const result = compareResults({ times: fastTimes }, { times: slowTimes }, true);
                        // A should win since it has lower times
                        return result.winner === 'A' || result.winner === 'tie';
                    }
                ),
                { numRuns: 100 }
            );
        });
    });
});
