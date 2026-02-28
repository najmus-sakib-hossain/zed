//! Async/Await Runtime Integration
//!
//! Provides async/await runtime integration for FastAPI applications,
//! including coroutine execution and event loop compatibility.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::task::Waker;
use thiserror::Error;

/// Errors that can occur during async operations
#[derive(Debug, Error)]
pub enum AsyncError {
    #[error("Event loop not running")]
    EventLoopNotRunning,

    #[error("Coroutine cancelled")]
    Cancelled,

    #[error("Timeout exceeded")]
    Timeout,

    #[error("Task failed: {0}")]
    TaskFailed(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// State of a coroutine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoroutineState {
    /// Coroutine is pending (not started or suspended)
    Pending,
    /// Coroutine is running
    Running,
    /// Coroutine completed successfully
    Completed,
    /// Coroutine was cancelled
    Cancelled,
    /// Coroutine failed with an error
    Failed,
}

/// Result of a coroutine execution
#[derive(Debug, Clone)]
pub enum CoroutineResult {
    /// Coroutine returned a value
    Value(serde_json::Value),
    /// Coroutine raised an exception
    Exception(String),
    /// Coroutine was cancelled
    Cancelled,
}

/// A Python-compatible coroutine representation
#[derive(Debug)]
pub struct Coroutine {
    /// Unique identifier
    pub id: u64,
    /// Coroutine name (function name)
    pub name: String,
    /// Current state
    pub state: CoroutineState,
    /// Result (if completed)
    pub result: Option<CoroutineResult>,
    /// Waker for async notification
    waker: Option<Waker>,
}

impl Coroutine {
    /// Create a new coroutine
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            state: CoroutineState::Pending,
            result: None,
            waker: None,
        }
    }

    /// Check if the coroutine is done
    pub fn is_done(&self) -> bool {
        matches!(
            self.state,
            CoroutineState::Completed | CoroutineState::Cancelled | CoroutineState::Failed
        )
    }

    /// Set the result and mark as completed
    pub fn set_result(&mut self, value: serde_json::Value) {
        self.result = Some(CoroutineResult::Value(value));
        self.state = CoroutineState::Completed;
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    /// Set an exception and mark as failed
    pub fn set_exception(&mut self, message: impl Into<String>) {
        self.result = Some(CoroutineResult::Exception(message.into()));
        self.state = CoroutineState::Failed;
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    /// Cancel the coroutine
    pub fn cancel(&mut self) {
        if !self.is_done() {
            self.result = Some(CoroutineResult::Cancelled);
            self.state = CoroutineState::Cancelled;
            if let Some(waker) = self.waker.take() {
                waker.wake();
            }
        }
    }
}

/// A task in the event loop
#[derive(Debug)]
pub struct Task {
    /// Task ID
    pub id: u64,
    /// Task name
    pub name: String,
    /// Associated coroutine
    pub coroutine_id: u64,
    /// Task state
    pub state: CoroutineState,
    /// Result
    pub result: Option<CoroutineResult>,
    /// Whether the task is cancelled
    pub cancelled: bool,
}

impl Task {
    /// Create a new task
    pub fn new(id: u64, name: impl Into<String>, coroutine_id: u64) -> Self {
        Self {
            id,
            name: name.into(),
            coroutine_id,
            state: CoroutineState::Pending,
            result: None,
            cancelled: false,
        }
    }

    /// Check if the task is done
    pub fn done(&self) -> bool {
        matches!(
            self.state,
            CoroutineState::Completed | CoroutineState::Cancelled | CoroutineState::Failed
        )
    }

    /// Cancel the task
    pub fn cancel(&mut self) -> bool {
        if self.done() {
            false
        } else {
            self.cancelled = true;
            self.state = CoroutineState::Cancelled;
            self.result = Some(CoroutineResult::Cancelled);
            true
        }
    }

    /// Get the result (panics if not done)
    pub fn get_result(&self) -> Option<&CoroutineResult> {
        self.result.as_ref()
    }
}

/// Event loop state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLoopState {
    /// Loop is not running
    Stopped,
    /// Loop is running
    Running,
    /// Loop is stopping
    Stopping,
    /// Loop is closed
    Closed,
}

