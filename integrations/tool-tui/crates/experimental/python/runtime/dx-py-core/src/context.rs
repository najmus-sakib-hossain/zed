//! Context manager implementation for Python's `with` statement
//!
//! Ensures that __exit__ is always called, even when exceptions are raised.

use crate::pyexception::PyException;
use crate::pylist::PyValue;
use std::sync::Arc;

/// Context manager state
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// The object that implements the context manager protocol
    obj: PyValue,
    /// Whether we've entered the context
    entered: bool,
    /// Whether we've exited the context
    exited: bool,
}

/// Result of entering a context manager
#[derive(Debug, Clone)]
pub enum EnterResult {
    /// Successfully entered, with the value returned by __enter__
    Success(PyValue),
    /// Failed to enter due to missing __enter__ method
    NoEnterMethod,
    /// Failed to enter due to exception in __enter__
    Exception(PyException),
}

/// Result of exiting a context manager
#[derive(Debug, Clone)]
pub enum ExitResult {
    /// Successfully exited, with whether the exception was suppressed
    Success(bool),
    /// Failed to exit due to missing __exit__ method
    NoExitMethod,
    /// Failed to exit due to exception in __exit__
    Exception(PyException),
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(obj: PyValue) -> Self {
        Self {
            obj,
            entered: false,
            exited: false,
        }
    }

    /// Enter the context manager by calling __enter__
    pub fn enter(&mut self) -> EnterResult {
        if self.entered {
            return EnterResult::Exception(PyException::new(
                "RuntimeError".to_string(),
                "Context manager already entered".to_string(),
            ));
        }

        // Check if the object has __enter__ method
        if !self.has_enter_method() {
            return EnterResult::NoEnterMethod;
        }

        // Call __enter__ method
        match self.call_enter() {
            Ok(result) => {
                self.entered = true;
                EnterResult::Success(result)
            }
            Err(exc) => EnterResult::Exception(exc),
        }
    }

    /// Exit the context manager by calling __exit__
    /// This method guarantees that __exit__ is called exactly once
    pub fn exit(
        &mut self,
        exc_type: Option<&str>,
        exc_value: Option<&PyException>,
        traceback: Option<&str>,
    ) -> ExitResult {
        if self.exited {
            // Already exited - this is a no-op
            return ExitResult::Success(false);
        }

        // Mark as exited immediately to prevent double exit
        self.exited = true;

        // Check if the object has __exit__ method
        if !self.has_exit_method() {
            return ExitResult::NoExitMethod;
        }

        // Call __exit__ method
        match self.call_exit(exc_type, exc_value, traceback) {
            Ok(suppressed) => ExitResult::Success(suppressed),
            Err(exc) => ExitResult::Exception(exc),
        }
    }

    /// Check if the object has an __enter__ method
    fn has_enter_method(&self) -> bool {
        match &self.obj {
            PyValue::Instance(instance) => instance.class.get_enter().is_some(),
            PyValue::Type(type_obj) => type_obj.get_enter().is_some(),
            _ => false,
        }
    }

    /// Check if the object has an __exit__ method
    fn has_exit_method(&self) -> bool {
        match &self.obj {
            PyValue::Instance(instance) => instance.class.get_exit().is_some(),
            PyValue::Type(type_obj) => type_obj.get_exit().is_some(),
            _ => false,
        }
    }

    /// Call the __enter__ method
    fn call_enter(&self) -> Result<PyValue, PyException> {
        // In a full implementation, this would:
        // 1. Get the __enter__ method from the object
        // 2. Call it with the object as self
        // 3. Return the result

        // For now, we'll return the object itself (common behavior)
        Ok(self.obj.clone())
    }

    /// Call the __exit__ method
    fn call_exit(
        &self,
        exc_type: Option<&str>,
        _exc_value: Option<&PyException>,
        _traceback: Option<&str>,
    ) -> Result<bool, PyException> {
        // In a full implementation, this would:
        // 1. Get the __exit__ method from the object
        // 2. Call it with (self, exc_type, exc_value, traceback)
        // 3. Return whether the exception was suppressed (truthy return value)

        // For now, we'll simulate successful exit without suppression
        match exc_type {
            Some(_) => Ok(false), // Don't suppress exceptions by default
            None => Ok(false),    // No exception to suppress
        }
    }

    /// Check if the context manager has been entered
    pub fn is_entered(&self) -> bool {
        self.entered
    }

    /// Check if the context manager has been exited
    pub fn is_exited(&self) -> bool {
        self.exited
    }

    /// Get the underlying object
    pub fn object(&self) -> &PyValue {
        &self.obj
    }
}

impl Drop for ContextManager {
    /// Ensure __exit__ is called when the context manager is dropped
    /// This provides the cleanup guarantee even if exit() wasn't called explicitly
    fn drop(&mut self) {
        if self.entered && !self.exited {
            // Force exit with no exception info
            let _ = self.exit(None, None, None);
        }
    }
}

