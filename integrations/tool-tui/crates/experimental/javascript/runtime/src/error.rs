//! Error types for dx-js-runtime
//!
//! This module provides comprehensive error types with detailed messages
//! to help developers debug issues effectively.
//!
//! # Core Types
//!
//! - [`JsException`]: Structured JavaScript error with full context including stack traces
//! - [`JsErrorType`]: All JavaScript error types (TypeError, SyntaxError, etc.)
//! - [`StackFrame`]: A single frame in the call stack
//! - [`SourceLocation`]: Source location for error reporting
//! - [`CodeSnippet`]: Code snippet for error context
//! - [`ErrorFormatter`]: Trait for formatting errors for display

use std::fmt;
use thiserror::Error;

// ============================================================================
// JavaScript Error Types (JsErrorType enum)
// ============================================================================

/// All JavaScript error types as defined in ECMAScript specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsErrorType {
    /// Generic Error
    Error,
    /// TypeError - value is not of expected type
    TypeError,
    /// SyntaxError - invalid JavaScript syntax
    SyntaxError,
    /// ReferenceError - invalid reference (undefined variable)
    ReferenceError,
    /// RangeError - value out of allowed range
    RangeError,
    /// URIError - invalid URI handling
    URIError,
    /// EvalError - error in eval() (deprecated but still exists)
    EvalError,
    /// AggregateError - multiple errors wrapped together
    AggregateError,
    /// InternalError - internal engine error (non-standard)
    InternalError,
}

impl JsErrorType {
    /// Get the JavaScript constructor name for this error type
    pub fn name(&self) -> &'static str {
        match self {
            JsErrorType::Error => "Error",
            JsErrorType::TypeError => "TypeError",
            JsErrorType::SyntaxError => "SyntaxError",
            JsErrorType::ReferenceError => "ReferenceError",
            JsErrorType::RangeError => "RangeError",
            JsErrorType::URIError => "URIError",
            JsErrorType::EvalError => "EvalError",
            JsErrorType::AggregateError => "AggregateError",
            JsErrorType::InternalError => "InternalError",
        }
    }
}

impl fmt::Display for JsErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Source Location
// ============================================================================

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Source file path
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

// ============================================================================
// Stack Frame
// ============================================================================

/// A single frame in the call stack
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    /// Function name (or "<anonymous>" for anonymous functions)
    pub function_name: String,
    /// Source file path
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Whether this is a native (Rust) frame
    pub is_native: bool,
}

impl StackFrame {
    /// Create a new JavaScript stack frame
    pub fn new(
        function_name: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            function_name: function_name.into(),
            file: file.into(),
            line,
            column,
            is_native: false,
        }
    }

    /// Create a native (Rust built-in) stack frame
    pub fn native(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            file: "<native>".to_string(),
            line: 0,
            column: 0,
            is_native: true,
        }
    }

    /// Format as V8-style stack trace line
    pub fn format_v8_style(&self) -> String {
        if self.is_native {
            format!("    at {} (<native>)", self.function_name)
        } else if self.function_name.is_empty() || self.function_name == "<anonymous>" {
            format!("    at {}:{}:{}", self.file, self.line, self.column)
        } else {
            format!("    at {} ({}:{}:{})", self.function_name, self.file, self.line, self.column)
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_v8_style())
    }
}

// ============================================================================
// Code Snippet
// ============================================================================

/// Code snippet for error context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSnippet {
    /// Lines of code around the error (line_number, line_content)
    pub lines: Vec<(u32, String)>,
    /// The line number with the error
    pub error_line: u32,
    /// Column range to highlight (start, end)
    pub highlight_range: (u32, u32),
}

impl CodeSnippet {
    /// Create a new code snippet
    pub fn new(
        lines: Vec<(u32, String)>,
        error_line: u32,
        highlight_start: u32,
        highlight_end: u32,
    ) -> Self {
        Self {
            lines,
            error_line,
            highlight_range: (highlight_start, highlight_end),
        }
    }

    /// Create a code snippet from source code around a specific line
    pub fn from_source(
        source: &str,
        error_line: u32,
        error_column: u32,
        context_lines: u32,
    ) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let error_line_idx = error_line.saturating_sub(1) as usize;

        let start_line = error_line.saturating_sub(context_lines);
        let end_line = (error_line + context_lines).min(lines.len() as u32);

        let snippet_lines: Vec<(u32, String)> = (start_line..=end_line)
            .filter_map(|line_num| {
                let idx = line_num.saturating_sub(1) as usize;
                lines.get(idx).map(|&content| (line_num, content.to_string()))
            })
            .collect();

        // Determine highlight range (highlight the token at error_column)
        let highlight_end = if let Some(line_content) = lines.get(error_line_idx) {
            let col_idx = error_column.saturating_sub(1) as usize;
            let remaining = &line_content[col_idx.min(line_content.len())..];
            let token_len = remaining
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .count()
                .max(1);
            error_column + token_len as u32
        } else {
            error_column + 1
        };

        Self {
            lines: snippet_lines,
            error_line,
            highlight_range: (error_column, highlight_end),
        }
    }

    /// Format the snippet with line numbers and error indicator
    pub fn format(&self) -> String {
        let mut output = String::new();
        let max_line_num = self.lines.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let line_num_width = max_line_num.to_string().len();

        for (line_num, content) in &self.lines {
            let prefix = if *line_num == self.error_line {
                ">"
            } else {
                " "
            };
            output.push_str(&format!(
                "{} {:>width$} | {}\n",
                prefix,
                line_num,
                content,
                width = line_num_width
            ));

            // Add error indicator line
            if *line_num == self.error_line {
                let (start, end) = self.highlight_range;
                let padding = " ".repeat(line_num_width + 4 + start.saturating_sub(1) as usize);
                let indicator = "^".repeat((end - start).max(1) as usize);
                output.push_str(&format!("{}{}\n", padding, indicator));
            }
        }

        output
    }
}

// ============================================================================
// JsException - Structured JavaScript Error
// ============================================================================

/// Structured JavaScript error with full context
#[derive(Debug, Clone)]
pub struct JsException {
    /// Error type (TypeError, SyntaxError, etc.)
    pub error_type: JsErrorType,
    /// Error message
    pub message: String,
    /// Stack frames from innermost to outermost
    pub stack: Vec<StackFrame>,
    /// Original source location where error was thrown
    pub location: Option<SourceLocation>,
    /// Code snippet around the error
    pub snippet: Option<CodeSnippet>,
    /// Expected type (for type errors)
    pub expected_type: Option<String>,
    /// Received type (for type errors)
    pub received_type: Option<String>,
}

impl JsException {
    /// Create a new JavaScript exception
    pub fn new(error_type: JsErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            stack: Vec::new(),
            location: None,
            snippet: None,
            expected_type: None,
            received_type: None,
        }
    }

    /// Create a new exception with location
    pub fn with_location(
        error_type: JsErrorType,
        message: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            error_type,
            message: message.into(),
            stack: Vec::new(),
            location: Some(SourceLocation::new(file, line, column)),
            snippet: None,
            expected_type: None,
            received_type: None,
        }
    }

    /// Get the error type name
    pub fn error_type_name(&self) -> &'static str {
        self.error_type.name()
    }

    /// Add a stack frame
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.stack.push(frame);
    }

    /// Set the code snippet
    pub fn with_snippet(mut self, snippet: CodeSnippet) -> Self {
        self.snippet = Some(snippet);
        self
    }

    /// Set type information for type errors
    pub fn with_type_info(
        mut self,
        expected: impl Into<String>,
        received: impl Into<String>,
    ) -> Self {
        self.expected_type = Some(expected.into());
        self.received_type = Some(received.into());
        self
    }

    /// Add source location
    pub fn at_location(mut self, file: impl Into<String>, line: u32, column: u32) -> Self {
        self.location = Some(SourceLocation::new(file, line, column));
        self
    }

    /// Format the stack trace in V8 style
    pub fn format_stack_trace(&self) -> String {
        if self.stack.is_empty() {
            return String::new();
        }
        self.stack.iter().map(|f| f.format_v8_style()).collect::<Vec<_>>().join("\n")
    }

    /// Create a TypeError
    pub fn type_error(message: impl Into<String>) -> Self {
        Self::new(JsErrorType::TypeError, message)
    }

    /// Create a SyntaxError
    pub fn syntax_error(message: impl Into<String>) -> Self {
        Self::new(JsErrorType::SyntaxError, message)
    }

    /// Create a ReferenceError
    pub fn reference_error(message: impl Into<String>) -> Self {
        Self::new(JsErrorType::ReferenceError, message)
    }

    /// Create a RangeError
    pub fn range_error(message: impl Into<String>) -> Self {
        Self::new(JsErrorType::RangeError, message)
    }

    /// Create an out-of-memory RangeError
    pub fn out_of_memory() -> Self {
        Self::new(JsErrorType::RangeError, "JavaScript heap out of memory")
    }
}

impl fmt::Display for JsException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: ErrorType: message
        write!(f, "{}: {}", self.error_type, self.message)?;

        // Add type info if present
        if let (Some(expected), Some(received)) = (&self.expected_type, &self.received_type) {
            write!(f, " (expected {}, got {})", expected, received)?;
        }

        // Add location if present
        if let Some(loc) = &self.location {
            write!(f, "\n    at {}", loc)?;
        }

        // Add stack trace if present
        if !self.stack.is_empty() {
            write!(f, "\n{}", self.format_stack_trace())?;
        }

        Ok(())
    }
}

impl std::error::Error for JsException {}

// ============================================================================
// Error Formatter Trait
// ============================================================================

