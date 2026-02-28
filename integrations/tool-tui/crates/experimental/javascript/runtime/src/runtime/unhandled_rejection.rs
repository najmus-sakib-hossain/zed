//! Unhandled Promise Rejection Tracking
//!
//! This module provides tracking and reporting of unhandled promise rejections.
//! It follows the Node.js behavior where:
//! - Rejections are tracked when a promise is rejected without a handler
//! - If a handler is added before the next tick, the rejection is considered handled
//! - Unhandled rejections are reported with full context including stack traces
//!
//! Requirements: 10.6

use crate::error::{capture_stack_trace, JsErrorType, JsException, StackFrame};
use crate::value::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Global registry of pending rejections
static REJECTION_REGISTRY: OnceLock<Mutex<RejectionRegistry>> = OnceLock::new();

fn get_rejection_registry() -> &'static Mutex<RejectionRegistry> {
    REJECTION_REGISTRY.get_or_init(|| Mutex::new(RejectionRegistry::new()))
}

/// Clear the rejection registry (for testing purposes)
pub fn clear_rejection_registry() {
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.pending.clear();
}

/// A tracked promise rejection
#[derive(Debug, Clone)]
pub struct TrackedRejection {
    /// Unique ID for this rejection
    pub id: u64,
    /// The rejection reason/value
    pub reason: Value,
    /// Stack trace at the point of rejection
    pub stack_trace: Vec<StackFrame>,
    /// Source file where rejection occurred
    pub source_file: Option<String>,
    /// Line number where rejection occurred
    pub line: Option<u32>,
    /// Column number where rejection occurred
    pub column: Option<u32>,
    /// Timestamp when rejection occurred
    pub timestamp: Instant,
    /// Whether this rejection has been handled
    pub handled: bool,
    /// Promise ID (for tracking)
    pub promise_id: u64,
}

impl TrackedRejection {
    /// Create a new tracked rejection
    pub fn new(promise_id: u64, reason: Value) -> Self {
        let stack_trace = capture_stack_trace();
        let (source_file, line, column) = stack_trace.first()
            .map(|f| (Some(f.file.clone()), Some(f.line), Some(f.column)))
            .unwrap_or((None, None, None));
        
        Self {
            id: generate_rejection_id(),
            reason,
            stack_trace,
            source_file,
            line,
            column,
            timestamp: Instant::now(),
            handled: false,
            promise_id,
        }
    }
    
    /// Convert to a JsException for error reporting
    pub fn to_exception(&self) -> JsException {
        let message = format_rejection_reason(&self.reason);
        let mut exception = JsException::new(JsErrorType::Error, format!("Unhandled promise rejection: {}", message));
        exception.stack = self.stack_trace.clone();
        
        if let (Some(file), Some(line), Some(column)) = (&self.source_file, self.line, self.column) {
            exception.location = Some(crate::error::SourceLocation::new(file, line, column));
        }
        
        exception
    }
}

/// Format a rejection reason for display
fn format_rejection_reason(reason: &Value) -> String {
    match reason {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Undefined => "undefined".to_string(),
        Value::Object(obj) => {
            // Check if it's an Error object with a message
            if let Some(msg) = obj.get("message") {
                return format_rejection_reason(msg);
            }
            "[object Object]".to_string()
        }
        _ => format!("{:?}", reason),
    }
}

/// Generate a unique rejection ID
fn generate_rejection_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Registry for tracking unhandled rejections
pub struct RejectionRegistry {
    /// Pending rejections (not yet handled)
    pending: HashMap<u64, TrackedRejection>,
    /// Callback for unhandled rejection events
    on_unhandled: Option<Arc<dyn Fn(&TrackedRejection) + Send + Sync>>,
    /// Callback for handled rejection events (when a rejection is later handled)
    on_handled: Option<Arc<dyn Fn(&TrackedRejection) + Send + Sync>>,
    /// Whether to exit on unhandled rejection (like Node.js --unhandled-rejections=strict)
    exit_on_unhandled: bool,
    /// Whether to warn on unhandled rejection (default behavior)
    warn_on_unhandled: bool,
}

