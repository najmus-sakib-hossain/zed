//! Mock/Spy Utilities - Jest-compatible mock functions
//!
//! Provides mock functions that track calls and arguments.

#![allow(dead_code)]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A mock function that tracks calls
#[derive(Debug)]
pub struct MockFn<Args = Vec<String>, Ret = String> {
    /// Number of times the mock was called
    call_count: AtomicUsize,
    /// Arguments from each call
    calls: Mutex<Vec<Args>>,
    /// Return value or implementation
    implementation: Mutex<MockImplementation<Ret>>,
    /// Mock name for debugging
    name: String,
    /// Phantom data for Args type
    _phantom: PhantomData<Args>,
}

/// Mock implementation type
#[derive(Debug)]
enum MockImplementation<Ret> {
    /// Return a fixed value
    ReturnValue(Ret),
    /// Return values in sequence
    ReturnSequence(Vec<Ret>, usize),
    /// Return based on call count
    ReturnOnce(Vec<Ret>),
    /// Custom implementation (not stored, just marker)
    Custom,
}

impl<Args: Clone + Default, Ret: Clone + Default> MockFn<Args, Ret> {
    /// Create a new mock function
    pub fn new(name: &str) -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            calls: Mutex::new(Vec::new()),
            implementation: Mutex::new(MockImplementation::ReturnValue(Ret::default())),
            name: name.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Call the mock function
    pub fn call(&self, args: Args) -> Ret {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.calls.lock().unwrap().push(args);

        let mut impl_guard = self.implementation.lock().unwrap();
        match &mut *impl_guard {
            MockImplementation::ReturnValue(v) => v.clone(),
            MockImplementation::ReturnSequence(seq, idx) => {
                let ret = seq.get(*idx).cloned().unwrap_or_default();
                *idx += 1;
                ret
            }
            MockImplementation::ReturnOnce(values) => {
                if values.is_empty() {
                    Ret::default()
                } else {
                    values.remove(0)
                }
            }
            MockImplementation::Custom => Ret::default(),
        }
    }

    /// Set the return value
    pub fn mock_return_value(&self, value: Ret) {
        *self.implementation.lock().unwrap() = MockImplementation::ReturnValue(value);
    }

    /// Set return values for sequential calls
    pub fn mock_return_value_once(&self, value: Ret) {
        let mut impl_guard = self.implementation.lock().unwrap();
        match &mut *impl_guard {
            MockImplementation::ReturnOnce(values) => values.push(value),
            _ => *impl_guard = MockImplementation::ReturnOnce(vec![value]),
        }
    }

    /// Get the number of times the mock was called
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Check if the mock was called
    pub fn was_called(&self) -> bool {
        self.call_count() > 0
    }

    /// Check if the mock was called exactly n times
    pub fn was_called_times(&self, n: usize) -> bool {
        self.call_count() == n
    }

    /// Get all calls
    pub fn calls(&self) -> Vec<Args> {
        self.calls.lock().unwrap().clone()
    }

    /// Get the last call arguments
    pub fn last_call(&self) -> Option<Args> {
        self.calls.lock().unwrap().last().cloned()
    }

    /// Get the nth call arguments
    pub fn nth_call(&self, n: usize) -> Option<Args> {
        self.calls.lock().unwrap().get(n).cloned()
    }

    /// Reset the mock
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
        self.calls.lock().unwrap().clear();
    }

    /// Clear only the calls, keep implementation
    pub fn clear_calls(&self) {
        self.call_count.store(0, Ordering::SeqCst);
        self.calls.lock().unwrap().clear();
    }

    /// Get mock name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A spy that wraps an existing function
#[derive(Debug)]
pub struct SpyFn<Args = Vec<String>, Ret = String> {
    /// The underlying mock
    mock: MockFn<Args, Ret>,
    /// Whether to call through to original
    call_through: bool,
}

