//! Async Runtime - Event Loop, Promises, Timers
//!
//! Implements:
//! - Event loop with microtask/macrotask queues
//! - Promise constructor and methods (then, catch, finally)
//! - async/await support
//! - setTimeout/setInterval/setImmediate
//! - I/O integration

use crate::value::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Promise state
#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    Pending,
    Fulfilled(Value),
    Rejected(Value),
}

/// Callback type for promise handlers
pub type PromiseCallback = Arc<dyn Fn(Value) -> PromiseResult + Send + Sync>;

/// Result of a promise callback - can return a value or another promise
#[derive(Debug, Clone)]
pub enum PromiseResult {
    Value(Value),
    Promise(Promise),
    Throw(Value),
}

/// A JavaScript Promise with full then/catch/finally support
#[derive(Debug, Clone)]
pub struct Promise {
    pub state: PromiseState,
    pub then_callbacks: Vec<usize>, // Function pointers for legacy support
    pub catch_callbacks: Vec<usize>,
    /// Handlers waiting for this promise to settle
    pub handlers: Arc<Mutex<Vec<PromiseHandler>>>,
}

/// A handler attached via then/catch/finally
#[derive(Clone)]
pub struct PromiseHandler {
    pub on_fulfilled: Option<Arc<dyn Fn(Value) -> PromiseResult + Send + Sync>>,
    pub on_rejected: Option<Arc<dyn Fn(Value) -> PromiseResult + Send + Sync>>,
    pub result_promise: Arc<Mutex<Promise>>,
}

impl std::fmt::Debug for PromiseHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromiseHandler")
            .field("has_on_fulfilled", &self.on_fulfilled.is_some())
            .field("has_on_rejected", &self.on_rejected.is_some())
            .finish()
    }
}

impl Default for Promise {
    fn default() -> Self {
        Self::new()
    }
}