/// A Python-compatible event loop
pub struct EventLoop {
    /// Loop state
    state: EventLoopState,
    /// Pending tasks
    tasks: HashMap<u64, Task>,
    /// Coroutines
    coroutines: HashMap<u64, Coroutine>,
    /// Next task ID
    next_task_id: u64,
    /// Next coroutine ID
    next_coroutine_id: u64,
    /// Debug mode
    debug: bool,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new() -> Self {
        Self {
            state: EventLoopState::Stopped,
            tasks: HashMap::new(),
            coroutines: HashMap::new(),
            next_task_id: 1,
            next_coroutine_id: 1,
            debug: false,
        }
    }

    /// Check if the loop is running
    pub fn is_running(&self) -> bool {
        self.state == EventLoopState::Running
    }

    /// Check if the loop is closed
    pub fn is_closed(&self) -> bool {
        self.state == EventLoopState::Closed
    }

    /// Get debug mode
    pub fn get_debug(&self) -> bool {
        self.debug
    }

    /// Set debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Create a new coroutine
    pub fn create_coroutine(&mut self, name: impl Into<String>) -> u64 {
        let id = self.next_coroutine_id;
        self.next_coroutine_id += 1;
        let coroutine = Coroutine::new(id, name);
        self.coroutines.insert(id, coroutine);
        id
    }

    /// Create a task from a coroutine
    pub fn create_task(
        &mut self,
        coroutine_id: u64,
        name: impl Into<String>,
    ) -> Result<u64, AsyncError> {
        if !self.coroutines.contains_key(&coroutine_id) {
            return Err(AsyncError::InvalidState("Coroutine not found".to_string()));
        }

        let task_id = self.next_task_id;
        self.next_task_id += 1;
        let task = Task::new(task_id, name, coroutine_id);
        self.tasks.insert(task_id, task);
        Ok(task_id)
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: u64) -> Option<&Task> {
        self.tasks.get(&task_id)
    }

    /// Get a mutable task by ID
    pub fn get_task_mut(&mut self, task_id: u64) -> Option<&mut Task> {
        self.tasks.get_mut(&task_id)
    }

    /// Get a coroutine by ID
    pub fn get_coroutine(&self, coroutine_id: u64) -> Option<&Coroutine> {
        self.coroutines.get(&coroutine_id)
    }

    /// Get a mutable coroutine by ID
    pub fn get_coroutine_mut(&mut self, coroutine_id: u64) -> Option<&mut Coroutine> {
        self.coroutines.get_mut(&coroutine_id)
    }

    /// Complete a coroutine with a result
    pub fn complete_coroutine(
        &mut self,
        coroutine_id: u64,
        result: serde_json::Value,
    ) -> Result<(), AsyncError> {
        let coroutine = self
            .coroutines
            .get_mut(&coroutine_id)
            .ok_or_else(|| AsyncError::InvalidState("Coroutine not found".to_string()))?;

        coroutine.set_result(result.clone());

        // Update associated tasks
        for task in self.tasks.values_mut() {
            if task.coroutine_id == coroutine_id {
                task.state = CoroutineState::Completed;
                task.result = Some(CoroutineResult::Value(result.clone()));
            }
        }

        Ok(())
    }

    /// Fail a coroutine with an exception
    pub fn fail_coroutine(
        &mut self,
        coroutine_id: u64,
        message: impl Into<String>,
    ) -> Result<(), AsyncError> {
        let msg = message.into();
        let coroutine = self
            .coroutines
            .get_mut(&coroutine_id)
            .ok_or_else(|| AsyncError::InvalidState("Coroutine not found".to_string()))?;

        coroutine.set_exception(msg.clone());

        // Update associated tasks
        for task in self.tasks.values_mut() {
            if task.coroutine_id == coroutine_id {
                task.state = CoroutineState::Failed;
                task.result = Some(CoroutineResult::Exception(msg.clone()));
            }
        }

        Ok(())
    }

    /// Cancel a task
    pub fn cancel_task(&mut self, task_id: u64) -> Result<bool, AsyncError> {
        let task = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| AsyncError::InvalidState("Task not found".to_string()))?;

        let cancelled = task.cancel();

        if cancelled {
            // Also cancel the coroutine
            if let Some(coroutine) = self.coroutines.get_mut(&task.coroutine_id) {
                coroutine.cancel();
            }
        }