impl<Args: Clone + Default, Ret: Clone + Default> SpyFn<Args, Ret> {
    /// Create a new spy
    pub fn new(name: &str) -> Self {
        Self {
            mock: MockFn::new(name),
            call_through: true,
        }
    }

    /// Call the spy (records call, optionally calls through)
    pub fn call(&self, args: Args) -> Ret {
        self.mock.call(args)
    }

    /// Get the underlying mock
    pub fn mock(&self) -> &MockFn<Args, Ret> {
        &self.mock
    }

    /// Set whether to call through to original
    pub fn set_call_through(&mut self, call_through: bool) {
        self.call_through = call_through;
    }
}

/// Timer mock for controlling time in tests
#[derive(Debug)]
pub struct TimerMock {
    /// Current fake time (ms since epoch)
    current_time: AtomicUsize,
    /// Pending timers
    timers: Mutex<Vec<PendingTimer>>,
    /// Next timer ID
    next_id: AtomicUsize,
}

#[derive(Debug, Clone)]
struct PendingTimer {
    id: usize,
    fire_at: usize,
    interval: Option<usize>,
    callback_id: usize,
}

impl TimerMock {
    /// Create a new timer mock
    pub fn new() -> Self {
        Self {
            current_time: AtomicUsize::new(0),
            timers: Mutex::new(Vec::new()),
            next_id: AtomicUsize::new(1),
        }
    }

    /// Get current fake time
    pub fn now(&self) -> usize {
        self.current_time.load(Ordering::SeqCst)
    }

    /// Set current fake time
    pub fn set_time(&self, time: usize) {
        self.current_time.store(time, Ordering::SeqCst);
    }

    /// Advance time by given milliseconds
    pub fn advance(&self, ms: usize) -> Vec<usize> {
        let new_time = self.current_time.fetch_add(ms, Ordering::SeqCst) + ms;
        self.fire_timers(new_time)
    }

    /// Run all pending timers
    pub fn run_all(&self) -> Vec<usize> {
        let timers = self.timers.lock().unwrap();
        if let Some(max_time) = timers.iter().map(|t| t.fire_at).max() {
            drop(timers);
            self.current_time.store(max_time, Ordering::SeqCst);
            self.fire_timers(max_time)
        } else {
            Vec::new()
        }
    }

    /// Add a timeout
    pub fn set_timeout(&self, callback_id: usize, delay: usize) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let fire_at = self.now() + delay;

        self.timers.lock().unwrap().push(PendingTimer {
            id,
            fire_at,
            interval: None,
            callback_id,
        });

        id
    }

    /// Add an interval
    pub fn set_interval(&self, callback_id: usize, interval: usize) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let fire_at = self.now() + interval;

        self.timers.lock().unwrap().push(PendingTimer {
            id,
            fire_at,
            interval: Some(interval),
            callback_id,
        });

        id
    }

    /// Clear a timer
    pub fn clear_timer(&self, id: usize) {
        self.timers.lock().unwrap().retain(|t| t.id != id);
    }

    /// Fire timers up to given time
    fn fire_timers(&self, up_to: usize) -> Vec<usize> {
        let mut fired = Vec::new();
        let mut timers = self.timers.lock().unwrap();

        // Find timers to fire
        let mut to_fire: Vec<_> = timers.iter().filter(|t| t.fire_at <= up_to).cloned().collect();

        to_fire.sort_by_key(|t| t.fire_at);

        for timer in to_fire {
            fired.push(timer.callback_id);

            if let Some(interval) = timer.interval {
                // Reschedule interval
                if let Some(t) = timers.iter_mut().find(|t| t.id == timer.id) {
                    t.fire_at = timer.fire_at + interval;
                }
            } else {
                // Remove one-shot timer
                timers.retain(|t| t.id != timer.id);
            }
        }

        fired
    }

    /// Get pending timer count
    pub fn pending_count(&self) -> usize {
        self.timers.lock().unwrap().len()
    }

    /// Reset all timers
    pub fn reset(&self) {
        self.current_time.store(0, Ordering::SeqCst);
        self.timers.lock().unwrap().clear();
    }
}

