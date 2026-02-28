//! JavaScript Context - Isolated execution environment
//!
//! This module provides `JsContext`, an isolated JavaScript execution context
//! that can run in parallel with other contexts without interference.
//!
//! Each context has its own:
//! - Global object
//! - Module cache
//! - Error state
//!
//! Thread Safety:
//! - Each context is designed to run in a single thread
//! - Multiple contexts can run in parallel in different threads
//! - The runtime heap is thread-local, so each thread has its own heap

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::error::{DxError, DxResult};
use crate::value::Value;

/// Unique identifier for a JavaScript context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextId(u64);

impl ContextId {
    /// Generate a new unique context ID
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        ContextId(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }

    /// Get the raw ID value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ContextId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Context({})", self.0)
    }
}

/// Configuration for a JavaScript context
#[derive(Debug, Clone, Default)]
pub struct ContextConfig {
    /// Name for debugging purposes
    pub name: Option<String>,
    /// Maximum execution time in milliseconds (0 = unlimited)
    pub timeout_ms: u64,
    /// Enable strict mode by default
    pub strict_mode: bool,
    /// Maximum heap size in bytes (0 = unlimited)
    pub max_heap_size: usize,
}

/// State of a JavaScript context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    /// Context is ready to execute code
    Ready,
    /// Context is currently executing code
    Running,
    /// Context has been terminated
    Terminated,
    /// Context encountered an error
    Error,
}

/// Result of executing code in a context
#[derive(Debug)]
pub struct ExecutionResult {
    /// The return value
    pub value: Value,
    /// Execution time
    pub duration: Duration,
    /// Whether execution was interrupted
    pub interrupted: bool,
}

/// An isolated JavaScript execution context
///
/// Each `JsContext` provides an isolated environment for executing JavaScript code.
/// Multiple contexts can run in parallel in different threads without interference.
///
/// # Thread Safety
///
/// - Each context is designed to run in a single thread
/// - The runtime heap is thread-local, so each thread has its own heap
/// - Global compiler state is protected by `Mutex`
///
/// # Example
///
/// ```no_run
/// use dx_js_runtime::runtime::context::{JsContext, ContextConfig};
///
/// let config = ContextConfig {
///     name: Some("worker-1".to_string()),
///     timeout_ms: 5000,
///     ..Default::default()
/// };
///
/// let mut ctx = JsContext::new(config);
/// let result = ctx.eval("1 + 2").unwrap();
/// ```
pub struct JsContext {
    /// Unique identifier for this context
    id: ContextId,
    /// Configuration
    config: ContextConfig,
    /// Current state
    state: ContextState,
    /// Global variables defined in this context
    globals: HashMap<String, Value>,
    /// Module cache for this context
    module_cache: HashMap<String, Value>,
    /// Last error that occurred
    last_error: Option<DxError>,
    /// Execution start time (for timeout tracking)
    execution_start: Option<Instant>,
}

impl JsContext {
    /// Create a new JavaScript context with the given configuration
    pub fn new(config: ContextConfig) -> Self {
        Self {
            id: ContextId::new(),
            config,
            state: ContextState::Ready,
            globals: HashMap::new(),
            module_cache: HashMap::new(),
            last_error: None,
            execution_start: None,
        }
    }

    /// Create a new context with default configuration
    pub fn default_context() -> Self {
        Self::new(ContextConfig::default())
    }

    /// Get the context ID
    pub fn id(&self) -> ContextId {
        self.id
    }

    /// Get the context name (if set)
    pub fn name(&self) -> Option<&str> {
        self.config.name.as_deref()
    }

    /// Get the current state
    pub fn state(&self) -> ContextState {
        self.state
    }

    /// Check if the context is ready to execute code
    pub fn is_ready(&self) -> bool {
        self.state == ContextState::Ready
    }

    /// Set a global variable in this context
    pub fn set_global(&mut self, name: impl Into<String>, value: Value) {
        self.globals.insert(name.into(), value);
    }

    /// Get a global variable from this context
    pub fn get_global(&self, name: &str) -> Option<&Value> {
        self.globals.get(name)
    }

    /// Remove a global variable from this context
    pub fn delete_global(&mut self, name: &str) -> Option<Value> {
        self.globals.remove(name)
    }

    /// Get all global variable names
    pub fn global_names(&self) -> impl Iterator<Item = &str> {
        self.globals.keys().map(|s| s.as_str())
    }

    /// Evaluate JavaScript code in this context
    ///
    /// # Arguments
    ///
    /// * `code` - JavaScript source code to evaluate
    ///
    /// # Returns
    ///
    /// The result of evaluating the code, or an error if execution failed.
    pub fn eval(&mut self, code: &str) -> DxResult<Value> {
        self.eval_with_filename(code, "<eval>")
    }

