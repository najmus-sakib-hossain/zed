//! Event debouncing utilities
//!
//! Provides event coalescing to prevent rapid repeated events from
//! overwhelming the system.
//!
//! Requirement 9.2: Event debouncing for file watchers
//! Requirement 12.6: Debounce rapid events

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Default debounce window (100ms)
pub const DEFAULT_DEBOUNCE_WINDOW: Duration = Duration::from_millis(100);

/// A pending event with its timestamp
#[derive(Debug, Clone)]
struct PendingEvent<T> {
    event: T,
    timestamp: Instant,
}

/// Event debouncer that coalesces rapid events within a time window
///
/// Requirement 9.2: Coalesce events within 100ms window
/// Requirement 12.6: Debounce rapid events
#[derive(Debug)]
pub struct Debouncer<T: Clone> {
    /// Debounce window duration
    window: Duration,
    /// Pending events by key
    pending: Arc<Mutex<HashMap<String, PendingEvent<T>>>>,
}

impl<T: Clone> Debouncer<T> {
    /// Create a new debouncer with the default window (100ms)
    pub fn new() -> Self {
        Self::with_window(DEFAULT_DEBOUNCE_WINDOW)
    }

    /// Create a new debouncer with a custom window
    pub fn with_window(window: Duration) -> Self {
        Self {
            window,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add an event with a key
    ///
    /// Returns `true` if this is a new event (not coalesced with an existing one).
    /// Returns `false` if the event was coalesced with an existing pending event.
    ///
    /// Events with the same key arriving within the debounce window are coalesced,
    /// keeping only the most recent event.
    pub fn add(&self, key: &str, event: T) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let now = Instant::now();

        // Check if there's an existing event within the window
        let should_coalesce = if let Some(existing) = pending.get(key) {
            now.duration_since(existing.timestamp) < self.window
        } else {
            false
        };

        if should_coalesce {
            // Get the original timestamp before removing
            let original_timestamp = pending.get(key).map(|e| e.timestamp).unwrap();
            // Coalesce: update the event but keep the original timestamp
            // This ensures the event will be emitted after the window expires
            pending.insert(
                key.to_string(),
                PendingEvent {
                    event,
                    timestamp: original_timestamp,
                },
            );
            return false; // Coalesced
        }

        // New event or outside window
        pending.insert(
            key.to_string(),
            PendingEvent {
                event,
                timestamp: now,
            },
        );
        true // New event
    }

    /// Check if an event is ready to be emitted (window has expired)
    pub fn is_ready(&self, key: &str) -> bool {
        let pending = self.pending.lock().unwrap();
        if let Some(event) = pending.get(key) {
            Instant::now().duration_since(event.timestamp) >= self.window
        } else {
            false
        }
    }

    /// Get and remove a ready event
    pub fn take_ready(&self, key: &str) -> Option<T> {
        let mut pending = self.pending.lock().unwrap();
        if let Some(event) = pending.get(key)
            && Instant::now().duration_since(event.timestamp) >= self.window
        {
            return pending.remove(key).map(|e| e.event);
        }
        None
    }

    /// Flush all pending events regardless of timing
    ///
    /// Returns all pending events, clearing the internal state.
    pub fn flush(&self) -> Vec<T> {
        let mut pending = self.pending.lock().unwrap();
        let events: Vec<T> = pending.values().map(|e| e.event.clone()).collect();
        pending.clear();
        events
    }

    /// Flush only events that are ready (window has expired)
    pub fn flush_ready(&self) -> Vec<T> {
        let mut pending = self.pending.lock().unwrap();
        let now = Instant::now();

        let ready_keys: Vec<String> = pending
            .iter()
            .filter(|(_, e)| now.duration_since(e.timestamp) >= self.window)
            .map(|(k, _)| k.clone())
            .collect();

        ready_keys
            .into_iter()
            .filter_map(|k| pending.remove(&k).map(|e| e.event))
            .collect()
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// Clear all pending events without emitting them
    pub fn clear(&self) {
        self.pending.lock().unwrap().clear();
    }

    /// Get the debounce window duration
    pub fn window(&self) -> Duration {
        self.window
    }
}

impl<T: Clone> Default for Debouncer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Debouncer<T> {
    fn clone(&self) -> Self {
        Self {
            window: self.window,
            pending: Arc::clone(&self.pending),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;

    // Feature: dx-cli-hardening, Property 30: Event Debouncing
    // **Validates: Requirements 9.2, 12.6**
    //
    // For any sequence of events with the same key arriving within the debounce
    // window (100ms), only the last event SHALL be emitted. Events outside the
    // window SHALL be emitted separately.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_events_within_window_coalesced(
            key in "[a-zA-Z0-9]{1,20}",
            event_count in 2usize..10
        ) {
            let debouncer: Debouncer<usize> = Debouncer::new();

            // Add multiple events with the same key rapidly (within window)
            let mut new_count = 0;
            for i in 0..event_count {
                if debouncer.add(&key, i) {
                    new_count += 1;
                }
            }

            // Only the first event should be "new", rest should be coalesced
            prop_assert_eq!(new_count, 1, "Only first event should be new, got {}", new_count);

            // Should have exactly one pending event
            prop_assert_eq!(debouncer.pending_count(), 1, "Should have exactly one pending event");

            // Flush should return exactly one event (the last one)
            let flushed = debouncer.flush();
            prop_assert_eq!(flushed.len(), 1, "Flush should return exactly one event");
            prop_assert_eq!(flushed[0], event_count - 1, "Should be the last event value");
        }

        #[test]
        fn prop_events_with_different_keys_not_coalesced(
            key_count in 2usize..5
        ) {
            let debouncer: Debouncer<String> = Debouncer::new();

            // Generate unique keys by using index
            let keys: Vec<String> = (0..key_count).map(|i| format!("unique_key_{}", i)).collect();

            // Add events with different keys
            let mut new_count = 0;
            for key in &keys {
                if debouncer.add(key, key.clone()) {
                    new_count += 1;
                }
            }

            // All events should be "new" since they have different keys
            prop_assert_eq!(new_count, keys.len(), "All events with different keys should be new");

            // Should have one pending event per key
            prop_assert_eq!(debouncer.pending_count(), keys.len());
        }

        #[test]
        fn prop_flush_returns_all_pending(
            keys in prop::collection::vec("[a-zA-Z0-9]{1,10}", 1..10)
        ) {
            let debouncer: Debouncer<String> = Debouncer::new();

            // Add events with unique keys
            let unique_keys: Vec<_> = keys.iter().enumerate()
                .map(|(i, k)| format!("{}_{}", k, i))
                .collect();

            for key in &unique_keys {
                debouncer.add(key, key.clone());
            }

            let pending_before = debouncer.pending_count();
            let flushed = debouncer.flush();
            let pending_after = debouncer.pending_count();

            prop_assert_eq!(flushed.len(), pending_before, "Flush should return all pending events");
            prop_assert_eq!(pending_after, 0, "Pending should be empty after flush");
        }

        #[test]
        fn prop_clear_removes_all_pending(
            event_count in 1usize..20
        ) {
            let debouncer: Debouncer<usize> = Debouncer::new();

            // Add events with unique keys
            for i in 0..event_count {
                debouncer.add(&format!("key_{}", i), i);
            }

            prop_assert_eq!(debouncer.pending_count(), event_count);

            debouncer.clear();

            prop_assert_eq!(debouncer.pending_count(), 0, "Clear should remove all pending events");
        }

        #[test]
        fn prop_debounce_window_configurable(
            window_ms in 10u64..500
        ) {
            let window = Duration::from_millis(window_ms);
            let debouncer: Debouncer<u32> = Debouncer::with_window(window);

            prop_assert_eq!(debouncer.window(), window, "Window should match configured value");
        }
    }

    #[test]
    fn test_events_outside_window_not_coalesced() {
        // Use a very short window for testing
        let debouncer: Debouncer<usize> = Debouncer::with_window(Duration::from_millis(10));

        // Add first event
        assert!(debouncer.add("key", 1));

        // Wait for window to expire
        thread::sleep(Duration::from_millis(20));

        // Add second event - should be new since window expired
        assert!(debouncer.add("key", 2));
    }

    #[test]
    fn test_take_ready() {
        let debouncer: Debouncer<String> = Debouncer::with_window(Duration::from_millis(10));

        debouncer.add("key", "value".to_string());

        // Not ready yet
        assert!(debouncer.take_ready("key").is_none());

        // Wait for window
        thread::sleep(Duration::from_millis(20));

        // Now ready
        let taken = debouncer.take_ready("key");
        assert_eq!(taken, Some("value".to_string()));

        // Should be removed
        assert!(debouncer.take_ready("key").is_none());
    }

    #[test]
    fn test_is_ready() {
        let debouncer: Debouncer<i32> = Debouncer::with_window(Duration::from_millis(10));

        debouncer.add("key", 42);

        // Not ready immediately
        assert!(!debouncer.is_ready("key"));

        // Wait for window
        thread::sleep(Duration::from_millis(20));

        // Now ready
        assert!(debouncer.is_ready("key"));
    }

    #[test]
    fn test_flush_ready() {
        let debouncer: Debouncer<String> = Debouncer::with_window(Duration::from_millis(10));

        // Add first event
        debouncer.add("key1", "value1".to_string());

        // Wait for window
        thread::sleep(Duration::from_millis(20));

        // Add second event (not ready yet)
        debouncer.add("key2", "value2".to_string());

        // Flush ready - should only get first event
        let ready = debouncer.flush_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "value1");

        // Second event should still be pending
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_clone_shares_state() {
        let debouncer1: Debouncer<i32> = Debouncer::new();
        let debouncer2 = debouncer1.clone();

        debouncer1.add("key", 42);

        // Both should see the same pending count
        assert_eq!(debouncer1.pending_count(), 1);
        assert_eq!(debouncer2.pending_count(), 1);

        // Flush from one should affect both
        debouncer2.flush();
        assert_eq!(debouncer1.pending_count(), 0);
    }

    #[test]
    fn test_default() {
        let debouncer: Debouncer<i32> = Debouncer::default();
        assert_eq!(debouncer.window(), DEFAULT_DEBOUNCE_WINDOW);
    }
}