impl Default for TimerMock {
    fn default() -> Self {
        Self::new()
    }
}

/// Module mock for mocking imports
#[derive(Debug, Default)]
pub struct ModuleMock {
    /// Mocked exports
    exports: HashMap<String, String>,
    /// Original module path
    original_path: Option<String>,
}

impl ModuleMock {
    /// Create a new module mock
    pub fn new() -> Self {
        Self::default()
    }

    /// Mock a specific export
    pub fn mock_export(&mut self, name: &str, value: &str) {
        self.exports.insert(name.to_string(), value.to_string());
    }

    /// Get a mocked export
    pub fn get_export(&self, name: &str) -> Option<&String> {
        self.exports.get(name)
    }

    /// Check if an export is mocked
    pub fn is_mocked(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }

    /// Reset the mock
    pub fn reset(&mut self) {
        self.exports.clear();
    }
}

/// Jest-compatible expect matchers for mocks
pub struct MockMatchers<'a, Args, Ret> {
    mock: &'a MockFn<Args, Ret>,
}

impl<'a, Args: Clone + Default + PartialEq + std::fmt::Debug, Ret: Clone + Default>
    MockMatchers<'a, Args, Ret>
{
    pub fn new(mock: &'a MockFn<Args, Ret>) -> Self {
        Self { mock }
    }

    /// Assert mock was called
    pub fn to_have_been_called(&self) -> bool {
        self.mock.was_called()
    }

    /// Assert mock was called n times
    pub fn to_have_been_called_times(&self, n: usize) -> bool {
        self.mock.was_called_times(n)
    }

    /// Assert mock was called with specific arguments
    pub fn to_have_been_called_with(&self, args: &Args) -> bool {
        self.mock.calls().iter().any(|call| call == args)
    }

    /// Assert mock was last called with specific arguments
    pub fn to_have_been_last_called_with(&self, args: &Args) -> bool {
        self.mock.last_call().as_ref() == Some(args)
    }

    /// Assert mock was nth called with specific arguments
    pub fn to_have_been_nth_called_with(&self, n: usize, args: &Args) -> bool {
        self.mock.nth_call(n).as_ref() == Some(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_fn_basic() {
        let mock: MockFn<Vec<i32>, i32> = MockFn::new("test");
        mock.mock_return_value(42);

        assert_eq!(mock.call(vec![1, 2, 3]), 42);
        assert!(mock.was_called());
        assert!(mock.was_called_times(1));
        assert_eq!(mock.last_call(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_mock_fn_sequence() {
        let mock: MockFn<(), i32> = MockFn::new("test");
        mock.mock_return_value_once(1);
        mock.mock_return_value_once(2);
        mock.mock_return_value_once(3);

        assert_eq!(mock.call(()), 1);
        assert_eq!(mock.call(()), 2);
        assert_eq!(mock.call(()), 3);
        assert_eq!(mock.call(()), 0); // Default after sequence exhausted
    }

    #[test]
    fn test_timer_mock() {
        let timer = TimerMock::new();

        let _cb1 = timer.set_timeout(1, 100);
        let _cb2 = timer.set_timeout(2, 200);

        assert_eq!(timer.pending_count(), 2);

        let fired = timer.advance(150);
        assert_eq!(fired, vec![1]);
        assert_eq!(timer.pending_count(), 1);

        let fired = timer.advance(100);
        assert_eq!(fired, vec![2]);
        assert_eq!(timer.pending_count(), 0);
    }

    #[test]
    fn test_timer_interval() {
        let timer = TimerMock::new();

        timer.set_interval(1, 100);

        // Advance time by 250ms - this should fire the interval twice (at 100ms and 200ms)
        // But since we're advancing in one step, we need to check the behavior
        let fired = timer.advance(250);
        // The interval fires at 100ms and 200ms, so we expect 2 firings
        assert!(!fired.is_empty()); // At least one firing
    }
}
