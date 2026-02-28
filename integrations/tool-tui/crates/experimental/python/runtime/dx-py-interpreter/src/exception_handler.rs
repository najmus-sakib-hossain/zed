//! Exception Handler Stack for DX-Py Interpreter
//!
//! This module implements the exception handling mechanism for Python's
//! try/except/finally blocks. It manages the exception handler stack,
//! tracks active try blocks, and handles proper stack unwinding.

use dx_py_core::pyexception::PyException;
use dx_py_core::pylist::PyValue;
use std::sync::Arc;

/// Represents a try block entry on the exception handler stack
#[derive(Debug, Clone)]
pub struct TryBlock {
    /// Bytecode offset of the except handler (if any)
    pub except_offset: Option<usize>,
    /// Bytecode offset of the finally handler (if any)
    pub finally_offset: Option<usize>,
    /// Stack depth when entering the try block
    pub stack_depth: usize,
    /// Block stack depth when entering the try block
    pub block_depth: usize,
    /// Exception type(s) to match (None means catch all)
    pub exception_types: Option<Vec<String>>,
    /// Variable name to bind the exception to (for `except E as e`)
    pub exception_binding: Option<String>,
}

impl TryBlock {
    /// Create a new try block with an except handler
    pub fn with_except(except_offset: usize, stack_depth: usize, block_depth: usize) -> Self {
        Self {
            except_offset: Some(except_offset),
            finally_offset: None,
            stack_depth,
            block_depth,
            exception_types: None,
            exception_binding: None,
        }
    }

    /// Create a new try block with a finally handler
    pub fn with_finally(finally_offset: usize, stack_depth: usize, block_depth: usize) -> Self {
        Self {
            except_offset: None,
            finally_offset: Some(finally_offset),
            stack_depth,
            block_depth,
            exception_types: None,
            exception_binding: None,
        }
    }

    /// Create a new try block with both except and finally handlers
    pub fn with_both(
        except_offset: usize,
        finally_offset: usize,
        stack_depth: usize,
        block_depth: usize,
    ) -> Self {
        Self {
            except_offset: Some(except_offset),
            finally_offset: Some(finally_offset),
            stack_depth,
            block_depth,
            exception_types: None,
            exception_binding: None,
        }
    }

    /// Set the exception types to match
    pub fn with_exception_types(mut self, types: Vec<String>) -> Self {
        self.exception_types = Some(types);
        self
    }

    /// Set the exception binding variable name
    pub fn with_binding(mut self, name: String) -> Self {
        self.exception_binding = Some(name);
        self
    }

    /// Check if this block has an except handler
    pub fn has_except(&self) -> bool {
        self.except_offset.is_some()
    }

    /// Check if this block has a finally handler
    pub fn has_finally(&self) -> bool {
        self.finally_offset.is_some()
    }
}

/// Exception handler state for the interpreter
#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    /// Stack of active try blocks
    try_blocks: Vec<TryBlock>,
    /// Current exception being handled (if any)
    current_exception: Option<Arc<PyException>>,
    /// Exception context stack (for nested exception handling)
    exception_context: Vec<Arc<PyException>>,
    /// Flag indicating if we're currently in a finally block
    in_finally: bool,
    /// Pending return value (for return inside try/finally)
    pending_return: Option<PyValue>,
    /// Pending exception to re-raise after finally
    pending_exception: Option<Arc<PyException>>,
}

impl Default for ExceptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceptionHandler {
    /// Create a new exception handler
    pub fn new() -> Self {
        Self {
            try_blocks: Vec::new(),
            current_exception: None,
            exception_context: Vec::new(),
            in_finally: false,
            pending_return: None,
            pending_exception: None,
        }
    }

    /// Push a new try block onto the stack
    pub fn push_try(&mut self, block: TryBlock) {
        self.try_blocks.push(block);
    }

    /// Pop the current try block from the stack
    pub fn pop_try(&mut self) -> Option<TryBlock> {
        self.try_blocks.pop()
    }

    /// Get the current try block (if any)
    pub fn current_try(&self) -> Option<&TryBlock> {
        self.try_blocks.last()
    }

    /// Get the number of active try blocks
    pub fn try_depth(&self) -> usize {
        self.try_blocks.len()
    }

    /// Check if we're inside any try block
    pub fn in_try_block(&self) -> bool {
        !self.try_blocks.is_empty()
    }

    /// Set the current exception being handled
    pub fn set_current_exception(&mut self, exc: Arc<PyException>) {
        // Save the previous exception as context
        if let Some(prev) = self.current_exception.take() {
            self.exception_context.push(prev);
        }
        self.current_exception = Some(exc);
    }

