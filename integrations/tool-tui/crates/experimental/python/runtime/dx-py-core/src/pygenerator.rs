//! PyGenerator - Python generator type
//!
//! Implements generators (functions with yield) and coroutines (async functions).

use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pyframe::PyFrame;
use crate::pyfunction::PyFunction;
use crate::pylist::PyValue;
use parking_lot::Mutex;
use std::sync::Arc;

/// Generator state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorState {
    /// Generator has not started yet
    Created,
    /// Generator is currently running
    Running,
    /// Generator is suspended at a yield
    Suspended,
    /// Generator has completed normally
    Completed,
    /// Generator raised an exception
    Failed,
}

/// Python generator object
pub struct PyGenerator {
    /// Object header
    pub header: PyObjectHeader,
    /// The generator function
    pub function: Arc<PyFunction>,
    /// The suspended frame
    pub frame: Mutex<Option<PyFrame>>,
    /// Current state
    pub state: Mutex<GeneratorState>,
    /// The value to send on next iteration
    pub send_value: Mutex<Option<PyValue>>,
    /// Exception to throw on next iteration
    pub throw_value: Mutex<Option<PyValue>>,
    /// Generator name (for debugging)
    pub name: String,
    /// Qualified name
    pub qualname: String,
}

impl PyGenerator {
    /// Create a new generator from a function and initial frame
    pub fn new(function: Arc<PyFunction>, frame: PyFrame) -> Self {
        let name = function.name.clone();
        let qualname = function.qualname.clone();
        Self {
            header: PyObjectHeader::new(TypeTag::Generator, ObjectFlags::NONE),
            function,
            frame: Mutex::new(Some(frame)),
            state: Mutex::new(GeneratorState::Created),
            send_value: Mutex::new(None),
            throw_value: Mutex::new(None),
            name,
            qualname,
        }
    }

    /// Get the current state
    pub fn get_state(&self) -> GeneratorState {
        *self.state.lock()
    }

    /// Check if the generator is exhausted
    pub fn is_exhausted(&self) -> bool {
        matches!(self.get_state(), GeneratorState::Completed | GeneratorState::Failed)
    }

    /// Implement __iter__ - generators are their own iterators
    pub fn iter(self: &Arc<Self>) -> PyValue {
        // Generators are their own iterators
        PyValue::Generator(Arc::clone(self))
    }

    /// Implement __next__ - advance the generator and return the next value
    pub fn next(&self) -> GeneratorResult {
        // __next__ is equivalent to send(None)
        self.send(PyValue::None)
    }

    /// Send a value to the generator (implements generator.send())
    pub fn send(&self, value: PyValue) -> GeneratorResult {
        let state = self.get_state();

        match state {
            GeneratorState::Created => {
                // First call - value must be None
                if !matches!(value, PyValue::None) {
                    return GeneratorResult::Error(
                        "can't send non-None value to a just-started generator".into(),
                    );
                }
                *self.state.lock() = GeneratorState::Running;
                *self.send_value.lock() = Some(value);
                GeneratorResult::NeedExecution
            }
            GeneratorState::Suspended => {
                *self.state.lock() = GeneratorState::Running;
                *self.send_value.lock() = Some(value);
                GeneratorResult::NeedExecution
            }
            GeneratorState::Running => GeneratorResult::Error("generator already executing".into()),
            GeneratorState::Completed => GeneratorResult::StopIteration(PyValue::None),
            GeneratorState::Failed => {
                GeneratorResult::Error("generator raised an exception".into())
            }
        }
    }

    /// Throw an exception into the generator (implements generator.throw())
    pub fn throw(&self, exc: PyValue) -> GeneratorResult {
        let state = self.get_state();

        match state {
            GeneratorState::Created | GeneratorState::Suspended => {
                *self.state.lock() = GeneratorState::Running;
                *self.throw_value.lock() = Some(exc);
                GeneratorResult::NeedExecution
            }
            GeneratorState::Running => GeneratorResult::Error("generator already executing".into()),
            GeneratorState::Completed | GeneratorState::Failed => {
                // Re-raise the exception
                GeneratorResult::Error("exception thrown into completed generator".to_string())
            }
        }
    }

