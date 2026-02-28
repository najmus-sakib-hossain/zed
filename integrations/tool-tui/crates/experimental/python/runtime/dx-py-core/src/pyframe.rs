//! PyFrame - Python stack frame

use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pyfunction::PyFunction;
use crate::pylist::{PyCell, PyValue};
use crate::PyDict;
use std::sync::Arc;

/// Stack frame for function execution
pub struct PyFrame {
    /// Object header
    pub header: PyObjectHeader,
    /// Function being executed
    pub function: Arc<PyFunction>,
    /// Instruction pointer (bytecode offset)
    pub ip: usize,
    /// Local variables
    pub locals: Vec<PyValue>,
    /// Operand stack
    pub stack: Vec<PyValue>,
    /// Block stack (for loops, try/except)
    pub block_stack: Vec<Block>,
    /// Previous frame (caller)
    pub back: Option<Arc<PyFrame>>,
    /// Local namespace (for exec/eval)
    pub local_ns: Option<Arc<PyDict>>,
    /// Line number (for debugging)
    pub lineno: u32,
    /// Cell variables (variables captured by nested functions)
    /// These are created when the function is called and may be shared with closures
    pub cells: Vec<Arc<PyCell>>,
    /// Free variables (variables captured from enclosing scope)
    /// These are passed in from the closure tuple when the function is called
    pub freevars: Vec<Arc<PyCell>>,
}

/// Block type for control flow
#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Loop,
    Except,
    Finally,
    With,
}

/// Block stack entry
#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub block_type: BlockType,
    pub handler: usize, // Target IP for break/exception
    pub level: usize,   // Stack level when block was entered
}

impl PyFrame {
    /// Create a new frame for a function call
    pub fn new(function: Arc<PyFunction>, back: Option<Arc<PyFrame>>) -> Self {
        let num_locals = function.code.num_locals as usize;
        let stack_size = function.code.stack_size as usize;

        Self {
            header: PyObjectHeader::new(TypeTag::Frame, ObjectFlags::NONE),
            function,
            ip: 0,
            locals: vec![PyValue::None; num_locals],
            stack: Vec::with_capacity(stack_size),
            block_stack: Vec::new(),
            back,
            local_ns: None,
            lineno: 0,
            cells: Vec::new(),
            freevars: Vec::new(),
        }
    }

    /// Create a new frame with cell and free variables
    ///
    /// # Arguments
    /// * `function` - The function being executed
    /// * `back` - The previous frame (caller)
    /// * `num_cells` - Number of cell variables to create
    /// * `freevars` - Free variables passed from the closure
    pub fn with_cells(
        function: Arc<PyFunction>,
        back: Option<Arc<PyFrame>>,
        num_cells: usize,
        freevars: Vec<Arc<PyCell>>,
    ) -> Self {
        let num_locals = function.code.num_locals as usize;
        let stack_size = function.code.stack_size as usize;

        // Create empty cells for cell variables
        let cells: Vec<Arc<PyCell>> = (0..num_cells).map(|_| Arc::new(PyCell::empty())).collect();

        Self {
            header: PyObjectHeader::new(TypeTag::Frame, ObjectFlags::NONE),
            function,
            ip: 0,
            locals: vec![PyValue::None; num_locals],
            stack: Vec::with_capacity(stack_size),
            block_stack: Vec::new(),
            back,
            local_ns: None,
            lineno: 0,
            cells,
            freevars,
        }
    }

    /// Get a cell variable by index
    ///
    /// Cell variables are indexed first, followed by free variables.
    /// Index 0..num_cells refers to cell variables.
    /// Index num_cells..num_cells+num_freevars refers to free variables.
    #[inline]
    pub fn get_cell(&self, index: usize) -> Option<&Arc<PyCell>> {
        let num_cells = self.cells.len();
        if index < num_cells {
            self.cells.get(index)
        } else {
            self.freevars.get(index - num_cells)
        }
    }

    /// Get the value from a cell variable
    ///
    /// Returns the value stored in the cell at the given index.
    /// Cell variables are indexed first, followed by free variables.
    #[inline]
    pub fn get_deref(&self, index: usize) -> PyValue {
        if let Some(cell) = self.get_cell(index) {
            cell.get()
        } else {
            PyValue::None
        }
    }

    /// Set the value in a cell variable
    ///
    /// Stores the value in the cell at the given index.
    /// Cell variables are indexed first, followed by free variables.
    #[inline]
    pub fn set_deref(&self, index: usize, value: PyValue) {
        if let Some(cell) = self.get_cell(index) {
            cell.set(value);
        }
    }

