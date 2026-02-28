//! Watch Mode and Debouncing - Feature #6
//!
//! File system watching with debouncing for template regeneration.
//!
//! ## Features
//!
//! - Debounce rapid file changes
//! - Configurable debounce window
//! - Event coalescing

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Debouncer
// ============================================================================

/// A debouncer that coalesces rapid events into single triggers.
///
/// When multiple events occur within the debounce window, only one
/// callback is triggered after the window expires.
///
/// # Example
///
/// ```rust,ignore
/// use dx_generator::watch::Debouncer;
/// use std::time::Duration;
///
/// let mut debouncer = Debouncer::new(Duration::from_millis(500));
///
/// // Rapid events
/// debouncer.event("file.rs");
/// debouncer.event("file.rs");
/// debouncer.event("file.rs");
///
/// // After debounce window, only one trigger
/// let triggers = debouncer.poll();
/// assert_eq!(triggers.len(), 1);
/// ```
#[derive(Debug)]
pub struct Debouncer {
    /// Debounce window duration
    window: Duration,
    /// Pending events (path -> last event time)
    pending: HashMap<PathBuf, Instant>,
    /// Triggered paths ready for processing
    triggered: Vec<PathBuf>,
}

impl Debouncer {
    /// Create a new debouncer with the given window duration.
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            pending: HashMap::new(),
            triggered: Vec::new(),
        }
    }

    /// Create a debouncer with a window in milliseconds.
    #[must_use]
    pub fn with_millis(millis: u64) -> Self {
        Self::new(Duration::from_millis(millis))
    }

    /// Get the debounce window duration.
    #[must_use]
    pub fn window(&self) -> Duration {
        self.window
    }

    /// Record an event for a path.
    ///
    /// If an event for this path is already pending, the timer is reset.
    pub fn event(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.pending.insert(path, Instant::now());
    }

    /// Record multiple events.
    pub fn events(&mut self, paths: impl IntoIterator<Item = impl Into<PathBuf>>) {
        for path in paths {
            self.event(path);
        }
    }

    /// Poll for triggered events.
    ///
    /// Returns paths that have been pending longer than the debounce window.
    /// These paths are removed from the pending set.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let now = Instant::now();
        self.triggered.clear();

        // Find events that have exceeded the debounce window
        let expired: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|&(_, &time)| now.duration_since(time) >= self.window)
            .map(|(path, _)| path.clone())
            .collect();

        // Move expired events to triggered
        for path in expired {
            self.pending.remove(&path);
            self.triggered.push(path);
        }

        self.triggered.clone()
    }

    /// Check if there are any pending events.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get the number of pending events.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending events.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.triggered.clear();
    }

    /// Get the time until the next event would trigger.
    ///
    /// Returns None if there are no pending events.
    #[must_use]
    pub fn time_until_next(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending
            .values()
            .map(|&time| {
                let elapsed = now.duration_since(time);
                self.window.saturating_sub(elapsed)
            })
            .min()
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

// ============================================================================
// Event Counter (for testing)
// ============================================================================

/// Counts events with debouncing for testing purposes.
///
/// This is a simplified model that counts how many triggers would occur
/// given a sequence of events and their timestamps.
#[derive(Debug)]
pub struct EventCounter {
    /// Debounce window in milliseconds
    window_ms: u64,
    /// Events as (path_id, timestamp_ms)
    events: Vec<(u32, u64)>,
}

impl EventCounter {
    /// Create a new event counter.
    #[must_use]
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            events: Vec::new(),
        }
    }

    /// Add an event.
    pub fn add_event(&mut self, path_id: u32, timestamp_ms: u64) {
        self.events.push((path_id, timestamp_ms));
    }

    /// Count how many triggers would occur.
    ///
    /// Events for the same path within the debounce window are coalesced.
    #[must_use]
    pub fn count_triggers(&self) -> usize {
        if self.events.is_empty() {
            return 0;
        }

        // Group events by path
        let mut by_path: HashMap<u32, Vec<u64>> = HashMap::new();
        for &(path_id, timestamp) in &self.events {
            by_path.entry(path_id).or_default().push(timestamp);
        }

        let mut total_triggers = 0;

        // For each path, count triggers
        for timestamps in by_path.values() {
            let mut sorted = timestamps.clone();
            sorted.sort();

            if sorted.is_empty() {
                continue;
            }

            // Count triggers: each gap > window_ms creates a new trigger
            let mut triggers = 1;
            let mut last_trigger_time = sorted[0];

            for &ts in &sorted[1..] {
                if ts - last_trigger_time >= self.window_ms {
                    triggers += 1;
                    last_trigger_time = ts;
                }
            }

            total_triggers += triggers;
        }

        total_triggers
    }

    /// Count events within the debounce window.
    ///
    /// Returns the number of events that would be coalesced into a single trigger.
    #[must_use]
    pub fn count_coalesced(&self) -> usize {
        self.events.len().saturating_sub(self.count_triggers())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debouncer_new() {
        let debouncer = Debouncer::new(Duration::from_millis(100));
        assert_eq!(debouncer.window(), Duration::from_millis(100));
        assert!(!debouncer.has_pending());
    }

    #[test]
    fn test_debouncer_event() {
        let mut debouncer = Debouncer::new(Duration::from_millis(100));
        debouncer.event("test.rs");
        assert!(debouncer.has_pending());
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_debouncer_multiple_events_same_path() {
        let mut debouncer = Debouncer::new(Duration::from_millis(100));
        debouncer.event("test.rs");
        debouncer.event("test.rs");
        debouncer.event("test.rs");

        // Same path should only have one pending event
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_debouncer_multiple_paths() {
        let mut debouncer = Debouncer::new(Duration::from_millis(100));
        debouncer.event("a.rs");
        debouncer.event("b.rs");
        debouncer.event("c.rs");

        assert_eq!(debouncer.pending_count(), 3);
    }

    #[test]
    fn test_debouncer_clear() {
        let mut debouncer = Debouncer::new(Duration::from_millis(100));
        debouncer.event("test.rs");
        debouncer.clear();
        assert!(!debouncer.has_pending());
    }

    #[test]
    fn test_event_counter_single_event() {
        let mut counter = EventCounter::new(100);
        counter.add_event(1, 0);
        assert_eq!(counter.count_triggers(), 1);
    }

    #[test]
    fn test_event_counter_coalesced_events() {
        let mut counter = EventCounter::new(100);
        // Events within 100ms window
        counter.add_event(1, 0);
        counter.add_event(1, 50);
        counter.add_event(1, 80);

        // Should coalesce to 1 trigger
        assert_eq!(counter.count_triggers(), 1);
    }

    #[test]
    fn test_event_counter_separate_triggers() {
        let mut counter = EventCounter::new(100);
        // Events with gaps > 100ms
        counter.add_event(1, 0);
        counter.add_event(1, 200);
        counter.add_event(1, 400);

        // Should be 3 separate triggers
        assert_eq!(counter.count_triggers(), 3);
    }

    #[test]
    fn test_event_counter_multiple_paths() {
        let mut counter = EventCounter::new(100);
        counter.add_event(1, 0);
        counter.add_event(2, 0);
        counter.add_event(1, 50);
        counter.add_event(2, 50);

        // 2 paths, each with coalesced events = 2 triggers
        assert_eq!(counter.count_triggers(), 2);
    }
}

// ============================================================================
// Property-Based Tests for Debouncing
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Feature: dx-generator-production
    // Property 10: Watch Mode Debounce Behavior
    // Validates: Requirements 6.5
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10.1: N events within debounce window trigger exactly 1 regeneration
        /// For any sequence of N file changes within the debounce window,
        /// the Generator SHALL trigger exactly 1 regeneration (not N).
        #[test]
        fn prop_events_within_window_single_trigger(
            event_count in 1usize..20usize,
            window_ms in 100u64..1000u64
        ) {
            let mut counter = EventCounter::new(window_ms);

            // All events within the window (timestamps 0 to window_ms - 1)
            for i in 0..event_count {
                let timestamp = (i as u64 * (window_ms - 1)) / event_count.max(1) as u64;
                counter.add_event(1, timestamp);
            }

            // Should trigger exactly 1 regeneration
            prop_assert_eq!(
                counter.count_triggers(),
                1,
                "{} events within {}ms window should trigger exactly 1 regeneration",
                event_count,
                window_ms
            );
        }

        /// Property 10.2: Events outside window trigger separate regenerations
        /// For any sequence of events with gaps larger than the debounce window,
        /// each event SHALL trigger a separate regeneration.
        #[test]
        fn prop_events_outside_window_separate_triggers(
            event_count in 1usize..10usize,
            window_ms in 100u64..500u64,
            gap_multiplier in 2u64..5u64
        ) {
            let mut counter = EventCounter::new(window_ms);

            // Events with gaps > window_ms
            for i in 0..event_count {
                let timestamp = i as u64 * window_ms * gap_multiplier;
                counter.add_event(1, timestamp);
            }

            // Should trigger event_count regenerations
            prop_assert_eq!(
                counter.count_triggers(),
                event_count,
                "{} events with {}ms gaps should trigger {} regenerations",
                event_count,
                window_ms * gap_multiplier,
                event_count
            );
        }

        /// Property 10.3: Different paths are independent
        /// Events for different paths SHALL be debounced independently.
        #[test]
        fn prop_different_paths_independent(
            path_count in 1usize..5usize,
            events_per_path in 1usize..10usize,
            window_ms in 100u64..500u64
        ) {
            let mut counter = EventCounter::new(window_ms);

            // Multiple events per path, all within window
            for path_id in 0..path_count {
                for i in 0..events_per_path {
                    let timestamp = (i as u64 * (window_ms - 1)) / events_per_path.max(1) as u64;
                    counter.add_event(path_id as u32, timestamp);
                }
            }

            // Should trigger path_count regenerations (one per path)
            prop_assert_eq!(
                counter.count_triggers(),
                path_count,
                "{} paths with {} events each should trigger {} regenerations",
                path_count,
                events_per_path,
                path_count
            );
        }

        /// Property 10.4: Coalesced count is correct
        /// The number of coalesced events should equal total events minus triggers.
        #[test]
        fn prop_coalesced_count_correct(
            events in proptest::collection::vec((0u32..5u32, 0u64..1000u64), 1..50),
            window_ms in 100u64..500u64
        ) {
            let mut counter = EventCounter::new(window_ms);

            for (path_id, timestamp) in &events {
                counter.add_event(*path_id, *timestamp);
            }

            let triggers = counter.count_triggers();
            let coalesced = counter.count_coalesced();

            prop_assert_eq!(
                triggers + coalesced,
                events.len(),
                "triggers ({}) + coalesced ({}) should equal total events ({})",
                triggers,
                coalesced,
                events.len()
            );
        }

        /// Property 10.5: Empty event list triggers zero regenerations
        /// An empty event list SHALL trigger zero regenerations.
        #[test]
        fn prop_empty_events_zero_triggers(
            window_ms in 100u64..1000u64
        ) {
            let counter = EventCounter::new(window_ms);
            prop_assert_eq!(counter.count_triggers(), 0);
        }

        /// Property 10.6: Debouncer pending count is bounded
        /// The pending count SHALL never exceed the number of unique paths.
        #[test]
        fn prop_pending_count_bounded(
            paths in proptest::collection::vec("[a-z]{1,5}\\.rs", 1..10),
            events_per_path in 1usize..5usize
        ) {
            let mut debouncer = Debouncer::new(Duration::from_millis(100));

            for path in &paths {
                for _ in 0..events_per_path {
                    debouncer.event(path);
                }
            }

            // Pending count should equal unique paths
            let unique_paths: std::collections::HashSet<_> = paths.iter().collect();
            prop_assert_eq!(
                debouncer.pending_count(),
                unique_paths.len(),
                "Pending count should equal unique path count"
            );
        }
    }
}
