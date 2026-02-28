/**
 * Property-Based Tests for Reporter Library
 * Feature: dx-bun-benchmarks
 * 
 * Uses fast-check for property-based testing
 */

const fc = require('fast-check');
const { newJsonReport, newMarkdownReport, newSummary, validateResults } = require('./reporter');

// Arbitrary generators for benchmark data
const statsArb = fc.record({
    min: fc.double({ min: 0.1, max: 100, noNaN: true }),
    max: fc.double({ min: 100, max: 1000, noNaN: true }),
    mean: fc.double({ min: 50, max: 500, noNaN: true }),
    median: fc.double({ min: 50, max: 500, noNaN: true }),
    stddev: fc.double({ min: 0, max: 50, noNaN: true }),
    p95: fc.double({ min: 100, max: 800, noNaN: true }),
    p99: fc.double({ min: 100, max: 900, noNaN: true }),
    count: fc.integer({ min: 10, max: 100 })
});

const measurementArb = fc.record({
    times: fc.array(fc.double({ min: 1, max: 1000, noNaN: true }), { minLength: 5, maxLength: 20 }),
    stats: statsArb
});

const benchmarkArb = fc.record({
    name: fc.string({ minLength: 1, maxLength: 30 }).filter(s => s.trim().length > 0),
    unit: fc.constantFrom('ms', 'µs', 'ops/s', 'MB', 'tests/s'),
    dx: measurementArb,
    bun: measurementArb,
    winner: fc.constantFrom('dx', 'bun', 'tie'),
    speedup: fc.double({ min: 1, max: 10, noNaN: true }),
    isSignificant: fc.boolean()
});

const suiteArb = fc.record({
    name: fc.string({ minLength: 1, maxLength: 30 }).filter(s => s.trim().length > 0),
    benchmarks: fc.array(benchmarkArb, { minLength: 1, maxLength: 5 }),
    winner: fc.constantFrom('dx', 'bun', 'tie'),
    totalSpeedup: fc.double({ min: 1, max: 10, noNaN: true })
});

const systemArb = fc.record({
    os: fc.constantFrom('Windows', 'Linux', 'macOS'),
    platform: fc.constantFrom('win32', 'linux', 'darwin'),
    cpu: fc.string({ minLength: 5, maxLength: 50 }),
    cores: fc.integer({ min: 1, max: 128 }),
    memory: fc.string({ minLength: 2, maxLength: 20 }),
    dxVersion: fc.string({ minLength: 1, maxLength: 20 }),
    bunVersion: fc.string({ minLength: 1, maxLength: 20 })
});

const summaryArb = fc.record({
    totalBenchmarks: fc.integer({ min: 1, max: 100 }),
    dxWins: fc.integer({ min: 0, max: 50 }),
    bunWins: fc.integer({ min: 0, max: 50 }),
    ties: fc.integer({ min: 0, max: 50 }),
    overallWinner: fc.constantFrom('dx', 'bun', 'tie'),
    categories: fc.array(
        fc.record({
            name: fc.string({ minLength: 1, maxLength: 20 }),
            winner: fc.constantFrom('dx', 'bun', 'tie'),
            speedup: fc.double({ min: 1, max: 10, noNaN: true })
        }),
        { minLength: 0, maxLength: 6 }
    )
});

const resultsArb = fc.record({
    name: fc.string({ minLength: 1, maxLength: 50 }),
    timestamp: fc.date().map(d => d.toISOString()),
    system: systemArb,
    suites: fc.array(suiteArb, { minLength: 1, maxLength: 6 }),
    summary: summaryArb
});