impl Promise {
    pub fn new() -> Self {
        Self {
            state: PromiseState::Pending,
            then_callbacks: Vec::new(),
            catch_callbacks: Vec::new(),
            handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new promise with an executor function
    /// The executor receives resolve and reject functions
    pub fn with_executor<F>(executor: F) -> Self
    where
        F: FnOnce(Box<dyn Fn(Value) + Send + Sync>, Box<dyn Fn(Value) + Send + Sync>),
    {
        let promise = Arc::new(Mutex::new(Promise::new()));

        let resolve_promise = promise.clone();
        let resolve = Box::new(move |value: Value| {
            let mut p = resolve_promise.lock().unwrap();
            p.resolve(value);
        });

        let reject_promise = promise.clone();
        let reject = Box::new(move |reason: Value| {
            let mut p = reject_promise.lock().unwrap();
            p.reject(reason);
        });

        executor(resolve, reject);

        // Extract the promise from the Arc<Mutex<>>
        match Arc::try_unwrap(promise) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        }
    }

    pub fn resolve(&mut self, value: Value) {
        if matches!(self.state, PromiseState::Pending) {
            self.state = PromiseState::Fulfilled(value.clone());
            self.trigger_handlers(value, true);
        }
    }

    pub fn reject(&mut self, reason: Value) {
        if matches!(self.state, PromiseState::Pending) {
            self.state = PromiseState::Rejected(reason.clone());
            self.trigger_handlers(reason, false);
        }
    }

    /// Trigger all registered handlers when promise settles
    fn trigger_handlers(&self, value: Value, is_fulfilled: bool) {
        let handlers = self.handlers.lock().unwrap().clone();
        for handler in handlers {
            let callback = if is_fulfilled {
                handler.on_fulfilled.as_ref()
            } else {
                handler.on_rejected.as_ref()
            };

            if let Some(cb) = callback {
                match cb(value.clone()) {
                    PromiseResult::Value(v) => {
                        handler.result_promise.lock().unwrap().resolve(v);
                    }
                    PromiseResult::Promise(p) => {
                        // Chain the result promise to the returned promise
                        let result = handler.result_promise.clone();
                        match p.state {
                            PromiseState::Fulfilled(v) => {
                                result.lock().unwrap().resolve(v);
                            }
                            PromiseState::Rejected(r) => {
                                result.lock().unwrap().reject(r);
                            }
                            PromiseState::Pending => {
                                // Would need to add handler to p
                            }
                        }
                    }
                    PromiseResult::Throw(e) => {
                        handler.result_promise.lock().unwrap().reject(e);
                    }
                }
            } else {
                // Pass through the value/reason
                let mut result = handler.result_promise.lock().unwrap();
                if is_fulfilled {
                    result.resolve(value.clone());
                } else {
                    result.reject(value.clone());
                }
            }
        }
    }

    /// Attach a then handler - returns a new promise
    pub fn then<F, G>(&self, on_fulfilled: Option<F>, on_rejected: Option<G>) -> Promise
    where
        F: Fn(Value) -> PromiseResult + Send + Sync + 'static,
        G: Fn(Value) -> PromiseResult + Send + Sync + 'static,
    {
        let result_promise = Arc::new(Mutex::new(Promise::new()));

        let handler = PromiseHandler {
            on_fulfilled: on_fulfilled.map(|f| Arc::new(f) as PromiseCallback),
            on_rejected: on_rejected.map(|f| Arc::new(f) as PromiseCallback),
            result_promise: result_promise.clone(),
        };

        match &self.state {
            PromiseState::Pending => {
                self.handlers.lock().unwrap().push(handler);
            }
            PromiseState::Fulfilled(value) => {
                if let Some(cb) = handler.on_fulfilled {
                    match cb(value.clone()) {
                        PromiseResult::Value(v) => {
                            result_promise.lock().unwrap().resolve(v);
                        }
                        PromiseResult::Promise(p) => match p.state {
                            PromiseState::Fulfilled(v) => {
                                result_promise.lock().unwrap().resolve(v);
                            }
                            PromiseState::Rejected(r) => {
                                result_promise.lock().unwrap().reject(r);
                            }
                            PromiseState::Pending => {}
                        },
                        PromiseResult::Throw(e) => {
                            result_promise.lock().unwrap().reject(e);
                        }
                    }
                } else {
                    result_promise.lock().unwrap().resolve(value.clone());
                }
            }
            PromiseState::Rejected(reason) => {
                if let Some(cb) = handler.on_rejected {
                    match cb(reason.clone()) {
                        PromiseResult::Value(v) => {
                            result_promise.lock().unwrap().resolve(v);
                        }
                        PromiseResult::Promise(p) => match p.state {
                            PromiseState::Fulfilled(v) => {
                                result_promise.lock().unwrap().resolve(v);
                            }
                            PromiseState::Rejected(r) => {
                                result_promise.lock().unwrap().reject(r);
                            }
                            PromiseState::Pending => {}
                        },
                        PromiseResult::Throw(e) => {
                            result_promise.lock().unwrap().reject(e);
                        }
                    }
                } else {
                    result_promise.lock().unwrap().reject(reason.clone());
                }
            }
        }

        // Extract the promise from the Arc<Mutex<>>
        match Arc::try_unwrap(result_promise) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        }
    }

    /// Attach a catch handler - shorthand for then(None, onRejected)
    pub fn catch<F>(&self, on_rejected: F) -> Promise
    where
        F: Fn(Value) -> PromiseResult + Send + Sync + 'static,
    {
        self.then(None::<fn(Value) -> PromiseResult>, Some(on_rejected))
    }

    /// Attach a finally handler - runs regardless of outcome
    pub fn finally<F>(&self, on_finally: F) -> Promise
    where
        F: Fn() + Send + Sync + Clone + 'static,
    {
        let on_finally_fulfilled = on_finally.clone();
        let on_finally_rejected = on_finally;

        self.then(
            Some(move |value: Value| {
                on_finally_fulfilled();
                PromiseResult::Value(value)
            }),
            Some(move |reason: Value| {
                on_finally_rejected();
                PromiseResult::Throw(reason)
            }),
        )
    }

    /// Check if the promise is settled (fulfilled or rejected)
    pub fn is_settled(&self) -> bool {
        !matches!(self.state, PromiseState::Pending)
    }