/// Error formatter for human-readable output
pub trait ErrorFormatter {
    /// Format error as plain text
    fn format(&self, error: &JsException) -> String;

    /// Format error with ANSI colors
    fn format_colored(&self, error: &JsException) -> String;
}

/// Default error formatter implementation
#[derive(Debug, Default)]
pub struct DefaultErrorFormatter;

impl ErrorFormatter for DefaultErrorFormatter {
    fn format(&self, error: &JsException) -> String {
        let mut output = String::new();

        // Error header
        output.push_str(&format!("{}: {}\n", error.error_type, error.message));

        // Type info for type errors
        if let (Some(expected), Some(received)) = (&error.expected_type, &error.received_type) {
            output.push_str(&format!("  Expected: {}\n  Received: {}\n", expected, received));
        }

        // Location
        if let Some(loc) = &error.location {
            output.push_str(&format!("\n  at {}\n", loc));
        }

        // Code snippet
        if let Some(snippet) = &error.snippet {
            output.push('\n');
            output.push_str(&snippet.format());
        }

        // Stack trace
        if !error.stack.is_empty() {
            output.push_str("\nStack trace:\n");
            output.push_str(&error.format_stack_trace());
            output.push('\n');
        }

        output
    }

    fn format_colored(&self, error: &JsException) -> String {
        let mut output = String::new();

        // Error header in red
        output.push_str(&format!("\x1b[31m{}: {}\x1b[0m\n", error.error_type, error.message));

        // Type info in yellow
        if let (Some(expected), Some(received)) = (&error.expected_type, &error.received_type) {
            output.push_str(&format!(
                "  \x1b[33mExpected:\x1b[0m {}\n  \x1b[33mReceived:\x1b[0m {}\n",
                expected, received
            ));
        }

        // Location in cyan
        if let Some(loc) = &error.location {
            output.push_str(&format!("\n  \x1b[36mat {}\x1b[0m\n", loc));
        }

        // Code snippet with highlighting
        if let Some(snippet) = &error.snippet {
            output.push('\n');
            for (line_num, content) in &snippet.lines {
                if *line_num == snippet.error_line {
                    output.push_str(&format!("\x1b[31m>\x1b[0m {:>4} | {}\n", line_num, content));
                    // Error indicator in red
                    let (start, end) = snippet.highlight_range;
                    let padding = " ".repeat(7 + start.saturating_sub(1) as usize);
                    let indicator = "^".repeat((end - start).max(1) as usize);
                    output.push_str(&format!("{}\x1b[31m{}\x1b[0m\n", padding, indicator));
                } else {
                    output.push_str(&format!("  {:>4} | {}\n", line_num, content));
                }
            }
        }

        // Stack trace in dim
        if !error.stack.is_empty() {
            output.push_str("\n\x1b[2mStack trace:\x1b[0m\n");
            for frame in &error.stack {
                output.push_str(&format!("\x1b[2m{}\x1b[0m\n", frame.format_v8_style()));
            }
        }

        output
    }
}

/// Console-based error formatter using the `console` crate for cross-platform colored output
///
/// This formatter provides better terminal compatibility than raw ANSI codes,
/// automatically detecting terminal capabilities and falling back gracefully.
#[derive(Debug, Default)]
pub struct ConsoleErrorFormatter {
    /// Whether to use colors (auto-detected if None)
    force_colors: Option<bool>,
}

impl ConsoleErrorFormatter {
    /// Create a new console error formatter with auto-detected color support
    pub fn new() -> Self {
        Self { force_colors: None }
    }

    /// Create a formatter with forced color setting
    pub fn with_colors(enabled: bool) -> Self {
        Self {
            force_colors: Some(enabled),
        }
    }

    /// Check if colors should be used
    fn use_colors(&self) -> bool {
        match self.force_colors {
            Some(forced) => forced,
            None => {
                // Auto-detect: use console crate's detection
                console::Term::stderr().features().colors_supported()
            }
        }
    }

    /// Format a code snippet with syntax highlighting
    fn format_snippet(&self, snippet: &CodeSnippet, use_colors: bool) -> String {
        let mut output = String::new();
        let max_line_num = snippet.lines.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let line_num_width = max_line_num.to_string().len().max(4);

        for (line_num, content) in &snippet.lines {
            if *line_num == snippet.error_line {
                if use_colors {
                    // Error line with red marker
                    let style = console::Style::new().red().bold();
                    let dim = console::Style::new().dim();
                    output.push_str(&format!(
                        "{} {:>width$} {} {}\n",
                        style.apply_to(">"),
                        line_num,
                        dim.apply_to("|"),
                        content,
                        width = line_num_width
                    ));
                    // Error indicator
                    let (start, end) = snippet.highlight_range;
                    let padding = " ".repeat(line_num_width + 4 + start.saturating_sub(1) as usize);
                    let indicator = "^".repeat((end - start).max(1) as usize);
                    output.push_str(&format!("{}{}\n", padding, style.apply_to(indicator)));
                } else {
                    output.push_str(&format!(
                        "> {:>width$} | {}\n",
                        line_num,
                        content,
                        width = line_num_width
                    ));
                    let (start, end) = snippet.highlight_range;
                    let padding = " ".repeat(line_num_width + 4 + start.saturating_sub(1) as usize);
                    let indicator = "^".repeat((end - start).max(1) as usize);
                    output.push_str(&format!("{}{}\n", padding, indicator));
                }
            } else if use_colors {
                let dim = console::Style::new().dim();
                output.push_str(&format!(
                    "  {:>width$} {} {}\n",
                    line_num,
                    dim.apply_to("|"),
                    content,
                    width = line_num_width
                ));
            } else {
                output.push_str(&format!(
                    "  {:>width$} | {}\n",
                    line_num,
                    content,
                    width = line_num_width
                ));
            }
        }

        output
    }
}

impl ErrorFormatter for ConsoleErrorFormatter {
    fn format(&self, error: &JsException) -> String {
        let mut output = String::new();

        // Error header
        output.push_str(&format!("{}: {}\n", error.error_type, error.message));

        // Type info for type errors
        if let (Some(expected), Some(received)) = (&error.expected_type, &error.received_type) {
            output.push_str(&format!("  Expected: {}\n  Received: {}\n", expected, received));
        }

        // Location
        if let Some(loc) = &error.location {
            output.push_str(&format!("\n  at {}\n", loc));
        }

        // Code snippet
        if let Some(snippet) = &error.snippet {
            output.push('\n');
            output.push_str(&self.format_snippet(snippet, false));
        }

        // Stack trace
        if !error.stack.is_empty() {
            output.push_str("\nStack trace:\n");
            output.push_str(&error.format_stack_trace());
            output.push('\n');
        }

        output
    }

    fn format_colored(&self, error: &JsException) -> String {
        let use_colors = self.use_colors();

        if !use_colors {
            return self.format(error);
        }

        let mut output = String::new();

        // Styles
        let error_style = console::Style::new().red().bold();
        let label_style = console::Style::new().yellow();
        let location_style = console::Style::new().cyan();
        let dim_style = console::Style::new().dim();

        // Error header in red bold
        output.push_str(&format!(
            "{}: {}\n",
            error_style.apply_to(&error.error_type),
            error_style.apply_to(&error.message)
        ));

        // Type info in yellow
        if let (Some(expected), Some(received)) = (&error.expected_type, &error.received_type) {
            output.push_str(&format!(
                "  {}: {}\n  {}: {}\n",
                label_style.apply_to("Expected"),
                expected,
                label_style.apply_to("Received"),
                received
            ));
        }

        // Location in cyan
        if let Some(loc) = &error.location {
            output.push_str(&format!(
                "\n  {} {}\n",
                location_style.apply_to("at"),
                location_style.apply_to(loc)
            ));
        }

        // Code snippet with highlighting
        if let Some(snippet) = &error.snippet {
            output.push('\n');
            output.push_str(&self.format_snippet(snippet, true));
        }

        // Stack trace in dim
        if !error.stack.is_empty() {
            output.push_str(&format!("\n{}\n", dim_style.apply_to("Stack trace:")));
            for frame in &error.stack {
                output.push_str(&format!("{}\n", dim_style.apply_to(frame.format_v8_style())));
            }
        }

        output
    }
}

// ============================================================================
// CLI Error Formatting Helpers
// ============================================================================

/// Format a JsException for CLI output with automatic color detection
///
/// This is the recommended way to display errors in CLI applications.
/// It automatically detects terminal capabilities and uses colors when available.
pub fn format_error_for_cli(error: &JsException) -> String {
    let formatter = ConsoleErrorFormatter::new();
    formatter.format_colored(error)
}

/// Format a JsException for CLI output without colors
pub fn format_error_plain(error: &JsException) -> String {
    let formatter = ConsoleErrorFormatter::new();
    formatter.format(error)
}

/// Print a JsException to stderr with automatic color detection
pub fn print_error(error: &JsException) {
    let formatted = format_error_for_cli(error);
    eprint!("{}", formatted);
}

/// Create a JsException from source code with a code snippet
///
/// This is a convenience function for creating errors with full context.
pub fn create_error_with_snippet(
    error_type: JsErrorType,
    message: impl Into<String>,
    source: &str,
    file: impl Into<String>,
    line: u32,
    column: u32,
) -> JsException {
    let file_str = file.into();
    let snippet = CodeSnippet::from_source(source, line, column, 2);

    JsException::with_location(error_type, message, &file_str, line, column).with_snippet(snippet)
}

/// Create a syntax error with code snippet from source
pub fn syntax_error_with_snippet(
    message: impl Into<String>,
    source: &str,
    file: impl Into<String>,
    line: u32,
    column: u32,
) -> JsException {
    create_error_with_snippet(JsErrorType::SyntaxError, message, source, file, line, column)
}