    /// Close the generator (implements generator.close())
    pub fn close(&self) -> GeneratorResult {
        let state = self.get_state();

        match state {
            GeneratorState::Created => {
                *self.state.lock() = GeneratorState::Completed;
                GeneratorResult::Closed
            }
            GeneratorState::Suspended => {
                // Throw GeneratorExit
                *self.state.lock() = GeneratorState::Running;
                *self.throw_value.lock() = Some(PyValue::Str(Arc::from("GeneratorExit")));
                GeneratorResult::NeedExecution
            }
            GeneratorState::Running => GeneratorResult::Error("generator already executing".into()),
            GeneratorState::Completed | GeneratorState::Failed => GeneratorResult::Closed,
        }
    }

    /// Mark the generator as yielded with a value
    pub fn yield_value(&self, _value: PyValue) {
        *self.state.lock() = GeneratorState::Suspended;
    }

    /// Mark the generator as completed
    pub fn complete(&self, _value: PyValue) {
        *self.state.lock() = GeneratorState::Completed;
    }

    /// Mark the generator as failed
    pub fn fail(&self) {
        *self.state.lock() = GeneratorState::Failed;
    }

    /// Take the send value (used by the VM)
    pub fn take_send_value(&self) -> Option<PyValue> {
        self.send_value.lock().take()
    }

    /// Take the throw value (used by the VM)
    pub fn take_throw_value(&self) -> Option<PyValue> {
        self.throw_value.lock().take()
    }

    /// Get the frame (for execution)
    pub fn get_frame(&self) -> Option<PyFrame> {
        self.frame.lock().take()
    }

    /// Set the frame (after yield)
    pub fn set_frame(&self, frame: PyFrame) {
        *self.frame.lock() = Some(frame);
    }
}

/// Result of a generator operation
#[derive(Debug)]
pub enum GeneratorResult {
    /// Generator yielded a value
    Yielded(PyValue),
    /// Generator completed (StopIteration)
    StopIteration(PyValue),
    /// Generator was closed
    Closed,
    /// Generator needs to be executed by the VM
    NeedExecution,
    /// Error occurred
    Error(String),
}

impl std::fmt::Debug for PyGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<generator {} at {:p}>", self.qualname, self)
    }
}

/// Python coroutine object (for async/await)
pub struct PyCoroutine {
    /// Object header
    pub header: PyObjectHeader,
    /// The coroutine function
    pub function: Arc<PyFunction>,
    /// The suspended frame
    pub frame: Mutex<Option<PyFrame>>,
    /// Current state
    pub state: Mutex<CoroutineState>,
    /// The value to send on next iteration
    pub send_value: Mutex<Option<PyValue>>,
    /// Exception to throw on next iteration
    pub throw_value: Mutex<Option<PyValue>>,
    /// Coroutine name
    pub name: String,
    /// Qualified name
    pub qualname: String,
    /// Origin (for debugging)
    pub origin: Option<String>,
}

/// Coroutine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineState {
    /// Coroutine has not started
    Created,
    /// Coroutine is running
    Running,
    /// Coroutine is suspended (awaiting)
    Suspended,
    /// Coroutine completed
    Completed,
    /// Coroutine raised an exception
    Failed,
}

impl PyCoroutine {
    /// Create a new coroutine
    pub fn new(function: Arc<PyFunction>, frame: PyFrame) -> Self {
        let name = function.name.clone();
        let qualname = function.qualname.clone();
        Self {
            header: PyObjectHeader::new(TypeTag::Coroutine, ObjectFlags::NONE),
            function,
            frame: Mutex::new(Some(frame)),
            state: Mutex::new(CoroutineState::Created),
            send_value: Mutex::new(None),
            throw_value: Mutex::new(None),
            name,
            qualname,
            origin: None,
        }
    }

    /// Get the current state
    pub fn get_state(&self) -> CoroutineState {
        *self.state.lock()
    }

    /// Check if the coroutine is done
    pub fn is_done(&self) -> bool {
        matches!(self.get_state(), CoroutineState::Completed | CoroutineState::Failed)
    }