    /// Initialize a cell variable with a value from locals
    ///
    /// This is used when a local variable is also a cell variable.
    /// The value is copied from locals to the cell.
    pub fn init_cell_from_local(&mut self, cell_index: usize, local_index: usize) {
        if let Some(cell) = self.cells.get(cell_index) {
            let value = self.locals.get(local_index).cloned().unwrap_or(PyValue::None);
            cell.set(value);
        }
    }

    /// Get all cells (both cell vars and free vars) for creating a closure
    pub fn get_closure_cells(&self) -> Vec<Arc<PyCell>> {
        let mut closure = self.cells.clone();
        closure.extend(self.freevars.iter().cloned());
        closure
    }

    /// Push a value onto the stack
    #[inline]
    pub fn push(&mut self, value: PyValue) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    #[inline]
    pub fn pop(&mut self) -> PyValue {
        self.stack.pop().unwrap_or(PyValue::None)
    }

    /// Peek at the top of the stack
    #[inline]
    pub fn peek(&self) -> &PyValue {
        self.stack.last().unwrap_or(&PyValue::None)
    }

    /// Peek at a value n positions from the top
    #[inline]
    pub fn peek_n(&self, n: usize) -> &PyValue {
        self.stack.get(self.stack.len().saturating_sub(n + 1)).unwrap_or(&PyValue::None)
    }

    /// Get a local variable
    #[inline]
    pub fn get_local(&self, index: usize) -> &PyValue {
        self.locals.get(index).unwrap_or(&PyValue::None)
    }

    /// Set a local variable
    #[inline]
    pub fn set_local(&mut self, index: usize, value: PyValue) {
        if index < self.locals.len() {
            self.locals[index] = value;
        }
    }

    /// Push a block onto the block stack
    pub fn push_block(&mut self, block_type: BlockType, handler: usize) {
        self.block_stack.push(Block {
            block_type,
            handler,
            level: self.stack.len(),
        });
    }

    /// Pop a block from the block stack
    pub fn pop_block(&mut self) -> Option<Block> {
        self.block_stack.pop()
    }

    /// Unwind the stack to a block level
    pub fn unwind_to(&mut self, level: usize) {
        while self.stack.len() > level {
            self.stack.pop();
        }
    }

    /// Get the current block
    pub fn current_block(&self) -> Option<&Block> {
        self.block_stack.last()
    }

    /// Find a handler for an exception
    pub fn find_exception_handler(&self) -> Option<&Block> {
        self.block_stack
            .iter()
            .rev()
            .find(|b| matches!(b.block_type, BlockType::Except | BlockType::Finally))
    }

    /// Find only an except handler (not finally)
    pub fn find_except_handler(&self) -> Option<&Block> {
        self.block_stack
            .iter()
            .rev()
            .find(|b| matches!(b.block_type, BlockType::Except))
    }

    /// Find only a finally handler
    pub fn find_finally_handler(&self) -> Option<&Block> {
        self.block_stack
            .iter()
            .rev()
            .find(|b| matches!(b.block_type, BlockType::Finally))
    }

    /// Pop blocks until we find an exception handler, returning all popped blocks
    /// This is used for proper finally execution during exception propagation
    pub fn unwind_to_handler(&mut self) -> Option<Block> {
        while let Some(block) = self.block_stack.pop() {
            match block.block_type {
                BlockType::Except | BlockType::Finally => {
                    return Some(block);
                }
                BlockType::Loop | BlockType::With => {
                    // Continue unwinding past loops and with blocks
                    continue;
                }
            }
        }
        None
    }

    /// Check if we're currently in an exception handler
    pub fn in_exception_handler(&self) -> bool {
        self.block_stack.iter().any(|b| matches!(b.block_type, BlockType::Except))
    }

    /// Get stack depth
    #[inline]
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }

    /// Clear the stack
    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    /// Get the function name
    pub fn func_name(&self) -> &str {
        &self.function.name
    }

    /// Get the qualified name
    pub fn qualname(&self) -> &str {
        &self.function.qualname
    }

    /// Get the module name
    pub fn module_name(&self) -> Option<&str> {
        self.function.module.as_deref()
    }
}

impl std::fmt::Debug for PyFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<frame {} at ip={}, stack_depth={}, cells={}, freevars={}>",
            self.function.qualname,
            self.ip,
            self.stack.len(),
            self.cells.len(),
            self.freevars.len()
        )
    }
}

/// Frame iterator for traceback
pub struct FrameIterator {
    current: Option<Arc<PyFrame>>,
}