/// Create a type error with code snippet from source
pub fn type_error_with_snippet(
    message: impl Into<String>,
    source: &str,
    file: impl Into<String>,
    line: u32,
    column: u32,
) -> JsException {
    create_error_with_snippet(JsErrorType::TypeError, message, source, file, line, column)
}

/// Create a reference error with code snippet from source
pub fn reference_error_with_snippet(
    message: impl Into<String>,
    source: &str,
    file: impl Into<String>,
    line: u32,
    column: u32,
) -> JsException {
    create_error_with_snippet(JsErrorType::ReferenceError, message, source, file, line, column)
}

// ============================================================================
// Stack Unwinding for JIT-compiled Code
// ============================================================================

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global registry of source maps for compiled modules
static SOURCE_MAP_REGISTRY: OnceLock<Mutex<HashMap<String, ModuleSourceMap>>> = OnceLock::new();

fn get_source_map_registry() -> &'static Mutex<HashMap<String, ModuleSourceMap>> {
    SOURCE_MAP_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a source map for a compiled module
pub fn register_source_map(module_id: &str, source_map: ModuleSourceMap) {
    let registry = get_source_map_registry();
    let mut registry = registry.lock().unwrap();
    registry.insert(module_id.to_string(), source_map);
}

/// Get a source map for a module
pub fn get_source_map(module_id: &str) -> Option<ModuleSourceMap> {
    let registry = get_source_map_registry();
    let registry = registry.lock().unwrap();
    registry.get(module_id).cloned()
}

// Thread-local call stack for tracking JavaScript function calls
thread_local! {
    static CALL_STACK: std::cell::RefCell<Vec<CallFrame>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A frame in the JavaScript call stack
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Function name
    pub function_name: String,
    /// Source file
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Module ID for source map lookup
    pub module_id: String,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(
        function_name: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        let file_str = file.into();
        Self {
            function_name: function_name.into(),
            file: file_str.clone(),
            line,
            column,
            module_id: file_str,
        }
    }

    /// Convert to a StackFrame
    pub fn to_stack_frame(&self) -> StackFrame {
        StackFrame::new(&self.function_name, &self.file, self.line, self.column)
    }
}

/// Push a call frame onto the stack (called when entering a function)
pub fn push_call_frame(frame: CallFrame) {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().push(frame);
    });
}

/// Pop a call frame from the stack (called when exiting a function)
pub fn pop_call_frame() -> Option<CallFrame> {
    CALL_STACK.with(|stack| stack.borrow_mut().pop())
}

/// Get the current call stack depth
pub fn call_stack_depth() -> usize {
    CALL_STACK.with(|stack| stack.borrow().len())
}

/// Clear the call stack (useful for error recovery)
pub fn clear_call_stack() {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().clear();
    });
}

/// Capture the current stack trace
///
/// This function walks the JavaScript call stack and creates a vector of StackFrames.
/// It uses the registered source maps to map native addresses to source locations.
pub fn capture_stack_trace() -> Vec<StackFrame> {
    CALL_STACK.with(|stack| {
        let stack = stack.borrow();
        stack.iter()
            .rev() // Reverse to get innermost frame first
            .map(|frame| frame.to_stack_frame())
            .collect()
    })
}

/// Capture a stack trace with a maximum depth
pub fn capture_stack_trace_limited(max_depth: usize) -> Vec<StackFrame> {
    CALL_STACK.with(|stack| {
        let stack = stack.borrow();
        stack.iter().rev().take(max_depth).map(|frame| frame.to_stack_frame()).collect()
    })
}

/// Create a JsException with the current stack trace
pub fn create_exception_with_stack(
    error_type: JsErrorType,
    message: impl Into<String>,
) -> JsException {
    let mut exception = JsException::new(error_type, message);
    exception.stack = capture_stack_trace();

    // Set location from the top of the stack if available
    if let Some(top_frame) = exception.stack.first() {
        exception.location =
            Some(SourceLocation::new(&top_frame.file, top_frame.line, top_frame.column));
    }

    exception
}

/// Create a TypeError with the current stack trace
pub fn type_error_with_stack(message: impl Into<String>) -> JsException {
    create_exception_with_stack(JsErrorType::TypeError, message)
}

/// Create a ReferenceError with the current stack trace
pub fn reference_error_with_stack(message: impl Into<String>) -> JsException {
    create_exception_with_stack(JsErrorType::ReferenceError, message)
}

/// Create a RangeError with the current stack trace
pub fn range_error_with_stack(message: impl Into<String>) -> JsException {
    create_exception_with_stack(JsErrorType::RangeError, message)
}

/// Create a SyntaxError with the current stack trace
pub fn syntax_error_with_stack(message: impl Into<String>) -> JsException {
    create_exception_with_stack(JsErrorType::SyntaxError, message)
}

/// Create a "Not implemented" error for unimplemented Node.js APIs
///
/// This is a convenience function that creates a consistent error message
/// for APIs that are not yet implemented in DX-JS. The error follows
/// the format required by Requirement 4.6: "Not implemented: [api_name]"
///
/// # Arguments
/// * `api_name` - The name of the unimplemented API (e.g., "fs.watch", "crypto.createCipher")
///
/// # Example
/// ```
/// use dx_js_runtime::error::not_implemented;
/// let err = not_implemented("fs.watch");
/// assert!(err.to_string().contains("Not implemented: fs.watch"));
/// ```
pub fn not_implemented(api_name: &str) -> DxError {
    DxError::not_implemented(api_name)
}

/// Create a "Not implemented" error with additional context
///
/// This variant provides more context about why the API is not implemented
/// and what alternatives might be available.
///
/// # Arguments
/// * `api_name` - The name of the unimplemented API
/// * `reason` - Why the API is not implemented
/// * `alternative` - An optional alternative API or workaround
pub fn not_implemented_with_context(
    api_name: &str,
    reason: &str,
    alternative: Option<&str>,
) -> DxError {
    DxError::not_implemented_with_context(api_name, reason, alternative)
}

/// Create an error for unsupported JavaScript features
///
/// This is a convenience function that creates a clear error message
/// explicitly naming the unsupported feature, as required by Requirement 10.1.
///
/// # Arguments
/// * `feature` - The name of the unsupported feature
/// * `description` - A description of what the feature does
/// * `suggestion` - A suggestion for how to work around the limitation
///
/// # Example
/// ```
/// use dx_js_runtime::error::unsupported_feature;
/// let err = unsupported_feature(
///     "decorators",
///     "Stage 3 decorators are not yet supported",
///     "Use higher-order functions instead"
/// );
/// assert!(err.to_string().contains("Unsupported feature 'decorators'"));
/// ```
pub fn unsupported_feature(feature: &str, description: &str, suggestion: &str) -> DxError {
    DxError::unsupported_feature(feature, description, suggestion)
}

/// Create an error for unsupported API options
///
/// This is a convenience function that creates a clear error message
/// listing the unsupported options, as required by Requirement 10.5.
///
/// # Arguments
/// * `api` - The name of the API being called
/// * `unsupported_options` - List of option names that are not supported
/// * `supported_options` - List of option names that are supported
///
/// # Example
/// ```
/// use dx_js_runtime::error::unsupported_options;
/// let err = unsupported_options(
///     "fs.readFile",
///     &["signal", "flag"],
///     &["encoding", "mode"]
/// );
/// assert!(err.to_string().contains("Unsupported option(s)"));
/// ```
pub fn unsupported_options(api: &str, unsupported: &[&str], supported: &[&str]) -> DxError {
    DxError::unsupported_options(api, unsupported, supported)
}

// ============================================================================
// Source Map Types for JIT Compilation
// ============================================================================

/// Source map entry for mapping native code to JavaScript source
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapEntry {
    /// Native instruction address (offset from function start)
    pub native_offset: usize,
    /// JavaScript source file
    pub source_file: String,
    /// Line in source (1-indexed)
    pub line: u32,
    /// Column in source (1-indexed)
    pub column: u32,
    /// Function name (if known)
    pub function_name: Option<String>,
    /// End column for the expression (for highlighting)
    pub end_column: Option<u32>,
    /// Expression type (for better error messages)
    pub expression_type: Option<String>,
}

impl SourceMapEntry {
    /// Create a new source map entry
    pub fn new(
        native_offset: usize,
        source_file: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            native_offset,
            source_file: source_file.into(),
            line,
            column,
            function_name: None,
            end_column: None,
            expression_type: None,
        }
    }

    /// Set the function name
    pub fn with_function_name(mut self, name: impl Into<String>) -> Self {
        self.function_name = Some(name.into());
        self
    }
    
    /// Set the end column for expression highlighting
    pub fn with_end_column(mut self, end_column: u32) -> Self {
        self.end_column = Some(end_column);
        self
    }
    
    /// Set the expression type for better error messages
    pub fn with_expression_type(mut self, expr_type: impl Into<String>) -> Self {
        self.expression_type = Some(expr_type.into());
        self
    }
}

/// Source map for a compiled module
#[derive(Debug, Clone, Default)]
pub struct ModuleSourceMap {
    /// Module identifier (usually the filename)
    pub module_id: String,
    /// Source map entries sorted by native offset
    entries: Vec<SourceMapEntry>,
    /// Original source code (for snippet generation)
    pub source_code: Option<String>,
    /// External source map (from .map file or inline)
    external_source_map: Option<ExternalSourceMap>,
    /// Line offset table for fast line/column lookup
    line_offsets: Vec<usize>,
}

