//! # dx-error — Binary Error Boundaries
//!
//! Isolate component failures without crashing the entire app.
//!
//! ## Features
//! - WASM panic hooks
//! - Component-level isolation
//! - Automatic retry logic
//! - Binary error reporting
//! - Safe mutex wrappers that recover from poisoning
//! - Structured error types with codes and recovery suggestions

// Clippy configuration
#![allow(clippy::collapsible_if)] // Intentional style choice for readability
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub mod safe_sync;
pub mod structured_errors;

pub use safe_sync::{
    LockError, SafeMutex, SafeMutexGuard, SafeRwLock, SafeRwLockReadGuard, SafeRwLockWriteGuard,
};
pub use structured_errors::{AuthErrorCode, DxError, RecoveryConfig};

/// Binary protocol opcodes for error handling
pub mod opcodes {
    pub const ERROR_BOUNDARY: u8 = 0xB0;
    pub const ERROR_RECOVER: u8 = 0xB1;
    pub const ERROR_REPORT: u8 = 0xB2;
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

/// Component error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentError {
    pub component_id: u16,
    pub component_name: Option<String>,
    pub error_code: u16,
    pub severity: ErrorSeverity,
    pub message: String,
    pub timestamp: i64,
    pub retry_count: u8,
    pub stack_trace: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
}

/// Error boundary state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryState {
    Normal,
    Failed,
    Recovering,
}

/// Error boundary (tracks component failures)
/// Uses SafeMutex to recover gracefully from poisoning instead of panicking.
#[derive(Clone)]
pub struct ErrorBoundary {
    #[allow(dead_code)] // Stored for debugging and future use
    component_id: u16,
    state: Arc<SafeMutex<BoundaryState>>,
    error: Arc<SafeMutex<Option<ComponentError>>>,
    max_retries: u8,
    retry_count: Arc<SafeMutex<u8>>,
}

impl ErrorBoundary {
    /// Create new error boundary
    pub fn new(component_id: u16, max_retries: u8) -> Self {
        Self {
            component_id,
            state: Arc::new(SafeMutex::new(BoundaryState::Normal)),
            error: Arc::new(SafeMutex::new(None)),
            max_retries,
            retry_count: Arc::new(SafeMutex::new(0)),
        }
    }

    /// Catch error and update state.
    /// Returns Ok(()) on success, or Err if lock acquisition fails.
    pub fn catch_error(&self, error: ComponentError) -> Result<(), LockError> {
        let mut state = self.state.lock()?;
        *state = BoundaryState::Failed;

        let mut error_slot = self.error.lock()?;
        *error_slot = Some(error);
        Ok(())
    }

    /// Get current state.
    /// Returns the state, defaulting to Normal if lock fails.
    pub fn get_state(&self) -> BoundaryState {
        self.state.lock().map(|guard| guard.clone()).unwrap_or(BoundaryState::Normal)
    }

    /// Get current error.
    /// Returns None if lock fails.
    pub fn get_error(&self) -> Option<ComponentError> {
        self.error.lock().ok().and_then(|guard| guard.clone())
    }

    /// Attempt recovery.
    /// Returns true if recovery was initiated, false if max retries exceeded or lock failed.
    pub fn recover(&self) -> bool {
        let mut retry_count = match self.retry_count.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        if *retry_count >= self.max_retries {
            return false;
        }

        *retry_count += 1;

        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        *state = BoundaryState::Recovering;

        true
    }

    /// Reset boundary on successful recovery.
    /// Returns Ok(()) on success, or Err if lock acquisition fails.
    pub fn reset(&self) -> Result<(), LockError> {
        let mut state = self.state.lock()?;
        *state = BoundaryState::Normal;

        let mut error = self.error.lock()?;
        *error = None;

        let mut retry_count = self.retry_count.lock()?;
        *retry_count = 0;
        Ok(())
    }

    /// Check if boundary has failed.
    /// Returns false if lock fails (conservative default).
    pub fn has_failed(&self) -> bool {
        self.state.lock().map(|guard| *guard == BoundaryState::Failed).unwrap_or(false)
    }
}

/// Global error boundary registry
/// Uses SafeMutex to recover gracefully from poisoning.
pub struct ErrorBoundaryRegistry {
    boundaries: Arc<SafeMutex<HashMap<u16, ErrorBoundary>>>,
}

