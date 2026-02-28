//! Mock and Spy Implementation for DX Test Runner
//!
//! Provides Jest-compatible mocking functionality:
//! - `jest.fn()` - Create mock functions with call tracking
//! - `jest.spyOn()` - Spy on existing object methods
//! - `jest.mock()` - Mock entire modules
//! - Timer mocks - Control time in tests

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// A mock function call record
#[derive(Debug, Clone)]
pub struct MockCall {
    /// Arguments passed to the call
    pub args: Vec<MockValue>,
    /// Return value from the call
    pub return_value: Option<MockValue>,
    /// Timestamp of the call
    pub timestamp: u64,
    /// Context (this) value
    pub this_value: Option<MockValue>,
}

/// Simplified value representation for mocking
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MockValue {
    #[default]
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<MockValue>),
    Object(HashMap<String, MockValue>),
    Function(u64), // Function ID reference
}

impl MockValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            MockValue::Undefined | MockValue::Null => false,
            MockValue::Boolean(b) => *b,
            MockValue::Number(n) => *n != 0.0 && !n.is_nan(),
            MockValue::String(s) => !s.is_empty(),
            MockValue::Array(_) | MockValue::Object(_) | MockValue::Function(_) => true,
        }
    }
}

/// Mock function implementation type
pub type MockImplementation = Arc<dyn Fn(&[MockValue]) -> MockValue + Send + Sync>;

/// A mock function that tracks calls and can return configured values
#[derive(Clone)]
pub struct MockFunction {
    /// Unique ID for this mock
    pub id: u64,
    /// Name of the mock (for debugging)
    pub name: Option<String>,
    /// All recorded calls
    calls: Arc<Mutex<Vec<MockCall>>>,
    /// Queue of return values (mockReturnValueOnce)
    return_value_queue: Arc<Mutex<VecDeque<MockValue>>>,
    /// Default return value (mockReturnValue)
    default_return_value: Arc<RwLock<MockValue>>,
    /// Custom implementation (mockImplementation)
    implementation: Arc<RwLock<Option<MockImplementation>>>,
    /// Queue of implementations (mockImplementationOnce)
    implementation_queue: Arc<Mutex<VecDeque<MockImplementation>>>,
    /// Resolved value for async mocks (mockResolvedValue)
    resolved_value: Arc<RwLock<Option<MockValue>>>,
    /// Rejected value for async mocks (mockRejectedValue)
    rejected_value: Arc<RwLock<Option<MockValue>>>,
}

impl MockFunction {
    /// Create a new mock function
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            name: None,
            calls: Arc::new(Mutex::new(Vec::new())),
            return_value_queue: Arc::new(Mutex::new(VecDeque::new())),
            default_return_value: Arc::new(RwLock::new(MockValue::Undefined)),
            implementation: Arc::new(RwLock::new(None)),
            implementation_queue: Arc::new(Mutex::new(VecDeque::new())),
            resolved_value: Arc::new(RwLock::new(None)),
            rejected_value: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a named mock function
    pub fn with_name(name: impl Into<String>) -> Self {
        let mut mock = Self::new();
        mock.name = Some(name.into());
        mock
    }

    /// Call the mock function and record the call
    pub fn call(&self, args: Vec<MockValue>) -> MockValue {
        self.call_with_this(None, args)
    }

    /// Call the mock function with a this context
    pub fn call_with_this(&self, this: Option<MockValue>, args: Vec<MockValue>) -> MockValue {
        // Determine return value
        let return_value = self.compute_return_value(&args);

        // Record the call
        let call = MockCall {
            args,
            return_value: Some(return_value.clone()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            this_value: this,
        };
        self.calls.lock().unwrap().push(call);

        return_value
    }

    fn compute_return_value(&self, args: &[MockValue]) -> MockValue {
        // 1. Check implementation queue first (mockImplementationOnce)
        if let Some(impl_fn) = self.implementation_queue.lock().unwrap().pop_front() {
            return impl_fn(args);
        }

        // 2. Check return value queue (mockReturnValueOnce)
        if let Some(value) = self.return_value_queue.lock().unwrap().pop_front() {
            return value;
        }

        // 3. Check custom implementation (mockImplementation)
        if let Some(ref impl_fn) = *self.implementation.read().unwrap() {
            return impl_fn(args);
        }

        // 4. Return default value (mockReturnValue)
        self.default_return_value.read().unwrap().clone()
    }

    /// Set the default return value
    pub fn mock_return_value(&self, value: MockValue) -> &Self {
        *self.default_return_value.write().unwrap() = value;
        self
    }

    /// Add a one-time return value to the queue
    pub fn mock_return_value_once(&self, value: MockValue) -> &Self {
        self.return_value_queue.lock().unwrap().push_back(value);
        self
    }

    /// Set a custom implementation
    pub fn mock_implementation<F>(&self, f: F) -> &Self
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        *self.implementation.write().unwrap() = Some(Arc::new(f));
        self
    }