    /// Send a value to the coroutine
    pub fn send(&self, value: PyValue) -> CoroutineResult {
        let state = self.get_state();

        match state {
            CoroutineState::Created => {
                if !matches!(value, PyValue::None) {
                    return CoroutineResult::Error(
                        "can't send non-None value to a just-started coroutine".into(),
                    );
                }
                *self.state.lock() = CoroutineState::Running;
                *self.send_value.lock() = Some(value);
                CoroutineResult::NeedExecution
            }
            CoroutineState::Suspended => {
                *self.state.lock() = CoroutineState::Running;
                *self.send_value.lock() = Some(value);
                CoroutineResult::NeedExecution
            }
            CoroutineState::Running => CoroutineResult::Error("coroutine already executing".into()),
            CoroutineState::Completed => CoroutineResult::StopIteration(PyValue::None),
            CoroutineState::Failed => {
                CoroutineResult::Error("coroutine raised an exception".into())
            }
        }
    }

    /// Throw an exception into the coroutine
    pub fn throw(&self, exc: PyValue) -> CoroutineResult {
        let state = self.get_state();

        match state {
            CoroutineState::Created | CoroutineState::Suspended => {
                *self.state.lock() = CoroutineState::Running;
                *self.throw_value.lock() = Some(exc);
                CoroutineResult::NeedExecution
            }
            CoroutineState::Running => CoroutineResult::Error("coroutine already executing".into()),
            CoroutineState::Completed | CoroutineState::Failed => {
                CoroutineResult::Error("exception thrown into completed coroutine".into())
            }
        }
    }

    /// Close the coroutine
    pub fn close(&self) -> CoroutineResult {
        let state = self.get_state();

        match state {
            CoroutineState::Created => {
                *self.state.lock() = CoroutineState::Completed;
                CoroutineResult::Closed
            }
            CoroutineState::Suspended => {
                *self.state.lock() = CoroutineState::Running;
                *self.throw_value.lock() = Some(PyValue::Str(Arc::from("GeneratorExit")));
                CoroutineResult::NeedExecution
            }
            CoroutineState::Running => CoroutineResult::Error("coroutine already executing".into()),
            CoroutineState::Completed | CoroutineState::Failed => CoroutineResult::Closed,
        }
    }

    /// Mark as suspended
    pub fn suspend(&self) {
        *self.state.lock() = CoroutineState::Suspended;
    }

    /// Mark as completed
    pub fn complete(&self) {
        *self.state.lock() = CoroutineState::Completed;
    }

    /// Mark as failed
    pub fn fail(&self) {
        *self.state.lock() = CoroutineState::Failed;
    }

    /// Take the send value
    pub fn take_send_value(&self) -> Option<PyValue> {
        self.send_value.lock().take()
    }

    /// Take the throw value
    pub fn take_throw_value(&self) -> Option<PyValue> {
        self.throw_value.lock().take()
    }

    /// Get the frame
    pub fn get_frame(&self) -> Option<PyFrame> {
        self.frame.lock().take()
    }

    /// Set the frame
    pub fn set_frame(&self, frame: PyFrame) {
        *self.frame.lock() = Some(frame);
    }
}

/// Result of a coroutine operation
#[derive(Debug)]
pub enum CoroutineResult {
    /// Coroutine yielded a value (awaiting)
    Awaiting(PyValue),
    /// Coroutine completed
    StopIteration(PyValue),
    /// Coroutine was closed
    Closed,
    /// Coroutine needs execution
    NeedExecution,
    /// Error occurred
    Error(String),
}