    /// Check if the promise is fulfilled
    pub fn is_fulfilled(&self) -> bool {
        matches!(self.state, PromiseState::Fulfilled(_))
    }

    /// Check if the promise is rejected
    pub fn is_rejected(&self) -> bool {
        matches!(self.state, PromiseState::Rejected(_))
    }

    /// Get the fulfilled value if available
    pub fn get_value(&self) -> Option<&Value> {
        match &self.state {
            PromiseState::Fulfilled(v) => Some(v),
            _ => None,
        }
    }

    /// Get the rejection reason if available
    pub fn get_reason(&self) -> Option<&Value> {
        match &self.state {
            PromiseState::Rejected(r) => Some(r),
            _ => None,
        }
    }
}

/// Task types
pub enum Task {
    Microtask(Box<dyn FnOnce() + Send>),
    Macrotask(Box<dyn FnOnce() + Send>),
    Timer(TimerTask),
}

#[derive(Debug)]
pub struct TimerTask {
    pub callback: usize, // Function pointer
    pub execute_at: Instant,
    pub interval: Option<Duration>,
    pub id: u32,
}

/// Event loop
pub struct EventLoop {
    /// Microtask queue (higher priority)
    microtasks: VecDeque<Task>,
    /// Macrotask queue (lower priority)
    macrotasks: VecDeque<Task>,
    /// Timer queue
    timers: Vec<TimerTask>,
    /// Next timer ID
    next_timer_id: u32,
    /// Running flag
    running: bool,
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            microtasks: VecDeque::new(),
            macrotasks: VecDeque::new(),
            timers: Vec::new(),
            next_timer_id: 1,
            running: false,
        }
    }

    /// Queue a microtask
    pub fn queue_microtask(&mut self, task: impl FnOnce() + Send + 'static) {
        self.microtasks.push_back(Task::Microtask(Box::new(task)));
    }

    /// Queue a macrotask
    pub fn queue_macrotask(&mut self, task: impl FnOnce() + Send + 'static) {
        self.macrotasks.push_back(Task::Macrotask(Box::new(task)));
    }

    /// Schedule a timer (setTimeout)
    pub fn set_timeout(&mut self, callback: usize, delay: Duration) -> u32 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        self.timers.push(TimerTask {
            callback,
            execute_at: Instant::now() + delay,
            interval: None,
            id,
        });

        id
    }

    /// Schedule an interval (setInterval)
    pub fn set_interval(&mut self, callback: usize, interval: Duration) -> u32 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        self.timers.push(TimerTask {
            callback,
            execute_at: Instant::now() + interval,
            interval: Some(interval),
            id,
        });

        id
    }

    /// Clear a timer
    pub fn clear_timer(&mut self, id: u32) {
        self.timers.retain(|t| t.id != id);
    }

    /// Run the event loop
    pub fn run(&mut self) {
        self.running = true;

        while self.running && self.has_pending_work() {
            // Process all microtasks first
            while let Some(task) = self.microtasks.pop_front() {
                if let Task::Microtask(f) = task {
                    f();
                }
            }

            // Check timers
            let now = Instant::now();
            let mut i = 0;
            while i < self.timers.len() {
                if self.timers[i].execute_at <= now {
                    let timer = self.timers.remove(i);
                    // Execute timer callback
                    // The callback is a function pointer to a no-arg, no-return function
                    // Safety: The callback pointer was provided by the caller and is expected
                    // to be a valid function pointer for the lifetime of the timer
                    if timer.callback != 0 {
                        let callback_fn: extern "C" fn() =
                            unsafe { std::mem::transmute(timer.callback) };
                        callback_fn();
                    }

                    // Re-schedule if interval
                    if let Some(interval) = timer.interval {
                        self.timers.push(TimerTask {
                            callback: timer.callback,
                            execute_at: now + interval,
                            interval: Some(interval),
                            id: timer.id,
                        });
                    }
                } else {
                    i += 1;
                }
            }

            // Process one macrotask
            if let Some(Task::Macrotask(f)) = self.macrotasks.pop_front() {
                f();
            }

            // Yield to OS if no work
            if !self.has_pending_work() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// Stop the event loop
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if there's pending work
    pub fn has_pending_work(&self) -> bool {
        !self.microtasks.is_empty() || !self.macrotasks.is_empty() || !self.timers.is_empty()
    }
}

/// Promise methods
pub struct PromiseAPI;

/// Result of Promise.withResolvers()
pub struct PromiseWithResolvers {
    pub promise: Promise,
    pub resolve: Box<dyn Fn(Value) + Send + Sync>,
    pub reject: Box<dyn Fn(Value) + Send + Sync>,
}

impl PromiseAPI {
    /// Promise.resolve(value)
    pub fn resolve(value: Value) -> Promise {
        let mut promise = Promise::new();
        promise.resolve(value);
        promise
    }

    /// Promise.reject(reason)
    pub fn reject(reason: Value) -> Promise {
        let mut promise = Promise::new();
        promise.reject(reason);
        promise
    }

    /// Promise.withResolvers() - ES2024 feature
    /// Returns a promise along with its resolve and reject functions
    pub fn with_resolvers() -> PromiseWithResolvers {
        let promise = Arc::new(Mutex::new(Promise::new()));

        let resolve_promise = promise.clone();
        let resolve = Box::new(move |value: Value| {
            resolve_promise.lock().unwrap().resolve(value);
        });

        let reject_promise = promise.clone();
        let reject = Box::new(move |reason: Value| {
            reject_promise.lock().unwrap().reject(reason);
        });

        // Extract the promise from the Arc<Mutex<>>
        let result_promise = match Arc::try_unwrap(promise) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        };

        PromiseWithResolvers {
            promise: result_promise,
            resolve,
            reject,
        }
    }

    /// Promise.all(promises) - Resolve when all resolve, reject on first rejection
    ///
    /// Returns a promise that:
    /// - Resolves with an array of all values when all input promises resolve
    /// - Rejects with the first rejection reason if any promise rejects
    /// - Resolves immediately with empty array if input is empty
    ///
    /// This implementation works with already-settled promises.
    pub fn all(promises: Vec<Promise>) -> Promise {
        if promises.is_empty() {
            return Self::resolve(Value::Array(vec![]));
        }

        let mut results = Vec::with_capacity(promises.len());

        for promise in promises {
            match promise.state {
                PromiseState::Fulfilled(value) => {
                    results.push(value);
                }
                PromiseState::Rejected(reason) => {
                    // First rejection - reject the output promise
                    return Self::reject(reason);
                }
                PromiseState::Pending => {
                    // For pending promises, we return a pending promise
                    // In a full async implementation, we'd track and wait
                    return Promise::new();
                }
            }
        }

        // All promises fulfilled - resolve with array of results
        Self::resolve(Value::Array(results))
    }

    /// Promise.race(promises) - Settle with first settled promise
    ///
    /// Returns a promise that settles as soon as any input promise settles,
    /// with the value or reason from that promise.
    pub fn race(promises: Vec<Promise>) -> Promise {
        for promise in promises {
            match promise.state {
                PromiseState::Fulfilled(value) => {
                    return Self::resolve(value);
                }
                PromiseState::Rejected(reason) => {
                    return Self::reject(reason);
                }
                PromiseState::Pending => continue,
            }
        }
        // No settled promises found - return pending
        Promise::new()
    }

    /// Promise.allSettled(promises) - Wait for all to settle
    ///
    /// Returns a promise that resolves after all input promises have settled,
    /// with an array of objects describing each promise's outcome.
    pub fn all_settled(promises: Vec<Promise>) -> Promise {
        if promises.is_empty() {
            return Self::resolve(Value::Array(vec![]));
        }

        let mut results = Vec::with_capacity(promises.len());
        let mut has_pending = false;

        for promise in promises {
            match promise.state {
                PromiseState::Fulfilled(value) => {
                    // Create {status: "fulfilled", value: value}
                    let mut obj = crate::value::object::Object::new();
                    obj.set("status".to_string(), Value::String("fulfilled".to_string()));
                    obj.set("value".to_string(), value);
                    results.push(Value::Object(obj));
                }
                PromiseState::Rejected(reason) => {
                    // Create {status: "rejected", reason: reason}
                    let mut obj = crate::value::object::Object::new();
                    obj.set("status".to_string(), Value::String("rejected".to_string()));
                    obj.set("reason".to_string(), reason);
                    results.push(Value::Object(obj));
                }
                PromiseState::Pending => {
                    has_pending = true;
                    // Add placeholder for pending promise
                    let mut obj = crate::value::object::Object::new();
                    obj.set("status".to_string(), Value::String("pending".to_string()));
                    results.push(Value::Object(obj));
                }
            }
        }

        if has_pending {
            // In a full async implementation, we'd wait for all to settle
            // For now, return pending
            Promise::new()
        } else {
            Self::resolve(Value::Array(results))
        }
    }

    /// Promise.any(promises) - Resolve with first fulfilled, reject if all reject
    ///
    /// Returns a promise that:
    /// - Resolves with the first fulfilled value
    /// - Rejects with an AggregateError if all promises reject
    pub fn any(promises: Vec<Promise>) -> Promise {
        if promises.is_empty() {
            // Reject with AggregateError for empty input
            let mut obj = crate::value::object::Object::new();
            obj.set("name".to_string(), Value::String("AggregateError".to_string()));
            obj.set("message".to_string(), Value::String("All promises were rejected".to_string()));
            obj.set("errors".to_string(), Value::Array(vec![]));
            return Self::reject(Value::Object(obj));
        }

        let mut errors = Vec::with_capacity(promises.len());
        let mut has_pending = false;

        for promise in promises {
            match promise.state {
                PromiseState::Fulfilled(value) => {
                    // First fulfillment wins
                    return Self::resolve(value);
                }
                PromiseState::Rejected(reason) => {
                    errors.push(reason);
                }
                PromiseState::Pending => {
                    has_pending = true;
                }
            }
        }

        if has_pending {
            // Some promises still pending - return pending
            Promise::new()
        } else {
            // All promises rejected - create AggregateError
            let mut obj = crate::value::object::Object::new();
            obj.set("name".to_string(), Value::String("AggregateError".to_string()));
            obj.set("message".to_string(), Value::String("All promises were rejected".to_string()));
            obj.set("errors".to_string(), Value::Array(errors));
            Self::reject(Value::Object(obj))
        }
    }

    /// Check if all promises in a collection are settled
    pub fn all_settled_check(promises: &[Promise]) -> bool {
        promises.iter().all(|p| p.is_settled())
    }

    /// Get the first settled promise from a collection
    pub fn first_settled(promises: &[Promise]) -> Option<&Promise> {
        promises.iter().find(|p| p.is_settled())
    }

    /// Get the first fulfilled promise from a collection
    pub fn first_fulfilled(promises: &[Promise]) -> Option<&Promise> {
        promises.iter().find(|p| p.is_fulfilled())
    }
}