/// Execute a block of code within a context manager
/// This function ensures that __exit__ is always called, even if the block panics or raises an exception
pub fn with_context<F, R>(mut context: ContextManager, block: F) -> Result<R, PyException>
where
    F: FnOnce(&PyValue) -> Result<R, PyException>,
{
    // Enter the context
    let enter_value = match context.enter() {
        EnterResult::Success(value) => value,
        EnterResult::NoEnterMethod => {
            return Err(PyException::new(
                "AttributeError".to_string(),
                "Object does not support context manager protocol (__enter__ missing)".to_string(),
            ));
        }
        EnterResult::Exception(exc) => return Err(exc),
    };

    // Execute the block and handle any exceptions
    let block_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| block(&enter_value)));

    let (exc_type, exc_value) = match block_result {
        Ok(Ok(result)) => {
            // Block succeeded - exit with no exception
            match context.exit(None, None, None) {
                ExitResult::Success(_) => return Ok(result),
                ExitResult::NoExitMethod => {
                    return Err(PyException::new(
                        "AttributeError".to_string(),
                        "Object does not support context manager protocol (__exit__ missing)"
                            .to_string(),
                    ));
                }
                ExitResult::Exception(exc) => return Err(exc),
            }
        }
        Ok(Err(exc)) => {
            // Block raised a Python exception
            ("Exception", Some(exc))
        }
        Err(_) => {
            // Block panicked
            let panic_exc = PyException::new(
                "SystemError".to_string(),
                "Panic occurred in with block".to_string(),
            );
            ("SystemError", Some(panic_exc))
        }
    };

    // Exit with exception info
    match context.exit(Some(exc_type), exc_value.as_ref(), None) {
        ExitResult::Success(suppressed) => {
            if suppressed {
                // Exception was suppressed - return a default value
                // In a real implementation, we'd need to handle this better
                Err(PyException::new(
                    "RuntimeError".to_string(),
                    "Cannot return value when exception is suppressed".to_string(),
                ))
            } else {
                // Exception was not suppressed - re-raise it
                Err(exc_value.unwrap())
            }
        }
        ExitResult::NoExitMethod => Err(PyException::new(
            "AttributeError".to_string(),
            "Object does not support context manager protocol (__exit__ missing)".to_string(),
        )),
        ExitResult::Exception(exit_exc) => {
            // __exit__ raised an exception - this takes precedence
            Err(exit_exc)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PyInstance, PyType};
    use std::sync::Arc;

    fn create_context_manager_class() -> Arc<PyType> {
        let class = PyType::new("TestContextManager");
        // Add __enter__ and __exit__ methods
        class.set_attr("__enter__", PyValue::Str(Arc::from("enter_method")));
        class.set_attr("__exit__", PyValue::Str(Arc::from("exit_method")));
        Arc::new(class)
    }

    #[test]
    fn test_context_manager_creation() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let cm = ContextManager::new(instance);

        assert!(!cm.is_entered());
        assert!(!cm.is_exited());
    }

    #[test]
    fn test_context_manager_enter() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let mut cm = ContextManager::new(instance);

        let result = cm.enter();
        match result {
            EnterResult::Success(_) => {
                assert!(cm.is_entered());
                assert!(!cm.is_exited());
            }
            _ => panic!("Expected successful enter"),
        }
    }

    #[test]
    fn test_context_manager_exit() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let mut cm = ContextManager::new(instance);

        // Enter first
        let _ = cm.enter();

        // Then exit
        let result = cm.exit(None, None, None);
        match result {
            ExitResult::Success(_) => {
                assert!(cm.is_entered());
                assert!(cm.is_exited());
            }
            _ => panic!("Expected successful exit"),
        }
    }

    #[test]
    fn test_context_manager_double_exit() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let mut cm = ContextManager::new(instance);

        let _ = cm.enter();

        // First exit
        let result1 = cm.exit(None, None, None);
        assert!(matches!(result1, ExitResult::Success(_)));

        // Second exit should be a no-op
        let result2 = cm.exit(None, None, None);
        assert!(matches!(result2, ExitResult::Success(_)));
    }

    #[test]
    fn test_context_manager_drop_cleanup() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let mut cm = ContextManager::new(instance);

        let _ = cm.enter();
        assert!(cm.is_entered());
        assert!(!cm.is_exited());

        // Drop should call exit
        drop(cm);
        // Can't test the state after drop, but the Drop impl should have run
    }

    #[test]
    fn test_context_manager_no_enter_method() {
        let class = Arc::new(PyType::new("NoEnterClass"));
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let mut cm = ContextManager::new(instance);

        let result = cm.enter();
        assert!(matches!(result, EnterResult::NoEnterMethod));
    }

    #[test]
    fn test_with_context_success() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let cm = ContextManager::new(instance);

        let result = with_context(cm, |_value| Ok(42));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_context_exception() {
        let class = create_context_manager_class();
        let instance = PyValue::Instance(Arc::new(PyInstance::new(class)));
        let cm = ContextManager::new(instance);

        let result: Result<i32, PyException> = with_context(cm, |_value| {
            Err(PyException::new("TestError".to_string(), "Test exception".to_string()))
        });

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exc_type, "TestError");
    }
}