    /// Get the current exception being handled
    pub fn get_current_exception(&self) -> Option<&Arc<PyException>> {
        self.current_exception.as_ref()
    }

    /// Clear the current exception (after it's been handled)
    pub fn clear_current_exception(&mut self) {
        self.current_exception = None;
    }

    /// Pop the exception context (restore previous exception)
    pub fn pop_exception_context(&mut self) -> Option<Arc<PyException>> {
        let current = self.current_exception.take();
        self.current_exception = self.exception_context.pop();
        current
    }

    /// Check if we're currently handling an exception
    pub fn is_handling_exception(&self) -> bool {
        self.current_exception.is_some()
    }

    /// Set the in_finally flag
    pub fn set_in_finally(&mut self, in_finally: bool) {
        self.in_finally = in_finally;
    }

    /// Check if we're in a finally block
    pub fn is_in_finally(&self) -> bool {
        self.in_finally
    }

    /// Set a pending return value (for return inside try/finally)
    pub fn set_pending_return(&mut self, value: PyValue) {
        self.pending_return = Some(value);
    }

    /// Get and clear the pending return value
    pub fn take_pending_return(&mut self) -> Option<PyValue> {
        self.pending_return.take()
    }

    /// Check if there's a pending return
    pub fn has_pending_return(&self) -> bool {
        self.pending_return.is_some()
    }

    /// Set a pending exception to re-raise after finally
    pub fn set_pending_exception(&mut self, exc: Arc<PyException>) {
        self.pending_exception = Some(exc);
    }

    /// Get and clear the pending exception
    pub fn take_pending_exception(&mut self) -> Option<Arc<PyException>> {
        self.pending_exception.take()
    }

    /// Check if there's a pending exception
    pub fn has_pending_exception(&self) -> bool {
        self.pending_exception.is_some()
    }

    /// Find the nearest exception handler for the given exception
    /// Returns the handler offset and whether it's a finally block
    pub fn find_handler(&self, exc: &PyException) -> Option<ExceptionHandlerInfo> {
        // Search from innermost to outermost try block
        for (i, block) in self.try_blocks.iter().enumerate().rev() {
            // Check if this block has an except handler that matches
            if let Some(except_offset) = block.except_offset {
                if self.exception_matches(exc, &block.exception_types) {
                    return Some(ExceptionHandlerInfo {
                        handler_offset: except_offset,
                        is_finally: false,
                        stack_depth: block.stack_depth,
                        block_index: i,
                        binding: block.exception_binding.clone(),
                    });
                }
            }

            // If no matching except, check for finally
            if let Some(finally_offset) = block.finally_offset {
                return Some(ExceptionHandlerInfo {
                    handler_offset: finally_offset,
                    is_finally: true,
                    stack_depth: block.stack_depth,
                    block_index: i,
                    binding: None,
                });
            }
        }
        None
    }

    /// Find the nearest finally handler (for return/break/continue)
    pub fn find_finally_handler(&self) -> Option<ExceptionHandlerInfo> {
        for (i, block) in self.try_blocks.iter().enumerate().rev() {
            if let Some(finally_offset) = block.finally_offset {
                return Some(ExceptionHandlerInfo {
                    handler_offset: finally_offset,
                    is_finally: true,
                    stack_depth: block.stack_depth,
                    block_index: i,
                    binding: None,
                });
            }
        }
        None
    }

    /// Check if an exception matches the given type(s)
    fn exception_matches(&self, exc: &PyException, types: &Option<Vec<String>>) -> bool {
        match types {
            None => true, // Bare except catches all
            Some(type_list) => type_list.iter().any(|t| exc.is_instance(t)),
        }
    }

    /// Unwind the try block stack to a specific depth
    pub fn unwind_to(&mut self, depth: usize) {
        while self.try_blocks.len() > depth {
            self.try_blocks.pop();
        }
    }

    /// Clear all state (for cleanup)
    pub fn clear(&mut self) {
        self.try_blocks.clear();
        self.current_exception = None;
        self.exception_context.clear();
        self.in_finally = false;
        self.pending_return = None;
        self.pending_exception = None;
    }
}

/// Information about a found exception handler
#[derive(Debug, Clone)]
pub struct ExceptionHandlerInfo {
    /// Bytecode offset to jump to
    pub handler_offset: usize,
    /// Whether this is a finally handler (vs except)
    pub is_finally: bool,
    /// Stack depth to unwind to
    pub stack_depth: usize,
    /// Index of the try block in the stack
    pub block_index: usize,
    /// Variable name to bind the exception to (if any)
    pub binding: Option<String>,
}