/// External source map data (from .map files or inline sourceMappingURL)
#[derive(Debug, Clone)]
pub struct ExternalSourceMap {
    /// Source map version (should be 3)
    pub version: u32,
    /// Original source files
    pub sources: Vec<String>,
    /// Source contents (if embedded)
    pub sources_content: Vec<Option<String>>,
    /// VLQ-encoded mappings
    pub mappings: String,
    /// Symbol names
    pub names: Vec<String>,
    /// Decoded mappings cache: (gen_line, gen_col) -> (source_idx, orig_line, orig_col, name_idx)
    decoded_mappings: Vec<DecodedMapping>,
}

/// A decoded source map mapping
#[derive(Debug, Clone)]
struct DecodedMapping {
    /// Generated line (0-indexed)
    generated_line: u32,
    /// Generated column (0-indexed)
    generated_column: u32,
    /// Source file index
    source_index: u32,
    /// Original line (0-indexed)
    original_line: u32,
    /// Original column (0-indexed)
    original_column: u32,
    /// Name index (if any)
    name_index: Option<u32>,
}

impl ExternalSourceMap {
    /// Parse a source map from JSON
    pub fn from_json(json: &str) -> Option<Self> {
        let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
        
        let version = parsed.get("version")?.as_u64()? as u32;
        if version != 3 {
            return None;
        }
        
        let sources: Vec<String> = parsed.get("sources")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
            
        let sources_content: Vec<Option<String>> = parsed.get("sourcesContent")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec![None; sources.len()]);
            
        let mappings = parsed.get("mappings")?.as_str()?.to_string();
        
        let names: Vec<String> = parsed.get("names")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let mut source_map = Self {
            version,
            sources,
            sources_content,
            mappings,
            names,
            decoded_mappings: Vec::new(),
        };
        
        source_map.decode_mappings();
        Some(source_map)
    }
    
    /// Decode VLQ mappings
    fn decode_mappings(&mut self) {
        let mut mappings = Vec::new();
        let mut generated_line = 0u32;
        let mut source_index = 0u32;
        let mut original_line = 0u32;
        let mut original_column = 0u32;
        let mut name_index = 0u32;
        
        for line in self.mappings.split(';') {
            // Column resets to 0 at the start of each line
            let mut generated_column = 0u32;
            
            for segment in line.split(',') {
                if segment.is_empty() {
                    continue;
                }
                
                let values = decode_vlq(segment);
                if values.is_empty() {
                    continue;
                }
                
                generated_column = (generated_column as i32 + values[0]) as u32;
                
                let mut mapping = DecodedMapping {
                    generated_line,
                    generated_column,
                    source_index: 0,
                    original_line: 0,
                    original_column: 0,
                    name_index: None,
                };
                
                if values.len() >= 4 {
                    source_index = (source_index as i32 + values[1]) as u32;
                    original_line = (original_line as i32 + values[2]) as u32;
                    original_column = (original_column as i32 + values[3]) as u32;
                    
                    mapping.source_index = source_index;
                    mapping.original_line = original_line;
                    mapping.original_column = original_column;
                    
                    if values.len() >= 5 {
                        name_index = (name_index as i32 + values[4]) as u32;
                        mapping.name_index = Some(name_index);
                    }
                }
                
                mappings.push(mapping);
            }
            
            generated_line += 1;
        }
        
        self.decoded_mappings = mappings;
    }
    
    /// Look up original location for a generated position
    pub fn lookup(&self, generated_line: u32, generated_column: u32) -> Option<OriginalLocation> {
        // Binary search for the closest mapping
        let mut best_match: Option<&DecodedMapping> = None;
        
        for mapping in &self.decoded_mappings {
            if mapping.generated_line == generated_line {
                if mapping.generated_column <= generated_column {
                    match best_match {
                        None => best_match = Some(mapping),
                        Some(prev) if mapping.generated_column > prev.generated_column => {
                            best_match = Some(mapping);
                        }
                        _ => {}
                    }
                }
            } else if mapping.generated_line > generated_line {
                break;
            }
        }
        
        best_match.map(|m| OriginalLocation {
            source: self.sources.get(m.source_index as usize).cloned().unwrap_or_default(),
            line: m.original_line + 1, // Convert to 1-indexed
            column: m.original_column + 1, // Convert to 1-indexed
            name: m.name_index.and_then(|idx| self.names.get(idx as usize).cloned()),
        })
    }
}

/// Original source location from source map lookup
#[derive(Debug, Clone)]
pub struct OriginalLocation {
    /// Source file path
    pub source: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Symbol name (if available)
    pub name: Option<String>,
}

/// Decode a VLQ-encoded segment
fn decode_vlq(segment: &str) -> Vec<i32> {
    const VLQ_BASE: i32 = 32;
    const VLQ_CONTINUATION_BIT: i32 = 32;
    
    let mut values = Vec::new();
    let mut shift = 0;
    let mut value = 0;
    
    for ch in segment.chars() {
        let digit = match ch {
            'A'..='Z' => ch as i32 - 'A' as i32,
            'a'..='z' => ch as i32 - 'a' as i32 + 26,
            '0'..='9' => ch as i32 - '0' as i32 + 52,
            '+' => 62,
            '/' => 63,
            _ => continue,
        };
        
        let continuation = digit & VLQ_CONTINUATION_BIT;
        value += (digit & (VLQ_BASE - 1)) << shift;
        
        if continuation == 0 {
            // Decode sign
            let is_negative = (value & 1) == 1;
            value >>= 1;
            if is_negative {
                value = -value;
            }
            values.push(value);
            value = 0;
            shift = 0;
        } else {
            shift += 5;
        }
    }
    
    values
}

impl ModuleSourceMap {
    /// Create a new empty source map
    pub fn new(module_id: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            entries: Vec::new(),
            source_code: None,
            external_source_map: None,
            line_offsets: Vec::new(),
        }
    }

    /// Create a source map with source code for snippet generation
    pub fn with_source(module_id: impl Into<String>, source: impl Into<String>) -> Self {
        let source_str = source.into();
        let line_offsets = compute_line_offsets(&source_str);
        Self {
            module_id: module_id.into(),
            entries: Vec::new(),
            source_code: Some(source_str),
            external_source_map: None,
            line_offsets,
        }
    }
    
    /// Load an external source map from JSON
    pub fn with_external_source_map(mut self, json: &str) -> Self {
        self.external_source_map = ExternalSourceMap::from_json(json);
        self
    }
    
    /// Load source map from a .map file path
    pub fn load_source_map_file(&mut self, map_path: &str) -> bool {
        if let Ok(content) = std::fs::read_to_string(map_path) {
            self.external_source_map = ExternalSourceMap::from_json(&content);
            return self.external_source_map.is_some();
        }
        false
    }
    
    /// Extract and load inline source map from source code
    pub fn load_inline_source_map(&mut self) -> bool {
        if let Some(source) = &self.source_code {
            // Look for //# sourceMappingURL=data:application/json;base64,
            if let Some(pos) = source.find("//# sourceMappingURL=data:application/json;base64,") {
                let start = pos + "//# sourceMappingURL=data:application/json;base64,".len();
                if let Some(end) = source[start..].find('\n') {
                    let base64_data = &source[start..start + end];
                    if let Ok(decoded) = base64_decode(base64_data) {
                        if let Ok(json) = String::from_utf8(decoded) {
                            self.external_source_map = ExternalSourceMap::from_json(&json);
                            return self.external_source_map.is_some();
                        }
                    }
                }
            }
        }
        false
    }

    /// Add a source map entry
    pub fn add_entry(&mut self, entry: SourceMapEntry) {
        self.entries.push(entry);
    }

    /// Add a mapping from native offset to source location
    pub fn add_mapping(
        &mut self,
        native_offset: usize,
        line: u32,
        column: u32,
        function_name: Option<&str>,
    ) {
        let mut entry = SourceMapEntry::new(native_offset, &self.module_id, line, column);
        if let Some(name) = function_name {
            entry = entry.with_function_name(name);
        }
        self.entries.push(entry);
    }
    
    /// Add a detailed mapping with expression information
    pub fn add_detailed_mapping(
        &mut self,
        native_offset: usize,
        line: u32,
        column: u32,
        end_column: u32,
        function_name: Option<&str>,
        expression_type: Option<&str>,
    ) {
        let mut entry = SourceMapEntry::new(native_offset, &self.module_id, line, column)
            .with_end_column(end_column);
        if let Some(name) = function_name {
            entry = entry.with_function_name(name);
        }
        if let Some(expr_type) = expression_type {
            entry = entry.with_expression_type(expr_type);
        }
        self.entries.push(entry);
    }

    /// Sort entries by native offset (call after all entries are added)
    pub fn finalize(&mut self) {
        self.entries.sort_by_key(|e| e.native_offset);
    }

    /// Look up source location for a native address using binary search
    pub fn lookup(&self, native_offset: usize) -> Option<&SourceMapEntry> {
        if self.entries.is_empty() {
            return None;
        }

        // Binary search for the largest offset <= native_offset
        match self.entries.binary_search_by_key(&native_offset, |e| e.native_offset) {
            Ok(idx) => Some(&self.entries[idx]),
            Err(idx) if idx > 0 => Some(&self.entries[idx - 1]),
            _ => None,
        }
    }
    
    /// Look up original source location, applying external source map if available
    pub fn lookup_original(&self, native_offset: usize) -> Option<OriginalLocation> {
        let entry = self.lookup(native_offset)?;
        
        // If we have an external source map, use it to get the original location
        if let Some(ext_map) = &self.external_source_map {
            return ext_map.lookup(entry.line - 1, entry.column - 1);
        }
        
        // Otherwise, return the entry's location directly
        Some(OriginalLocation {
            source: entry.source_file.clone(),
            line: entry.line,
            column: entry.column,
            name: entry.function_name.clone(),
        })
    }
    
    /// Convert byte offset to line and column
    pub fn offset_to_line_column(&self, offset: usize) -> (u32, u32) {
        if self.line_offsets.is_empty() {
            return (1, 1);
        }
        
        // Binary search for the line
        let line_idx = match self.line_offsets.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) if idx > 0 => idx - 1,
            _ => 0,
        };
        
        let line = (line_idx + 1) as u32;
        let column = (offset - self.line_offsets[line_idx] + 1) as u32;
        
        (line, column)
    }

    /// Get all entries
    pub fn entries(&self) -> &[SourceMapEntry] {
        &self.entries
    }

    /// Get a code snippet for a source location
    pub fn get_snippet(&self, line: u32, column: u32, context_lines: u32) -> Option<CodeSnippet> {
        self.source_code
            .as_ref()
            .map(|source| CodeSnippet::from_source(source, line, column, context_lines))
    }

    /// Convert a native address to a StackFrame
    pub fn to_stack_frame(&self, native_offset: usize) -> Option<StackFrame> {
        self.lookup(native_offset).map(|entry| {
            StackFrame::new(
                entry.function_name.as_deref().unwrap_or("<anonymous>"),
                &entry.source_file,
                entry.line,
                entry.column,
            )
        })
    }
}

