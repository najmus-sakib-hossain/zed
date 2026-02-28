//! Statistical analysis implementation

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Statistical metrics computed from timing measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub confidence_interval_95: (f64, f64),
    pub coefficient_of_variation: f64,
    pub outliers: Vec<usize>,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            confidence_interval_95: (0.0, 0.0),
            coefficient_of_variation: 0.0,
            outliers: vec![],
        }
    }
}

/// Result of comparing two measurement sets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub baseline_stats: Statistics,
    pub subject_stats: Statistics,
    pub speedup: f64,
    pub speedup_ci: (f64, f64),
    pub is_significant: bool,
    pub p_value: f64,
}

/// Statistical analyzer for benchmark results
#[derive(Default)]
pub struct StatisticalAnalyzer;

impl StatisticalAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Compute statistics from timing measurements
    pub fn compute_statistics(&self, timings: &[Duration]) -> Statistics {
        if timings.is_empty() {
            return Statistics::default();
        }

        let values: Vec<f64> = timings.iter().map(|d| d.as_secs_f64()).collect();
        self.compute_statistics_from_f64(&values)
    }

    /// Compute statistics from f64 values (in seconds)
    pub fn compute_statistics_from_f64(&self, values: &[f64]) -> Statistics {
        if values.is_empty() {
            return Statistics::default();
        }

        let n = values.len();

        // Single value case
        if n == 1 {
            let val = values[0];
            return Statistics {
                mean: val,
                median: val,
                std_dev: 0.0,
                min: val,
                max: val,
                p50: val,
                p95: val,
                p99: val,
                confidence_interval_95: (val, val),
                coefficient_of_variation: 0.0,
                outliers: vec![],
            };
        }

        // Sort for percentile calculations
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mean = values.iter().sum::<f64>() / n as f64;
        let median = Self::percentile(&sorted, 50.0);
        let min = sorted[0];
        let max = sorted[n - 1];
        let p50 = median;
        let p95 = Self::percentile(&sorted, 95.0);
        let p99 = Self::percentile(&sorted, 99.0);

        // Standard deviation (sample)
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();

        // Coefficient of variation
        let coefficient_of_variation = if mean > 0.0 { std_dev / mean } else { 0.0 };

        // Detect outliers using IQR method
        let outliers = self.detect_outliers_internal(&sorted, values);

        // Confidence interval (computed later with proper method)
        let confidence_interval_95 = self.compute_confidence_interval_internal(mean, std_dev, n);

        Statistics {
            mean,
            median,
            std_dev,
            min,
            max,
            p50,
            p95,
            p99,
            confidence_interval_95,
            coefficient_of_variation,
            outliers,
        }
    }

    /// Calculate percentile from sorted values
    fn percentile(sorted: &[f64], p: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        if sorted.len() == 1 {
            return sorted[0];
        }

        let n = sorted.len();
        let rank = (p / 100.0) * (n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        let frac = rank - lower as f64;

        if upper >= n {
            sorted[n - 1]
        } else if lower == upper {
            sorted[lower]
        } else {
            sorted[lower] * (1.0 - frac) + sorted[upper] * frac
        }
    }

    /// Internal outlier detection using IQR method
    fn detect_outliers_internal(&self, sorted: &[f64], original: &[f64]) -> Vec<usize> {
        if sorted.len() < 4 {
            return vec![];
        }

        let q1 = Self::percentile(sorted, 25.0);
        let q3 = Self::percentile(sorted, 75.0);
        let iqr = q3 - q1;
        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        original
            .iter()
            .enumerate()
            .filter(|(_, &v)| v < lower_bound || v > upper_bound)
            .map(|(i, _)| i)
            .collect()
    }

    /// Internal confidence interval computation
    fn compute_confidence_interval_internal(
        &self,
        mean: f64,
        std_dev: f64,
        n: usize,
    ) -> (f64, f64) {
        if n < 2 {
            return (mean, mean);
        }

        // Use t-distribution critical value for 95% CI
        // For n >= 30, use z = 1.96; for smaller n, use approximate t-values
        let t_critical = if n >= 30 {
            1.96
        } else {
            // Approximate t-values for common sample sizes
            match n {
                2 => 12.706,
                3 => 4.303,
                4 => 3.182,
                5 => 2.776,
                6 => 2.571,
                7 => 2.447,
                8 => 2.365,
                9 => 2.306,
                10 => 2.262,
                11..=15 => 2.145,
                16..=20 => 2.086,
                21..=29 => 2.045,
                _ => 1.96,
            }
        };

        let standard_error = std_dev / (n as f64).sqrt();
        let margin = t_critical * standard_error;

        (mean - margin, mean + margin)
    }

    /// Compute 95% confidence interval for a set of values
    /// Uses t-distribution for sample sizes < 30, normal approximation for >= 30
    pub fn compute_confidence_interval(&self, values: &[f64]) -> (f64, f64) {
        if values.is_empty() {
            return (0.0, 0.0);
        }
        if values.len() == 1 {
            return (values[0], values[0]);
        }

        let n = values.len();
        let mean = values.iter().sum::<f64>() / n as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();

        self.compute_confidence_interval_internal(mean, std_dev, n)
    }

    /// Detect outliers using IQR (Interquartile Range) method
    /// Returns indices of values that fall below Q1 - 1.5*IQR or above Q3 + 1.5*IQR
    pub fn detect_outliers(&self, values: &[f64]) -> Vec<usize> {
        if values.len() < 4 {
            return vec![];
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        self.detect_outliers_internal(&sorted, values)
    }

    /// Get the IQR bounds used for outlier detection
    /// Returns (lower_bound, upper_bound) where outliers are values outside this range
    pub fn get_outlier_bounds(&self, values: &[f64]) -> Option<(f64, f64)> {
        if values.len() < 4 {
            return None;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1 = Self::percentile(&sorted, 25.0);
        let q3 = Self::percentile(&sorted, 75.0);
        let iqr = q3 - q1;
        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        Some((lower_bound, upper_bound))
    }

    /// Perform Welch's t-test for two samples with unequal variances
    /// Returns (t_statistic, p_value)
    ///
    /// Welch's t-test is more robust than Student's t-test when the two samples
    /// have unequal variances and/or unequal sample sizes.
    pub fn welch_t_test(&self, a: &[f64], b: &[f64]) -> (f64, f64) {
        if a.len() < 2 || b.len() < 2 {
            return (0.0, 1.0); // Cannot compute, return non-significant
        }

        let n1 = a.len() as f64;
        let n2 = b.len() as f64;

        let mean1 = a.iter().sum::<f64>() / n1;
        let mean2 = b.iter().sum::<f64>() / n2;

        let var1 = a.iter().map(|x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
        let var2 = b.iter().map(|x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);

        // Handle zero variance case
        if var1 == 0.0 && var2 == 0.0 {
            if (mean1 - mean2).abs() < 1e-10 {
                return (0.0, 1.0); // Identical distributions
            } else {
                return (f64::INFINITY, 0.0); // Perfectly different
            }
        }

        let se1 = var1 / n1;
        let se2 = var2 / n2;
        let se = (se1 + se2).sqrt();

        if se == 0.0 {
            return (0.0, 1.0);
        }

        let t_statistic = (mean1 - mean2) / se;

        // Welch-Satterthwaite degrees of freedom
        let df_num = (se1 + se2).powi(2);
        let df_denom = (se1.powi(2) / (n1 - 1.0)) + (se2.powi(2) / (n2 - 1.0));
        let df = if df_denom > 0.0 {
            df_num / df_denom
        } else {
            1.0
        };

        // Approximate p-value using t-distribution
        let p_value = self.t_distribution_p_value(t_statistic.abs(), df);

        (t_statistic, p_value)
    }

    /// Approximate two-tailed p-value from t-distribution
    /// Uses approximation suitable for most practical purposes
    fn t_distribution_p_value(&self, t: f64, df: f64) -> f64 {
        if df <= 0.0 {
            return 1.0;
        }

        // Use approximation: for large df, t-distribution approaches normal
        // For smaller df, use a simple approximation
        let x = df / (df + t * t);

        // Incomplete beta function approximation for t-distribution CDF
        // This is a simplified approximation
        let p = if df > 100.0 {
            // Use normal approximation for large df
            2.0 * (1.0 - self.normal_cdf(t))
        } else {
            // Use regularized incomplete beta function approximation
            self.incomplete_beta_approx(df / 2.0, 0.5, x)
        };

        p.clamp(0.0, 1.0)
    }

    /// Approximate standard normal CDF
    fn normal_cdf(&self, x: f64) -> f64 {
        // Approximation using error function
        0.5 * (1.0 + self.erf(x / std::f64::consts::SQRT_2))
    }

    /// Approximate error function
    fn erf(&self, x: f64) -> f64 {
        // Horner form approximation
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }

    /// Approximate regularized incomplete beta function
    fn incomplete_beta_approx(&self, a: f64, b: f64, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }

        // Simple approximation using continued fraction
        // This is accurate enough for p-value estimation
        let bt = if x == 0.0 || x == 1.0 {
            0.0
        } else {
            (Self::ln_gamma(a + b) - Self::ln_gamma(a) - Self::ln_gamma(b)
                + a * x.ln()
                + b * (1.0 - x).ln())
            .exp()
        };

        if x < (a + 1.0) / (a + b + 2.0) {
            bt * self.beta_cf(a, b, x) / a
        } else {
            1.0 - bt * self.beta_cf(b, a, 1.0 - x) / b
        }
    }

    /// Continued fraction for incomplete beta
    fn beta_cf(&self, a: f64, b: f64, x: f64) -> f64 {
        let max_iter = 100;
        let eps = 1e-10;

        let qab = a + b;
        let qap = a + 1.0;
        let qam = a - 1.0;

        let mut c = 1.0;
        let mut d = 1.0 - qab * x / qap;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        d = 1.0 / d;
        let mut h = d;

        for m in 1..=max_iter {
            let m = m as f64;
            let m2 = 2.0 * m;

            // Even step
            let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
            d = 1.0 + aa * d;
            if d.abs() < 1e-30 {
                d = 1e-30;
            }
            c = 1.0 + aa / c;
            if c.abs() < 1e-30 {
                c = 1e-30;
            }
            d = 1.0 / d;
            h *= d * c;

            // Odd step
            let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
            d = 1.0 + aa * d;
            if d.abs() < 1e-30 {
                d = 1e-30;
            }
            c = 1.0 + aa / c;
            if c.abs() < 1e-30 {
                c = 1e-30;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;

            if (del - 1.0).abs() < eps {
                break;
            }
        }

        h
    }

    /// Approximate log gamma function using Stirling's approximation
    fn ln_gamma(x: f64) -> f64 {
        if x <= 0.0 {
            return f64::INFINITY;
        }

        // Lanczos approximation coefficients
        #[allow(clippy::excessive_precision)]
        let g = 7.0;
        #[allow(clippy::excessive_precision)]
        let c = [
            0.99999999999980993,
            676.5203681218851,
            -1259.1392167224028,
            771.32342877765313,
            -176.61502916214059,
            12.507343278686905,
            -0.13857109526572012,
            9.9843695780195716e-6,
            1.5056327351493116e-7,
        ];

        if x < 0.5 {
            std::f64::consts::PI.ln()
                - (std::f64::consts::PI * x).sin().ln()
                - Self::ln_gamma(1.0 - x)
        } else {
            let x = x - 1.0;
            let mut a = c[0];
            for (i, coeff) in c.iter().enumerate().skip(1) {
                a += coeff / (x + i as f64);
            }
            let t = x + g + 0.5;
            0.5 * (2.0 * std::f64::consts::PI).ln() + (t.ln() * (x + 0.5)) - t + a.ln()
        }
    }

    /// Compare two measurement sets and return comparison result
    pub fn compare(&self, baseline: &[f64], subject: &[f64]) -> ComparisonResult {
        let baseline_stats = self.compute_statistics_from_f64(baseline);
        let subject_stats = self.compute_statistics_from_f64(subject);

        let speedup = if subject_stats.mean > 0.0 {
            baseline_stats.mean / subject_stats.mean
        } else {
            f64::INFINITY
        };

        let (_t_stat, p_value) = self.welch_t_test(baseline, subject);
        let is_significant = p_value < 0.05;

        // Approximate speedup confidence interval
        let speedup_ci = self.compute_speedup_ci(&baseline_stats, &subject_stats);

        ComparisonResult {
            baseline_stats,
            subject_stats,
            speedup,
            speedup_ci,
            is_significant,
            p_value,
        }
    }

    /// Compute approximate confidence interval for speedup ratio
    fn compute_speedup_ci(&self, baseline: &Statistics, subject: &Statistics) -> (f64, f64) {
        if subject.mean <= 0.0 {
            return (0.0, f64::INFINITY);
        }

        // Use Fieller's method approximation for ratio CI
        let ratio = baseline.mean / subject.mean;

        // Approximate relative error
        let cv_baseline = baseline.coefficient_of_variation;
        let cv_subject = subject.coefficient_of_variation;
        let combined_cv = (cv_baseline.powi(2) + cv_subject.powi(2)).sqrt();

        let margin = 1.96 * combined_cv * ratio;

        ((ratio - margin).max(0.0), ratio + margin)
    }

    /// Check if results have high variance (coefficient of variation > 10%)
    /// Returns true if the results should be flagged as potentially unreliable
    pub fn has_high_variance(&self, stats: &Statistics) -> bool {
        stats.coefficient_of_variation > 0.10
    }

    /// Check if values have high variance (coefficient of variation > 10%)
    /// Returns true if the results should be flagged as potentially unreliable
    pub fn check_variance_warning(&self, values: &[f64]) -> bool {
        if values.len() < 2 {
            return false;
        }

        let stats = self.compute_statistics_from_f64(values);
        self.has_high_variance(&stats)
    }

    /// Get variance warning message if applicable
    pub fn get_variance_warning(&self, stats: &Statistics) -> Option<String> {
        if self.has_high_variance(stats) {
            Some(format!(
                "High variance detected (CV = {:.1}%). Results may be unreliable.",
                stats.coefficient_of_variation * 100.0
            ))
        } else {
            None
        }
    }
}