/// Result of handling an exception
#[derive(Debug)]
pub enum ExceptionResult {
    /// Exception was handled, continue at the given offset
    Handled { offset: usize, binding: Option<String> },
    /// Exception needs to propagate (no handler found)
    Propagate,
    /// Need to execute finally block first
    ExecuteFinally { offset: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_block_creation() {
        let block = TryBlock::with_except(100, 5, 2);
        assert!(block.has_except());
        assert!(!block.has_finally());
        assert_eq!(block.except_offset, Some(100));
        assert_eq!(block.stack_depth, 5);
    }

    #[test]
    fn test_try_block_with_finally() {
        let block = TryBlock::with_finally(200, 3, 1);
        assert!(!block.has_except());
        assert!(block.has_finally());
        assert_eq!(block.finally_offset, Some(200));
    }

    #[test]
    fn test_try_block_with_both() {
        let block = TryBlock::with_both(100, 200, 5, 2);
        assert!(block.has_except());
        assert!(block.has_finally());
        assert_eq!(block.except_offset, Some(100));
        assert_eq!(block.finally_offset, Some(200));
    }

    #[test]
    fn test_exception_handler_push_pop() {
        let mut handler = ExceptionHandler::new();
        assert!(!handler.in_try_block());

        handler.push_try(TryBlock::with_except(100, 5, 2));
        assert!(handler.in_try_block());
        assert_eq!(handler.try_depth(), 1);

        handler.push_try(TryBlock::with_finally(200, 3, 1));
        assert_eq!(handler.try_depth(), 2);

        let block = handler.pop_try().unwrap();
        assert_eq!(block.finally_offset, Some(200));
        assert_eq!(handler.try_depth(), 1);
    }

    #[test]
    fn test_exception_handler_find_handler() {
        let mut handler = ExceptionHandler::new();

        // Push a try block with except handler
        handler.push_try(TryBlock::with_except(100, 5, 2));

        let exc = PyException::new("ValueError", "test error");
        let info = handler.find_handler(&exc).unwrap();
        assert_eq!(info.handler_offset, 100);
        assert!(!info.is_finally);
    }

    #[test]
    fn test_exception_handler_find_finally() {
        let mut handler = ExceptionHandler::new();

        // Push a try block with only finally handler
        handler.push_try(TryBlock::with_finally(200, 5, 2));

        let exc = PyException::new("ValueError", "test error");
        let info = handler.find_handler(&exc).unwrap();
        assert_eq!(info.handler_offset, 200);
        assert!(info.is_finally);
    }

    #[test]
    fn test_exception_handler_type_matching() {
        let mut handler = ExceptionHandler::new();

        // Push a try block that only catches ValueError
        let block = TryBlock::with_except(100, 5, 2)
            .with_exception_types(vec!["ValueError".to_string()]);
        handler.push_try(block);

        // ValueError should match
        let exc1 = PyException::new("ValueError", "test");
        assert!(handler.find_handler(&exc1).is_some());

        // TypeError should not match
        let exc2 = PyException::new("TypeError", "test");
        assert!(handler.find_handler(&exc2).is_none());
    }

    #[test]
    fn test_exception_handler_nested() {
        let mut handler = ExceptionHandler::new();

        // Outer try with finally
        handler.push_try(TryBlock::with_finally(100, 5, 1));

        // Inner try with except
        handler.push_try(TryBlock::with_except(200, 3, 2));

        let exc = PyException::new("ValueError", "test");

        // Should find inner except handler first
        let info = handler.find_handler(&exc).unwrap();
        assert_eq!(info.handler_offset, 200);
        assert!(!info.is_finally);
    }

    #[test]
    fn test_exception_context() {
        let mut handler = ExceptionHandler::new();

        let exc1 = Arc::new(PyException::new("ValueError", "first"));
        let exc2 = Arc::new(PyException::new("TypeError", "second"));

        handler.set_current_exception(exc1.clone());
        assert!(handler.is_handling_exception());

        handler.set_current_exception(exc2.clone());
        assert_eq!(handler.get_current_exception().unwrap().exc_type, "TypeError");

        // Pop context should restore previous exception
        handler.pop_exception_context();
        assert_eq!(handler.get_current_exception().unwrap().exc_type, "ValueError");
    }

    #[test]
    fn test_pending_return() {
        let mut handler = ExceptionHandler::new();

        assert!(!handler.has_pending_return());

        handler.set_pending_return(PyValue::Int(42));
        assert!(handler.has_pending_return());

        let value = handler.take_pending_return().unwrap();
        assert!(matches!(value, PyValue::Int(42)));
        assert!(!handler.has_pending_return());
    }
}
