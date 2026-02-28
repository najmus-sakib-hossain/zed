//! Metrics Tracking - Feature #10
//!
//! Generation metrics for tracking efficiency, performance, and token savings.
//!
//! ## Features
//!
//! - Track total generations, bytes, and time
//! - Cache hit/miss statistics
//! - XOR patching savings
//! - Token savings estimation
//! - Per-template statistics
//! - Persistence to `.dx/stats.json`

#[cfg(feature = "serde-compat")]
use crate::error::GeneratorError;
use crate::error::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ============================================================================
// Generation Metrics
// ============================================================================

/// Generation metrics for tracking efficiency.
///
/// Tracks all generation operations including bytes generated, time spent,
/// cache performance, and estimated token savings.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct GenerationMetrics {
    /// Total generations performed
    pub total_generations: u64,
    /// Total bytes generated
    pub total_bytes: u64,
    /// Total time spent (microseconds)
    pub total_time_us: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Bytes saved via XOR patching
    pub bytes_saved_patching: u64,
    /// Estimated tokens saved (vs manual writing)
    pub estimated_tokens_saved: u64,
    /// Per-template statistics
    pub per_template: HashMap<String, TemplateStats>,
}

impl GenerationMetrics {
    /// Create new empty metrics.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a generation operation.
    pub fn record_generation(
        &mut self,
        template_id: &str,
        bytes: usize,
        time_us: u64,
        template_bytes: usize,
    ) {
        self.total_generations += 1;
        self.total_bytes += bytes as u64;
        self.total_time_us += time_us;

        // Estimate tokens saved
        let tokens = Self::estimate_tokens_saved(bytes, template_bytes);
        self.estimated_tokens_saved += tokens;

        // Update per-template stats
        let stats = self.per_template.entry(template_id.to_string()).or_default();
        stats.record(bytes, time_us);
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record a cache miss.
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Record bytes saved via XOR patching.
    pub fn record_patching_savings(&mut self, bytes_saved: usize) {
        self.bytes_saved_patching += bytes_saved as u64;
    }

    /// Estimate tokens saved based on output size.
    ///
    /// Assumes ~4 characters per token average.
    #[must_use]
    pub fn estimate_tokens_saved(output_bytes: usize, template_bytes: usize) -> u64 {
        let expansion = output_bytes.saturating_sub(template_bytes);
        (expansion / 4) as u64
    }

    /// Get cache hit rate (0.0 to 1.0).
    #[must_use]
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Get average generation time in microseconds.
    #[must_use]
    pub fn avg_time_us(&self) -> u64 {
        if self.total_generations == 0 {
            0
        } else {
            self.total_time_us / self.total_generations
        }
    }

    /// Get average bytes per generation.
    #[must_use]
    pub fn avg_bytes(&self) -> u64 {
        if self.total_generations == 0 {
            0
        } else {
            self.total_bytes / self.total_generations
        }
    }

    /// Get total patching efficiency (bytes saved / total bytes).
    #[must_use]
    pub fn patching_efficiency(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.bytes_saved_patching as f64 / self.total_bytes as f64
        }
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Merge another metrics instance into this one.
    pub fn merge(&mut self, other: &GenerationMetrics) {
        self.total_generations += other.total_generations;
        self.total_bytes += other.total_bytes;
        self.total_time_us += other.total_time_us;
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;
        self.bytes_saved_patching += other.bytes_saved_patching;
        self.estimated_tokens_saved += other.estimated_tokens_saved;

        for (id, stats) in &other.per_template {
            let entry = self.per_template.entry(id.clone()).or_default();
            entry.merge(stats);
        }
    }

    /// Get statistics for a specific template.
    #[must_use]
    pub fn get_template_stats(&self, template_id: &str) -> Option<&TemplateStats> {
        self.per_template.get(template_id)
    }

    /// Get all template IDs with statistics.
    #[must_use]
    pub fn template_ids(&self) -> Vec<&str> {
        self.per_template.keys().map(String::as_str).collect()
    }
}

// ============================================================================
// Template Statistics
// ============================================================================

/// Per-template statistics.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde-compat", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateStats {
    /// Number of uses
    pub uses: u64,
    /// Total bytes generated
    pub total_bytes: u64,
    /// Total time spent (microseconds)
    pub total_time_us: u64,
}