impl Default for RejectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RejectionRegistry {
    /// Create a new rejection registry
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            on_unhandled: None,
            on_handled: None,
            exit_on_unhandled: false,
            warn_on_unhandled: true,
        }
    }
    
    /// Track a new rejection
    pub fn track_rejection(&mut self, promise_id: u64, reason: Value) -> u64 {
        let rejection = TrackedRejection::new(promise_id, reason);
        let id = rejection.id;
        self.pending.insert(promise_id, rejection);
        id
    }
    
    /// Mark a rejection as handled
    pub fn mark_handled(&mut self, promise_id: u64) -> bool {
        if let Some(mut rejection) = self.pending.remove(&promise_id) {
            rejection.handled = true;
            
            // Trigger on_handled callback
            if let Some(callback) = &self.on_handled {
                callback(&rejection);
            }
            
            return true;
        }
        false
    }
    
    /// Check for unhandled rejections and report them
    pub fn check_unhandled(&mut self) -> Vec<TrackedRejection> {
        let mut unhandled = Vec::new();
        
        for (_, rejection) in self.pending.drain() {
            if !rejection.handled {
                // Trigger on_unhandled callback
                if let Some(callback) = &self.on_unhandled {
                    callback(&rejection);
                }
                
                // Print warning if enabled
                if self.warn_on_unhandled {
                    eprintln!("{}", format_unhandled_rejection(&rejection));
                }
                
                unhandled.push(rejection);
            }
        }
        
        unhandled
    }
    
    /// Get all pending rejections
    pub fn get_pending(&self) -> Vec<&TrackedRejection> {
        self.pending.values().collect()
    }
    
    /// Set the unhandled rejection callback
    pub fn set_on_unhandled<F>(&mut self, callback: F)
    where
        F: Fn(&TrackedRejection) + Send + Sync + 'static,
    {
        self.on_unhandled = Some(Arc::new(callback));
    }
    
    /// Set the handled rejection callback
    pub fn set_on_handled<F>(&mut self, callback: F)
    where
        F: Fn(&TrackedRejection) + Send + Sync + 'static,
    {
        self.on_handled = Some(Arc::new(callback));
    }
    
    /// Configure exit behavior on unhandled rejection
    pub fn set_exit_on_unhandled(&mut self, exit: bool) {
        self.exit_on_unhandled = exit;
    }
    
    /// Configure warning behavior on unhandled rejection
    pub fn set_warn_on_unhandled(&mut self, warn: bool) {
        self.warn_on_unhandled = warn;
    }
}

/// Format an unhandled rejection for display
fn format_unhandled_rejection(rejection: &TrackedRejection) -> String {
    let mut output = String::new();
    
    output.push_str("\x1b[31m(node:dx) UnhandledPromiseRejectionWarning:\x1b[0m ");
    output.push_str(&format_rejection_reason(&rejection.reason));
    output.push('\n');
    
    // Add location if available
    if let (Some(file), Some(line), Some(column)) = (&rejection.source_file, rejection.line, rejection.column) {
        output.push_str(&format!("    at {}:{}:{}\n", file, line, column));
    }
    
    // Add stack trace
    for frame in &rejection.stack_trace {
        output.push_str(&format!("{}\n", frame.format_v8_style()));
    }
    
    output.push_str("\x1b[33m(Use `node --trace-warnings ...` to show where the warning was created)\x1b[0m\n");
    output.push_str("\x1b[31m(node:dx) UnhandledPromiseRejectionWarning: Unhandled promise rejection. This error originated either by throwing inside of an async function without a catch block, or by rejecting a promise which was not handled with .catch(). To terminate the node process on unhandled promise rejection, use the CLI flag `--unhandled-rejections=strict`.\x1b[0m\n");
    
    output
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Track a promise rejection
/// Call this when a promise is rejected without a handler
pub fn track_rejection(promise_id: u64, reason: Value) -> u64 {
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.track_rejection(promise_id, reason)
}

/// Mark a rejection as handled
/// Call this when a rejection handler is attached to a previously rejected promise
pub fn mark_rejection_handled(promise_id: u64) -> bool {
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.mark_handled(promise_id)
}

/// Check for unhandled rejections
/// Call this at the end of each event loop tick
pub fn check_unhandled_rejections() -> Vec<TrackedRejection> {
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.check_unhandled()
}

/// Get all pending rejections
pub fn get_pending_rejections() -> Vec<TrackedRejection> {
    let registry = get_rejection_registry();
    let registry = registry.lock().unwrap();
    registry.get_pending().into_iter().cloned().collect()
}

/// Set the callback for unhandled rejection events
pub fn on_unhandled_rejection<F>(callback: F)
where
    F: Fn(&TrackedRejection) + Send + Sync + 'static,
{
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.set_on_unhandled(callback);
}

/// Set the callback for handled rejection events
pub fn on_rejection_handled<F>(callback: F)
where
    F: Fn(&TrackedRejection) + Send + Sync + 'static,
{
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.set_on_handled(callback);
}

/// Configure unhandled rejection behavior
pub fn configure_unhandled_rejections(exit_on_unhandled: bool, warn_on_unhandled: bool) {
    let registry = get_rejection_registry();
    let mut registry = registry.lock().unwrap();
    registry.set_exit_on_unhandled(exit_on_unhandled);
    registry.set_warn_on_unhandled(warn_on_unhandled);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_track_rejection() {
        let id = track_rejection(1, Value::String("test error".to_string()));
        assert!(id > 0);
        
        let pending = get_pending_rejections();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].promise_id, 1);
    }
    
    #[test]
    fn test_mark_handled() {
        track_rejection(2, Value::String("another error".to_string()));
        
        let handled = mark_rejection_handled(2);
        assert!(handled);
        
        // Should not be in pending anymore
        let pending = get_pending_rejections();
        assert!(pending.iter().all(|r| r.promise_id != 2));
    }
    
    #[test]
    fn test_check_unhandled() {
        track_rejection(3, Value::String("unhandled error".to_string()));
        
        let unhandled = check_unhandled_rejections();
        assert!(unhandled.iter().any(|r| r.promise_id == 3));
    }
}
