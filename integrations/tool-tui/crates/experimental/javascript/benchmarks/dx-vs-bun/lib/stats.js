/**
 * Statistics Library for DX vs Bun Benchmarks
 * JavaScript implementation for testing and cross-platform use
 */

/**
 * Calculate comprehensive statistics from an array of measurements.
 * @param {number[]} values - Array of numeric measurements
 * @returns {Object} Statistics object with min, max, mean, median, stddev, p95, p99, count
 */
function getStats(values) {
    if (!values || values.length === 0) {
        return { min: 0, max: 0, mean: 0, median: 0, stddev: 0, p95: 0, p99: 0, count: 0 };
    }

    const sorted = [...values].sort((a, b) => a - b);
    const count = values.length;

    const min = sorted[0];
    const max = sorted[count - 1];
    const sum = values.reduce((a, b) => a + b, 0);
    const mean = sum / count;

    // Median
    let median;
    if (count % 2 === 0) {
        median = (sorted[count / 2 - 1] + sorted[count / 2]) / 2;
    } else {
        median = sorted[Math.floor(count / 2)];
    }

    // Standard deviation (sample)
    const sumSquaredDiff = values.reduce((acc, v) => acc + Math.pow(v - mean, 2), 0);
    const variance = count > 1 ? sumSquaredDiff / (count - 1) : 0;
    const stddev = Math.sqrt(variance);

    // Percentiles
    const p95Index = Math.min(Math.ceil(0.95 * count) - 1, count - 1);
    const p99Index = Math.min(Math.ceil(0.99 * count) - 1, count - 1);
    const p95 = sorted[p95Index];
    const p99 = sorted[p99Index];

    return { min, max, mean, median, stddev, p95, p99, count };
}

/**
 * Remove outliers using the IQR method.
 * @param {number[]} values - Array of numeric measurements
 * @param {number} factor - IQR multiplier (default 1.5)
 * @returns {number[]} Array with outliers removed
 */
function removeOutliers(values, factor = 1.5) {
    if (!values || values.length < 4) {
        return values || [];
    }

    const sorted = [...values].sort((a, b) => a - b);
    const count = values.length;

    const q1Index = Math.floor(count * 0.25);
    const q3Index = Math.floor(count * 0.75);

    const q1 = sorted[q1Index];
    const q3 = sorted[q3Index];
    const iqr = q3 - q1;

    const lowerBound = q1 - factor * iqr;
    const upperBound = q3 + factor * iqr;

    return values.filter(v => v >= lowerBound && v <= upperBound);
}

/**
 * T-distribution critical values for common confidence levels
 */
const T_VALUES = {
    0.90: { 5: 2.015, 10: 1.812, 20: 1.725, 30: 1.697, 100: 1.660 },
    0.95: { 5: 2.571, 10: 2.228, 20: 2.086, 30: 2.042, 100: 1.984 },
    0.99: { 5: 4.032, 10: 3.169, 20: 2.845, 30: 2.750, 100: 2.626 }
};

/**
 * Get t-value for given degrees of freedom and confidence level.
 * @param {number} df - Degrees of freedom
 * @param {number} confidence - Confidence level
 * @returns {number} T-value
 */
function getTValue(df, confidence) {
    const confKey = confidence.toString();
    const tTable = T_VALUES[confKey] || T_VALUES['0.95'];

    if (df <= 5) return tTable[5];
    if (df <= 10) return tTable[10];
    if (df <= 20) return tTable[20];
    if (df <= 30) return tTable[30];
    return tTable[100];
}

/**
 * Calculate confidence interval for measurements.
 * @param {number[]} values - Array of numeric measurements
 * @param {number} confidence - Confidence level (default 0.95)
 * @returns {Object} Object with lower, upper, marginOfError, mean
 */
function getConfidenceInterval(values, confidence = 0.95) {
    if (!values || values.length < 2) {
        const mean = values && values.length === 1 ? values[0] : 0;
        return { lower: mean, upper: mean, marginOfError: 0, mean };
    }

    const stats = getStats(values);
    const n = values.length;
    const df = n - 1;

    const tValue = getTValue(df, confidence);
    const standardError = stats.stddev / Math.sqrt(n);
    const marginOfError = tValue * standardError;

    return {
        lower: stats.mean - marginOfError,
        upper: stats.mean + marginOfError,
        marginOfError,
        mean: stats.mean
    };
}

/**
 * Compare two result sets and determine the winner.
 * @param {Object} resultA - First measurement set with 'times' array
 * @param {Object} resultB - Second measurement set with 'times' array
 * @param {boolean} lowerIsBetter - If true, lower values win (default true)
 * @param {number} threshold - Minimum percentage difference to declare winner (default 0.05)
 * @returns {Object} Comparison result with winner, speedup, significance
 */
function compareResults(resultA, resultB, lowerIsBetter = true, threshold = 0.05) {
    const statsA = getStats(resultA.times);
    const statsB = getStats(resultB.times);

    const ciA = getConfidenceInterval(resultA.times);
    const ciB = getConfidenceInterval(resultB.times);

    // Check for overlap in confidence intervals
    const overlaps = ciA.lower <= ciB.upper && ciB.lower <= ciA.upper;

    const meanA = statsA.mean;
    const meanB = statsB.mean;

    if (meanA === 0 || meanB === 0) {
        return {
            winner: 'tie',
            speedup: 1.0,
            isSignificant: false,
            statsA,
            statsB
        };
    }

    let winner, speedup;
    if (lowerIsBetter) {
        if (meanA < meanB) {
            winner = 'A';
            speedup = meanB / meanA;
        } else if (meanB < meanA) {
            winner = 'B';
            speedup = meanA / meanB;
        } else {
            winner = 'tie';
            speedup = 1.0;
        }
    } else {
        if (meanA > meanB) {
            winner = 'A';
            speedup = meanA / meanB;
        } else if (meanB > meanA) {
            winner = 'B';
            speedup = meanB / meanA;
        } else {
            winner = 'tie';
            speedup = 1.0;
        }
    }

    const percentDiff = Math.abs(meanA - meanB) / Math.max(meanA, meanB);
    const isSignificant = !overlaps && percentDiff > threshold;

    if (!isSignificant) {
        winner = 'tie';
    }

    return {
        winner,
        speedup: Math.round(speedup * 100) / 100,
        isSignificant,
        percentDiff: Math.round(percentDiff * 1000) / 10,
        statsA,
        statsB,
        ciA,
        ciB
    };
}

module.exports = {
    getStats,
    removeOutliers,
    getConfidenceInterval,
    compareResults,
    getTValue
};