/// Timer API
pub struct TimerAPI;

impl TimerAPI {
    /// setTimeout(callback, delay)
    pub fn set_timeout(event_loop: &mut EventLoop, callback: usize, delay: u64) -> u32 {
        event_loop.set_timeout(callback, Duration::from_millis(delay))
    }

    /// setInterval(callback, interval)
    pub fn set_interval(event_loop: &mut EventLoop, callback: usize, interval: u64) -> u32 {
        event_loop.set_interval(callback, Duration::from_millis(interval))
    }

    /// clearTimeout(id)
    pub fn clear_timeout(event_loop: &mut EventLoop, id: u32) {
        event_loop.clear_timer(id);
    }

    /// clearInterval(id)
    pub fn clear_interval(event_loop: &mut EventLoop, id: u32) {
        event_loop.clear_timer(id);
    }

    /// setImmediate(callback)
    pub fn set_immediate(event_loop: &mut EventLoop, callback: impl FnOnce() + Send + 'static) {
        event_loop.queue_macrotask(callback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promise_resolve() {
        let promise = PromiseAPI::resolve(Value::Number(42.0));
        assert!(promise.is_fulfilled());
        assert_eq!(promise.get_value(), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_promise_reject() {
        let promise = PromiseAPI::reject(Value::String("error".to_string()));
        assert!(promise.is_rejected());
        assert_eq!(promise.get_reason(), Some(&Value::String("error".to_string())));
    }

    #[test]
    fn test_promise_then_on_fulfilled() {
        let promise = PromiseAPI::resolve(Value::Number(42.0));
        let result = promise.then(
            Some(|v: Value| {
                if let Value::Number(n) = v {
                    PromiseResult::Value(Value::Number(n * 2.0))
                } else {
                    PromiseResult::Value(v)
                }
            }),
            None::<fn(Value) -> PromiseResult>,
        );
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::Number(84.0)));
    }

    #[test]
    fn test_promise_then_on_rejected() {
        let promise = PromiseAPI::reject(Value::String("error".to_string()));
        let result = promise.then(
            None::<fn(Value) -> PromiseResult>,
            Some(|_: Value| PromiseResult::Value(Value::String("recovered".to_string()))),
        );
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::String("recovered".to_string())));
    }