    /// Evaluate JavaScript code with a custom filename
    ///
    /// # Arguments
    ///
    /// * `code` - JavaScript source code to evaluate
    /// * `filename` - Filename for error messages and source maps
    ///
    /// # Returns
    ///
    /// The result of evaluating the code, or an error if execution failed.
    pub fn eval_with_filename(&mut self, code: &str, filename: &str) -> DxResult<Value> {
        // Check state
        if self.state == ContextState::Terminated {
            return Err(DxError::RuntimeError("Context has been terminated".to_string()));
        }

        // Set state to running
        self.state = ContextState::Running;
        self.execution_start = Some(Instant::now());
        self.last_error = None;

        // Execute the code
        let result = self.execute_internal(code, filename);

        // Update state based on result
        match &result {
            Ok(_) => {
                self.state = ContextState::Ready;
            }
            Err(e) => {
                self.state = ContextState::Error;
                // Store error message since DxError doesn't implement Clone
                self.last_error = Some(DxError::RuntimeError(e.to_string()));
            }
        }

        self.execution_start = None;
        result
    }

    /// Internal execution implementation
    fn execute_internal(&mut self, code: &str, filename: &str) -> DxResult<Value> {
        use crate::{compiler::OptLevel, Compiler, CompilerConfig};

        // Create a compiler for this execution
        let mut compiler = Compiler::new(CompilerConfig {
            type_check: false,
            optimization_level: OptLevel::Aggressive,
        })?;

        // Compile the code
        let module = compiler.compile(code, filename)?;

        // Check timeout before execution
        if let Some(start) = self.execution_start {
            if self.config.timeout_ms > 0 {
                let elapsed = start.elapsed();
                if elapsed.as_millis() as u64 >= self.config.timeout_ms {
                    return Err(DxError::RuntimeError("Execution timeout".to_string()));
                }
            }
        }

        // Execute the module
        module.execute()
    }

    /// Execute code and return detailed execution result
    pub fn execute(&mut self, code: &str) -> DxResult<ExecutionResult> {
        let start = Instant::now();
        let result = self.eval(code);
        let duration = start.elapsed();

        match result {
            Ok(value) => Ok(ExecutionResult {
                value,
                duration,
                interrupted: false,
            }),
            Err(e) => Err(e),
        }
    }

    /// Get the last error that occurred in this context
    pub fn last_error(&self) -> Option<&DxError> {
        self.last_error.as_ref()
    }

    /// Clear the last error
    pub fn clear_error(&mut self) {
        self.last_error = None;
        if self.state == ContextState::Error {
            self.state = ContextState::Ready;
        }
    }

    /// Terminate this context
    ///
    /// After termination, the context cannot be used for execution.
    pub fn terminate(&mut self) {
        self.state = ContextState::Terminated;
        self.globals.clear();
        self.module_cache.clear();
    }

    /// Reset the context to its initial state
    ///
    /// Clears all globals and module cache, but keeps the configuration.
    pub fn reset(&mut self) {
        self.state = ContextState::Ready;
        self.globals.clear();
        self.module_cache.clear();
        self.last_error = None;
        self.execution_start = None;
    }

    /// Check if execution has exceeded the timeout
    pub fn is_timed_out(&self) -> bool {
        if self.config.timeout_ms == 0 {
            return false;
        }

        if let Some(start) = self.execution_start {
            start.elapsed().as_millis() as u64 >= self.config.timeout_ms
        } else {
            false
        }
    }

    /// Get execution duration (if currently running)
    pub fn execution_duration(&self) -> Option<Duration> {
        self.execution_start.map(|start| start.elapsed())
    }
}

impl Default for JsContext {
    fn default() -> Self {
        Self::default_context()
    }
}

impl std::fmt::Debug for JsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsContext")
            .field("id", &self.id)
            .field("name", &self.config.name)
            .field("state", &self.state)
            .field("globals_count", &self.globals.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_id_uniqueness() {
        let id1 = ContextId::new();
        let id2 = ContextId::new();
        let id3 = ContextId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_context_creation() {
        let ctx = JsContext::default_context();
        assert!(ctx.is_ready());
        assert!(ctx.name().is_none());
    }

    #[test]
    fn test_context_with_name() {
        let config = ContextConfig {
            name: Some("test-context".to_string()),
            ..Default::default()
        };
        let ctx = JsContext::new(config);
        assert_eq!(ctx.name(), Some("test-context"));
    }

    #[test]
    fn test_context_globals() {
        let mut ctx = JsContext::default_context();

        ctx.set_global("x", Value::Number(42.0));
        assert_eq!(ctx.get_global("x"), Some(&Value::Number(42.0)));

        ctx.delete_global("x");
        assert_eq!(ctx.get_global("x"), None);
    }

    #[test]
    fn test_context_reset() {
        let mut ctx = JsContext::default_context();
        ctx.set_global("x", Value::Number(42.0));

        ctx.reset();

        assert!(ctx.is_ready());
        assert_eq!(ctx.get_global("x"), None);
    }

    #[test]
    fn test_context_terminate() {
        let mut ctx = JsContext::default_context();
        ctx.terminate();

        assert_eq!(ctx.state(), ContextState::Terminated);
        assert!(ctx.eval("1 + 1").is_err());
    }
}