    /// Add a one-time implementation
    pub fn mock_implementation_once<F>(&self, f: F) -> &Self
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        self.implementation_queue.lock().unwrap().push_back(Arc::new(f));
        self
    }

    /// Set resolved value for async mocks (returns a "promise-like" value)
    pub fn mock_resolved_value(&self, value: MockValue) -> &Self {
        *self.resolved_value.write().unwrap() = Some(value);
        self
    }

    /// Set rejected value for async mocks
    pub fn mock_rejected_value(&self, value: MockValue) -> &Self {
        *self.rejected_value.write().unwrap() = Some(value);
        self
    }

    /// Clear all recorded calls
    pub fn mock_clear(&self) -> &Self {
        self.calls.lock().unwrap().clear();
        self
    }

    /// Reset the mock to initial state
    pub fn mock_reset(&self) -> &Self {
        self.calls.lock().unwrap().clear();
        self.return_value_queue.lock().unwrap().clear();
        *self.default_return_value.write().unwrap() = MockValue::Undefined;
        *self.implementation.write().unwrap() = None;
        self.implementation_queue.lock().unwrap().clear();
        *self.resolved_value.write().unwrap() = None;
        *self.rejected_value.write().unwrap() = None;
        self
    }

    /// Restore the mock (same as reset for jest.fn())
    pub fn mock_restore(&self) -> &Self {
        self.mock_reset()
    }

    // === Call inspection methods ===

    /// Get all recorded calls
    pub fn calls(&self) -> Vec<MockCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Get the number of times the mock was called
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Check if the mock was called
    pub fn was_called(&self) -> bool {
        !self.calls.lock().unwrap().is_empty()
    }

    /// Check if the mock was called exactly n times
    pub fn was_called_times(&self, n: usize) -> bool {
        self.calls.lock().unwrap().len() == n
    }

    /// Get the arguments of the last call
    pub fn last_call(&self) -> Option<MockCall> {
        self.calls.lock().unwrap().last().cloned()
    }

    /// Get the arguments of the nth call (0-indexed)
    pub fn nth_call(&self, n: usize) -> Option<MockCall> {
        self.calls.lock().unwrap().get(n).cloned()
    }

    /// Check if the mock was called with specific arguments
    pub fn was_called_with(&self, expected_args: &[MockValue]) -> bool {
        self.calls.lock().unwrap().iter().any(|call| call.args == expected_args)
    }

    /// Check if the last call had specific arguments
    pub fn was_last_called_with(&self, expected_args: &[MockValue]) -> bool {
        self.calls
            .lock()
            .unwrap()
            .last()
            .map(|call| call.args == expected_args)
            .unwrap_or(false)
    }

    /// Check if the nth call had specific arguments
    pub fn was_nth_called_with(&self, n: usize, expected_args: &[MockValue]) -> bool {
        self.calls
            .lock()
            .unwrap()
            .get(n)
            .map(|call| call.args == expected_args)
            .unwrap_or(false)
    }

    /// Get all return values
    pub fn return_values(&self) -> Vec<MockValue> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter_map(|call| call.return_value.clone())
            .collect()
    }
}

impl Default for MockFunction {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MockFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockFunction")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("call_count", &self.call_count())
            .finish()
    }
}

/// Spy on an existing function/method
/// Unlike MockFunction, a spy preserves the original implementation by default
#[derive(Clone)]
pub struct Spy {
    /// The underlying mock function
    mock: MockFunction,
    /// Original implementation (if captured)
    original_impl: Arc<RwLock<Option<MockImplementation>>>,
    /// Whether to call through to original
    call_through: Arc<RwLock<bool>>,
}

impl Spy {
    /// Create a new spy
    pub fn new() -> Self {
        Self {
            mock: MockFunction::new(),
            original_impl: Arc::new(RwLock::new(None)),
            call_through: Arc::new(RwLock::new(true)),
        }
    }