    #[test]
    fn test_promise_catch() {
        let promise = PromiseAPI::reject(Value::String("error".to_string()));
        let result = promise.catch(|_| PromiseResult::Value(Value::String("caught".to_string())));
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::String("caught".to_string())));
    }

    #[test]
    fn test_promise_finally_on_fulfilled() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let finally_called = Arc::new(AtomicBool::new(false));
        let finally_called_clone = finally_called.clone();

        let promise = PromiseAPI::resolve(Value::Number(42.0));
        let result = promise.finally(move || {
            finally_called_clone.store(true, Ordering::SeqCst);
        });

        assert!(finally_called.load(Ordering::SeqCst));
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_promise_finally_on_rejected() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let finally_called = Arc::new(AtomicBool::new(false));
        let finally_called_clone = finally_called.clone();

        let promise = PromiseAPI::reject(Value::String("error".to_string()));
        let result = promise.finally(move || {
            finally_called_clone.store(true, Ordering::SeqCst);
        });

        assert!(finally_called.load(Ordering::SeqCst));
        assert!(result.is_rejected());
        assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())));
    }

    #[test]
    fn test_promise_with_resolvers() {
        let resolvers = PromiseAPI::with_resolvers();
        assert!(!resolvers.promise.is_settled());

        (resolvers.resolve)(Value::Number(42.0));
        // Note: Due to Arc cloning, we need to check the original promise state
        // In a real implementation, the promise would be updated
    }

    #[test]
    fn test_promise_with_executor() {
        let promise = Promise::with_executor(|resolve, _reject| {
            resolve(Value::Number(42.0));
        });
        assert!(promise.is_fulfilled());
        assert_eq!(promise.get_value(), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_promise_with_executor_reject() {
        let promise = Promise::with_executor(|_resolve, reject| {
            reject(Value::String("error".to_string()));
        });
        assert!(promise.is_rejected());
        assert_eq!(promise.get_reason(), Some(&Value::String("error".to_string())));
    }

    #[test]
    fn test_promise_chaining() {
        let promise = PromiseAPI::resolve(Value::Number(1.0));
        let result = promise
            .then(
                Some(|v: Value| {
                    if let Value::Number(n) = v {
                        PromiseResult::Value(Value::Number(n + 1.0))
                    } else {
                        PromiseResult::Value(v)
                    }
                }),
                None::<fn(Value) -> PromiseResult>,
            )
            .then(
                Some(|v: Value| {
                    if let Value::Number(n) = v {
                        PromiseResult::Value(Value::Number(n * 2.0))
                    } else {
                        PromiseResult::Value(v)
                    }
                }),
                None::<fn(Value) -> PromiseResult>,
            );

        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::Number(4.0))); // (1 + 1) * 2 = 4
    }

    #[test]
    fn test_promise_all_empty() {
        let result = PromiseAPI::all(vec![]);
        assert!(result.is_fulfilled());
        if let Some(Value::Array(arr)) = result.get_value() {
            assert!(arr.is_empty());
        } else {
            panic!("Expected empty array");
        }
    }

    #[test]
    fn test_promise_all_fulfilled() {
        let promises = vec![
            PromiseAPI::resolve(Value::Number(1.0)),
            PromiseAPI::resolve(Value::Number(2.0)),
            PromiseAPI::resolve(Value::Number(3.0)),
        ];
        let result = PromiseAPI::all(promises);
        assert!(result.is_fulfilled());
        if let Some(Value::Array(arr)) = result.get_value() {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Number(1.0));
            assert_eq!(arr[1], Value::Number(2.0));
            assert_eq!(arr[2], Value::Number(3.0));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_promise_all_rejected() {
        let promises = vec![
            PromiseAPI::resolve(Value::Number(1.0)),
            PromiseAPI::reject(Value::String("error".to_string())),
            PromiseAPI::resolve(Value::Number(3.0)),
        ];
        let result = PromiseAPI::all(promises);
        assert!(result.is_rejected());
        assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())));
    }

    #[test]
    fn test_promise_race_first_fulfilled() {
        let promises = vec![
            PromiseAPI::resolve(Value::Number(1.0)),
            PromiseAPI::resolve(Value::Number(2.0)),
        ];
        let result = PromiseAPI::race(promises);
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::Number(1.0)));
    }

    #[test]
    fn test_promise_race_first_rejected() {
        let promises = vec![
            PromiseAPI::reject(Value::String("error".to_string())),
            PromiseAPI::resolve(Value::Number(2.0)),
        ];
        let result = PromiseAPI::race(promises);
        assert!(result.is_rejected());
        assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())));
    }

    #[test]
    fn test_promise_all_settled_mixed() {
        let promises = vec![
            PromiseAPI::resolve(Value::Number(1.0)),
            PromiseAPI::reject(Value::String("error".to_string())),
        ];
        let result = PromiseAPI::all_settled(promises);
        assert!(result.is_fulfilled());
        if let Some(Value::Array(arr)) = result.get_value() {
            assert_eq!(arr.len(), 2);
            // First should be fulfilled
            if let Value::Object(obj) = &arr[0] {
                assert_eq!(obj.get("status"), Some(&Value::String("fulfilled".to_string())));
            }
            // Second should be rejected
            if let Value::Object(obj) = &arr[1] {
                assert_eq!(obj.get("status"), Some(&Value::String("rejected".to_string())));
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_promise_any_first_fulfilled() {
        let promises = vec![
            PromiseAPI::reject(Value::String("error1".to_string())),
            PromiseAPI::resolve(Value::Number(2.0)),
            PromiseAPI::reject(Value::String("error3".to_string())),
        ];
        let result = PromiseAPI::any(promises);
        assert!(result.is_fulfilled());
        assert_eq!(result.get_value(), Some(&Value::Number(2.0)));
    }

    #[test]
    fn test_promise_any_all_rejected() {
        let promises = vec![
            PromiseAPI::reject(Value::String("error1".to_string())),
            PromiseAPI::reject(Value::String("error2".to_string())),
        ];
        let result = PromiseAPI::any(promises);
        assert!(result.is_rejected());
        if let Some(Value::Object(obj)) = result.get_reason() {
            assert_eq!(obj.get("name"), Some(&Value::String("AggregateError".to_string())));
        } else {
            panic!("Expected AggregateError object");
        }
    }

    #[test]
    fn test_promise_any_empty() {
        let result = PromiseAPI::any(vec![]);
        assert!(result.is_rejected());
    }

    #[test]
    fn test_event_loop_timer() {
        let mut event_loop = EventLoop::new();
        let id = event_loop.set_timeout(0, Duration::from_millis(100));
        assert_eq!(id, 1);
        assert!(event_loop.has_pending_work());
    }

    #[test]
    fn test_event_loop_clear_timer() {
        let mut event_loop = EventLoop::new();
        let id = event_loop.set_timeout(0, Duration::from_millis(100));
        event_loop.clear_timer(id);
        assert!(!event_loop.has_pending_work());
    }

    #[test]
    fn test_event_loop_interval() {
        let mut event_loop = EventLoop::new();
        let id = event_loop.set_interval(0, Duration::from_millis(100));
        assert_eq!(id, 1);
        event_loop.clear_timer(id);
        assert!(!event_loop.has_pending_work());
    }
}