describe('Reporter Library Property Tests', () => {
    /**
     * Property 7: JSON Output Validity
     * For any benchmark run, the generated JSON output SHALL be valid JSON
     * that can be parsed without errors, and SHALL contain all required fields.
     * **Validates: Requirements 14.6**
     */
    describe('Property 7: JSON Output Validity', () => {
        test('JSON output is valid and parseable', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const jsonStr = newJsonReport(results);

                    // Should not throw when parsing
                    let parsed;
                    try {
                        parsed = JSON.parse(jsonStr);
                    } catch (e) {
                        return false;
                    }

                    return parsed !== null && typeof parsed === 'object';
                }),
                { numRuns: 100 }
            );
        });

        test('JSON output contains all required fields', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const jsonStr = newJsonReport(results);
                    const parsed = JSON.parse(jsonStr);

                    const requiredFields = ['name', 'timestamp', 'system', 'suites', 'summary'];
                    return requiredFields.every(field => field in parsed);
                }),
                { numRuns: 100 }
            );
        });

        test('JSON round-trip preserves structure', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const jsonStr = newJsonReport(results);
                    const parsed = JSON.parse(jsonStr);

                    // Re-serialize and parse again
                    const jsonStr2 = JSON.stringify(parsed, null, 2);
                    const parsed2 = JSON.parse(jsonStr2);

                    // Should have same structure
                    return parsed2.name === parsed.name &&
                        parsed2.timestamp === parsed.timestamp &&
                        Array.isArray(parsed2.suites) &&
                        typeof parsed2.summary === 'object';
                }),
                { numRuns: 100 }
            );
        });

        test('JSON output handles missing optional fields gracefully', () => {
            fc.assert(
                fc.property(
                    fc.record({
                        name: fc.option(fc.string(), { nil: undefined }),
                        timestamp: fc.option(fc.date().map(d => d.toISOString()), { nil: undefined }),
                        system: fc.option(systemArb, { nil: undefined }),
                        suites: fc.option(fc.array(suiteArb, { minLength: 0, maxLength: 3 }), { nil: undefined }),
                        summary: fc.option(summaryArb, { nil: undefined })
                    }),
                    (partialResults) => {
                        const jsonStr = newJsonReport(partialResults);

                        // Should not throw
                        let parsed;
                        try {
                            parsed = JSON.parse(jsonStr);
                        } catch (e) {
                            return false;
                        }

                        // Should have default values for missing fields
                        return typeof parsed.name === 'string' &&
                            typeof parsed.timestamp === 'string' &&
                            typeof parsed.system === 'object' &&
                            Array.isArray(parsed.suites) &&
                            typeof parsed.summary === 'object';
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Additional reporter properties
     */
    describe('Markdown Report Properties', () => {
        test('markdown output is non-empty string', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const markdown = newMarkdownReport(results);
                    return typeof markdown === 'string' && markdown.length > 0;
                }),
                { numRuns: 100 }
            );
        });

        test('markdown contains header', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const markdown = newMarkdownReport(results);
                    return markdown.includes('# DX vs Bun Benchmark Results');
                }),
                { numRuns: 100 }
            );
        });

        test('markdown contains system information section', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const markdown = newMarkdownReport(results);
                    return markdown.includes('## System Information');
                }),
                { numRuns: 100 }
            );
        });

        test('markdown contains summary section', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const markdown = newMarkdownReport(results);
                    return markdown.includes('## Summary');
                }),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Summary calculation properties
     */
    describe('Summary Calculation Properties', () => {
        test('total benchmarks equals sum of wins and ties', () => {
            fc.assert(
                fc.property(
                    fc.array(suiteArb, { minLength: 1, maxLength: 6 }),
                    (suites) => {
                        const summary = newSummary(suites);
                        return summary.totalBenchmarks ===
                            summary.dxWins + summary.bunWins + summary.ties;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('overall winner is consistent with win counts', () => {
            fc.assert(
                fc.property(
                    fc.array(suiteArb, { minLength: 1, maxLength: 6 }),
                    (suites) => {
                        const summary = newSummary(suites);

                        if (summary.dxWins > summary.bunWins) {
                            return summary.overallWinner === 'dx';
                        } else if (summary.bunWins > summary.dxWins) {
                            return summary.overallWinner === 'bun';
                        } else {
                            return summary.overallWinner === 'tie';
                        }
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('categories array has one entry per suite', () => {
            fc.assert(
                fc.property(
                    fc.array(suiteArb, { minLength: 1, maxLength: 6 }),
                    (suites) => {
                        const summary = newSummary(suites);
                        return summary.categories.length === suites.length;
                    }
                ),
                { numRuns: 100 }
            );
        });

        test('category names match suite names', () => {
            fc.assert(
                fc.property(
                    fc.array(suiteArb, { minLength: 1, maxLength: 6 }),
                    (suites) => {
                        const summary = newSummary(suites);
                        return suites.every((suite, i) =>
                            summary.categories[i].name === suite.name
                        );
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    /**
     * Validation properties
     */
    describe('Validation Properties', () => {
        test('complete results pass validation', () => {
            fc.assert(
                fc.property(resultsArb, (results) => {
                    const validation = validateResults(results);
                    return validation.isValid === true &&
                        validation.missingFields.length === 0;
                }),
                { numRuns: 100 }
            );
        });

        test('missing fields are correctly identified', () => {
            fc.assert(
                fc.property(
                    fc.constantFrom('name', 'timestamp', 'system', 'suites', 'summary'),
                    resultsArb,
                    (fieldToRemove, results) => {
                        const partialResults = { ...results };
                        delete partialResults[fieldToRemove];

                        const validation = validateResults(partialResults);
                        return validation.isValid === false &&
                            validation.missingFields.includes(fieldToRemove);
                    }
                ),
                { numRuns: 100 }
            );
        });
    });
});