    /// Create a spy with an original implementation
    pub fn with_original<F>(f: F) -> Self
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        let spy = Self::new();
        *spy.original_impl.write().unwrap() = Some(Arc::new(f));
        spy
    }

    /// Call the spy
    pub fn call(&self, args: Vec<MockValue>) -> MockValue {
        // If call_through is enabled and we have an original, use it
        let should_call_through = *self.call_through.read().unwrap();

        if should_call_through {
            if let Some(ref original) = *self.original_impl.read().unwrap() {
                let result = original(&args);
                // Record the call
                let call = MockCall {
                    args,
                    return_value: Some(result.clone()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    this_value: None,
                };
                self.mock.calls.lock().unwrap().push(call);
                return result;
            }
        }

        // Otherwise use mock behavior
        self.mock.call(args)
    }

    /// Stop calling through to original implementation
    pub fn mock_implementation<F>(&self, f: F) -> &Self
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        *self.call_through.write().unwrap() = false;
        self.mock.mock_implementation(f);
        self
    }

    /// Set return value (stops calling through)
    pub fn mock_return_value(&self, value: MockValue) -> &Self {
        *self.call_through.write().unwrap() = false;
        self.mock.mock_return_value(value);
        self
    }

    /// Restore original implementation
    pub fn mock_restore(&self) -> &Self {
        *self.call_through.write().unwrap() = true;
        self.mock.mock_reset();
        self
    }

    /// Clear call history but keep mock configuration
    pub fn mock_clear(&self) -> &Self {
        self.mock.mock_clear();
        self
    }

    /// Reset mock configuration but keep original
    pub fn mock_reset(&self) -> &Self {
        *self.call_through.write().unwrap() = true;
        self.mock.mock_reset();
        self
    }

    /// Get call count
    pub fn call_count(&self) -> usize {
        self.mock.call_count()
    }

    /// Check if called
    pub fn was_called(&self) -> bool {
        self.mock.was_called()
    }

    /// Get all calls
    pub fn calls(&self) -> Vec<MockCall> {
        self.mock.calls()
    }
}

impl Default for Spy {
    fn default() -> Self {
        Self::new()
    }
}

/// Module mock registry
/// Tracks mocked modules and their mock implementations
pub struct ModuleMockRegistry {
    /// Mocked modules: module path -> mock exports
    mocks: RwLock<HashMap<String, HashMap<String, MockFunction>>>,
    /// Auto-mock enabled modules
    auto_mocked: RwLock<HashMap<String, bool>>,
    /// Original module cache (for unmocking)
    #[allow(dead_code)]
    originals: RwLock<HashMap<String, HashMap<String, MockImplementation>>>,
}

impl ModuleMockRegistry {
    pub fn new() -> Self {
        Self {
            mocks: RwLock::new(HashMap::new()),
            auto_mocked: RwLock::new(HashMap::new()),
            originals: RwLock::new(HashMap::new()),
        }
    }

    /// Mock a module with a factory function result
    pub fn mock_module(&self, module_path: &str, exports: HashMap<String, MockFunction>) {
        self.mocks.write().unwrap().insert(module_path.to_string(), exports);
    }

    /// Create auto-mock for a module (all exports become jest.fn())
    pub fn auto_mock(&self, module_path: &str) {
        self.auto_mocked.write().unwrap().insert(module_path.to_string(), true);
    }

    /// Check if a module is mocked
    pub fn is_mocked(&self, module_path: &str) -> bool {
        self.mocks.read().unwrap().contains_key(module_path)
            || self.auto_mocked.read().unwrap().get(module_path).copied().unwrap_or(false)
    }

    /// Get mock for a module export
    pub fn get_mock(&self, module_path: &str, export_name: &str) -> Option<MockFunction> {
        self.mocks
            .read()
            .unwrap()
            .get(module_path)
            .and_then(|exports| exports.get(export_name).cloned())
    }

    /// Unmock a module
    pub fn unmock(&self, module_path: &str) {
        self.mocks.write().unwrap().remove(module_path);
        self.auto_mocked.write().unwrap().remove(module_path);
    }

    /// Reset all module mocks
    pub fn reset_all(&self) {
        self.mocks.write().unwrap().clear();
        self.auto_mocked.write().unwrap().clear();
    }

    /// Clear all mock call histories
    pub fn clear_all(&self) {
        for exports in self.mocks.read().unwrap().values() {
            for mock in exports.values() {
                mock.mock_clear();
            }
        }
    }
}

impl Default for ModuleMockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer entry for fake timers
#[derive(Debug, Clone)]
pub struct FakeTimer {
    /// Timer ID
    pub id: u64,
    /// Callback to execute
    pub callback_id: u64,
    /// Fire time (virtual milliseconds)
    pub fire_at: u64,
    /// Interval (None for setTimeout, Some for setInterval)
    pub interval: Option<u64>,
    /// Whether the timer is active
    pub active: bool,
}

/// Fake timer controller for jest.useFakeTimers()
pub struct FakeTimers {
    /// Whether fake timers are enabled
    enabled: RwLock<bool>,
    /// Current virtual time (milliseconds)
    current_time: RwLock<u64>,
    /// Pending timers sorted by fire time
    timers: Mutex<Vec<FakeTimer>>,
    /// Next timer ID
    next_id: std::sync::atomic::AtomicU64,
    /// Callbacks registered for timers
    callbacks: Mutex<HashMap<u64, MockFunction>>,
    /// Real time when fake timers were enabled
    real_start_time: RwLock<Option<Instant>>,
}

impl FakeTimers {
    pub fn new() -> Self {
        Self {
            enabled: RwLock::new(false),
            current_time: RwLock::new(0),
            timers: Mutex::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
            callbacks: Mutex::new(HashMap::new()),
            real_start_time: RwLock::new(None),
        }
    }