        Ok(cancelled)
    }

    /// Get all pending tasks
    pub fn pending_tasks(&self) -> Vec<u64> {
        self.tasks.iter().filter(|(_, t)| !t.done()).map(|(id, _)| *id).collect()
    }

    /// Get all completed tasks
    pub fn completed_tasks(&self) -> Vec<u64> {
        self.tasks.iter().filter(|(_, t)| t.done()).map(|(id, _)| *id).collect()
    }

    /// Run the event loop (simulated for testing)
    pub fn run(&mut self) -> Result<(), AsyncError> {
        if self.state == EventLoopState::Closed {
            return Err(AsyncError::InvalidState("Event loop is closed".to_string()));
        }

        self.state = EventLoopState::Running;
        Ok(())
    }

    /// Stop the event loop
    pub fn stop(&mut self) {
        if self.state == EventLoopState::Running {
            self.state = EventLoopState::Stopping;
        }
    }

    /// Close the event loop
    pub fn close(&mut self) {
        self.state = EventLoopState::Closed;
        self.tasks.clear();
        self.coroutines.clear();
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

/// Async context manager support
#[derive(Debug)]
pub struct AsyncContextManager {
    /// Name of the context manager
    pub name: String,
    /// Whether we're inside the context
    pub entered: bool,
    /// Setup coroutine ID
    pub enter_coro: Option<u64>,
    /// Cleanup coroutine ID
    pub exit_coro: Option<u64>,
}

impl AsyncContextManager {
    /// Create a new async context manager
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entered: false,
            enter_coro: None,
            exit_coro: None,
        }
    }

    /// Enter the context (async __aenter__)
    pub fn enter(&mut self, loop_: &mut EventLoop) -> u64 {
        let coro_id = loop_.create_coroutine(format!("{}.__aenter__", self.name));
        self.enter_coro = Some(coro_id);
        self.entered = true;
        coro_id
    }

    /// Exit the context (async __aexit__)
    pub fn exit(&mut self, loop_: &mut EventLoop) -> u64 {
        let coro_id = loop_.create_coroutine(format!("{}.__aexit__", self.name));
        self.exit_coro = Some(coro_id);
        self.entered = false;
        coro_id
    }
}

/// Async iterator support
#[derive(Debug)]
pub struct AsyncIterator {
    /// Iterator name
    pub name: String,
    /// Items to yield
    items: Vec<serde_json::Value>,
    /// Current index
    index: usize,
    /// Whether iteration is exhausted
    pub exhausted: bool,
}

impl AsyncIterator {
    /// Create a new async iterator
    pub fn new(name: impl Into<String>, items: Vec<serde_json::Value>) -> Self {
        Self {
            name: name.into(),
            items,
            index: 0,
            exhausted: false,
        }
    }

    /// Get the next item (async __anext__)
    pub fn next_item(&mut self) -> Option<serde_json::Value> {
        if self.index < self.items.len() {
            let item = self.items[self.index].clone();
            self.index += 1;
            Some(item)
        } else {
            self.exhausted = true;
            None
        }
    }

    /// Reset the iterator
    pub fn reset(&mut self) {
        self.index = 0;
        self.exhausted = false;
    }

    /// Get remaining count
    pub fn remaining(&self) -> usize {
        self.items.len().saturating_sub(self.index)
    }
}

/// Async generator support
#[derive(Debug)]
pub struct AsyncGenerator {
    /// Generator name
    pub name: String,
    /// Yielded values
    yielded: Vec<serde_json::Value>,
    /// Sent values
    sent: Vec<serde_json::Value>,
    /// Whether the generator is closed
    pub closed: bool,
    /// Current state
    pub state: CoroutineState,
}

impl AsyncGenerator {
    /// Create a new async generator
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            yielded: Vec::new(),
            sent: Vec::new(),
            closed: false,
            state: CoroutineState::Pending,
        }
    }

    /// Yield a value
    pub fn yield_value(&mut self, value: serde_json::Value) {
        self.yielded.push(value);
        self.state = CoroutineState::Pending;
    }

    /// Send a value to the generator
    pub fn send(&mut self, value: serde_json::Value) -> Result<(), AsyncError> {
        if self.closed {
            return Err(AsyncError::InvalidState("Generator is closed".to_string()));
        }
        self.sent.push(value);
        self.state = CoroutineState::Running;
        Ok(())
    }

    /// Get the last sent value
    pub fn get_sent(&self) -> Option<&serde_json::Value> {
        self.sent.last()
    }

    /// Get all yielded values
    pub fn get_yielded(&self) -> &[serde_json::Value] {
        &self.yielded
    }

    /// Close the generator
    pub fn close(&mut self) {
        self.closed = true;
        self.state = CoroutineState::Completed;
    }

    /// Throw an exception into the generator
    pub fn throw(&mut self, message: impl Into<String>) -> Result<(), AsyncError> {
        if self.closed {
            return Err(AsyncError::InvalidState("Generator is closed".to_string()));
        }
        self.state = CoroutineState::Failed;
        Err(AsyncError::TaskFailed(message.into()))
    }
}