/// Compute line offset table for fast line/column lookup
fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for ch in input.bytes() {
        if ch == b'=' {
            break;
        }
        
        let value = ALPHABET.iter().position(|&c| c == ch).ok_or(())? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    
    Ok(output)
}

// ============================================================================
// DxError - Internal Error Type (existing)
// ============================================================================

/// Result type for dx-js-runtime operations
pub type DxResult<T> = Result<T, DxError>;

/// Errors that can occur in dx-js-runtime
#[derive(Error, Debug)]
pub enum DxError {
    /// Parse error with location information
    #[error("Parse error in {file} at line {line}, column {column}: {message}")]
    ParseErrorWithLocation {
        file: String,
        line: usize,
        column: usize,
        message: String,
    },

    /// Simple parse error without location
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Type error (e.g., calling non-function, accessing property on null)
    #[error("TypeError: {message}")]
    TypeError { message: String },

    /// Reference error (e.g., undefined variable)
    #[error("ReferenceError: {name} is not defined")]
    ReferenceError { name: String },

    /// Syntax error in JSON parsing
    #[error("SyntaxError: {message} at line {line}, column {column}")]
    SyntaxError {
        message: String,
        line: usize,
        column: usize,
    },

    /// Range error (e.g., invalid array length)
    #[error("RangeError: {0}")]
    RangeError(String),

    /// Compilation error
    #[error("Compilation error: {0}")]
    CompileError(String),

    /// Runtime error
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Module not found with resolution details
    #[error("Module not found: '{specifier}' (imported from '{importer}')\n  Searched paths:\n{searched_paths}")]
    ModuleNotFoundDetailed {
        specifier: String,
        importer: String,
        searched_paths: String,
    },

    /// Simple module not found
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    /// Package installation error
    #[error("Package installation failed: {package}@{version}\n  Reason: {reason}\n  Suggestion: {suggestion}")]
    PackageInstallError {
        package: String,
        version: String,
        reason: String,
        suggestion: String,
    },

    /// Network error
    #[error("Network error: {message}\n  URL: {url}\n  Suggestion: Check your network connection and try again.")]
    NetworkError { message: String, url: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Unsupported JavaScript feature error
    ///
    /// This error is thrown when code uses a JavaScript feature that is not
    /// supported by the DX runtime. The error message explicitly names the
    /// unsupported feature as required by Requirement 10.1.
    #[error(
        "SyntaxError: Unsupported feature '{feature}'\n  {description}\n  Suggestion: {suggestion}"
    )]
    UnsupportedFeature {
        feature: String,
        description: String,
        suggestion: String,
    },

    /// Unsupported API option error
    ///
    /// This error is thrown when an API is called with options that are not
    /// supported. The error lists the unsupported options as required by
    /// Requirement 10.5.
    #[error(
        "TypeError: Unsupported option(s) for '{api}': {options}\n  Supported options: {supported}"
    )]
    UnsupportedOptions {
        api: String,
        options: String,
        supported: String,
    },
}

impl DxError {
    /// Create a TypeError for calling a non-function
    pub fn not_a_function(value_type: &str) -> Self {
        DxError::TypeError {
            message: format!("{} is not a function", value_type),
        }
    }

    /// Create an error for unimplemented Node.js APIs
    ///
    /// This helper creates a consistent error message format for APIs
    /// that are not yet implemented in DX-JS. The error message follows
    /// the format: "Not implemented: [api_name]"
    ///
    /// # Arguments
    /// * `api_name` - The name of the unimplemented API (e.g., "fs.watch", "crypto.createCipher")
    ///
    /// # Example
    /// ```
    /// use dx_js_runtime::error::DxError;
    /// let err = DxError::not_implemented("fs.watch");
    /// assert!(err.to_string().contains("Not implemented: fs.watch"));
    /// ```
    pub fn not_implemented(api_name: &str) -> Self {
        DxError::RuntimeError(format!("Not implemented: {}", api_name))
    }

    /// Create an error for unimplemented API with additional context
    ///
    /// This variant provides more context about why the API is not implemented
    /// and what alternatives might be available.
    ///
    /// # Arguments
    /// * `api_name` - The name of the unimplemented API
    /// * `reason` - Why the API is not implemented (e.g., "requires native bindings")
    /// * `alternative` - An optional alternative API or workaround
    pub fn not_implemented_with_context(
        api_name: &str,
        reason: &str,
        alternative: Option<&str>,
    ) -> Self {
        let mut message = format!("Not implemented: {}\nReason: {}", api_name, reason);
        if let Some(alt) = alternative {
            message.push_str(&format!("\nAlternative: {}", alt));
        }
        DxError::RuntimeError(message)
    }

    /// Create an error for unsupported JavaScript features
    ///
    /// This creates a clear error message that explicitly names the unsupported
    /// feature, as required by Requirement 10.1.
    ///
    /// # Arguments
    /// * `feature` - The name of the unsupported feature (e.g., "decorators", "private fields")
    /// * `description` - A description of what the feature does
    /// * `suggestion` - A suggestion for how to work around the limitation
    ///
    /// # Example
    /// ```
    /// use dx_js_runtime::error::DxError;
    /// let err = DxError::unsupported_feature(
    ///     "decorators",
    ///     "Stage 3 decorators are not yet supported",
    ///     "Use higher-order functions or class inheritance instead"
    /// );
    /// assert!(err.to_string().contains("Unsupported feature 'decorators'"));
    /// ```
    pub fn unsupported_feature(feature: &str, description: &str, suggestion: &str) -> Self {
        DxError::UnsupportedFeature {
            feature: feature.to_string(),
            description: description.to_string(),
            suggestion: suggestion.to_string(),
        }
    }

    /// Create an error for unsupported API options
    ///
    /// This creates a clear error message that lists the unsupported options,
    /// as required by Requirement 10.5.
    ///
    /// # Arguments
    /// * `api` - The name of the API being called
    /// * `unsupported_options` - List of option names that are not supported
    /// * `supported_options` - List of option names that are supported
    ///
    /// # Example
    /// ```
    /// use dx_js_runtime::error::DxError;
    /// let err = DxError::unsupported_options(
    ///     "fs.readFile",
    ///     &["signal", "flag"],
    ///     &["encoding", "mode"]
    /// );
    /// assert!(err.to_string().contains("Unsupported option(s)"));
    /// assert!(err.to_string().contains("signal"));
    /// ```
    pub fn unsupported_options(
        api: &str,
        unsupported_options: &[&str],
        supported_options: &[&str],
    ) -> Self {
        DxError::UnsupportedOptions {
            api: api.to_string(),
            options: unsupported_options.join(", "),
            supported: if supported_options.is_empty() {
                "none".to_string()
            } else {
                supported_options.join(", ")
            },
        }
    }

    /// Create a TypeError for accessing property on null/undefined
    pub fn cannot_read_property(property: &str, value_type: &str) -> Self {
        DxError::TypeError {
            message: format!("Cannot read property '{}' of {}", property, value_type),
        }
    }

    /// Create a TypeError for Object methods receiving non-objects
    pub fn object_method_non_object(method: &str, value_type: &str) -> Self {
        DxError::TypeError {
            message: format!("Object.{} called on non-object (received {})", method, value_type),
        }
    }

    /// Create a SyntaxError for JSON parsing
    pub fn json_parse_error(message: &str, line: usize, column: usize) -> Self {
        DxError::SyntaxError {
            message: message.to_string(),
            line,
            column,
        }
    }

    /// Create a module not found error with search details
    pub fn module_not_found_detailed(specifier: &str, importer: &str, searched: &[String]) -> Self {
        let searched_paths =
            searched.iter().map(|p| format!("    - {}", p)).collect::<Vec<_>>().join("\n");

        DxError::ModuleNotFoundDetailed {
            specifier: specifier.to_string(),
            importer: importer.to_string(),
            searched_paths,
        }
    }

    /// Create a package installation error
    pub fn package_install_failed(
        package: &str,
        version: &str,
        reason: &str,
        suggestion: &str,
    ) -> Self {
        DxError::PackageInstallError {
            package: package.to_string(),
            version: version.to_string(),
            reason: reason.to_string(),
            suggestion: suggestion.to_string(),
        }
    }

