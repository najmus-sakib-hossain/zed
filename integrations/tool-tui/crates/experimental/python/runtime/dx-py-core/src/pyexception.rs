//! PyException - Python exception type
//!
//! Implements Python exception objects with traceback support.

use crate::debug::{Traceback, TracebackFrame};
use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use crate::pylist::PyValue;
use std::sync::Arc;

/// Python exception object
#[derive(Debug, Clone)]
pub struct PyException {
    /// Exception type name (e.g., "ValueError", "TypeError")
    pub exc_type: String,
    /// Exception message
    pub message: String,
    /// Exception arguments (args tuple)
    pub args: Vec<PyValue>,
    /// Traceback
    pub traceback: Option<Traceback>,
    /// Cause (__cause__ - explicit chaining via `raise X from Y`)
    pub cause: Option<Arc<PyException>>,
    /// Context (__context__ - implicit chaining)
    pub context: Option<Arc<PyException>>,
    /// Whether to suppress context display
    pub suppress_context: bool,
    /// Notes attached to the exception (Python 3.11+)
    pub notes: Vec<String>,
}

impl PyException {
    /// Create a new exception
    pub fn new(exc_type: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            exc_type: exc_type.into(),
            args: vec![PyValue::Str(Arc::from(message.clone()))],
            message,
            traceback: None,
            cause: None,
            context: None,
            suppress_context: false,
            notes: Vec::new(),
        }
    }

    /// Create exception with args
    pub fn with_args(exc_type: impl Into<String>, args: Vec<PyValue>) -> Self {
        let message = if args.is_empty() {
            String::new()
        } else {
            match &args[0] {
                PyValue::Str(s) => s.to_string(),
                other => format!("{:?}", other),
            }
        };
        Self {
            exc_type: exc_type.into(),
            args,
            message,
            traceback: None,
            cause: None,
            context: None,
            suppress_context: false,
            notes: Vec::new(),
        }
    }

    /// Get the object header (created on demand)
    pub fn header(&self) -> PyObjectHeader {
        PyObjectHeader::new(TypeTag::Exception, ObjectFlags::NONE)
    }

    /// Set the traceback
    pub fn with_traceback(mut self, traceback: Traceback) -> Self {
        self.traceback = Some(traceback);
        self
    }

    /// Add a traceback frame
    pub fn add_traceback_frame(&mut self, frame: TracebackFrame) {
        if let Some(ref mut tb) = self.traceback {
            tb.push(frame);
        } else {
            let mut tb = Traceback::new();
            tb.push(frame);
            self.traceback = Some(tb);
        }
    }

    /// Add a traceback frame at the front (during stack unwinding)
    pub fn add_traceback_frame_front(&mut self, frame: TracebackFrame) {
        if let Some(ref mut tb) = self.traceback {
            tb.push_front(frame);
        } else {
            let mut tb = Traceback::new();
            tb.push(frame);
            self.traceback = Some(tb);
        }
    }

    /// Set the cause (explicit chaining via `raise X from Y`)
    pub fn with_cause(mut self, cause: Arc<PyException>) -> Self {
        self.cause = Some(cause);
        self.suppress_context = true;
        self
    }

    /// Set the context (implicit chaining)
    pub fn with_context(mut self, context: Arc<PyException>) -> Self {
        self.context = Some(context);
        self
    }

    /// Add a note to the exception (Python 3.11+)
    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    /// Check if this exception is an instance of a type
    pub fn is_instance(&self, exc_type: &str) -> bool {
        // Check exact match or base class match
        self.exc_type == exc_type || self.is_subclass_of(exc_type)
    }

    /// Check if exception type is a subclass of another
    fn is_subclass_of(&self, base: &str) -> bool {
        // Built-in exception hierarchy
        match (self.exc_type.as_str(), base) {
            // All exceptions inherit from BaseException
            (_, "BaseException") => true,
            // Most exceptions inherit from Exception
            (t, "Exception")
                if t != "SystemExit" && t != "KeyboardInterrupt" && t != "GeneratorExit" =>
            {
                true
            }
            // ArithmeticError hierarchy
            ("ZeroDivisionError" | "OverflowError" | "FloatingPointError", "ArithmeticError") => {
                true
            }
            // LookupError hierarchy
            ("IndexError" | "KeyError", "LookupError") => true,
            // OSError hierarchy (includes EnvironmentError and IOError as aliases)
            (
                "FileNotFoundError"
                | "PermissionError"
                | "FileExistsError"
                | "IsADirectoryError"
                | "NotADirectoryError"
                | "ConnectionError"
                | "BrokenPipeError"
                | "ConnectionAbortedError"
                | "ConnectionRefusedError"
                | "ConnectionResetError"
                | "TimeoutError"
                | "BlockingIOError"
                | "ChildProcessError"
                | "InterruptedError"
                | "ProcessLookupError"
                | "IOError"
                | "EnvironmentError",
                "OSError",
            ) => true,
            // ValueError hierarchy
            (
                "UnicodeError"
                | "UnicodeDecodeError"
                | "UnicodeEncodeError"
                | "UnicodeTranslateError",
                "ValueError",
            ) => true,
            (
                "UnicodeDecodeError" | "UnicodeEncodeError" | "UnicodeTranslateError",
                "UnicodeError",
            ) => true,
            // ImportError hierarchy
            ("ModuleNotFoundError", "ImportError") => true,
            // SyntaxError hierarchy
            ("IndentationError" | "TabError", "SyntaxError") => true,
            ("TabError", "IndentationError") => true,
            // RuntimeError hierarchy
            ("RecursionError" | "NotImplementedError", "RuntimeError") => true,
            // Warning hierarchy
            (
                "DeprecationWarning"
                | "PendingDeprecationWarning"
                | "RuntimeWarning"
                | "SyntaxWarning"
                | "UserWarning"
                | "FutureWarning"
                | "ImportWarning"
                | "UnicodeWarning"
                | "BytesWarning"
                | "ResourceWarning",
                "Warning",
            ) => true,
            // ConnectionError hierarchy
            (
                "BrokenPipeError"
                | "ConnectionAbortedError"
                | "ConnectionRefusedError"
                | "ConnectionResetError",
                "ConnectionError",
            ) => true,
            _ => false,
        }
    }

    /// Format the exception for display (Python-compatible format)
    pub fn format(&self) -> String {
        let mut result = String::new();

        // Format chained exceptions first (cause or context)
        if let Some(ref cause) = self.cause {
            result.push_str(&cause.format());
            result.push_str(
                "\nThe above exception was the direct cause of the following exception:\n\n",
            );
        } else if !self.suppress_context {
            if let Some(ref context) = self.context {
                result.push_str(&context.format());
                result.push_str(
                    "\nDuring handling of the above exception, another exception occurred:\n\n",
                );
            }
        }

        // Format traceback
        if let Some(ref tb) = self.traceback {
            result.push_str("Traceback (most recent call last):\n");
            for frame in tb.frames() {
                result.push_str(&format!("{}\n", frame));
            }
        }

        // Format exception type and message
        if self.message.is_empty() {
            result.push_str(&self.exc_type);
        } else {
            result.push_str(&format!("{}: {}", self.exc_type, self.message));
        }

        // Format notes (Python 3.11+)
        for note in &self.notes {
            result.push_str(&format!("\n{}", note));
        }

        result
    }

    /// Get the exception as a PyValue
    pub fn as_value(self) -> PyValue {
        PyValue::Exception(Arc::new(self))
    }

    /// Get the __traceback__ attribute
    pub fn get_traceback(&self) -> Option<&Traceback> {
        self.traceback.as_ref()
    }

    /// Get the __cause__ attribute
    pub fn get_cause(&self) -> Option<&Arc<PyException>> {
        self.cause.as_ref()
    }

    /// Get the __context__ attribute
    pub fn get_context(&self) -> Option<&Arc<PyException>> {
        self.context.as_ref()
    }

    /// Get the __suppress_context__ attribute
    pub fn get_suppress_context(&self) -> bool {
        self.suppress_context
    }

    /// Set __suppress_context__
    pub fn set_suppress_context(&mut self, suppress: bool) {
        self.suppress_context = suppress;
    }
}