/// Global event loop registry (thread-local simulation)
pub struct EventLoopRegistry {
    loops: HashMap<String, EventLoop>,
    current: Option<String>,
}

impl EventLoopRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            loops: HashMap::new(),
            current: None,
        }
    }

    /// Get or create the current event loop
    pub fn get_event_loop(&mut self) -> &mut EventLoop {
        if self.current.is_none() {
            let name = "default".to_string();
            self.loops.insert(name.clone(), EventLoop::new());
            self.current = Some(name);
        }

        let name = self.current.as_ref().unwrap();
        self.loops.get_mut(name).unwrap()
    }

    /// Create a new event loop
    pub fn new_event_loop(&mut self, name: impl Into<String>) -> &mut EventLoop {
        let name = name.into();
        self.loops.insert(name.clone(), EventLoop::new());
        self.current = Some(name.clone());
        self.loops.get_mut(&name).unwrap()
    }

    /// Set the current event loop
    pub fn set_event_loop(&mut self, name: impl Into<String>) -> Result<(), AsyncError> {
        let name = name.into();
        if self.loops.contains_key(&name) {
            self.current = Some(name);
            Ok(())
        } else {
            Err(AsyncError::InvalidState("Event loop not found".to_string()))
        }
    }

    /// Get a named event loop
    pub fn get_loop(&mut self, name: &str) -> Option<&mut EventLoop> {
        self.loops.get_mut(name)
    }
}

impl Default for EventLoopRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_coroutine_lifecycle() {
        let mut coro = Coroutine::new(1, "test_coro");

        assert_eq!(coro.state, CoroutineState::Pending);
        assert!(!coro.is_done());

        coro.set_result(json!(42));

        assert_eq!(coro.state, CoroutineState::Completed);
        assert!(coro.is_done());