impl TemplateStats {
    /// Create new empty stats.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a generation.
    pub fn record(&mut self, bytes: usize, time_us: u64) {
        self.uses += 1;
        self.total_bytes += bytes as u64;
        self.total_time_us += time_us;
    }

    /// Get average time per use in microseconds.
    #[must_use]
    pub fn avg_time_us(&self) -> u64 {
        if self.uses == 0 {
            0
        } else {
            self.total_time_us / self.uses
        }
    }

    /// Get average bytes per use.
    #[must_use]
    pub fn avg_bytes(&self) -> u64 {
        if self.uses == 0 {
            0
        } else {
            self.total_bytes / self.uses
        }
    }

    /// Merge another stats instance into this one.
    pub fn merge(&mut self, other: &TemplateStats) {
        self.uses += other.uses;
        self.total_bytes += other.total_bytes;
        self.total_time_us += other.total_time_us;
    }
}

// ============================================================================
// Metrics Tracker
// ============================================================================

/// Metrics tracker with persistence support.
///
/// Provides a convenient interface for recording metrics and persisting
/// them to disk.
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::MetricsTracker;
///
/// let mut tracker = MetricsTracker::new(".dx/stats.json");
/// tracker.load()?;
///
/// // Record a generation
/// let timer = tracker.start_timer();
/// // ... generate ...
/// tracker.record_generation("component", 1024, timer, 256);
///
/// tracker.save()?;
/// ```
#[derive(Debug)]
pub struct MetricsTracker {
    /// Current metrics
    metrics: GenerationMetrics,
    /// Path to persistence file
    path: PathBuf,
    /// Whether metrics have been modified
    dirty: bool,
}

impl MetricsTracker {
    /// Create a new tracker with the given persistence path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            metrics: GenerationMetrics::new(),
            path: path.into(),
            dirty: false,
        }
    }

    /// Get the persistence path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the current metrics.
    #[must_use]
    pub fn metrics(&self) -> &GenerationMetrics {
        &self.metrics
    }

    /// Get mutable access to metrics.
    pub fn metrics_mut(&mut self) -> &mut GenerationMetrics {
        self.dirty = true;
        &mut self.metrics
    }

    /// Check if metrics have been modified.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Start a timer for measuring generation time.
    #[must_use]
    pub fn start_timer(&self) -> Instant {
        Instant::now()
    }

    /// Record a generation with a timer.
    pub fn record_generation(
        &mut self,
        template_id: &str,
        bytes: usize,
        timer: Instant,
        template_bytes: usize,
    ) {
        let time_us = timer.elapsed().as_micros() as u64;
        self.metrics.record_generation(template_id, bytes, time_us, template_bytes);
        self.dirty = true;
    }

    /// Record a generation with explicit time.
    pub fn record_generation_with_time(
        &mut self,
        template_id: &str,
        bytes: usize,
        time_us: u64,
        template_bytes: usize,
    ) {
        self.metrics.record_generation(template_id, bytes, time_us, template_bytes);
        self.dirty = true;
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&mut self) {
        self.metrics.record_cache_hit();
        self.dirty = true;
    }

    /// Record a cache miss.
    pub fn record_cache_miss(&mut self) {
        self.metrics.record_cache_miss();
        self.dirty = true;
    }

    /// Record patching savings.
    pub fn record_patching_savings(&mut self, bytes_saved: usize) {
        self.metrics.record_patching_savings(bytes_saved);
        self.dirty = true;
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        self.metrics.reset();
        self.dirty = true;
    }

    /// Load metrics from disk.
    #[cfg(feature = "serde-compat")]
    pub fn load(&mut self) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.path).map_err(|e| GeneratorError::Io(e))?;

        self.metrics =
            serde_json::from_str(&content).map_err(|e| GeneratorError::InvalidTemplate {
                reason: format!("Failed to parse metrics: {}", e),
            })?;

        self.dirty = false;
        Ok(())
    }

    /// Load metrics from disk (no-op without serde feature).
    #[cfg(not(feature = "serde-compat"))]
    pub fn load(&mut self) -> Result<()> {
        Ok(())
    }

    /// Save metrics to disk.
    #[cfg(feature = "serde-compat")]
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GeneratorError::Io(e))?;
        }

        let content = serde_json::to_string_pretty(&self.metrics).map_err(|e| {
            GeneratorError::InvalidTemplate {
                reason: format!("Failed to serialize metrics: {}", e),
            }
        })?;

        std::fs::write(&self.path, content).map_err(|e| GeneratorError::Io(e))?;

        self.dirty = false;
        Ok(())
    }

    /// Save metrics to disk (no-op without serde feature).
    #[cfg(not(feature = "serde-compat"))]
    pub fn save(&mut self) -> Result<()> {
        self.dirty = false;
        Ok(())
    }
}

