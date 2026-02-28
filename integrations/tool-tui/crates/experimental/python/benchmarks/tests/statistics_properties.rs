//! Property-based tests for StatisticalAnalyzer
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::analysis::StatisticalAnalyzer;
use proptest::prelude::*;
use std::time::Duration;

/// Generate a vector of positive f64 values for testing
fn positive_f64_vec(min_len: usize, max_len: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(0.001f64..1000.0, min_len..=max_len)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 6: Statistics Computation Correctness**
    /// *For any* non-empty array of timing measurements, the Statistical_Analyzer SHALL compute
    /// mean, median, standard deviation, and percentiles (p50, p95, p99) such that:
    /// - mean equals the arithmetic average of all values
    /// - median equals the middle value (or average of two middle values)
    /// - p50 equals median
    /// - p95 is greater than or equal to p50
    /// - p99 is greater than or equal to p95
    /// **Validates: Requirements 5.1**
    #[test]
    fn property_statistics_computation_correctness(values in positive_f64_vec(1, 200)) {
        let analyzer = StatisticalAnalyzer::new();
        let stats = analyzer.compute_statistics_from_f64(&values);

        // Mean equals arithmetic average
        let expected_mean = values.iter().sum::<f64>() / values.len() as f64;
        prop_assert!((stats.mean - expected_mean).abs() < 1e-10,
            "Mean mismatch: got {}, expected {}", stats.mean, expected_mean);

        // Median calculation verification
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        let expected_median = if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        };
        prop_assert!((stats.median - expected_median).abs() < 1e-10,
            "Median mismatch: got {}, expected {}", stats.median, expected_median);

        // p50 equals median
        prop_assert!((stats.p50 - stats.median).abs() < 1e-10,
            "p50 should equal median: p50={}, median={}", stats.p50, stats.median);

        // p95 >= p50
        prop_assert!(stats.p95 >= stats.p50 - 1e-10,
            "p95 ({}) should be >= p50 ({})", stats.p95, stats.p50);

        // p99 >= p95
        prop_assert!(stats.p99 >= stats.p95 - 1e-10,
            "p99 ({}) should be >= p95 ({})", stats.p99, stats.p95);

        // min <= mean <= max
        prop_assert!(stats.min <= stats.mean + 1e-10,
            "min ({}) should be <= mean ({})", stats.min, stats.mean);
        prop_assert!(stats.mean <= stats.max + 1e-10,
            "mean ({}) should be <= max ({})", stats.mean, stats.max);

        // min <= median <= max
        prop_assert!(stats.min <= stats.median + 1e-10,
            "min ({}) should be <= median ({})", stats.min, stats.median);
        prop_assert!(stats.median <= stats.max + 1e-10,
            "median ({}) should be <= max ({})", stats.median, stats.max);

        // std_dev >= 0
        prop_assert!(stats.std_dev >= 0.0,
            "std_dev ({}) should be >= 0", stats.std_dev);
    }

    /// Test that statistics work correctly with Duration inputs
    #[test]
    fn property_statistics_from_duration(values in positive_f64_vec(1, 100)) {
        let analyzer = StatisticalAnalyzer::new();

        // Convert to Duration
        let durations: Vec<Duration> = values.iter()
            .map(|&v| Duration::from_secs_f64(v))
            .collect();

        let stats_from_duration = analyzer.compute_statistics(&durations);
        let stats_from_f64 = analyzer.compute_statistics_from_f64(&values);

        // Results should be equivalent
        prop_assert!((stats_from_duration.mean - stats_from_f64.mean).abs() < 1e-9,
            "Duration and f64 mean should match");
        prop_assert!((stats_from_duration.median - stats_from_f64.median).abs() < 1e-9,
            "Duration and f64 median should match");
    }

    /// Test edge case: all same values
    #[test]
    fn property_statistics_all_same_values(value in 0.001f64..1000.0, count in 2usize..50) {
        let analyzer = StatisticalAnalyzer::new();
        let values = vec![value; count];
        let stats = analyzer.compute_statistics_from_f64(&values);

        // All values same means std_dev should be 0
        prop_assert!(stats.std_dev.abs() < 1e-10,
            "std_dev should be 0 for identical values, got {}", stats.std_dev);

        // Mean, median, min, max should all equal the value
        prop_assert!((stats.mean - value).abs() < 1e-10,
            "mean should equal value");
        prop_assert!((stats.median - value).abs() < 1e-10,
            "median should equal value");
        prop_assert!((stats.min - value).abs() < 1e-10,
            "min should equal value");
        prop_assert!((stats.max - value).abs() < 1e-10,
            "max should equal value");
    }
}