impl std::fmt::Debug for PyCoroutine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<coroutine {} at {:p}>", self.qualname, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyfunction::CodeRef;

    fn make_test_generator_function() -> Arc<PyFunction> {
        let mut func = PyFunction::new(
            "test_gen",
            CodeRef {
                bytecode_offset: 0,
                num_locals: 2,
                stack_size: 4,
                num_args: 0,
                num_kwonly_args: 0,
            },
            vec![],
        );
        func.flags.is_generator = true;
        Arc::new(func)
    }

    fn make_test_frame(func: Arc<PyFunction>) -> PyFrame {
        PyFrame::new(func, None)
    }

    #[test]
    fn test_generator_creation() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = PyGenerator::new(func, frame);

        assert_eq!(gen.get_state(), GeneratorState::Created);
        assert!(!gen.is_exhausted());
    }

    #[test]
    fn test_generator_send_initial() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = PyGenerator::new(func, frame);

        // First send must be None
        let result = gen.send(PyValue::None);
        assert!(matches!(result, GeneratorResult::NeedExecution));
        assert_eq!(gen.get_state(), GeneratorState::Running);
    }

    #[test]
    fn test_generator_send_non_none_initial() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = PyGenerator::new(func, frame);

        // First send with non-None should error
        let result = gen.send(PyValue::Int(42));
        assert!(matches!(result, GeneratorResult::Error(_)));
    }

    #[test]
    fn test_generator_close() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = PyGenerator::new(func, frame);

        // Close a fresh generator
        let result = gen.close();
        assert!(matches!(result, GeneratorResult::Closed));
        assert_eq!(gen.get_state(), GeneratorState::Completed);
    }

    #[test]
    fn test_generator_exhausted() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = PyGenerator::new(func, frame);

        gen.complete(PyValue::None);
        assert!(gen.is_exhausted());

        // Send to exhausted generator should return StopIteration
        let result = gen.send(PyValue::None);
        assert!(matches!(result, GeneratorResult::StopIteration(_)));
    }

    #[test]
    fn test_coroutine_creation() {
        let mut func = PyFunction::new(
            "test_coro",
            CodeRef {
                bytecode_offset: 0,
                num_locals: 2,
                stack_size: 4,
                num_args: 0,
                num_kwonly_args: 0,
            },
            vec![],
        );
        func.flags.is_coroutine = true;
        let func = Arc::new(func);
        let frame = make_test_frame(Arc::clone(&func));
        let coro = PyCoroutine::new(func, frame);

        assert_eq!(coro.get_state(), CoroutineState::Created);
        assert!(!coro.is_done());
    }

    #[test]
    fn test_coroutine_send() {
        let mut func = PyFunction::new(
            "test_coro",
            CodeRef {
                bytecode_offset: 0,
                num_locals: 2,
                stack_size: 4,
                num_args: 0,
                num_kwonly_args: 0,
            },
            vec![],
        );
        func.flags.is_coroutine = true;
        let func = Arc::new(func);
        let frame = make_test_frame(Arc::clone(&func));
        let coro = PyCoroutine::new(func, frame);

        let result = coro.send(PyValue::None);
        assert!(matches!(result, CoroutineResult::NeedExecution));
        assert_eq!(coro.get_state(), CoroutineState::Running);
    }

    #[test]
    fn test_generator_iterator_protocol() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Test __iter__ - should return self
        let iter_result = gen.iter();
        match iter_result {
            PyValue::Generator(g) => {
                assert!(Arc::ptr_eq(&g, &gen));
            }
            _ => panic!("Expected generator"),
        }

        // Test __next__ - should be equivalent to send(None)
        let next_result = gen.next();
        assert!(matches!(next_result, GeneratorResult::NeedExecution));
        assert_eq!(gen.get_state(), GeneratorState::Running);
    }

    #[test]
    fn test_generator_next_exhausted() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Mark as completed
        gen.complete(PyValue::None);

        // __next__ on exhausted generator should return StopIteration
        let result = gen.next();
        assert!(matches!(result, GeneratorResult::StopIteration(_)));
    }

    #[test]
    fn test_generator_send_value() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // First send must be None
        let result = gen.send(PyValue::None);
        assert!(matches!(result, GeneratorResult::NeedExecution));

        // Simulate suspension
        gen.yield_value(PyValue::Int(42));

        // Now we can send a value
        let result = gen.send(PyValue::Str(Arc::from("hello")));
        assert!(matches!(result, GeneratorResult::NeedExecution));

        // Check that the send value was stored
        let send_value = gen.take_send_value();
        match send_value {
            Some(PyValue::Str(s)) => assert_eq!(&*s, "hello"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_generator_throw_exception() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Throw an exception into the generator
        let exc = PyValue::Str(Arc::from("ValueError"));
        let result = gen.throw(exc.clone());
        assert!(matches!(result, GeneratorResult::NeedExecution));

        // Check that the exception was stored
        let throw_value = gen.take_throw_value();
        match throw_value {
            Some(PyValue::Str(s)) => assert_eq!(&*s, "ValueError"),
            _ => panic!("Expected exception value"),
        }
    }

    #[test]
    fn test_generator_close_fresh() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Close a fresh generator
        let result = gen.close();
        assert!(matches!(result, GeneratorResult::Closed));
        assert_eq!(gen.get_state(), GeneratorState::Completed);
    }

    #[test]
    fn test_generator_close_suspended() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Simulate suspension
        gen.yield_value(PyValue::Int(42));

        // Close a suspended generator should throw GeneratorExit
        let result = gen.close();
        assert!(matches!(result, GeneratorResult::NeedExecution));

        // Check that GeneratorExit was thrown
        let throw_value = gen.take_throw_value();
        match throw_value {
            Some(PyValue::Str(s)) => assert_eq!(&*s, "GeneratorExit"),
            _ => panic!("Expected GeneratorExit"),
        }
    }

    #[test]
    fn test_generator_running_state_error() {
        let func = make_test_generator_function();
        let frame = make_test_frame(Arc::clone(&func));
        let gen = Arc::new(PyGenerator::new(func, frame));

        // Set to running state
        *gen.state.lock() = GeneratorState::Running;

        // Operations on running generator should error
        let send_result = gen.send(PyValue::None);
        assert!(matches!(send_result, GeneratorResult::Error(_)));

        let throw_result = gen.throw(PyValue::Str(Arc::from("Exception")));
        assert!(matches!(throw_result, GeneratorResult::Error(_)));

        let close_result = gen.close();
        assert!(matches!(close_result, GeneratorResult::Error(_)));
    }
}