impl std::fmt::Display for PyException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Standard exception types
pub mod exceptions {
    use super::*;

    // Base exceptions
    pub fn base_exception(msg: impl Into<String>) -> PyException {
        PyException::new("BaseException", msg)
    }

    pub fn exception(msg: impl Into<String>) -> PyException {
        PyException::new("Exception", msg)
    }

    pub fn system_exit(code: Option<i32>) -> PyException {
        let msg = code.map(|c| c.to_string()).unwrap_or_default();
        PyException::new("SystemExit", msg)
    }

    pub fn keyboard_interrupt() -> PyException {
        PyException::new("KeyboardInterrupt", "")
    }

    pub fn generator_exit() -> PyException {
        PyException::new("GeneratorExit", "")
    }

    // Standard exceptions
    pub fn type_error(msg: impl Into<String>) -> PyException {
        PyException::new("TypeError", msg)
    }

    pub fn value_error(msg: impl Into<String>) -> PyException {
        PyException::new("ValueError", msg)
    }

    pub fn index_error(msg: impl Into<String>) -> PyException {
        PyException::new("IndexError", msg)
    }

    pub fn key_error(msg: impl Into<String>) -> PyException {
        PyException::new("KeyError", msg)
    }

    pub fn name_error(msg: impl Into<String>) -> PyException {
        PyException::new("NameError", msg)
    }