    /// Enable fake timers (jest.useFakeTimers())
    pub fn use_fake_timers(&self) {
        *self.enabled.write().unwrap() = true;
        *self.current_time.write().unwrap() = 0;
        *self.real_start_time.write().unwrap() = Some(Instant::now());
        self.timers.lock().unwrap().clear();
        self.callbacks.lock().unwrap().clear();
    }

    /// Disable fake timers and restore real timers (jest.useRealTimers())
    pub fn use_real_timers(&self) {
        *self.enabled.write().unwrap() = false;
        *self.real_start_time.write().unwrap() = None;
        self.timers.lock().unwrap().clear();
        self.callbacks.lock().unwrap().clear();
    }

    /// Check if fake timers are enabled
    pub fn is_using_fake_timers(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// Get current virtual time
    pub fn now(&self) -> u64 {
        *self.current_time.read().unwrap()
    }

    /// Set a timeout (returns timer ID)
    pub fn set_timeout(&self, callback: MockFunction, delay: u64) -> u64 {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let callback_id = callback.id;

        self.callbacks.lock().unwrap().insert(callback_id, callback);

        let timer = FakeTimer {
            id,
            callback_id,
            fire_at: *self.current_time.read().unwrap() + delay,
            interval: None,
            active: true,
        };

        let mut timers = self.timers.lock().unwrap();
        timers.push(timer);
        timers.sort_by_key(|t| t.fire_at);

        id
    }

    /// Set an interval (returns timer ID)
    pub fn set_interval(&self, callback: MockFunction, interval: u64) -> u64 {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let callback_id = callback.id;

        self.callbacks.lock().unwrap().insert(callback_id, callback);

        let timer = FakeTimer {
            id,
            callback_id,
            fire_at: *self.current_time.read().unwrap() + interval,
            interval: Some(interval),
            active: true,
        };

        let mut timers = self.timers.lock().unwrap();
        timers.push(timer);
        timers.sort_by_key(|t| t.fire_at);

        id
    }

    /// Clear a timeout
    pub fn clear_timeout(&self, id: u64) {
        let mut timers = self.timers.lock().unwrap();
        if let Some(timer) = timers.iter_mut().find(|t| t.id == id) {
            timer.active = false;
        }
    }

    /// Clear an interval
    pub fn clear_interval(&self, id: u64) {
        self.clear_timeout(id);
    }

    /// Advance timers by specified milliseconds (jest.advanceTimersByTime())
    pub fn advance_timers_by_time(&self, ms: u64) -> Vec<u64> {
        let target_time = *self.current_time.read().unwrap() + ms;
        let mut fired_callbacks = Vec::new();

        loop {
            let timers = self.timers.lock().unwrap();

            // Find next timer to fire
            let next_timer = timers
                .iter()
                .filter(|t| t.active && t.fire_at <= target_time)
                .min_by_key(|t| t.fire_at)
                .cloned();

            drop(timers);

            match next_timer {
                Some(timer) => {
                    // Advance time to timer fire time
                    *self.current_time.write().unwrap() = timer.fire_at;

                    // Execute callback
                    if let Some(callback) = self.callbacks.lock().unwrap().get(&timer.callback_id) {
                        callback.call(vec![]);
                        fired_callbacks.push(timer.callback_id);
                    }

                    // Handle interval rescheduling
                    let mut timers = self.timers.lock().unwrap();
                    if let Some(interval) = timer.interval {
                        if let Some(t) = timers.iter_mut().find(|t| t.id == timer.id) {
                            t.fire_at = timer.fire_at + interval;
                        }
                        timers.sort_by_key(|t| t.fire_at);
                    } else {
                        // Remove one-shot timer
                        timers.retain(|t| t.id != timer.id);
                    }
                }
                None => break,
            }
        }

        // Set final time
        *self.current_time.write().unwrap() = target_time;
        fired_callbacks
    }

    /// Run all pending timers (jest.runAllTimers())
    pub fn run_all_timers(&self) -> Vec<u64> {
        let mut fired_callbacks = Vec::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100_000;

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                panic!("Exceeded maximum timer iterations - possible infinite loop");
            }

            let timers = self.timers.lock().unwrap();

            // Find next active timer
            let next_timer = timers.iter().filter(|t| t.active).min_by_key(|t| t.fire_at).cloned();

            drop(timers);

            match next_timer {
                Some(timer) => {
                    // Advance time
                    *self.current_time.write().unwrap() = timer.fire_at;

                    // Execute callback
                    if let Some(callback) = self.callbacks.lock().unwrap().get(&timer.callback_id) {
                        callback.call(vec![]);
                        fired_callbacks.push(timer.callback_id);
                    }

                    // Handle interval - for runAllTimers, we only run intervals once
                    let mut timers = self.timers.lock().unwrap();
                    timers.retain(|t| t.id != timer.id);
                }
                None => break,
            }
        }