impl ErrorBoundaryRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            boundaries: Arc::new(SafeMutex::new(HashMap::new())),
        }
    }

    /// Register error boundary.
    /// Returns Ok(()) on success, or Err if lock acquisition fails.
    pub fn register(&self, component_id: u16, max_retries: u8) -> Result<(), LockError> {
        let mut boundaries = self.boundaries.lock()?;
        boundaries.insert(component_id, ErrorBoundary::new(component_id, max_retries));
        Ok(())
    }

    /// Get error boundary.
    /// Returns None if lock fails or boundary not found.
    pub fn get(&self, component_id: u16) -> Option<ErrorBoundary> {
        self.boundaries
            .lock()
            .ok()
            .and_then(|boundaries| boundaries.get(&component_id).cloned())
    }

    /// Report error for component.
    /// Silently fails if lock acquisition fails (error is logged by SafeMutex).
    pub fn report_error(&self, error: ComponentError) {
        if let Ok(boundaries) = self.boundaries.lock() {
            if let Some(boundary) = boundaries.get(&error.component_id) {
                let _ = boundary.catch_error(error);
            }
        }
    }

    /// Recover component.
    /// Returns false if lock fails or boundary not found.
    pub fn recover(&self, component_id: u16) -> bool {
        self.boundaries
            .lock()
            .ok()
            .and_then(|boundaries| boundaries.get(&component_id).map(|b| b.recover()))
            .unwrap_or(false)
    }
}

impl Default for ErrorBoundaryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Install panic hook for WASM
#[cfg(target_arch = "wasm32")]
pub fn install_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Binary error encoding
pub mod binary {
    use super::*;

    /// Encode error boundary message
    pub fn encode_error_boundary(component_id: u16, error_code: u16) -> Vec<u8> {
        let mut buf = Vec::with_capacity(5);
        buf.push(opcodes::ERROR_BOUNDARY);
        buf.extend_from_slice(&component_id.to_le_bytes());
        buf.extend_from_slice(&error_code.to_le_bytes());
        buf
    }

    /// Encode recovery message
    pub fn encode_recover(component_id: u16) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3);
        buf.push(opcodes::ERROR_RECOVER);
        buf.extend_from_slice(&component_id.to_le_bytes());
        buf
    }

    /// Encode error report
    pub fn encode_report(error: &ComponentError) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(opcodes::ERROR_REPORT);

        // Serialize error to JSON (compact, no bincode dependency)
        let error_bytes = serde_json::to_vec(error).unwrap_or_default();
        buf.extend_from_slice(&(error_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&error_bytes);

        buf
    }

    /// Decode error report
    pub fn decode_report(data: &[u8]) -> Option<ComponentError> {
        if data.len() < 5 {
            return None;
        }

        let len = u32::from_le_bytes(data[1..5].try_into().ok()?) as usize;
        if data.len() < 5 + len {
            return None;
        }

        serde_json::from_slice(&data[5..5 + len]).ok()
    }
}

/// Fallback UI configuration
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    pub show_error_details: bool,
    pub show_retry_button: bool,
    pub custom_message: Option<String>,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            show_error_details: cfg!(debug_assertions),
            show_retry_button: true,
            custom_message: None,
        }
    }
}