        if let Some(CoroutineResult::Value(v)) = &coro.result {
            assert_eq!(*v, json!(42));
        } else {
            panic!("Expected value result");
        }
    }

    #[test]
    fn test_coroutine_exception() {
        let mut coro = Coroutine::new(1, "test_coro");

        coro.set_exception("Something went wrong");

        assert_eq!(coro.state, CoroutineState::Failed);
        assert!(coro.is_done());

        if let Some(CoroutineResult::Exception(msg)) = &coro.result {
            assert_eq!(msg, "Something went wrong");
        } else {
            panic!("Expected exception result");
        }
    }

    #[test]
    fn test_coroutine_cancel() {
        let mut coro = Coroutine::new(1, "test_coro");

        coro.cancel();

        assert_eq!(coro.state, CoroutineState::Cancelled);
        assert!(coro.is_done());
        assert!(matches!(coro.result, Some(CoroutineResult::Cancelled)));
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new(1, "test_task", 1);

        assert!(!task.done());
        assert!(!task.cancelled);

        task.cancel();

        assert!(task.done());
        assert!(task.cancelled);
    }

    #[test]
    fn test_event_loop_basic() {
        let mut loop_ = EventLoop::new();

        assert!(!loop_.is_running());
        assert!(!loop_.is_closed());

        loop_.run().unwrap();
        assert!(loop_.is_running());

        loop_.stop();
        assert_eq!(loop_.state, EventLoopState::Stopping);

        loop_.close();
        assert!(loop_.is_closed());
    }

    #[test]
    fn test_event_loop_create_task() {
        let mut loop_ = EventLoop::new();

        let coro_id = loop_.create_coroutine("my_coro");
        let task_id = loop_.create_task(coro_id, "my_task").unwrap();

        let task = loop_.get_task(task_id).unwrap();
        assert_eq!(task.name, "my_task");
        assert_eq!(task.coroutine_id, coro_id);
        assert!(!task.done());
    }

    #[test]
    fn test_event_loop_complete_coroutine() {
        let mut loop_ = EventLoop::new();

        let coro_id = loop_.create_coroutine("my_coro");
        let task_id = loop_.create_task(coro_id, "my_task").unwrap();

        loop_.complete_coroutine(coro_id, json!({"result": "success"})).unwrap();

        let coro = loop_.get_coroutine(coro_id).unwrap();
        assert!(coro.is_done());

        let task = loop_.get_task(task_id).unwrap();
        assert!(task.done());
    }

    #[test]
    fn test_event_loop_cancel_task() {
        let mut loop_ = EventLoop::new();

        let coro_id = loop_.create_coroutine("my_coro");
        let task_id = loop_.create_task(coro_id, "my_task").unwrap();

        let cancelled = loop_.cancel_task(task_id).unwrap();
        assert!(cancelled);

        let task = loop_.get_task(task_id).unwrap();
        assert!(task.done());
        assert!(task.cancelled);
    }

    #[test]
    fn test_async_context_manager() {
        let mut loop_ = EventLoop::new();
        let mut ctx = AsyncContextManager::new("my_context");

        assert!(!ctx.entered);

        let enter_coro = ctx.enter(&mut loop_);
        assert!(ctx.entered);
        assert_eq!(ctx.enter_coro, Some(enter_coro));

        let exit_coro = ctx.exit(&mut loop_);
        assert!(!ctx.entered);
        assert_eq!(ctx.exit_coro, Some(exit_coro));
    }

    #[test]
    fn test_async_iterator() {
        let items = vec![json!(1), json!(2), json!(3)];
        let mut iter = AsyncIterator::new("my_iter", items);

        assert_eq!(iter.remaining(), 3);

        assert_eq!(iter.next_item(), Some(json!(1)));
        assert_eq!(iter.next_item(), Some(json!(2)));
        assert_eq!(iter.next_item(), Some(json!(3)));
        assert_eq!(iter.next_item(), None);

        assert!(iter.exhausted);
        assert_eq!(iter.remaining(), 0);

        iter.reset();
        assert!(!iter.exhausted);
        assert_eq!(iter.remaining(), 3);
    }

    #[test]
    fn test_async_generator() {
        let mut gen = AsyncGenerator::new("my_gen");

        assert!(!gen.closed);

        gen.yield_value(json!(1));
        gen.yield_value(json!(2));

        assert_eq!(gen.get_yielded(), &[json!(1), json!(2)]);

        gen.send(json!("hello")).unwrap();
        assert_eq!(gen.get_sent(), Some(&json!("hello")));

        gen.close();
        assert!(gen.closed);

        // Can't send to closed generator
        assert!(gen.send(json!("world")).is_err());
    }

    #[test]
    fn test_event_loop_registry() {
        let mut registry = EventLoopRegistry::new();

        // Get default loop
        let loop_ = registry.get_event_loop();
        assert!(!loop_.is_running());

        // Create named loop
        let named_loop = registry.new_event_loop("custom");
        named_loop.run().unwrap();

        // Switch back to default
        registry.set_event_loop("default").unwrap();

        // Invalid loop name
        assert!(registry.set_event_loop("nonexistent").is_err());
    }

    #[test]
    fn test_pending_and_completed_tasks() {
        let mut loop_ = EventLoop::new();

        let coro1 = loop_.create_coroutine("coro1");
        let coro2 = loop_.create_coroutine("coro2");
        let coro3 = loop_.create_coroutine("coro3");

        let _task1 = loop_.create_task(coro1, "task1").unwrap();
        let task2 = loop_.create_task(coro2, "task2").unwrap();
        let _task3 = loop_.create_task(coro3, "task3").unwrap();

        // All pending initially
        assert_eq!(loop_.pending_tasks().len(), 3);
        assert_eq!(loop_.completed_tasks().len(), 0);

        // Complete one
        loop_.complete_coroutine(coro1, json!(null)).unwrap();

        assert_eq!(loop_.pending_tasks().len(), 2);
        assert_eq!(loop_.completed_tasks().len(), 1);

        // Cancel one
        loop_.cancel_task(task2).unwrap();

        assert_eq!(loop_.pending_tasks().len(), 1);
        assert_eq!(loop_.completed_tasks().len(), 2);
    }
}