    pub fn attribute_error(msg: impl Into<String>) -> PyException {
        PyException::new("AttributeError", msg)
    }

    pub fn runtime_error(msg: impl Into<String>) -> PyException {
        PyException::new("RuntimeError", msg)
    }

    pub fn recursion_error(msg: impl Into<String>) -> PyException {
        PyException::new("RecursionError", msg)
    }

    pub fn not_implemented_error(msg: impl Into<String>) -> PyException {
        PyException::new("NotImplementedError", msg)
    }

    pub fn stop_iteration(value: Option<PyValue>) -> PyException {
        let mut exc = PyException::new("StopIteration", "");
        if let Some(v) = value {
            exc.args = vec![v];
        }
        exc
    }

    pub fn stop_async_iteration() -> PyException {
        PyException::new("StopAsyncIteration", "")
    }

    // Arithmetic errors
    pub fn arithmetic_error(msg: impl Into<String>) -> PyException {
        PyException::new("ArithmeticError", msg)
    }

    pub fn zero_division_error() -> PyException {
        PyException::new("ZeroDivisionError", "division by zero")
    }

    pub fn overflow_error(msg: impl Into<String>) -> PyException {
        PyException::new("OverflowError", msg)
    }

    pub fn floating_point_error(msg: impl Into<String>) -> PyException {
        PyException::new("FloatingPointError", msg)
    }

    // Lookup errors
    pub fn lookup_error(msg: impl Into<String>) -> PyException {
        PyException::new("LookupError", msg)
    }

    // Import errors
    pub fn import_error(msg: impl Into<String>) -> PyException {
        PyException::new("ImportError", msg)
    }

    pub fn module_not_found_error(name: &str) -> PyException {
        PyException::new("ModuleNotFoundError", format!("No module named '{}'", name))
    }

    // OS/IO errors
    pub fn os_error(msg: impl Into<String>) -> PyException {
        PyException::new("OSError", msg)
    }

    pub fn file_not_found_error(path: &str) -> PyException {
        PyException::new("FileNotFoundError", format!("No such file or directory: '{}'", path))
    }

    pub fn file_exists_error(path: &str) -> PyException {
        PyException::new("FileExistsError", format!("File exists: '{}'", path))
    }

    pub fn permission_error(msg: impl Into<String>) -> PyException {
        PyException::new("PermissionError", msg)
    }

    pub fn is_a_directory_error(path: &str) -> PyException {
        PyException::new("IsADirectoryError", format!("Is a directory: '{}'", path))
    }