        fired_callbacks
    }

    /// Run only pending timers (not intervals) (jest.runOnlyPendingTimers())
    pub fn run_only_pending_timers(&self) -> Vec<u64> {
        let mut fired_callbacks = Vec::new();
        let snapshot_time = *self.current_time.read().unwrap();

        // Get all timers that were pending at snapshot time
        let pending: Vec<_> = self
            .timers
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.active && t.fire_at >= snapshot_time)
            .cloned()
            .collect();

        for timer in pending {
            if timer.interval.is_some() {
                continue; // Skip intervals
            }

            *self.current_time.write().unwrap() = timer.fire_at;

            if let Some(callback) = self.callbacks.lock().unwrap().get(&timer.callback_id) {
                callback.call(vec![]);
                fired_callbacks.push(timer.callback_id);
            }

            self.timers.lock().unwrap().retain(|t| t.id != timer.id);
        }

        fired_callbacks
    }

    /// Advance to next timer (jest.advanceTimersToNextTimer())
    pub fn advance_timers_to_next_timer(&self) -> Option<u64> {
        let timers = self.timers.lock().unwrap();
        let next = timers.iter().filter(|t| t.active).min_by_key(|t| t.fire_at).cloned();
        drop(timers);

        if let Some(timer) = next {
            let advance_by = timer.fire_at.saturating_sub(*self.current_time.read().unwrap());
            let fired = self.advance_timers_by_time(advance_by);
            fired.first().copied()
        } else {
            None
        }
    }

    /// Get count of pending timers
    pub fn get_timer_count(&self) -> usize {
        self.timers.lock().unwrap().iter().filter(|t| t.active).count()
    }

    /// Clear all timers
    pub fn clear_all_timers(&self) {
        self.timers.lock().unwrap().clear();
        self.callbacks.lock().unwrap().clear();
    }

    /// Set system time (jest.setSystemTime())
    pub fn set_system_time(&self, time: u64) {
        *self.current_time.write().unwrap() = time;
    }

    /// Get real system time elapsed since fake timers were enabled
    pub fn get_real_system_time(&self) -> Option<Duration> {
        self.real_start_time.read().unwrap().map(|start| start.elapsed())
    }
}

impl Default for FakeTimers {
    fn default() -> Self {
        Self::new()
    }
}

/// Jest-compatible mock API
/// Provides the familiar jest.fn(), jest.spyOn(), jest.mock() interface
pub struct Jest {
    /// Module mock registry
    pub modules: ModuleMockRegistry,
    /// Fake timers controller
    pub timers: FakeTimers,
    /// All created mocks (for cleanup)
    mocks: Mutex<Vec<MockFunction>>,
    /// All created spies (for cleanup)
    spies: Mutex<Vec<Spy>>,
}

impl Jest {
    pub fn new() -> Self {
        Self {
            modules: ModuleMockRegistry::new(),
            timers: FakeTimers::new(),
            mocks: Mutex::new(Vec::new()),
            spies: Mutex::new(Vec::new()),
        }
    }

    /// Create a mock function (jest.fn())
    pub fn fn_mock(&self) -> MockFunction {
        let mock = MockFunction::new();
        self.mocks.lock().unwrap().push(mock.clone());
        mock
    }

    /// Create a mock function with implementation (jest.fn(impl))
    pub fn fn_with_impl<F>(&self, f: F) -> MockFunction
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        let mock = MockFunction::new();
        mock.mock_implementation(f);
        self.mocks.lock().unwrap().push(mock.clone());
        mock
    }

    /// Create a spy on an object method (jest.spyOn())
    pub fn spy_on<F>(&self, original: F) -> Spy
    where
        F: Fn(&[MockValue]) -> MockValue + Send + Sync + 'static,
    {
        let spy = Spy::with_original(original);
        self.spies.lock().unwrap().push(spy.clone());
        spy
    }

    /// Mock a module (jest.mock())
    pub fn mock_module(&self, module_path: &str, exports: HashMap<String, MockFunction>) {
        self.modules.mock_module(module_path, exports);
    }

    /// Unmock a module (jest.unmock())
    pub fn unmock(&self, module_path: &str) {
        self.modules.unmock(module_path);
    }

    /// Enable fake timers (jest.useFakeTimers())
    pub fn use_fake_timers(&self) {
        self.timers.use_fake_timers();
    }

    /// Disable fake timers (jest.useRealTimers())
    pub fn use_real_timers(&self) {
        self.timers.use_real_timers();
    }

    /// Advance timers (jest.advanceTimersByTime())
    pub fn advance_timers_by_time(&self, ms: u64) -> Vec<u64> {
        self.timers.advance_timers_by_time(ms)
    }

    /// Run all timers (jest.runAllTimers())
    pub fn run_all_timers(&self) -> Vec<u64> {
        self.timers.run_all_timers()
    }

    /// Run only pending timers (jest.runOnlyPendingTimers())
    pub fn run_only_pending_timers(&self) -> Vec<u64> {
        self.timers.run_only_pending_timers()
    }

    /// Clear all mocks (jest.clearAllMocks())
    pub fn clear_all_mocks(&self) {
        for mock in self.mocks.lock().unwrap().iter() {
            mock.mock_clear();
        }
        for spy in self.spies.lock().unwrap().iter() {
            spy.mock_clear();
        }
        self.modules.clear_all();
    }

    /// Reset all mocks (jest.resetAllMocks())
    pub fn reset_all_mocks(&self) {
        for mock in self.mocks.lock().unwrap().iter() {
            mock.mock_reset();
        }
        for spy in self.spies.lock().unwrap().iter() {
            spy.mock_reset();
        }
        self.modules.reset_all();
    }

    /// Restore all mocks (jest.restoreAllMocks())
    pub fn restore_all_mocks(&self) {
        for mock in self.mocks.lock().unwrap().iter() {
            mock.mock_restore();
        }
        for spy in self.spies.lock().unwrap().iter() {
            spy.mock_restore();
        }
        self.modules.reset_all();
        self.timers.use_real_timers();
    }

    /// Get current fake time (jest.now() when using fake timers)
    pub fn now(&self) -> u64 {
        self.timers.now()
    }

    /// Set system time (jest.setSystemTime())
    pub fn set_system_time(&self, time: u64) {
        self.timers.set_system_time(time);
    }

    /// Get timer count
    pub fn get_timer_count(&self) -> usize {
        self.timers.get_timer_count()
    }
}