impl ComponentError {
    /// Create a new component error with basic information
    pub fn new(
        component_id: u16,
        error_code: u16,
        severity: ErrorSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            component_id,
            component_name: None,
            error_code,
            severity,
            message: message.into(),
            timestamp: 0, // Should be set by caller
            retry_count: 0,
            stack_trace: None,
            file_path: None,
            line_number: None,
        }
    }

    /// Create a component error with component name
    pub fn with_component_name(
        component_id: u16,
        component_name: impl Into<String>,
        error_code: u16,
        severity: ErrorSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            component_id,
            component_name: Some(component_name.into()),
            error_code,
            severity,
            message: message.into(),
            timestamp: 0,
            retry_count: 0,
            stack_trace: None,
            file_path: None,
            line_number: None,
        }
    }

    /// Add stack trace to the error
    pub fn with_stack_trace(mut self, trace: impl Into<String>) -> Self {
        self.stack_trace = Some(trace.into());
        self
    }

    /// Add file location to the error
    pub fn with_location(mut self, file_path: impl Into<String>, line_number: u32) -> Self {
        self.file_path = Some(file_path.into());
        self.line_number = Some(line_number);
        self
    }

    /// Set the timestamp
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Format the error for display
    pub fn format_detailed(&self) -> String {
        let mut output = String::new();

        // Error header with component info
        if let Some(ref name) = self.component_name {
            output.push_str(&format!(
                "error[E{:04}]: {} in component '{}'\n",
                self.error_code, self.message, name
            ));
        } else {
            output.push_str(&format!(
                "error[E{:04}]: {} (component_id: {})\n",
                self.error_code, self.message, self.component_id
            ));
        }

        // Location info
        if let (Some(file), Some(line)) = (&self.file_path, self.line_number) {
            output.push_str(&format!("  --> {}:{}\n", file, line));
        }

        // Severity
        output.push_str(&format!("  severity: {:?}\n", self.severity));

        // Stack trace
        if let Some(ref trace) = self.stack_trace {
            output.push_str("\nStack trace:\n");
            for line in trace.lines() {
                output.push_str(&format!("    {}\n", line));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_boundary() {
        let boundary = ErrorBoundary::new(1, 3);

        assert_eq!(boundary.get_state(), BoundaryState::Normal);
        assert!(!boundary.has_failed());

        let error = ComponentError::with_component_name(
            1,
            "TestComponent",
            500,
            ErrorSeverity::Error,
            "Test error",
        )
        .with_timestamp(12345);

        boundary.catch_error(error.clone()).unwrap();

        assert_eq!(boundary.get_state(), BoundaryState::Failed);
        assert!(boundary.has_failed());

        let caught_error = boundary.get_error().unwrap();
        assert_eq!(caught_error.error_code, 500);
        assert_eq!(caught_error.component_name, Some("TestComponent".to_string()));
    }

    #[test]
    fn test_error_recovery() {
        let boundary = ErrorBoundary::new(1, 3);

        let error =
            ComponentError::new(1, 500, ErrorSeverity::Error, "Test error").with_timestamp(12345);

        boundary.catch_error(error).unwrap();

        assert!(boundary.recover());
        assert_eq!(boundary.get_state(), BoundaryState::Recovering);

        boundary.reset().unwrap();
        assert_eq!(boundary.get_state(), BoundaryState::Normal);
    }

    #[test]
    fn test_max_retries() {
        let boundary = ErrorBoundary::new(1, 2);

        let error =
            ComponentError::new(1, 500, ErrorSeverity::Error, "Test error").with_timestamp(12345);

        boundary.catch_error(error.clone()).unwrap();

        assert!(boundary.recover()); // Retry 1
        boundary.catch_error(error.clone()).unwrap();
        assert!(boundary.recover()); // Retry 2
        boundary.catch_error(error).unwrap();
        assert!(!boundary.recover()); // Max retries exceeded
    }

    #[test]
    fn test_registry() {
        let registry = ErrorBoundaryRegistry::new();

        registry.register(1, 3).unwrap();
        registry.register(2, 5).unwrap();

        let error = ComponentError::with_component_name(
            1,
            "MyComponent",
            404,
            ErrorSeverity::Warning,
            "Not found",
        )
        .with_timestamp(99999);

        registry.report_error(error);

        let boundary = registry.get(1).unwrap();
        assert!(boundary.has_failed());
    }

    #[test]
    fn test_binary_encoding() {
        let error = ComponentError::with_component_name(
            42,
            "FatalComponent",
            500,
            ErrorSeverity::Critical,
            "Fatal error",
        )
        .with_timestamp(123456789)
        .with_stack_trace("at FatalComponent.render\nat App.render");

        let encoded = binary::encode_report(&error);
        assert_eq!(encoded[0], opcodes::ERROR_REPORT);

        let decoded = binary::decode_report(&encoded).unwrap();
        assert_eq!(decoded.component_id, 42);
        assert_eq!(decoded.error_code, 500);
        assert_eq!(decoded.message, "Fatal error");
        assert_eq!(decoded.component_name, Some("FatalComponent".to_string()));
        assert!(decoded.stack_trace.is_some());
    }

    #[test]
    fn test_error_format_detailed() {
        let error = ComponentError::with_component_name(
            1,
            "Counter",
            100,
            ErrorSeverity::Error,
            "State update failed",
        )
        .with_location("src/Counter.tsx", 42)
        .with_stack_trace("at Counter.increment\nat onClick");

        let formatted = error.format_detailed();
        assert!(formatted.contains("Counter"));
        assert!(formatted.contains("State update failed"));
        assert!(formatted.contains("src/Counter.tsx:42"));
        assert!(formatted.contains("Stack trace:"));
        assert!(formatted.contains("increment"));
    }

    #[test]
    fn test_component_error_builder() {
        let error = ComponentError::new(1, 500, ErrorSeverity::Error, "Test")
            .with_timestamp(12345)
            .with_stack_trace("trace")
            .with_location("file.tsx", 10);

        assert_eq!(error.timestamp, 12345);
        assert_eq!(error.stack_trace, Some("trace".to_string()));
        assert_eq!(error.file_path, Some("file.tsx".to_string()));
        assert_eq!(error.line_number, Some(10));
    }
}