    pub fn not_a_directory_error(path: &str) -> PyException {
        PyException::new("NotADirectoryError", format!("Not a directory: '{}'", path))
    }

    pub fn io_error(msg: impl Into<String>) -> PyException {
        PyException::new("IOError", msg)
    }

    pub fn eof_error(msg: impl Into<String>) -> PyException {
        PyException::new("EOFError", msg)
    }

    pub fn timeout_error(msg: impl Into<String>) -> PyException {
        PyException::new("TimeoutError", msg)
    }

    pub fn connection_error(msg: impl Into<String>) -> PyException {
        PyException::new("ConnectionError", msg)
    }

    pub fn broken_pipe_error() -> PyException {
        PyException::new("BrokenPipeError", "Broken pipe")
    }

    pub fn connection_aborted_error() -> PyException {
        PyException::new("ConnectionAbortedError", "Connection aborted")
    }

    pub fn connection_refused_error() -> PyException {
        PyException::new("ConnectionRefusedError", "Connection refused")
    }

    pub fn connection_reset_error() -> PyException {
        PyException::new("ConnectionResetError", "Connection reset")
    }

    // Syntax errors
    pub fn syntax_error(msg: impl Into<String>) -> PyException {
        PyException::new("SyntaxError", msg)
    }

    pub fn indentation_error(msg: impl Into<String>) -> PyException {
        PyException::new("IndentationError", msg)
    }

    pub fn tab_error(msg: impl Into<String>) -> PyException {
        PyException::new("TabError", msg)
    }

    // Other standard exceptions
    pub fn assertion_error(msg: impl Into<String>) -> PyException {
        PyException::new("AssertionError", msg)
    }

    pub fn memory_error() -> PyException {
        PyException::new("MemoryError", "")
    }

    pub fn buffer_error(msg: impl Into<String>) -> PyException {
        PyException::new("BufferError", msg)
    }

    pub fn unbound_local_error(name: &str) -> PyException {
        PyException::new(
            "UnboundLocalError",
            format!("local variable '{}' referenced before assignment", name),
        )
    }

    // Unicode errors
    pub fn unicode_error(msg: impl Into<String>) -> PyException {
        PyException::new("UnicodeError", msg)
    }

    pub fn unicode_decode_error(msg: impl Into<String>) -> PyException {
        PyException::new("UnicodeDecodeError", msg)
    }

    pub fn unicode_encode_error(msg: impl Into<String>) -> PyException {
        PyException::new("UnicodeEncodeError", msg)
    }

    pub fn unicode_translate_error(msg: impl Into<String>) -> PyException {
        PyException::new("UnicodeTranslateError", msg)
    }

    // Warning types
    pub fn warning(msg: impl Into<String>) -> PyException {
        PyException::new("Warning", msg)
    }

    pub fn deprecation_warning(msg: impl Into<String>) -> PyException {
        PyException::new("DeprecationWarning", msg)
    }

    pub fn pending_deprecation_warning(msg: impl Into<String>) -> PyException {
        PyException::new("PendingDeprecationWarning", msg)
    }

    pub fn runtime_warning(msg: impl Into<String>) -> PyException {
        PyException::new("RuntimeWarning", msg)
    }

    pub fn syntax_warning(msg: impl Into<String>) -> PyException {
        PyException::new("SyntaxWarning", msg)
    }

    pub fn user_warning(msg: impl Into<String>) -> PyException {
        PyException::new("UserWarning", msg)
    }

    pub fn future_warning(msg: impl Into<String>) -> PyException {
        PyException::new("FutureWarning", msg)
    }

    pub fn import_warning(msg: impl Into<String>) -> PyException {
        PyException::new("ImportWarning", msg)
    }

    pub fn unicode_warning(msg: impl Into<String>) -> PyException {
        PyException::new("UnicodeWarning", msg)
    }

    pub fn bytes_warning(msg: impl Into<String>) -> PyException {
        PyException::new("BytesWarning", msg)
    }