impl FrameIterator {
    pub fn new(frame: Arc<PyFrame>) -> Self {
        Self {
            current: Some(frame),
        }
    }
}

impl Iterator for FrameIterator {
    type Item = Arc<PyFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.current.take()?;
        self.current = frame.back.clone();
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyfunction::{CodeRef, Parameter};

    fn make_test_function() -> Arc<PyFunction> {
        Arc::new(PyFunction::new(
            "test_func",
            CodeRef {
                bytecode_offset: 0,
                num_locals: 4,
                stack_size: 8,
                num_args: 2,
                num_kwonly_args: 0,
            },
            vec![
                Parameter {
                    name: "a".into(),
                    kind: crate::pyfunction::ParameterKind::PositionalOrKeyword,
                    default: None,
                    annotation: None,
                },
                Parameter {
                    name: "b".into(),
                    kind: crate::pyfunction::ParameterKind::PositionalOrKeyword,
                    default: None,
                    annotation: None,
                },
            ],
        ))
    }

    #[test]
    fn test_frame_creation() {
        let func = make_test_function();
        let frame = PyFrame::new(func, None);

        assert_eq!(frame.ip, 0);
        assert_eq!(frame.locals.len(), 4);
        assert!(frame.stack.is_empty());
    }

    #[test]
    fn test_frame_stack_ops() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        frame.push(PyValue::Int(1));
        frame.push(PyValue::Int(2));

        assert_eq!(frame.stack_depth(), 2);

        if let PyValue::Int(v) = frame.pop() {
            assert_eq!(v, 2);
        }

        if let PyValue::Int(v) = frame.peek() {
            assert_eq!(*v, 1);
        }
    }

    #[test]
    fn test_frame_locals() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        frame.set_local(0, PyValue::Int(42));

        if let PyValue::Int(v) = frame.get_local(0) {
            assert_eq!(*v, 42);
        }
    }

    #[test]
    fn test_frame_blocks() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        frame.push(PyValue::Int(1));
        frame.push_block(BlockType::Loop, 100);
        frame.push(PyValue::Int(2));
        frame.push(PyValue::Int(3));

        let block = frame.pop_block().unwrap();
        assert_eq!(block.level, 1);

        frame.unwind_to(block.level);
        assert_eq!(frame.stack_depth(), 1);
    }

    #[test]
    fn test_frame_exception_handler() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        // Push an except block
        frame.push_block(BlockType::Except, 50);

        // Should find the except handler
        let handler = frame.find_exception_handler();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().handler, 50);

        // Should also find it as except handler
        let except_handler = frame.find_except_handler();
        assert!(except_handler.is_some());

        // Should not find it as finally handler
        let finally_handler = frame.find_finally_handler();
        assert!(finally_handler.is_none());
    }

    #[test]
    fn test_frame_finally_handler() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        // Push a finally block
        frame.push_block(BlockType::Finally, 100);

        // Should find the finally handler
        let handler = frame.find_exception_handler();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().handler, 100);

        // Should find it as finally handler
        let finally_handler = frame.find_finally_handler();
        assert!(finally_handler.is_some());

        // Should not find it as except handler
        let except_handler = frame.find_except_handler();
        assert!(except_handler.is_none());
    }

    #[test]
    fn test_frame_nested_handlers() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        // Push nested blocks: loop -> except -> finally
        frame.push_block(BlockType::Loop, 10);
        frame.push_block(BlockType::Except, 50);
        frame.push_block(BlockType::Finally, 100);

        // Should find the innermost handler (finally)
        let handler = frame.find_exception_handler();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().handler, 100);
    }

    #[test]
    fn test_frame_unwind_to_handler() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        // Push nested blocks: loop -> except
        frame.push_block(BlockType::Loop, 10);
        frame.push_block(BlockType::Except, 50);

        // Unwind should skip the loop and return the except block
        let handler = frame.unwind_to_handler();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().handler, 50);

        // Loop block should still be there
        assert_eq!(frame.block_stack.len(), 1);
    }

    #[test]
    fn test_frame_in_exception_handler() {
        let func = make_test_function();
        let mut frame = PyFrame::new(func, None);

        // Initially not in exception handler
        assert!(!frame.in_exception_handler());

        // Push an except block
        frame.push_block(BlockType::Except, 50);

        // Now in exception handler
        assert!(frame.in_exception_handler());

        // Pop the block
        frame.pop_block();

        // No longer in exception handler
        assert!(!frame.in_exception_handler());
    }
}