impl Default for Jest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_function_basic() {
        let mock = MockFunction::new();

        // Initially not called
        assert!(!mock.was_called());
        assert_eq!(mock.call_count(), 0);

        // Call it
        let result = mock.call(vec![MockValue::Number(42.0)]);
        assert_eq!(result, MockValue::Undefined);

        // Now it's been called
        assert!(mock.was_called());
        assert_eq!(mock.call_count(), 1);
        assert!(mock.was_called_with(&[MockValue::Number(42.0)]));
    }

    #[test]
    fn test_mock_return_value() {
        let mock = MockFunction::new();
        mock.mock_return_value(MockValue::String("hello".to_string()));

        let result = mock.call(vec![]);
        assert_eq!(result, MockValue::String("hello".to_string()));
    }

    #[test]
    fn test_mock_return_value_once() {
        let mock = MockFunction::new();
        mock.mock_return_value(MockValue::String("default".to_string()));
        mock.mock_return_value_once(MockValue::String("first".to_string()));
        mock.mock_return_value_once(MockValue::String("second".to_string()));

        assert_eq!(mock.call(vec![]), MockValue::String("first".to_string()));
        assert_eq!(mock.call(vec![]), MockValue::String("second".to_string()));
        assert_eq!(mock.call(vec![]), MockValue::String("default".to_string()));
        assert_eq!(mock.call(vec![]), MockValue::String("default".to_string()));
    }

    #[test]
    fn test_mock_implementation() {
        let mock = MockFunction::new();
        mock.mock_implementation(|args| {
            if let Some(MockValue::Number(n)) = args.first() {
                MockValue::Number(n * 2.0)
            } else {
                MockValue::Undefined
            }
        });

        let result = mock.call(vec![MockValue::Number(21.0)]);
        assert_eq!(result, MockValue::Number(42.0));
    }

    #[test]
    fn test_mock_clear_and_reset() {
        let mock = MockFunction::new();
        mock.mock_return_value(MockValue::Boolean(true));
        mock.call(vec![]);
        mock.call(vec![]);

        assert_eq!(mock.call_count(), 2);

        // Clear only clears calls
        mock.mock_clear();
        assert_eq!(mock.call_count(), 0);
        assert_eq!(mock.call(vec![]), MockValue::Boolean(true)); // Return value preserved

        // Reset clears everything
        mock.mock_reset();
        assert_eq!(mock.call_count(), 0);
        assert_eq!(mock.call(vec![]), MockValue::Undefined); // Return value reset
    }

    #[test]
    fn test_spy_call_through() {
        let spy = Spy::with_original(|args| {
            if let Some(MockValue::Number(n)) = args.first() {
                MockValue::Number(n + 1.0)
            } else {
                MockValue::Undefined
            }
        });

        // Spy calls through to original by default
        let result = spy.call(vec![MockValue::Number(5.0)]);
        assert_eq!(result, MockValue::Number(6.0));
        assert_eq!(spy.call_count(), 1);
    }

    #[test]
    fn test_spy_mock_implementation() {
        let spy = Spy::with_original(|_| MockValue::String("original".to_string()));

        // Override with mock implementation
        spy.mock_implementation(|_| MockValue::String("mocked".to_string()));

        let result = spy.call(vec![]);
        assert_eq!(result, MockValue::String("mocked".to_string()));

        // Restore original
        spy.mock_restore();
        let result = spy.call(vec![]);
        assert_eq!(result, MockValue::String("original".to_string()));
    }

    #[test]
    fn test_fake_timers_basic() {
        let timers = FakeTimers::new();
        timers.use_fake_timers();

        let callback = MockFunction::new();
        timers.set_timeout(callback.clone(), 1000);

        assert_eq!(callback.call_count(), 0);

        // Advance by 500ms - not fired yet
        timers.advance_timers_by_time(500);
        assert_eq!(callback.call_count(), 0);

        // Advance by another 500ms - now fired
        timers.advance_timers_by_time(500);
        assert_eq!(callback.call_count(), 1);
    }

    #[test]
    fn test_fake_timers_interval() {
        let timers = FakeTimers::new();
        timers.use_fake_timers();

        let callback = MockFunction::new();
        timers.set_interval(callback.clone(), 100);

        // Advance by 350ms - should fire 3 times
        timers.advance_timers_by_time(350);
        assert_eq!(callback.call_count(), 3);
    }

    #[test]
    fn test_fake_timers_clear() {
        let timers = FakeTimers::new();
        timers.use_fake_timers();

        let callback = MockFunction::new();
        let id = timers.set_timeout(callback.clone(), 1000);

        // Clear before it fires
        timers.clear_timeout(id);
        timers.advance_timers_by_time(2000);

        assert_eq!(callback.call_count(), 0);
    }

    #[test]
    fn test_fake_timers_run_all() {
        let timers = FakeTimers::new();
        timers.use_fake_timers();

        let cb1 = MockFunction::new();
        let cb2 = MockFunction::new();
        let cb3 = MockFunction::new();

        timers.set_timeout(cb1.clone(), 100);
        timers.set_timeout(cb2.clone(), 200);
        timers.set_timeout(cb3.clone(), 300);

        timers.run_all_timers();

        assert_eq!(cb1.call_count(), 1);
        assert_eq!(cb2.call_count(), 1);
        assert_eq!(cb3.call_count(), 1);
        assert_eq!(timers.now(), 300);
    }

    #[test]
    fn test_jest_api() {
        let jest = Jest::new();

        // Create mock
        let mock = jest.fn_mock();
        mock.mock_return_value(MockValue::Number(42.0));

        assert_eq!(mock.call(vec![]), MockValue::Number(42.0));
        assert!(mock.was_called());

        // Clear all
        jest.clear_all_mocks();
        assert!(!mock.was_called());
    }

    #[test]
    fn test_module_mock_registry() {
        let registry = ModuleMockRegistry::new();

        let mut exports = HashMap::new();
        exports.insert("default".to_string(), MockFunction::new());
        exports.insert("helper".to_string(), MockFunction::new());

        registry.mock_module("./utils", exports);

        assert!(registry.is_mocked("./utils"));
        assert!(!registry.is_mocked("./other"));

        let mock = registry.get_mock("./utils", "default");
        assert!(mock.is_some());

        registry.unmock("./utils");
        assert!(!registry.is_mocked("./utils"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary MockValue
    fn arb_mock_value() -> impl Strategy<Value = MockValue> {
        prop_oneof![
            Just(MockValue::Undefined),
            Just(MockValue::Null),
            any::<bool>().prop_map(MockValue::Boolean),
            any::<f64>().prop_map(MockValue::Number),
            "[a-zA-Z0-9]{0,20}".prop_map(MockValue::String),
        ]
    }

    /// Generate a vector of MockValues
    fn arb_mock_values(max_len: usize) -> impl Strategy<Value = Vec<MockValue>> {
        prop::collection::vec(arb_mock_value(), 0..max_len)
    }

    proptest! {
        /// Property 9: Test Isolation - Mock state is independent between instances
        /// Feature: dx-js-production-complete, Property 9: Test Isolation
        /// Validates: Requirements 11.1, 11.2, 11.3
        #[test]
        fn prop_mock_isolation(
            args1 in arb_mock_values(5),
            return_val in arb_mock_value()
        ) {
            let mock1 = MockFunction::new();
            let mock2 = MockFunction::new();

            // Configure mock1
            mock1.mock_return_value(return_val.clone());
            mock1.call(args1.clone());

            // mock2 should be unaffected
            prop_assert!(!mock2.was_called());
            prop_assert_eq!(mock2.call_count(), 0);
            prop_assert_eq!(mock2.call(vec![]), MockValue::Undefined);

            // mock1 should have its state (1 call from above)
            prop_assert!(mock1.was_called());
            prop_assert_eq!(mock1.call_count(), 1);
        }

        /// Property: Mock call tracking is accurate
        /// All calls are recorded with correct arguments
        #[test]
        fn prop_mock_call_tracking(
            calls in prop::collection::vec(arb_mock_values(3), 1..10)
        ) {
            let mock = MockFunction::new();

            for args in &calls {
                mock.call(args.clone());
            }

            prop_assert_eq!(mock.call_count(), calls.len());

            let recorded = mock.calls();
            for (i, expected_args) in calls.iter().enumerate() {
                prop_assert_eq!(&recorded[i].args, expected_args);
            }
        }

        /// Property: mockReturnValueOnce queue is FIFO
        /// Return values are consumed in order
        #[test]
        fn prop_return_value_queue_fifo(
            values in prop::collection::vec(arb_mock_value(), 1..10),
            default in arb_mock_value()
        ) {
            let mock = MockFunction::new();
            mock.mock_return_value(default.clone());

            for value in &values {
                mock.mock_return_value_once(value.clone());
            }

            // Values should come out in order
            for expected in &values {
                let result = mock.call(vec![]);
                prop_assert_eq!(&result, expected);
            }

            // After queue exhausted, should return default
            let result = mock.call(vec![]);
            prop_assert_eq!(result, default);
        }

        /// Property: mock_clear preserves configuration
        /// Clearing calls doesn't affect return values
        #[test]
        fn prop_clear_preserves_config(
            return_val in arb_mock_value(),
            call_count in 1usize..20
        ) {
            let mock = MockFunction::new();
            mock.mock_return_value(return_val.clone());

            // Make some calls
            for _ in 0..call_count {
                mock.call(vec![]);
            }
            prop_assert_eq!(mock.call_count(), call_count);

            // Clear
            mock.mock_clear();
            prop_assert_eq!(mock.call_count(), 0);

            // Return value should still work
            let result = mock.call(vec![]);
            prop_assert_eq!(result, return_val);
        }

        /// Property: mock_reset clears everything
        /// After reset, mock behaves as new
        #[test]
        fn prop_reset_clears_all(
            return_val in arb_mock_value(),
            call_count in 1usize..20
        ) {
            let mock = MockFunction::new();
            mock.mock_return_value(return_val);

            for _ in 0..call_count {
                mock.call(vec![]);
            }

            mock.mock_reset();

            prop_assert_eq!(mock.call_count(), 0);
            prop_assert_eq!(mock.call(vec![]), MockValue::Undefined);
        }

        /// Property: Spy preserves original behavior by default
        #[test]
        fn prop_spy_preserves_original(
            input in -1e100f64..1e100f64  // Constrain to avoid overflow when multiplying by 2
        ) {
            let spy = Spy::with_original(move |args| {
                if let Some(MockValue::Number(n)) = args.first() {
                    MockValue::Number(n * 2.0)
                } else {
                    MockValue::Undefined
                }
            });

            let result = spy.call(vec![MockValue::Number(input)]);

            if let MockValue::Number(n) = result {
                let expected = input * 2.0;
                // Use relative tolerance for floating point comparison
                let tolerance = if expected.abs() < 1e-10 { 1e-15 } else { expected.abs() * 1e-10 };
                prop_assert!((n - expected).abs() < tolerance || (n.is_nan() && expected.is_nan()),
                    "Expected {} but got {}", expected, n);
            } else {
                prop_assert!(false, "Expected Number result");
            }

            prop_assert_eq!(spy.call_count(), 1);
        }

        /// Property: Timer ordering is correct
        /// Timers fire in chronological order
        #[test]
        fn prop_timer_ordering(
            delays in prop::collection::vec(1u64..1000, 1..10)
        ) {
            let timers = FakeTimers::new();
            timers.use_fake_timers();

            let mut callbacks: Vec<MockFunction> = Vec::new();
            let mut sorted_delays = delays.clone();
            sorted_delays.sort();

            for delay in &delays {
                let cb = MockFunction::new();
                timers.set_timeout(cb.clone(), *delay);
                callbacks.push(cb);
            }

            // Run all timers
            timers.run_all_timers();

            // All callbacks should have been called exactly once
            for cb in &callbacks {
                prop_assert_eq!(cb.call_count(), 1);
            }

            // Final time should be the max delay
            if let Some(&max_delay) = sorted_delays.last() {
                prop_assert_eq!(timers.now(), max_delay);
            }
        }

        /// Property: Cleared timers don't fire
        #[test]
        fn prop_cleared_timers_dont_fire(
            delay in 1u64..1000,
            advance in 0u64..2000
        ) {
            let timers = FakeTimers::new();
            timers.use_fake_timers();

            let cb = MockFunction::new();
            let id = timers.set_timeout(cb.clone(), delay);

            // Clear the timer
            timers.clear_timeout(id);

            // Advance time
            timers.advance_timers_by_time(advance);

            // Callback should never have been called
            prop_assert_eq!(cb.call_count(), 0);
        }

        /// Property: Interval fires correct number of times
        #[test]
        fn prop_interval_fires_correctly(
            interval in 10u64..100,
            total_time in 0u64..500
        ) {
            let timers = FakeTimers::new();
            timers.use_fake_timers();

            let cb = MockFunction::new();
            timers.set_interval(cb.clone(), interval);

            timers.advance_timers_by_time(total_time);

            let expected_fires = total_time / interval;
            prop_assert_eq!(cb.call_count() as u64, expected_fires);
        }
    }
}