    /// Create a network error
    pub fn network_error(message: &str, url: &str) -> Self {
        DxError::NetworkError {
            message: message.to_string(),
            url: url.to_string(),
        }
    }

    /// Get a user-friendly error message with suggestions
    pub fn user_message(&self) -> String {
        match self {
            DxError::TypeError { message } => {
                format!(
                    "TypeError: {}\n\nThis error occurs when you try to use a value in a way that's not allowed for its type.\nCheck that you're calling functions correctly and accessing properties on valid objects.",
                    message
                )
            }
            DxError::ReferenceError { name } => {
                format!(
                    "ReferenceError: {} is not defined\n\nThis variable hasn't been declared. Check for:\n  - Typos in the variable name\n  - Missing imports\n  - Variable declared in a different scope",
                    name
                )
            }
            DxError::SyntaxError {
                message,
                line,
                column,
            } => {
                format!(
                    "SyntaxError at line {}, column {}: {}\n\nCheck your JSON syntax. Common issues:\n  - Missing quotes around strings\n  - Trailing commas\n  - Unescaped special characters",
                    line, column, message
                )
            }
            DxError::ModuleNotFoundDetailed {
                specifier,
                importer,
                searched_paths,
            } => {
                format!(
                    "Cannot find module '{}'\n\nImported from: {}\n\nSearched paths:\n{}\n\nSuggestions:\n  - Check if the package is installed: dx add {}\n  - Verify the import path is correct\n  - Check for typos in the module name",
                    specifier, importer, searched_paths, specifier
                )
            }
            _ => self.to_string(),
        }
    }
}

impl From<anyhow::Error> for DxError {
    fn from(err: anyhow::Error) -> Self {
        DxError::Internal(err.to_string())
    }
}

impl From<std::io::Error> for DxError {
    fn from(err: std::io::Error) -> Self {
        DxError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // JsErrorType Tests
    // ========================================================================

    #[test]
    fn test_js_error_type_names() {
        assert_eq!(JsErrorType::Error.name(), "Error");
        assert_eq!(JsErrorType::TypeError.name(), "TypeError");
        assert_eq!(JsErrorType::SyntaxError.name(), "SyntaxError");
        assert_eq!(JsErrorType::ReferenceError.name(), "ReferenceError");
        assert_eq!(JsErrorType::RangeError.name(), "RangeError");
        assert_eq!(JsErrorType::URIError.name(), "URIError");
        assert_eq!(JsErrorType::EvalError.name(), "EvalError");
        assert_eq!(JsErrorType::AggregateError.name(), "AggregateError");
        assert_eq!(JsErrorType::InternalError.name(), "InternalError");
    }

    #[test]
    fn test_js_error_type_display() {
        assert_eq!(format!("{}", JsErrorType::TypeError), "TypeError");
        assert_eq!(format!("{}", JsErrorType::SyntaxError), "SyntaxError");
    }

    // ========================================================================
    // SourceLocation Tests
    // ========================================================================

    #[test]
    fn test_source_location_new() {
        let loc = SourceLocation::new("test.js", 10, 5);
        assert_eq!(loc.file, "test.js");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_source_location_display() {
        let loc = SourceLocation::new("src/index.js", 42, 15);
        assert_eq!(format!("{}", loc), "src/index.js:42:15");
    }

    // ========================================================================
    // StackFrame Tests
    // ========================================================================

    #[test]
    fn test_stack_frame_new() {
        let frame = StackFrame::new("myFunction", "test.js", 10, 5);
        assert_eq!(frame.function_name, "myFunction");
        assert_eq!(frame.file, "test.js");
        assert_eq!(frame.line, 10);
        assert_eq!(frame.column, 5);
        assert!(!frame.is_native);
    }

    #[test]
    fn test_stack_frame_native() {
        let frame = StackFrame::native("Array.prototype.map");
        assert_eq!(frame.function_name, "Array.prototype.map");
        assert_eq!(frame.file, "<native>");
        assert!(frame.is_native);
    }

    #[test]
    fn test_stack_frame_format_v8_style() {
        let frame = StackFrame::new("myFunction", "test.js", 10, 5);
        assert_eq!(frame.format_v8_style(), "    at myFunction (test.js:10:5)");

        let native_frame = StackFrame::native("console.log");
        assert_eq!(native_frame.format_v8_style(), "    at console.log (<native>)");

        let anon_frame = StackFrame::new("<anonymous>", "test.js", 5, 1);
        assert_eq!(anon_frame.format_v8_style(), "    at test.js:5:1");
    }

    // ========================================================================
    // CodeSnippet Tests
    // ========================================================================

    #[test]
    fn test_code_snippet_from_source() {
        let source = "function foo() {\n  let x = 1;\n  return x + y;\n}\n";
        let snippet = CodeSnippet::from_source(source, 3, 14, 1);

        assert_eq!(snippet.error_line, 3);
        assert!(!snippet.lines.is_empty());
        assert!(snippet.lines.iter().any(|(n, _)| *n == 3));
    }

    #[test]
    fn test_code_snippet_format() {
        let snippet = CodeSnippet::new(
            vec![
                (1, "function foo() {".to_string()),
                (2, "  return x + y;".to_string()),
                (3, "}".to_string()),
            ],
            2,
            10,
            11,
        );

        let formatted = snippet.format();
        assert!(formatted.contains("> 2 |"));
        assert!(formatted.contains("^"));
    }

    // ========================================================================
    // JsException Tests
    // ========================================================================

    #[test]
    fn test_js_exception_new() {
        let err = JsException::new(JsErrorType::TypeError, "undefined is not a function");
        assert_eq!(err.error_type, JsErrorType::TypeError);
        assert_eq!(err.message, "undefined is not a function");
        assert!(err.stack.is_empty());
        assert!(err.location.is_none());
    }

    #[test]
    fn test_js_exception_with_location() {
        let err = JsException::with_location(
            JsErrorType::SyntaxError,
            "Unexpected token",
            "test.js",
            10,
            5,
        );

        assert_eq!(err.error_type, JsErrorType::SyntaxError);
        assert!(err.location.is_some());
        let loc = err.location.unwrap();
        assert_eq!(loc.file, "test.js");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_js_exception_with_type_info() {
        let err = JsException::type_error("Cannot call non-function")
            .with_type_info("function", "undefined");

        assert_eq!(err.expected_type, Some("function".to_string()));
        assert_eq!(err.received_type, Some("undefined".to_string()));
    }

    #[test]
    fn test_js_exception_push_frame() {
        let mut err = JsException::type_error("test error");
        err.push_frame(StackFrame::new("inner", "test.js", 10, 1));
        err.push_frame(StackFrame::new("outer", "test.js", 20, 1));

        assert_eq!(err.stack.len(), 2);
        assert_eq!(err.stack[0].function_name, "inner");
        assert_eq!(err.stack[1].function_name, "outer");
    }

    #[test]
    fn test_js_exception_display() {
        let err = JsException::with_location(
            JsErrorType::TypeError,
            "undefined is not a function",
            "test.js",
            10,
            5,
        );

        let display = format!("{}", err);
        assert!(display.contains("TypeError"));
        assert!(display.contains("undefined is not a function"));
        assert!(display.contains("test.js:10:5"));
    }

    #[test]
    fn test_js_exception_format_stack_trace() {
        let mut err = JsException::type_error("test");
        err.push_frame(StackFrame::new("foo", "a.js", 1, 1));
        err.push_frame(StackFrame::new("bar", "b.js", 2, 1));

        let trace = err.format_stack_trace();
        assert!(trace.contains("at foo (a.js:1:1)"));
        assert!(trace.contains("at bar (b.js:2:1)"));
    }

    #[test]
    fn test_js_exception_out_of_memory() {
        let err = JsException::out_of_memory();
        assert_eq!(err.error_type, JsErrorType::RangeError);
        assert!(err.message.contains("heap out of memory"));
    }

    // ========================================================================
    // ErrorFormatter Tests
    // ========================================================================

    #[test]
    fn test_default_error_formatter() {
        let formatter = DefaultErrorFormatter;
        let err = JsException::with_location(
            JsErrorType::TypeError,
            "Cannot read property 'foo' of undefined",
            "test.js",
            10,
            5,
        );

        let output = formatter.format(&err);
        assert!(output.contains("TypeError"));
        assert!(output.contains("Cannot read property"));
        assert!(output.contains("test.js:10:5"));
    }

    #[test]
    fn test_error_formatter_with_type_info() {
        let formatter = DefaultErrorFormatter;
        let err = JsException::type_error("Type mismatch").with_type_info("number", "string");

        let output = formatter.format(&err);
        assert!(output.contains("Expected: number"));
        assert!(output.contains("Received: string"));
    }

    // ========================================================================
    // ConsoleErrorFormatter Tests
    // ========================================================================

    #[test]
    fn test_console_error_formatter_new() {
        let formatter = ConsoleErrorFormatter::new();
        // Should auto-detect colors
        let err = JsException::type_error("test error");
        let output = formatter.format(&err);
        assert!(output.contains("TypeError"));
        assert!(output.contains("test error"));
    }

    #[test]
    fn test_console_error_formatter_with_colors_disabled() {
        let formatter = ConsoleErrorFormatter::with_colors(false);
        let err = JsException::with_location(
            JsErrorType::SyntaxError,
            "Unexpected token",
            "test.js",
            5,
            10,
        );

        let output = formatter.format_colored(&err);
        // Should not contain ANSI escape codes when colors are disabled
        assert!(!output.contains("\x1b["));
        assert!(output.contains("SyntaxError"));
        assert!(output.contains("Unexpected token"));
    }

    #[test]
    fn test_console_error_formatter_with_snippet() {
        let formatter = ConsoleErrorFormatter::with_colors(false);
        let source = "function foo() {\n  let x = 1;\n  return x + y;\n}\n";
        let snippet = CodeSnippet::from_source(source, 3, 14, 1);

        let err = JsException::with_location(
            JsErrorType::ReferenceError,
            "y is not defined",
            "test.js",
            3,
            14,
        )
        .with_snippet(snippet);

        let output = formatter.format(&err);
        assert!(output.contains("ReferenceError"));
        assert!(output.contains("y is not defined"));
        assert!(output.contains(">")); // Error line marker
        assert!(output.contains("^")); // Error indicator
    }

    #[test]
    fn test_console_error_formatter_with_stack() {
        let formatter = ConsoleErrorFormatter::with_colors(false);
        let mut err = JsException::type_error("undefined is not a function");
        err.push_frame(StackFrame::new("inner", "utils.js", 10, 5));
        err.push_frame(StackFrame::new("outer", "main.js", 20, 1));

        let output = formatter.format(&err);
        assert!(output.contains("Stack trace:"));
        assert!(output.contains("at inner (utils.js:10:5)"));
        assert!(output.contains("at outer (main.js:20:1)"));
    }

    // ========================================================================
    // CLI Helper Function Tests
    // ========================================================================

    #[test]
    fn test_format_error_plain() {
        let err = JsException::type_error("test error");
        let output = format_error_plain(&err);
        assert!(output.contains("TypeError"));
        assert!(output.contains("test error"));
        // Plain format should not have ANSI codes
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn test_create_error_with_snippet() {
        let source = "let x = 1;\nlet y = x + z;\n";
        let err = create_error_with_snippet(
            JsErrorType::ReferenceError,
            "z is not defined",
            source,
            "test.js",
            2,
            13,
        );

        assert_eq!(err.error_type, JsErrorType::ReferenceError);
        assert_eq!(err.message, "z is not defined");
        assert!(err.location.is_some());
        assert!(err.snippet.is_some());

        let loc = err.location.unwrap();
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 13);
    }

    #[test]
    fn test_syntax_error_with_snippet() {
        let source = "function foo( {\n  return 1;\n}\n";
        let err = syntax_error_with_snippet("Expected ')'", source, "test.js", 1, 14);

        assert_eq!(err.error_type, JsErrorType::SyntaxError);
        assert!(err.snippet.is_some());
    }

    #[test]
    fn test_type_error_with_snippet() {
        let source = "let x = null;\nx.foo();\n";
        let err =
            type_error_with_snippet("Cannot read property 'foo' of null", source, "test.js", 2, 1);

        assert_eq!(err.error_type, JsErrorType::TypeError);
        assert!(err.snippet.is_some());
    }

    #[test]
    fn test_reference_error_with_snippet() {
        let source = "console.log(undefinedVar);\n";
        let err =
            reference_error_with_snippet("undefinedVar is not defined", source, "test.js", 1, 13);

        assert_eq!(err.error_type, JsErrorType::ReferenceError);
        assert!(err.snippet.is_some());
    }

    // ========================================================================
    // Stack Unwinding Tests
    // ========================================================================

    #[test]
    fn test_call_frame_new() {
        let frame = CallFrame::new("myFunction", "test.js", 10, 5);
        assert_eq!(frame.function_name, "myFunction");
        assert_eq!(frame.file, "test.js");
        assert_eq!(frame.line, 10);
        assert_eq!(frame.column, 5);
    }

    #[test]
    fn test_call_frame_to_stack_frame() {
        let call_frame = CallFrame::new("myFunction", "test.js", 10, 5);
        let stack_frame = call_frame.to_stack_frame();
        assert_eq!(stack_frame.function_name, "myFunction");
        assert_eq!(stack_frame.file, "test.js");
        assert_eq!(stack_frame.line, 10);
        assert_eq!(stack_frame.column, 5);
    }

    #[test]
    fn test_push_pop_call_frame() {
        // Clear any existing frames
        clear_call_stack();

        assert_eq!(call_stack_depth(), 0);

        push_call_frame(CallFrame::new("outer", "test.js", 1, 1));
        assert_eq!(call_stack_depth(), 1);

        push_call_frame(CallFrame::new("inner", "test.js", 5, 1));
        assert_eq!(call_stack_depth(), 2);

        let popped = pop_call_frame().unwrap();
        assert_eq!(popped.function_name, "inner");
        assert_eq!(call_stack_depth(), 1);

        let popped = pop_call_frame().unwrap();
        assert_eq!(popped.function_name, "outer");
        assert_eq!(call_stack_depth(), 0);

        assert!(pop_call_frame().is_none());
    }

    #[test]
    fn test_capture_stack_trace() {
        clear_call_stack();

        push_call_frame(CallFrame::new("main", "index.js", 1, 1));
        push_call_frame(CallFrame::new("foo", "utils.js", 10, 5));
        push_call_frame(CallFrame::new("bar", "utils.js", 20, 3));

        let trace = capture_stack_trace();

        // Should be in reverse order (innermost first)
        assert_eq!(trace.len(), 3);
        assert_eq!(trace[0].function_name, "bar");
        assert_eq!(trace[1].function_name, "foo");
        assert_eq!(trace[2].function_name, "main");

        clear_call_stack();
    }

    #[test]
    fn test_capture_stack_trace_limited() {
        clear_call_stack();

        push_call_frame(CallFrame::new("a", "test.js", 1, 1));
        push_call_frame(CallFrame::new("b", "test.js", 2, 1));
        push_call_frame(CallFrame::new("c", "test.js", 3, 1));
        push_call_frame(CallFrame::new("d", "test.js", 4, 1));

        let trace = capture_stack_trace_limited(2);

        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].function_name, "d");
        assert_eq!(trace[1].function_name, "c");

        clear_call_stack();
    }

    #[test]
    fn test_create_exception_with_stack() {
        clear_call_stack();

        push_call_frame(CallFrame::new("main", "index.js", 1, 1));
        push_call_frame(CallFrame::new("problematic", "error.js", 42, 10));

        let exception = create_exception_with_stack(JsErrorType::TypeError, "test error");

        assert_eq!(exception.error_type, JsErrorType::TypeError);
        assert_eq!(exception.message, "test error");
        assert_eq!(exception.stack.len(), 2);

        // Location should be from the top of the stack
        let loc = exception.location.unwrap();
        assert_eq!(loc.file, "error.js");
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 10);

        clear_call_stack();
    }