impl Default for MetricsTracker {
    fn default() -> Self {
        Self::new(".dx/stats.json")
    }
}

// ============================================================================
// Generation Timer
// ============================================================================

/// A scoped timer for measuring generation time.
///
/// Automatically records the elapsed time when dropped.
pub struct GenerationTimer<'a> {
    tracker: &'a mut MetricsTracker,
    template_id: String,
    bytes: usize,
    template_bytes: usize,
    start: Instant,
}

impl<'a> GenerationTimer<'a> {
    /// Create a new generation timer.
    pub fn new(
        tracker: &'a mut MetricsTracker,
        template_id: impl Into<String>,
        template_bytes: usize,
    ) -> Self {
        Self {
            tracker,
            template_id: template_id.into(),
            bytes: 0,
            template_bytes,
            start: Instant::now(),
        }
    }

    /// Set the output bytes.
    pub fn set_bytes(&mut self, bytes: usize) {
        self.bytes = bytes;
    }

    /// Get elapsed time.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Finish and record the generation.
    pub fn finish(self) {
        // Drop will handle recording
    }
}

impl Drop for GenerationTimer<'_> {
    fn drop(&mut self) {
        let time_us = self.start.elapsed().as_micros() as u64;
        self.tracker.metrics.record_generation(
            &self.template_id,
            self.bytes,
            time_us,
            self.template_bytes,
        );
        self.tracker.dirty = true;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_metrics_new() {
        let metrics = GenerationMetrics::new();

        assert_eq!(metrics.total_generations, 0);
        assert_eq!(metrics.total_bytes, 0);
        assert_eq!(metrics.total_time_us, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
        assert!(metrics.per_template.is_empty());
    }

    #[test]
    fn test_record_generation() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_generation("component", 1024, 100, 256);

        assert_eq!(metrics.total_generations, 1);
        assert_eq!(metrics.total_bytes, 1024);
        assert_eq!(metrics.total_time_us, 100);

        let stats = metrics.get_template_stats("component").unwrap();
        assert_eq!(stats.uses, 1);
        assert_eq!(stats.total_bytes, 1024);
    }

    #[test]
    fn test_cache_statistics() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        assert_eq!(metrics.cache_hits, 2);
        assert_eq!(metrics.cache_misses, 1);
        assert!((metrics.cache_hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_estimate_tokens_saved() {
        // 1000 bytes output, 200 bytes template = 800 expansion = 200 tokens
        let tokens = GenerationMetrics::estimate_tokens_saved(1000, 200);
        assert_eq!(tokens, 200);

        // No expansion
        let tokens = GenerationMetrics::estimate_tokens_saved(100, 100);
        assert_eq!(tokens, 0);

        // Template larger than output (shouldn't happen but handle gracefully)
        let tokens = GenerationMetrics::estimate_tokens_saved(100, 200);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_averages() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_generation("a", 100, 10, 50);
        metrics.record_generation("b", 200, 20, 100);

        assert_eq!(metrics.avg_bytes(), 150);
        assert_eq!(metrics.avg_time_us(), 15);
    }

    #[test]
    fn test_patching_efficiency() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_generation("a", 1000, 100, 500);
        metrics.record_patching_savings(500);

        assert!((metrics.patching_efficiency() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_generation("a", 100, 10, 50);
        metrics.record_cache_hit();

        metrics.reset();

        assert_eq!(metrics.total_generations, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert!(metrics.per_template.is_empty());
    }

    #[test]
    fn test_merge() {
        let mut metrics1 = GenerationMetrics::new();
        metrics1.record_generation("a", 100, 10, 50);

        let mut metrics2 = GenerationMetrics::new();
        metrics2.record_generation("b", 200, 20, 100);

        metrics1.merge(&metrics2);

        assert_eq!(metrics1.total_generations, 2);
        assert_eq!(metrics1.total_bytes, 300);
        assert_eq!(metrics1.per_template.len(), 2);
    }

    #[test]
    fn test_template_stats() {
        let mut stats = TemplateStats::new();

        stats.record(100, 10);
        stats.record(200, 20);

        assert_eq!(stats.uses, 2);
        assert_eq!(stats.total_bytes, 300);
        assert_eq!(stats.total_time_us, 30);
        assert_eq!(stats.avg_bytes(), 150);
        assert_eq!(stats.avg_time_us(), 15);
    }

    #[test]
    fn test_template_stats_merge() {
        let mut stats1 = TemplateStats::new();
        stats1.record(100, 10);

        let mut stats2 = TemplateStats::new();
        stats2.record(200, 20);

        stats1.merge(&stats2);

        assert_eq!(stats1.uses, 2);
        assert_eq!(stats1.total_bytes, 300);
    }

    #[test]
    fn test_metrics_tracker_new() {
        let tracker = MetricsTracker::new(".dx/stats.json");

        assert_eq!(tracker.path(), Path::new(".dx/stats.json"));
        assert!(!tracker.is_dirty());
    }

    #[test]
    fn test_metrics_tracker_record() {
        let mut tracker = MetricsTracker::new(".dx/stats.json");

        tracker.record_generation_with_time("component", 1024, 100, 256);

        assert!(tracker.is_dirty());
        assert_eq!(tracker.metrics().total_generations, 1);
    }

    #[test]
    fn test_metrics_tracker_cache() {
        let mut tracker = MetricsTracker::new(".dx/stats.json");

        tracker.record_cache_hit();
        tracker.record_cache_miss();

        assert_eq!(tracker.metrics().cache_hits, 1);
        assert_eq!(tracker.metrics().cache_misses, 1);
    }

    #[test]
    fn test_metrics_tracker_reset() {
        let mut tracker = MetricsTracker::new(".dx/stats.json");

        tracker.record_generation_with_time("a", 100, 10, 50);
        tracker.reset();

        assert_eq!(tracker.metrics().total_generations, 0);
        assert!(tracker.is_dirty()); // Reset marks as dirty
    }

    #[test]
    fn test_template_ids() {
        let mut metrics = GenerationMetrics::new();

        metrics.record_generation("a", 100, 10, 50);
        metrics.record_generation("b", 200, 20, 100);
        metrics.record_generation("c", 300, 30, 150);

        let ids = metrics.template_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_zero_division_safety() {
        let metrics = GenerationMetrics::new();

        // These should not panic
        assert_eq!(metrics.cache_hit_rate(), 0.0);
        assert_eq!(metrics.avg_time_us(), 0);
        assert_eq!(metrics.avg_bytes(), 0);
        assert_eq!(metrics.patching_efficiency(), 0.0);

        let stats = TemplateStats::new();
        assert_eq!(stats.avg_time_us(), 0);
        assert_eq!(stats.avg_bytes(), 0);
    }
}

// ============================================================================
// Property-Based Tests for Metrics
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Feature: dx-generator-production
    // Property 6: Metrics Accuracy
    // Validates: Requirements 10.1, 10.2, 10.5
    // ========================================================================

    /// Strategy for generating template IDs
    fn template_id_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6.1: bytes_generated equals actual output size
        /// The recorded bytes must exactly match the input bytes.
        #[test]
        fn prop_bytes_equals_actual(
            template_id in template_id_strategy(),
            bytes in 1usize..1_000_000usize,
            time_us in 1u64..1_000_000u64,
            template_bytes in 0usize..1000usize
        ) {
            let mut metrics = GenerationMetrics::new();
            metrics.record_generation(&template_id, bytes, time_us, template_bytes);

            // Property: total_bytes equals input bytes
            prop_assert_eq!(metrics.total_bytes, bytes as u64);

            // Property: per-template bytes equals input bytes
            let stats = metrics.get_template_stats(&template_id).unwrap();
            prop_assert_eq!(stats.total_bytes, bytes as u64);
        }

        /// Property 6.2: time_us is recorded accurately
        /// The recorded time must exactly match the input time.
        #[test]
        fn prop_time_recorded_accurately(
            template_id in template_id_strategy(),
            bytes in 1usize..10000usize,
            time_us in 1u64..1_000_000u64,
            template_bytes in 0usize..1000usize
        ) {
            let mut metrics = GenerationMetrics::new();
            metrics.record_generation(&template_id, bytes, time_us, template_bytes);

            // Property: total_time_us equals input time
            prop_assert_eq!(metrics.total_time_us, time_us);

            // Property: per-template time equals input time
            let stats = metrics.get_template_stats(&template_id).unwrap();
            prop_assert_eq!(stats.total_time_us, time_us);
        }

        /// Property 6.3: tokens_saved equals (output_bytes - template_bytes) / 4
        /// Token estimation must follow the formula exactly.
        #[test]
        fn prop_tokens_saved_formula(
            output_bytes in 0usize..1_000_000usize,
            template_bytes in 0usize..1_000_000usize
        ) {
            let tokens = GenerationMetrics::estimate_tokens_saved(output_bytes, template_bytes);

            let expected = output_bytes.saturating_sub(template_bytes) / 4;

            // Property: tokens equals expected formula
            prop_assert_eq!(tokens, expected as u64);
        }

        /// Property 6.4: Generation count is accurate
        /// Each record_generation call increments the count by exactly 1.
        #[test]
        fn prop_generation_count_accurate(
            count in 1usize..100usize
        ) {
            let mut metrics = GenerationMetrics::new();

            for i in 0..count {
                metrics.record_generation(&format!("t{}", i), 100, 10, 50);
            }

            // Property: total_generations equals number of calls
            prop_assert_eq!(metrics.total_generations, count as u64);
        }

        /// Property 6.5: Cache hit rate is accurate
        /// Hit rate must equal hits / (hits + misses).
        #[test]
        fn prop_cache_hit_rate_accurate(
            hits in 0u64..1000u64,
            misses in 0u64..1000u64
        ) {
            let mut metrics = GenerationMetrics::new();

            for _ in 0..hits {
                metrics.record_cache_hit();
            }
            for _ in 0..misses {
                metrics.record_cache_miss();
            }

            let total = hits + misses;
            let expected_rate = if total == 0 {
                0.0
            } else {
                hits as f64 / total as f64
            };

            // Property: cache_hit_rate equals expected
            prop_assert!((metrics.cache_hit_rate() - expected_rate).abs() < 0.0001);
        }

        /// Property 6.6: Averages are accurate
        /// Average bytes and time must equal total / count.
        #[test]
        fn prop_averages_accurate(
            count in 1usize..20usize,
            bytes_base in 1usize..10000usize,
            time_base in 1u64..10000u64
        ) {
            let mut metrics = GenerationMetrics::new();

            let mut total_bytes: u64 = 0;
            let mut total_time: u64 = 0;

            for i in 0..count {
                let bytes = bytes_base + i * 100;
                let time = time_base + i as u64 * 10;
                metrics.record_generation(&format!("t{}", i), bytes, time, 0);
                total_bytes += bytes as u64;
                total_time += time;
            }

            // Property: avg_bytes equals total_bytes / count
            prop_assert_eq!(metrics.avg_bytes(), total_bytes / count as u64);

            // Property: avg_time_us equals total_time / count
            prop_assert_eq!(metrics.avg_time_us(), total_time / count as u64);
        }

        /// Property 6.7: Patching savings are cumulative
        /// Multiple patching savings calls should sum correctly.
        #[test]
        fn prop_patching_savings_cumulative(
            savings in proptest::collection::vec(1usize..10000usize, 1..20)
        ) {
            let mut metrics = GenerationMetrics::new();

            for s in &savings {
                metrics.record_patching_savings(*s);
            }

            let total: usize = savings.iter().sum();

            // Property: bytes_saved_patching equals sum of all savings
            prop_assert_eq!(metrics.bytes_saved_patching, total as u64);
        }

        /// Property 6.8: Merge is additive
        /// Merging two metrics should sum all values.
        #[test]
        fn prop_merge_additive(
            gen1 in 1u64..1000u64,
            gen2 in 1u64..1000u64,
            bytes1 in 1u64..100000u64,
            bytes2 in 1u64..100000u64
        ) {
            let mut metrics1 = GenerationMetrics::new();
            metrics1.total_generations = gen1;
            metrics1.total_bytes = bytes1;

            let mut metrics2 = GenerationMetrics::new();
            metrics2.total_generations = gen2;
            metrics2.total_bytes = bytes2;

            metrics1.merge(&metrics2);

            // Property: merged values are sums
            prop_assert_eq!(metrics1.total_generations, gen1 + gen2);
            prop_assert_eq!(metrics1.total_bytes, bytes1 + bytes2);
        }

        /// Property 6.9: Reset clears all values
        /// After reset, all metrics should be zero.
        #[test]
        fn prop_reset_clears_all(
            template_id in template_id_strategy(),
            bytes in 1usize..10000usize
        ) {
            let mut metrics = GenerationMetrics::new();

            metrics.record_generation(&template_id, bytes, 100, 50);
            metrics.record_cache_hit();
            metrics.record_patching_savings(100);

            metrics.reset();

            // Property: all values are zero after reset
            prop_assert_eq!(metrics.total_generations, 0);
            prop_assert_eq!(metrics.total_bytes, 0);
            prop_assert_eq!(metrics.total_time_us, 0);
            prop_assert_eq!(metrics.cache_hits, 0);
            prop_assert_eq!(metrics.cache_misses, 0);
            prop_assert_eq!(metrics.bytes_saved_patching, 0);
            prop_assert_eq!(metrics.estimated_tokens_saved, 0);
            prop_assert!(metrics.per_template.is_empty());
        }

        /// Property 6.10: Per-template stats are isolated
        /// Recording for one template should not affect another.
        #[test]
        fn prop_per_template_isolated(
            id1 in template_id_strategy(),
            id2 in template_id_strategy(),
            bytes1 in 1usize..10000usize,
            bytes2 in 1usize..10000usize
        ) {
            prop_assume!(id1 != id2);

            let mut metrics = GenerationMetrics::new();

            metrics.record_generation(&id1, bytes1, 100, 50);
            metrics.record_generation(&id2, bytes2, 200, 100);

            let stats1 = metrics.get_template_stats(&id1).unwrap();
            let stats2 = metrics.get_template_stats(&id2).unwrap();

            // Property: each template has its own stats
            prop_assert_eq!(stats1.total_bytes, bytes1 as u64);
            prop_assert_eq!(stats2.total_bytes, bytes2 as u64);
            prop_assert_eq!(stats1.uses, 1);
            prop_assert_eq!(stats2.uses, 1);
        }
    }
}