// ===== Property Tests =====

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::pyfunction::CodeRef;
    use proptest::prelude::*;

    /// Generate a test generator function
    fn arb_generator_function() -> impl Strategy<Value = Arc<PyFunction>> {
        prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]*").unwrap().prop_map(|name| {
            let mut func = PyFunction::new(
                name,
                CodeRef {
                    bytecode_offset: 0,
                    num_locals: 2,
                    stack_size: 4,
                    num_args: 0,
                    num_kwonly_args: 0,
                },
                vec![],
            );
            func.flags.is_generator = true;
            Arc::new(func)
        })
    }

    /// Generate a test coroutine function
    fn arb_coroutine_function() -> impl Strategy<Value = Arc<PyFunction>> {
        prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]*").unwrap().prop_map(|name| {
            let mut func = PyFunction::new(
                name,
                CodeRef {
                    bytecode_offset: 0,
                    num_locals: 2,
                    stack_size: 4,
                    num_args: 0,
                    num_kwonly_args: 0,
                },
                vec![],
            );
            func.flags.is_coroutine = true;
            Arc::new(func)
        })
    }

    /// Generate a PyValue for testing
    fn arb_py_value() -> impl Strategy<Value = PyValue> {
        prop_oneof![
            Just(PyValue::None),
            any::<bool>().prop_map(PyValue::Bool),
            any::<i64>().prop_map(PyValue::Int),
            any::<f64>().prop_map(PyValue::Float),
            prop::string::string_regex(".*")
                .unwrap()
                .prop_map(|s| PyValue::Str(Arc::from(s))),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_iter_returns_self(func in arb_generator_function()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // __iter__ should return the generator itself
            let iter_result = gen.iter();
            match iter_result {
                PyValue::Generator(g) => {
                    prop_assert!(Arc::ptr_eq(&g, &gen));
                }
                _ => prop_assert!(false, "Expected generator from __iter__"),
            }
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_next_is_send_none(func in arb_generator_function()) {
            let frame1 = PyFrame::new(Arc::clone(&func), None);
            let frame2 = PyFrame::new(Arc::clone(&func), None);
            let gen1 = Arc::new(PyGenerator::new(Arc::clone(&func), frame1));
            let gen2 = Arc::new(PyGenerator::new(func, frame2));

            // __next__ should be equivalent to send(None)
            let next_result = gen1.next();
            let send_result = gen2.send(PyValue::None);

            // Both should have the same result type
            match (next_result, send_result) {
                (GeneratorResult::NeedExecution, GeneratorResult::NeedExecution) => {},
                (GeneratorResult::StopIteration(_), GeneratorResult::StopIteration(_)) => {},
                (GeneratorResult::Error(_), GeneratorResult::Error(_)) => {},
                _ => prop_assert!(false, "__next__ and send(None) should have same result"),
            }
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_send_first_must_be_none(func in arb_generator_function(), value in arb_py_value()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // First send with non-None value should error
            if !matches!(value, PyValue::None) {
                let result = gen.send(value);
                prop_assert!(matches!(result, GeneratorResult::Error(_)));
            }
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_exhausted_returns_stop_iteration(func in arb_generator_function()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // Mark as completed
            gen.complete(PyValue::None);

            // Any operation on exhausted generator should return StopIteration
            let send_result = gen.send(PyValue::None);
            prop_assert!(matches!(send_result, GeneratorResult::StopIteration(_)));

            let next_result = gen.next();
            prop_assert!(matches!(next_result, GeneratorResult::StopIteration(_)));
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_running_state_errors(func in arb_generator_function(), value in arb_py_value()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // Set to running state
            *gen.state.lock() = GeneratorState::Running;

            // All operations should error when generator is running
            let send_result = gen.send(value.clone());
            prop_assert!(matches!(send_result, GeneratorResult::Error(_)));

            let throw_result = gen.throw(value);
            prop_assert!(matches!(throw_result, GeneratorResult::Error(_)));

            let close_result = gen.close();
            prop_assert!(matches!(close_result, GeneratorResult::Error(_)));
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_close_fresh_completes(func in arb_generator_function()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // Closing a fresh generator should complete it
            let result = gen.close();
            prop_assert!(matches!(result, GeneratorResult::Closed));
            prop_assert_eq!(gen.get_state(), GeneratorState::Completed);
        }

        /// Feature: dx-py-production-ready, Property 18: Generator Protocol
        /// Validates: Requirements 8.1, 8.2, 8.3, 8.4
        #[test]
        fn prop_generator_close_suspended_throws_generator_exit(func in arb_generator_function()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let gen = Arc::new(PyGenerator::new(func, frame));

            // Simulate suspension
            gen.yield_value(PyValue::Int(42));

            // Closing a suspended generator should throw GeneratorExit
            let result = gen.close();
            prop_assert!(matches!(result, GeneratorResult::NeedExecution));

            // Check that GeneratorExit was thrown
            let throw_value = gen.take_throw_value();
            prop_assert!(throw_value.is_some());
            match throw_value.unwrap() {
                PyValue::Str(s) => prop_assert_eq!(&*s, "GeneratorExit"),
                _ => prop_assert!(false, "Expected GeneratorExit string"),
            }
        }

        /// Feature: dx-py-production-ready, Property 19: Coroutine Protocol
        /// Validates: Requirements 8.5, 8.6, 8.7, 8.8
        #[test]
        fn prop_coroutine_send_first_must_be_none(func in arb_coroutine_function(), value in arb_py_value()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let coro = PyCoroutine::new(func, frame);

            // First send with non-None value should error
            if !matches!(value, PyValue::None) {
                let result = coro.send(value);
                prop_assert!(matches!(result, CoroutineResult::Error(_)));
            }
        }

        /// Feature: dx-py-production-ready, Property 19: Coroutine Protocol
        /// Validates: Requirements 8.5, 8.6, 8.7, 8.8
        #[test]
        fn prop_coroutine_running_state_errors(func in arb_coroutine_function(), value in arb_py_value()) {
            let frame = PyFrame::new(Arc::clone(&func), None);
            let coro = PyCoroutine::new(func, frame);

            // Set to running state
            *coro.state.lock() = CoroutineState::Running;

            // All operations should error when coroutine is running
            let send_result = coro.send(value.clone());
            prop_assert!(matches!(send_result, CoroutineResult::Error(_)));

            let throw_result = coro.throw(value);
            prop_assert!(matches!(throw_result, CoroutineResult::Error(_)));

            let close_result = coro.close();
            prop_assert!(matches!(close_result, CoroutineResult::Error(_)));
        }
    }
}