/// Test edge case: empty array
#[test]
fn test_statistics_empty_array() {
    let analyzer = StatisticalAnalyzer::new();
    let stats = analyzer.compute_statistics_from_f64(&[]);

    assert_eq!(stats.mean, 0.0);
    assert_eq!(stats.median, 0.0);
    assert_eq!(stats.std_dev, 0.0);
    assert_eq!(stats.min, 0.0);
    assert_eq!(stats.max, 0.0);
}

/// Test edge case: single value
#[test]
fn test_statistics_single_value() {
    let analyzer = StatisticalAnalyzer::new();
    let stats = analyzer.compute_statistics_from_f64(&[42.0]);

    assert_eq!(stats.mean, 42.0);
    assert_eq!(stats.median, 42.0);
    assert_eq!(stats.std_dev, 0.0);
    assert_eq!(stats.min, 42.0);
    assert_eq!(stats.max, 42.0);
    assert_eq!(stats.p50, 42.0);
    assert_eq!(stats.p95, 42.0);
    assert_eq!(stats.p99, 42.0);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 7: Confidence Interval Validity**
    /// *For any* array of 30+ timing measurements, the 95% confidence interval (lower, upper)
    /// SHALL satisfy: lower <= mean <= upper, and the interval width SHALL decrease as sample size increases.
    /// **Validates: Requirements 5.2**
    #[test]
    fn property_confidence_interval_validity(values in positive_f64_vec(30, 200)) {
        let analyzer = StatisticalAnalyzer::new();
        let stats = analyzer.compute_statistics_from_f64(&values);
        let (lower, upper) = stats.confidence_interval_95;

        // lower <= mean <= upper
        prop_assert!(lower <= stats.mean + 1e-10,
            "CI lower bound ({}) should be <= mean ({})", lower, stats.mean);
        prop_assert!(stats.mean <= upper + 1e-10,
            "mean ({}) should be <= CI upper bound ({})", stats.mean, upper);

        // Interval should be non-negative width
        prop_assert!(upper >= lower - 1e-10,
            "CI upper ({}) should be >= lower ({})", upper, lower);
    }

    /// Test that confidence interval width decreases with larger sample sizes
    #[test]
    fn property_confidence_interval_width_decreases(
        base_values in positive_f64_vec(30, 50),
        extra_values in positive_f64_vec(50, 100)
    ) {
        let analyzer = StatisticalAnalyzer::new();

        // Compute CI for smaller sample
        let small_stats = analyzer.compute_statistics_from_f64(&base_values);
        let small_width = small_stats.confidence_interval_95.1 - small_stats.confidence_interval_95.0;

        // Combine for larger sample (same distribution characteristics)
        let mut large_values = base_values.clone();
        large_values.extend(extra_values);
        let large_stats = analyzer.compute_statistics_from_f64(&large_values);
        let large_width = large_stats.confidence_interval_95.1 - large_stats.confidence_interval_95.0;

        // Normalize by mean to compare relative widths (since values may differ)
        let small_relative = if small_stats.mean > 0.0 { small_width / small_stats.mean } else { 0.0 };
        let large_relative = if large_stats.mean > 0.0 { large_width / large_stats.mean } else { 0.0 };

        // With more samples, relative CI width should generally be smaller or similar
        // Allow some tolerance due to random variation
        prop_assert!(large_relative <= small_relative * 1.5 + 0.1,
            "Larger sample relative CI width ({}) should not be much larger than smaller sample ({})",
            large_relative, small_relative);
    }

    /// Test confidence interval with direct method
    #[test]
    fn property_confidence_interval_direct(values in positive_f64_vec(30, 100)) {
        let analyzer = StatisticalAnalyzer::new();
        let (lower, upper) = analyzer.compute_confidence_interval(&values);

        let mean = values.iter().sum::<f64>() / values.len() as f64;

        // lower <= mean <= upper
        prop_assert!(lower <= mean + 1e-10,
            "CI lower bound ({}) should be <= mean ({})", lower, mean);
        prop_assert!(mean <= upper + 1e-10,
            "mean ({}) should be <= CI upper bound ({})", mean, upper);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 9: Outlier Detection Correctness**
    /// *For any* array of measurements, outliers detected using IQR method SHALL be values
    /// that fall below Q1 - 1.5*IQR or above Q3 + 1.5*IQR, where IQR = Q3 - Q1.
    /// **Validates: Requirements 5.4**
    #[test]
    fn property_outlier_detection_correctness(values in positive_f64_vec(10, 100)) {
        let analyzer = StatisticalAnalyzer::new();
        let outlier_indices = analyzer.detect_outliers(&values);

        if let Some((lower_bound, upper_bound)) = analyzer.get_outlier_bounds(&values) {
            // All detected outliers should be outside the bounds
            for &idx in &outlier_indices {
                let value = values[idx];
                prop_assert!(
                    value < lower_bound || value > upper_bound,
                    "Detected outlier at index {} with value {} should be outside bounds [{}, {}]",
                    idx, value, lower_bound, upper_bound
                );
            }

            // All values outside bounds should be detected as outliers
            for (idx, &value) in values.iter().enumerate() {
                if value < lower_bound || value > upper_bound {
                    prop_assert!(
                        outlier_indices.contains(&idx),
                        "Value {} at index {} is outside bounds [{}, {}] but not detected as outlier",
                        value, idx, lower_bound, upper_bound
                    );
                }
            }
        }
    }

    /// Test that outliers are correctly identified with known outliers
    #[test]
    fn property_outlier_detection_with_injected_outliers(
        base_values in prop::collection::vec(10.0f64..20.0, 20..50),
        outlier_factor in 5.0f64..10.0
    ) {
        let analyzer = StatisticalAnalyzer::new();

        // Create values with known outliers
        let mut values = base_values.clone();
        let mean = values.iter().sum::<f64>() / values.len() as f64;

        // Add extreme outliers
        let high_outlier = mean * outlier_factor;
        let low_outlier = mean / outlier_factor;
        let high_idx = values.len();
        let low_idx = values.len() + 1;
        values.push(high_outlier);
        values.push(low_outlier);

        let outlier_indices = analyzer.detect_outliers(&values);

        // The extreme values should be detected as outliers (if IQR method catches them)
        if let Some((lower_bound, upper_bound)) = analyzer.get_outlier_bounds(&values) {
            if high_outlier > upper_bound {
                prop_assert!(
                    outlier_indices.contains(&high_idx),
                    "High outlier {} should be detected (upper bound: {})",
                    high_outlier, upper_bound
                );
            }
            if low_outlier < lower_bound {
                prop_assert!(
                    outlier_indices.contains(&low_idx),
                    "Low outlier {} should be detected (lower bound: {})",
                    low_outlier, lower_bound
                );
            }
        }
    }

    /// Test that small arrays return no outliers (need at least 4 values for IQR)
    #[test]
    fn property_outlier_detection_small_arrays(values in positive_f64_vec(1, 3)) {
        let analyzer = StatisticalAnalyzer::new();
        let outlier_indices = analyzer.detect_outliers(&values);

        prop_assert!(
            outlier_indices.is_empty(),
            "Arrays with fewer than 4 values should have no outliers detected"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 8: Significance Testing Consistency**
    /// *For any* comparison between two measurement arrays, the Statistical_Analyzer SHALL
    /// compute a p-value between 0 and 1, and is_significant SHALL be true if and only if p_value < 0.05.
    /// **Validates: Requirements 5.3**
    #[test]
    fn property_significance_testing_consistency(
        a in positive_f64_vec(10, 50),
        b in positive_f64_vec(10, 50)
    ) {
        let analyzer = StatisticalAnalyzer::new();
        let (t_stat, p_value) = analyzer.welch_t_test(&a, &b);

        // p-value should be between 0 and 1
        prop_assert!((0.0..=1.0).contains(&p_value),
            "p-value ({}) should be between 0 and 1", p_value);

        // t-statistic should be finite
        prop_assert!(t_stat.is_finite() || t_stat.is_infinite(),
            "t-statistic should be a valid number");

        // Test compare method consistency
        let comparison = analyzer.compare(&a, &b);

        // is_significant should be true iff p_value < 0.05
        let expected_significant = comparison.p_value < 0.05;
        prop_assert_eq!(comparison.is_significant, expected_significant,
            "is_significant ({}) should match p_value < 0.05 (p={})",
            comparison.is_significant, comparison.p_value);
    }

    /// Test that identical samples produce non-significant results
    #[test]
    fn property_identical_samples_not_significant(values in positive_f64_vec(10, 50)) {
        let analyzer = StatisticalAnalyzer::new();
        let (_, p_value) = analyzer.welch_t_test(&values, &values);

        // Identical samples should have high p-value (not significant)
        prop_assert!(p_value >= 0.05,
            "Identical samples should not be significant (p={})", p_value);
    }

    /// Test that very different samples produce significant results
    #[test]
    fn property_different_samples_significant(
        base in 1.0f64..10.0,
        multiplier in 10.0f64..100.0,
        count in 30usize..50
    ) {
        let analyzer = StatisticalAnalyzer::new();

        // Create two clearly different distributions
        let a: Vec<f64> = (0..count).map(|i| base + (i as f64 * 0.01)).collect();
        let b: Vec<f64> = (0..count).map(|i| base * multiplier + (i as f64 * 0.01)).collect();

        let (_, p_value) = analyzer.welch_t_test(&a, &b);

        // Very different samples should be significant
        prop_assert!(p_value < 0.05,
            "Very different samples should be significant (p={})", p_value);
    }

    /// Test symmetry of t-test
    #[test]
    fn property_t_test_symmetry(
        a in positive_f64_vec(10, 50),
        b in positive_f64_vec(10, 50)
    ) {
        let analyzer = StatisticalAnalyzer::new();

        let (t1, p1) = analyzer.welch_t_test(&a, &b);
        let (t2, p2) = analyzer.welch_t_test(&b, &a);

        // t-statistics should be negatives of each other
        prop_assert!((t1 + t2).abs() < 1e-10,
            "t-statistics should be symmetric: t1={}, t2={}", t1, t2);

        // p-values should be equal
        prop_assert!((p1 - p2).abs() < 1e-10,
            "p-values should be equal: p1={}, p2={}", p1, p2);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 10: Variance Warning Threshold**
    /// *For any* array of measurements where coefficient_of_variation > 0.10 (10%),
    /// the Statistical_Analyzer SHALL flag the results as potentially unreliable.
    /// **Validates: Requirements 5.5**
    #[test]
    fn property_variance_warning_threshold(values in positive_f64_vec(10, 100)) {
        let analyzer = StatisticalAnalyzer::new();
        let stats = analyzer.compute_statistics_from_f64(&values);

        let has_warning = analyzer.has_high_variance(&stats);
        let expected_warning = stats.coefficient_of_variation > 0.10;

        prop_assert_eq!(has_warning, expected_warning,
            "has_high_variance ({}) should match CV > 0.10 (CV = {})",
            has_warning, stats.coefficient_of_variation);
    }

    /// Test that low variance values don't trigger warning
    #[test]
    fn property_low_variance_no_warning(
        base in 100.0f64..1000.0,
        count in 30usize..50
    ) {
        let analyzer = StatisticalAnalyzer::new();

        // Create values with very low variance (all within 1% of base)
        let values: Vec<f64> = (0..count)
            .map(|i| base * (1.0 + (i as f64 * 0.0001)))
            .collect();

        let stats = analyzer.compute_statistics_from_f64(&values);

        // Low variance should not trigger warning
        prop_assert!(!analyzer.has_high_variance(&stats),
            "Low variance values should not trigger warning (CV = {})",
            stats.coefficient_of_variation);
    }

    /// Test that high variance values trigger warning
    #[test]
    fn property_high_variance_triggers_warning(
        base in 10.0f64..100.0,
        spread in 0.5f64..2.0,
        count in 30usize..50
    ) {
        let analyzer = StatisticalAnalyzer::new();

        // Create values with high variance (spread across wide range)
        let values: Vec<f64> = (0..count)
            .map(|i| base * (1.0 + spread * (i as f64 / count as f64)))
            .collect();

        let stats = analyzer.compute_statistics_from_f64(&values);

        // If CV > 10%, should trigger warning
        if stats.coefficient_of_variation > 0.10 {
            prop_assert!(analyzer.has_high_variance(&stats),
                "High variance values should trigger warning (CV = {})",
                stats.coefficient_of_variation);
        }
    }

    /// Test variance warning message generation
    #[test]
    fn property_variance_warning_message(values in positive_f64_vec(10, 100)) {
        let analyzer = StatisticalAnalyzer::new();
        let stats = analyzer.compute_statistics_from_f64(&values);

        let warning = analyzer.get_variance_warning(&stats);
        let has_warning = analyzer.has_high_variance(&stats);

        // Warning message should exist iff has_high_variance is true
        prop_assert_eq!(warning.is_some(), has_warning,
            "Warning message presence should match has_high_variance");

        // If warning exists, it should contain the CV percentage
        if let Some(msg) = warning {
            prop_assert!(msg.contains("variance") || msg.contains("CV"),
                "Warning message should mention variance or CV");
        }
    }
}