    pub fn resource_warning(msg: impl Into<String>) -> PyException {
        PyException::new("ResourceWarning", msg)
    }

    // Additional standard exceptions
    pub fn environment_error(msg: impl Into<String>) -> PyException {
        PyException::new("EnvironmentError", msg)
    }

    pub fn windows_error(msg: impl Into<String>) -> PyException {
        PyException::new("WindowsError", msg)
    }

    pub fn block_io_error(msg: impl Into<String>) -> PyException {
        PyException::new("BlockingIOError", msg)
    }

    pub fn child_process_error(msg: impl Into<String>) -> PyException {
        PyException::new("ChildProcessError", msg)
    }

    pub fn interrupted_error(msg: impl Into<String>) -> PyException {
        PyException::new("InterruptedError", msg)
    }

    pub fn process_lookup_error(msg: impl Into<String>) -> PyException {
        PyException::new("ProcessLookupError", msg)
    }

    /// Create an exception from a type name and message
    pub fn from_type_name(exc_type: &str, msg: impl Into<String>) -> PyException {
        PyException::new(exc_type, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_creation() {
        let exc = PyException::new("ValueError", "invalid value");
        assert_eq!(exc.exc_type, "ValueError");
        assert_eq!(exc.message, "invalid value");
        assert!(exc.notes.is_empty());
    }

    #[test]
    fn test_exception_is_instance() {
        let exc = PyException::new("ValueError", "test");
        assert!(exc.is_instance("ValueError"));
        assert!(exc.is_instance("Exception"));
        assert!(exc.is_instance("BaseException"));
        assert!(!exc.is_instance("TypeError"));
    }

    #[test]
    fn test_exception_hierarchy() {
        let exc = PyException::new("IndexError", "out of range");
        assert!(exc.is_instance("IndexError"));
        assert!(exc.is_instance("LookupError"));
        assert!(exc.is_instance("Exception"));

        let exc2 = PyException::new("ZeroDivisionError", "division by zero");
        assert!(exc2.is_instance("ArithmeticError"));

        let exc3 = PyException::new("ModuleNotFoundError", "no module");
        assert!(exc3.is_instance("ImportError"));
        assert!(exc3.is_instance("Exception"));

        let exc4 = PyException::new("RecursionError", "max depth");
        assert!(exc4.is_instance("RuntimeError"));
    }

    #[test]
    fn test_exception_hierarchy_extended() {
        // Test OSError hierarchy
        let fnf = exceptions::file_not_found_error("/path/to/file");
        assert!(fnf.is_instance("FileNotFoundError"));
        assert!(fnf.is_instance("OSError"));
        assert!(fnf.is_instance("Exception"));

        let blocking = exceptions::block_io_error("would block");
        assert!(blocking.is_instance("BlockingIOError"));
        assert!(blocking.is_instance("OSError"));

        // Test ConnectionError hierarchy
        let broken_pipe = exceptions::broken_pipe_error();
        assert!(broken_pipe.is_instance("BrokenPipeError"));
        assert!(broken_pipe.is_instance("ConnectionError"));
        assert!(broken_pipe.is_instance("OSError"));

        // Test UnicodeError hierarchy
        let unicode_decode = exceptions::unicode_decode_error("invalid utf-8");
        assert!(unicode_decode.is_instance("UnicodeDecodeError"));
        assert!(unicode_decode.is_instance("UnicodeError"));
        assert!(unicode_decode.is_instance("ValueError"));

        // Test Warning hierarchy
        let dep_warn = exceptions::deprecation_warning("deprecated");
        assert!(dep_warn.is_instance("DeprecationWarning"));
        assert!(dep_warn.is_instance("Warning"));

        // Test SyntaxError hierarchy
        let tab_err = exceptions::tab_error("mixed tabs and spaces");
        assert!(tab_err.is_instance("TabError"));
        assert!(tab_err.is_instance("IndentationError"));
        assert!(tab_err.is_instance("SyntaxError"));
    }

    #[test]
    fn test_exception_chaining_cause() {
        let cause = Arc::new(PyException::new("IOError", "file not found"));
        let exc = PyException::new("RuntimeError", "failed to load config").with_cause(cause);

        assert!(exc.cause.is_some());
        assert!(exc.suppress_context);
        assert_eq!(exc.get_cause().unwrap().exc_type, "IOError");
    }

    #[test]
    fn test_exception_chaining_context() {
        let context = Arc::new(PyException::new("KeyError", "missing key"));
        let exc = PyException::new("ValueError", "invalid data").with_context(context);

        assert!(exc.context.is_some());
        assert!(!exc.suppress_context);
        assert_eq!(exc.get_context().unwrap().exc_type, "KeyError");
    }

    #[test]
    fn test_exception_format() {
        let exc = PyException::new("ValueError", "invalid input");
        let formatted = exc.format();
        assert!(formatted.contains("ValueError"));
        assert!(formatted.contains("invalid input"));
    }

    #[test]
    fn test_exception_format_with_traceback() {
        let mut exc = PyException::new("ValueError", "test error");
        let frame = TracebackFrame::new("test_func", Some("test.py".to_string()), 42);
        exc.add_traceback_frame(frame);

        let formatted = exc.format();
        assert!(formatted.contains("Traceback"));
        assert!(formatted.contains("test.py"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("test_func"));
    }

    #[test]
    fn test_exception_format_with_cause() {
        let cause = Arc::new(PyException::new("IOError", "file error"));
        let exc = PyException::new("RuntimeError", "load failed").with_cause(cause);

        let formatted = exc.format();
        assert!(formatted.contains("IOError"));
        assert!(formatted.contains("RuntimeError"));
        assert!(formatted.contains("direct cause"));
    }

    #[test]
    fn test_exception_format_with_context() {
        let context = Arc::new(PyException::new("KeyError", "key error"));
        let exc = PyException::new("ValueError", "value error").with_context(context);

        let formatted = exc.format();
        assert!(formatted.contains("KeyError"));
        assert!(formatted.contains("ValueError"));
        assert!(formatted.contains("another exception occurred"));
    }

    #[test]
    fn test_exception_notes() {
        let mut exc = PyException::new("ValueError", "test");
        exc.add_note("Note 1");
        exc.add_note("Note 2");

        assert_eq!(exc.notes.len(), 2);
        let formatted = exc.format();
        assert!(formatted.contains("Note 1"));
        assert!(formatted.contains("Note 2"));
    }

    #[test]
    fn test_standard_exceptions() {
        let exc = exceptions::type_error("expected int");
        assert_eq!(exc.exc_type, "TypeError");

        let exc = exceptions::stop_iteration(Some(PyValue::Int(42)));
        assert_eq!(exc.exc_type, "StopIteration");
        assert_eq!(exc.args.len(), 1);

        let exc = exceptions::recursion_error("max depth exceeded");
        assert!(exc.is_instance("RuntimeError"));

        let exc = exceptions::module_not_found_error("foo");
        assert!(exc.is_instance("ImportError"));
        assert!(exc.message.contains("foo"));
    }

    #[test]
    fn test_traceback_frame_front() {
        let mut exc = PyException::new("ValueError", "test");
        exc.add_traceback_frame(TracebackFrame::new("inner", Some("test.py".to_string()), 20));
        exc.add_traceback_frame_front(TracebackFrame::new(
            "outer",
            Some("test.py".to_string()),
            10,
        ));

        let tb = exc.get_traceback().unwrap();
        assert_eq!(tb.frames()[0].func_name, "outer");
        assert_eq!(tb.frames()[1].func_name, "inner");
    }

    #[test]
    fn test_suppress_context() {
        let context = Arc::new(PyException::new("KeyError", "key error"));
        let mut exc = PyException::new("ValueError", "value error").with_context(context);

        // Context should be shown by default
        let formatted = exc.format();
        assert!(formatted.contains("KeyError"));

        // Suppress context
        exc.set_suppress_context(true);
        let formatted = exc.format();
        assert!(!formatted.contains("KeyError"));
    }
}