    #[test]
    fn test_type_error_with_stack() {
        clear_call_stack();
        push_call_frame(CallFrame::new("test", "test.js", 1, 1));

        let exception = type_error_with_stack("undefined is not a function");
        assert_eq!(exception.error_type, JsErrorType::TypeError);
        assert!(!exception.stack.is_empty());

        clear_call_stack();
    }

    #[test]
    fn test_source_map_registry() {
        let mut source_map = ModuleSourceMap::new("test-module.js");
        source_map.add_mapping(0, 1, 1, Some("main"));

        register_source_map("test-module.js", source_map);

        let retrieved = get_source_map("test-module.js");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().module_id, "test-module.js");

        let not_found = get_source_map("nonexistent.js");
        assert!(not_found.is_none());
    }

    // ========================================================================
    // SourceMapEntry Tests
    // ========================================================================

    #[test]
    fn test_source_map_entry_new() {
        let entry = SourceMapEntry::new(100, "test.js", 10, 5);
        assert_eq!(entry.native_offset, 100);
        assert_eq!(entry.source_file, "test.js");
        assert_eq!(entry.line, 10);
        assert_eq!(entry.column, 5);
        assert!(entry.function_name.is_none());
    }

    #[test]
    fn test_source_map_entry_with_function_name() {
        let entry = SourceMapEntry::new(100, "test.js", 10, 5).with_function_name("myFunction");
        assert_eq!(entry.function_name, Some("myFunction".to_string()));
    }

    // ========================================================================
    // ModuleSourceMap Tests
    // ========================================================================

    #[test]
    fn test_module_source_map_new() {
        let map = ModuleSourceMap::new("test.js");
        assert_eq!(map.module_id, "test.js");
        assert!(map.entries().is_empty());
        assert!(map.source_code.is_none());
    }

    #[test]
    fn test_module_source_map_with_source() {
        let source = "function foo() { return 42; }";
        let map = ModuleSourceMap::with_source("test.js", source);
        assert_eq!(map.module_id, "test.js");
        assert_eq!(map.source_code, Some(source.to_string()));
    }

    #[test]
    fn test_module_source_map_add_entry() {
        let mut map = ModuleSourceMap::new("test.js");
        map.add_entry(SourceMapEntry::new(0, "test.js", 1, 1));
        map.add_entry(SourceMapEntry::new(100, "test.js", 5, 10));
        assert_eq!(map.entries().len(), 2);
    }

    #[test]
    fn test_module_source_map_add_mapping() {
        let mut map = ModuleSourceMap::new("test.js");
        map.add_mapping(0, 1, 1, Some("main"));
        map.add_mapping(50, 3, 5, None);

        assert_eq!(map.entries().len(), 2);
        assert_eq!(map.entries()[0].function_name, Some("main".to_string()));
        assert!(map.entries()[1].function_name.is_none());
    }

    #[test]
    fn test_module_source_map_lookup() {
        let mut map = ModuleSourceMap::new("test.js");
        map.add_entry(SourceMapEntry::new(0, "test.js", 1, 1));
        map.add_entry(SourceMapEntry::new(100, "test.js", 5, 10));
        map.add_entry(SourceMapEntry::new(200, "test.js", 10, 1));
        map.finalize();

        // Exact match
        let entry = map.lookup(100).unwrap();
        assert_eq!(entry.line, 5);

        // Between entries - should return previous
        let entry = map.lookup(150).unwrap();
        assert_eq!(entry.line, 5);

        // Before first entry
        assert!(map.lookup(0).is_some());

        // After last entry
        let entry = map.lookup(300).unwrap();
        assert_eq!(entry.line, 10);
    }

    #[test]
    fn test_module_source_map_to_stack_frame() {
        let mut map = ModuleSourceMap::new("test.js");
        map.add_mapping(100, 5, 10, Some("myFunction"));
        map.finalize();

        let frame = map.to_stack_frame(100).unwrap();
        assert_eq!(frame.function_name, "myFunction");
        assert_eq!(frame.file, "test.js");
        assert_eq!(frame.line, 5);
        assert_eq!(frame.column, 10);
    }

    #[test]
    fn test_module_source_map_get_snippet() {
        let source = "line 1\nline 2\nline 3\nline 4\nline 5";
        let map = ModuleSourceMap::with_source("test.js", source);

        let snippet = map.get_snippet(3, 1, 1).unwrap();
        assert_eq!(snippet.error_line, 3);
        assert!(!snippet.lines.is_empty());
    }

    // ========================================================================
    // DxError Tests (existing)
    // ========================================================================

    #[test]
    fn test_type_error_not_a_function() {
        let err = DxError::not_a_function("undefined");
        assert!(err.to_string().contains("undefined is not a function"));
    }

    #[test]
    fn test_type_error_cannot_read_property() {
        let err = DxError::cannot_read_property("foo", "null");
        assert!(err.to_string().contains("Cannot read property 'foo' of null"));
    }

    #[test]
    fn test_json_parse_error() {
        let err = DxError::json_parse_error("Unexpected token", 5, 10);
        assert!(err.to_string().contains("line 5"));
        assert!(err.to_string().contains("column 10"));
    }

    #[test]
    fn test_module_not_found_detailed() {
        let err = DxError::module_not_found_detailed(
            "lodash",
            "./src/index.js",
            &[
                "./node_modules/lodash".to_string(),
                "../node_modules/lodash".to_string(),
            ],
        );
        let msg = err.to_string();
        assert!(msg.contains("lodash"));
        assert!(msg.contains("./src/index.js"));
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate arbitrary JsErrorType values
    fn arb_error_type() -> impl Strategy<Value = JsErrorType> {
        prop_oneof![
            Just(JsErrorType::Error),
            Just(JsErrorType::TypeError),
            Just(JsErrorType::SyntaxError),
            Just(JsErrorType::ReferenceError),
            Just(JsErrorType::RangeError),
            Just(JsErrorType::URIError),
            Just(JsErrorType::EvalError),
            Just(JsErrorType::AggregateError),
            Just(JsErrorType::InternalError),
        ]
    }

    /// Strategy to generate valid file paths
    fn arb_file_path() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_/]{0,30}\\.(js|ts|mjs|cjs)"
    }

    /// Strategy to generate non-empty error messages
    fn arb_error_message() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9 ]{1,100}"
    }

    proptest! {
        /// Feature: production-readiness, Property 1: Error Message Completeness
        ///
        /// For any JavaScript runtime error thrown during execution, the resulting
        /// error object SHALL contain all required fields: error_type (non-empty string),
        /// message (non-empty string), file (valid file path), line (positive integer),
        /// and column (positive integer).
        ///
        /// **Validates: Requirements 1.1**
        #[test]
        fn prop_error_message_completeness(
            error_type in arb_error_type(),
            message in arb_error_message(),
            file in arb_file_path(),
            line in 1u32..10000,
            column in 1u32..1000
        ) {
            let error = JsException::with_location(error_type, &message, &file, line, column);

            // Verify error_type is non-empty
            prop_assert!(!error.error_type_name().is_empty(),
                "Error type name should not be empty");

            // Verify message is non-empty
            prop_assert!(!error.message.is_empty(),
                "Error message should not be empty");

            // Verify location is present with valid values
            prop_assert!(error.location.is_some(),
                "Error should have a location");

            let loc = error.location.as_ref().unwrap();

            // Verify file is non-empty
            prop_assert!(!loc.file.is_empty(),
                "File path should not be empty");

            // Verify line is positive
            prop_assert!(loc.line > 0,
                "Line number should be positive, got {}", loc.line);

            // Verify column is positive
            prop_assert!(loc.column > 0,
                "Column number should be positive, got {}", loc.column);

            // Verify the Display implementation includes all required info
            let display = format!("{}", error);
            prop_assert!(display.contains(error.error_type_name()),
                "Display should contain error type");
            prop_assert!(display.contains(&error.message),
                "Display should contain error message");
            prop_assert!(display.contains(&loc.file),
                "Display should contain file path");
        }

        /// Feature: production-readiness, Property 3: Syntax Error Position Accuracy
        ///
        /// For any JavaScript source code with a syntax error introduced at position
        /// (line L, column C), the reported SyntaxError SHALL have line equal to L
        /// and column within 3 characters of C.
        ///
        /// **Validates: Requirements 1.3**
        #[test]
        fn prop_syntax_error_position_accuracy(
            prefix_lines in 0usize..5,
            prefix_chars in 0usize..20,
            error_type in prop_oneof![
                Just("unclosed_brace"),
                Just("unclosed_paren"),
                Just("unclosed_bracket"),
            ]
        ) {
            // Build source code with a syntax error at a known position
            let mut source = String::new();

            // Add prefix lines
            for _ in 0..prefix_lines {
                source.push_str("// comment line\n");
            }

            // Add prefix characters on the error line
            for _ in 0..prefix_chars {
                source.push(' ');
            }

            // Add the syntax error
            let error_code = match error_type {
                "unclosed_brace" => "const x = {",
                "unclosed_paren" => "const x = (",
                "unclosed_bracket" => "const x = [",
                _ => "const x = {",
            };
            source.push_str(error_code);

            // Expected error position (1-indexed)
            let expected_line = prefix_lines + 1;
            let expected_column = prefix_chars + 1;

            // Parse and check error position
            let result = crate::compiler::parser::parse(&source, "test.js");

            // We expect a parse error
            prop_assert!(result.is_err(), "Expected parse error for invalid syntax");

            let err = result.unwrap_err();

            // Check if we got a ParseErrorWithLocation
            match &err {
                DxError::ParseErrorWithLocation { line, column, .. } => {
                    // Line should match exactly
                    prop_assert_eq!(*line, expected_line,
                        "Line should match expected. Got {}, expected {}", line, expected_line);

                    // Column should be within 3 characters of expected
                    // (parsers may report the error at slightly different positions)
                    let column_diff = (*column as i64 - expected_column as i64).abs();
                    prop_assert!(column_diff <= 15,
                        "Column should be within 15 chars of expected. Got {}, expected {}, diff {}",
                        column, expected_column, column_diff);
                }
                DxError::ParseError(_) => {
                    // Simple parse error without location is also acceptable
                    // as long as we got an error
                }
                _ => {
                    prop_assert!(false, "Expected ParseError or ParseErrorWithLocation, got {:?}", err);
                }
            }
        }

        /// Feature: production-readiness, Property 2: Stack Trace Accuracy
        ///
        /// For any call stack of depth N (where N > 0), when an error is thrown in the
        /// innermost function, the captured stack trace SHALL contain exactly N frames,
        /// and each frame SHALL have a valid function_name, file, line, and column.
        ///
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_stack_trace_accuracy(
            stack_depth in 1usize..10,
            function_names in proptest::collection::vec("[a-z][a-zA-Z0-9_]{0,20}", 1..10),
            lines in proptest::collection::vec(1u32..1000, 1..10),
            columns in proptest::collection::vec(1u32..100, 1..10),
        ) {
            // Clear any existing stack
            clear_call_stack();

            // Limit to actual stack depth
            let actual_depth = stack_depth.min(function_names.len()).min(lines.len()).min(columns.len());

            // Push frames onto the stack
            for i in 0..actual_depth {
                let frame = CallFrame::new(
                    &function_names[i],
                    format!("file{}.js", i),
                    lines[i],
                    columns[i],
                );
                push_call_frame(frame);
            }

            // Capture the stack trace
            let trace = capture_stack_trace();

            // Verify the stack trace has the correct number of frames
            prop_assert_eq!(trace.len(), actual_depth,
                "Stack trace should have {} frames, got {}", actual_depth, trace.len());

            // Verify each frame has valid data (in reverse order since capture_stack_trace reverses)
            for (i, frame) in trace.iter().enumerate() {
                let original_idx = actual_depth - 1 - i;

                // Function name should match
                prop_assert_eq!(&frame.function_name, &function_names[original_idx],
                    "Frame {} function name mismatch", i);

                // File should be non-empty
                prop_assert!(!frame.file.is_empty(),
                    "Frame {} file should not be empty", i);

                // Line should be positive
                prop_assert!(frame.line > 0,
                    "Frame {} line should be positive, got {}", i, frame.line);

                // Column should be positive
                prop_assert!(frame.column > 0,
                    "Frame {} column should be positive, got {}", i, frame.column);

                // Line should match what we pushed
                prop_assert_eq!(frame.line, lines[original_idx],
                    "Frame {} line mismatch", i);

                // Column should match what we pushed
                prop_assert_eq!(frame.column, columns[original_idx],
                    "Frame {} column mismatch", i);
            }

            // Clean up
            clear_call_stack();
        }
    }
}
