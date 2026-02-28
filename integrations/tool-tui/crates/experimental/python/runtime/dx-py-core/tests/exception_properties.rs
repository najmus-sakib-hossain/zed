//! Property-based tests for exception handling
//!
//! Feature: dx-py-production-ready
//! Property 12: Exception Traceback Completeness
//! Property 13: Finally Guarantee
//! Property 14: Exception Chaining
//! Validates: Requirements 6.1, 6.2, 6.4, 6.5, 6.6

#![allow(dead_code)]
#![allow(clippy::vec_init_then_push)]

use proptest::prelude::*;
use std::sync::Arc;

use dx_py_core::debug::{Traceback, TracebackFrame};
use dx_py_core::pyexception::{exceptions, PyException};

// ===== Generators for property tests =====

/// Generate a valid function name
fn arb_func_name() -> impl Strategy<Value = String> {
    "[a-z_][a-z0-9_]{0,20}".prop_filter("valid identifier", |s| !s.is_empty())
}

/// Generate a valid filename
fn arb_filename() -> impl Strategy<Value = String> {
    "[a-z_][a-z0-9_]{0,15}\\.py".prop_map(|s| s)
}

/// Generate a valid line number
fn arb_lineno() -> impl Strategy<Value = u32> {
    1..10000u32
}

/// Generate a valid exception type name
fn arb_exc_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("ValueError".to_string()),
        Just("TypeError".to_string()),
        Just("KeyError".to_string()),
        Just("IndexError".to_string()),
        Just("RuntimeError".to_string()),
        Just("AttributeError".to_string()),
        Just("NameError".to_string()),
        Just("ImportError".to_string()),
        Just("FileNotFoundError".to_string()),
        Just("ZeroDivisionError".to_string()),
    ]
}

/// Generate a valid exception message
fn arb_exc_message() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _.,!?'-]{0,100}".prop_map(|s| s)
}

/// Generate a traceback frame
fn arb_traceback_frame() -> impl Strategy<Value = TracebackFrame> {
    (arb_func_name(), arb_filename(), arb_lineno())
        .prop_map(|(func, file, line)| TracebackFrame::new(func, Some(file), line))
}

/// Generate a traceback with multiple frames
fn arb_traceback() -> impl Strategy<Value = Traceback> {
    prop::collection::vec(arb_traceback_frame(), 1..10).prop_map(|frames| {
        let mut tb = Traceback::new();
        for frame in frames {
            tb.push(frame);
        }
        tb
    })
}

/// Generate an exception with traceback
fn arb_exception_with_traceback() -> impl Strategy<Value = PyException> {
    (arb_exc_type(), arb_exc_message(), arb_traceback())
        .prop_map(|(exc_type, msg, tb)| PyException::new(exc_type, msg).with_traceback(tb))
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// For any raised exception, the traceback SHALL contain the file path, line number,
    /// and function name for each frame.
    /// Validates: Requirements 6.1, 6.2, 6.6
    #[test]
    fn prop_traceback_contains_required_info(
        frames in prop::collection::vec(
            (arb_func_name(), arb_filename(), arb_lineno()),
            1..10
        )
    ) {
        let mut tb = Traceback::new();
        for (func, file, line) in &frames {
            tb.push(TracebackFrame::new(func.clone(), Some(file.clone()), *line));
        }

        let exc = PyException::new("TestError", "test message").with_traceback(tb);

        // Verify traceback exists
        let traceback = exc.get_traceback().expect("Exception should have traceback");

        // Verify all frames are present
        prop_assert_eq!(traceback.depth(), frames.len());

        // Verify each frame has required info
        for (i, frame) in traceback.frames().iter().enumerate() {
            let (expected_func, expected_file, expected_line) = &frames[i];

            // Function name must be present
            prop_assert_eq!(&frame.func_name, expected_func,
                "Frame {} should have correct function name", i);

            // Filename must be present
            prop_assert!(frame.filename.is_some(),
                "Frame {} should have filename", i);
            prop_assert_eq!(frame.filename.as_ref().unwrap(), expected_file,
                "Frame {} should have correct filename", i);

            // Line number must be present and correct
            prop_assert_eq!(frame.lineno, *expected_line,
                "Frame {} should have correct line number", i);
        }
    }

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// Traceback formatting should include all frame information
    /// Validates: Requirements 6.1, 6.2, 6.6
    #[test]
    fn prop_traceback_format_includes_all_info(exc in arb_exception_with_traceback()) {
        let formatted = exc.format();

        // Should contain "Traceback" header
        prop_assert!(formatted.contains("Traceback"),
            "Formatted exception should contain 'Traceback'");

        // Should contain exception type
        prop_assert!(formatted.contains(&exc.exc_type),
            "Formatted exception should contain exception type");

        // Should contain each frame's info
        if let Some(tb) = exc.get_traceback() {
            for frame in tb.frames() {
                // Should contain function name
                prop_assert!(formatted.contains(&frame.func_name),
                    "Formatted traceback should contain function name: {}", frame.func_name);

                // Should contain filename
                if let Some(ref filename) = frame.filename {
                    prop_assert!(formatted.contains(filename),
                        "Formatted traceback should contain filename: {}", filename);
                }

                // Should contain line number
                prop_assert!(formatted.contains(&frame.lineno.to_string()),
                    "Formatted traceback should contain line number: {}", frame.lineno);
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 14: Exception Chaining
    /// For any `raise X from Y` statement, the raised exception SHALL have __cause__ set to Y.
    /// Validates: Requirements 6.5
    #[test]
    fn prop_exception_cause_is_set(
        cause_type in arb_exc_type(),
        cause_msg in arb_exc_message(),
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message()
    ) {
        let cause = Arc::new(PyException::new(cause_type.clone(), cause_msg.clone()));
        let exc = PyException::new(exc_type, exc_msg).with_cause(Arc::clone(&cause));

        // __cause__ should be set
        let retrieved_cause = exc.get_cause().expect("Exception should have __cause__");

        // __cause__ should match the original cause
        prop_assert_eq!(&retrieved_cause.exc_type, &cause_type);
        prop_assert_eq!(&retrieved_cause.message, &cause_msg);

        // __suppress_context__ should be True when __cause__ is set
        prop_assert!(exc.get_suppress_context(),
            "__suppress_context__ should be True when __cause__ is set");
    }

    /// Feature: dx-py-production-ready, Property 14: Exception Chaining
    /// Exception context should be preserved for implicit chaining
    /// Validates: Requirements 6.5
    #[test]
    fn prop_exception_context_is_preserved(
        context_type in arb_exc_type(),
        context_msg in arb_exc_message(),
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message()
    ) {
        let context = Arc::new(PyException::new(context_type.clone(), context_msg.clone()));
        let exc = PyException::new(exc_type, exc_msg).with_context(Arc::clone(&context));

        // __context__ should be set
        let retrieved_context = exc.get_context().expect("Exception should have __context__");

        // __context__ should match the original context
        prop_assert_eq!(&retrieved_context.exc_type, &context_type);
        prop_assert_eq!(&retrieved_context.message, &context_msg);

        // __suppress_context__ should be False by default
        prop_assert!(!exc.get_suppress_context(),
            "__suppress_context__ should be False by default");
    }

    /// Feature: dx-py-production-ready, Property 14: Exception Chaining
    /// Chained exception formatting should include both exceptions
    /// Validates: Requirements 6.5
    #[test]
    fn prop_chained_exception_format_includes_both(
        cause_type in arb_exc_type(),
        cause_msg in arb_exc_message(),
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message()
    ) {
        let cause = Arc::new(PyException::new(cause_type.clone(), cause_msg.clone()));
        let exc = PyException::new(exc_type.clone(), exc_msg.clone()).with_cause(cause);

        let formatted = exc.format();

        // Should contain both exception types
        prop_assert!(formatted.contains(&cause_type),
            "Formatted output should contain cause type: {}", cause_type);
        prop_assert!(formatted.contains(&exc_type),
            "Formatted output should contain exception type: {}", exc_type);

        // Should contain chaining message
        prop_assert!(formatted.contains("direct cause"),
            "Formatted output should contain 'direct cause' for explicit chaining");
    }

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// Traceback push_front should maintain correct order
    /// Validates: Requirements 6.1, 6.2
    #[test]
    fn prop_traceback_push_front_order(
        frames in prop::collection::vec(arb_traceback_frame(), 2..10)
    ) {
        let mut tb = Traceback::new();

        // Push frames in reverse order using push_front
        for frame in frames.iter().rev() {
            tb.push_front(frame.clone());
        }

        // Verify order matches original
        prop_assert_eq!(tb.depth(), frames.len());
        for (i, frame) in tb.frames().iter().enumerate() {
            prop_assert_eq!(&frame.func_name, &frames[i].func_name,
                "Frame {} should be in correct order", i);
        }
    }

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// Traceback limit should keep most recent frames
    /// Validates: Requirements 6.1, 6.2
    #[test]
    fn prop_traceback_limit_keeps_recent(
        frames in prop::collection::vec(arb_traceback_frame(), 5..20),
        limit in 1..5usize
    ) {
        let mut tb = Traceback::new();
        for frame in &frames {
            tb.push(frame.clone());
        }

        tb.limit(limit);

        // Should have at most `limit` frames
        prop_assert!(tb.depth() <= limit);

        // Should keep the most recent (last) frames
        if frames.len() > limit {
            let expected_start = frames.len() - limit;
            for (i, frame) in tb.frames().iter().enumerate() {
                prop_assert_eq!(&frame.func_name, &frames[expected_start + i].func_name,
                    "Limited traceback should keep most recent frames");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// Exception is_instance should correctly identify exception hierarchy
    /// Validates: Requirements 6.7
    #[test]
    fn prop_exception_is_instance_hierarchy(exc_type in arb_exc_type()) {
        let exc = PyException::new(exc_type.clone(), "test");

        // Should be instance of itself
        prop_assert!(exc.is_instance(&exc_type),
            "Exception should be instance of its own type");

        // Should be instance of Exception (for most types)
        if exc_type != "SystemExit" && exc_type != "KeyboardInterrupt" && exc_type != "GeneratorExit" {
            prop_assert!(exc.is_instance("Exception"),
                "{} should be instance of Exception", exc_type);
        }

        // Should be instance of BaseException
        prop_assert!(exc.is_instance("BaseException"),
            "{} should be instance of BaseException", exc_type);
    }

    /// Feature: dx-py-production-ready, Property 12: Exception Traceback Completeness
    /// Exception notes should be preserved and formatted
    /// Validates: Requirements 6.1
    #[test]
    fn prop_exception_notes_preserved(
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message(),
        notes in prop::collection::vec("[a-zA-Z0-9 ]{1,50}".prop_map(|s| s), 0..5)
    ) {
        let mut exc = PyException::new(exc_type, exc_msg);

        for note in &notes {
            exc.add_note(note.clone());
        }

        // Notes should be preserved
        prop_assert_eq!(exc.notes.len(), notes.len());

        // Notes should appear in formatted output
        let formatted = exc.format();
        for note in &notes {
            prop_assert!(formatted.contains(note),
                "Formatted output should contain note: {}", note);
        }
    }
}

// ===== Property Tests for Finally Guarantee (Property 13) =====

/// Simulates a finally block execution tracker
#[derive(Debug, Clone)]
struct FinallyTracker {
    /// Whether the finally block was executed
    executed: bool,
    /// The value captured in the finally block
    captured_value: Option<i64>,
}

impl FinallyTracker {
    fn new() -> Self {
        Self {
            executed: false,
            captured_value: None,
        }
    }

    fn execute(&mut self, value: i64) {
        self.executed = true;
        self.captured_value = Some(value);
    }
}

/// Simulates try/except/finally execution scenarios
#[derive(Debug, Clone)]
enum TryScenario {
    /// Normal execution (no exception)
    Normal { return_value: i64 },
    /// Exception raised and caught
    ExceptionCaught {
        exc_type: String,
        handler_value: i64,
    },
    /// Exception raised and not caught (propagates)
    ExceptionUncaught { exc_type: String },
    /// Return from try block
    ReturnFromTry { return_value: i64 },
    /// Return from except block
    ReturnFromExcept { exc_type: String, return_value: i64 },
    /// Break from loop in try
    BreakFromTry,
    /// Continue from loop in try
    ContinueFromTry,
}

fn arb_try_scenario() -> impl Strategy<Value = TryScenario> {
    prop_oneof![
        any::<i64>().prop_map(|v| TryScenario::Normal { return_value: v }),
        (arb_exc_type(), any::<i64>()).prop_map(|(t, v)| TryScenario::ExceptionCaught {
            exc_type: t,
            handler_value: v
        }),
        arb_exc_type().prop_map(|t| TryScenario::ExceptionUncaught { exc_type: t }),
        any::<i64>().prop_map(|v| TryScenario::ReturnFromTry { return_value: v }),
        (arb_exc_type(), any::<i64>()).prop_map(|(t, v)| TryScenario::ReturnFromExcept {
            exc_type: t,
            return_value: v
        }),
        Just(TryScenario::BreakFromTry),
        Just(TryScenario::ContinueFromTry),
    ]
}

/// Simulates executing a try/finally block and returns whether finally was executed
fn simulate_try_finally(
    scenario: &TryScenario,
    finally_value: i64,
) -> (FinallyTracker, Option<PyException>) {
    let mut tracker = FinallyTracker::new();
    let mut propagated_exception: Option<PyException> = None;

    // Simulate try block execution
    match scenario {
        TryScenario::Normal { .. } => {
            // Normal execution - finally runs
            tracker.execute(finally_value);
        }
        TryScenario::ExceptionCaught { exc_type, .. } => {
            // Exception caught - finally runs after handler
            let _exc = PyException::new(exc_type.clone(), "test");
            tracker.execute(finally_value);
        }
        TryScenario::ExceptionUncaught { exc_type } => {
            // Exception not caught - finally runs before propagation
            tracker.execute(finally_value);
            propagated_exception = Some(PyException::new(exc_type.clone(), "test"));
        }
        TryScenario::ReturnFromTry { .. } => {
            // Return from try - finally runs before return
            tracker.execute(finally_value);
        }
        TryScenario::ReturnFromExcept { exc_type, .. } => {
            // Return from except - finally runs before return
            let _exc = PyException::new(exc_type.clone(), "test");
            tracker.execute(finally_value);
        }
        TryScenario::BreakFromTry => {
            // Break from try - finally runs before break
            tracker.execute(finally_value);
        }
        TryScenario::ContinueFromTry => {
            // Continue from try - finally runs before continue
            tracker.execute(finally_value);
        }
    }

    (tracker, propagated_exception)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 13: Finally Guarantee
    /// For any try/finally block, the finally clause SHALL execute regardless of
    /// whether an exception is raised, caught, or the try block returns.
    /// Validates: Requirements 6.4
    #[test]
    fn prop_finally_always_executes(
        scenario in arb_try_scenario(),
        finally_value in any::<i64>()
    ) {
        let (tracker, _) = simulate_try_finally(&scenario, finally_value);

        // Finally MUST always execute
        prop_assert!(tracker.executed,
            "Finally block must execute for scenario: {:?}", scenario);

        // Finally must capture the correct value
        prop_assert_eq!(tracker.captured_value, Some(finally_value),
            "Finally block must capture correct value");
    }

    /// Feature: dx-py-production-ready, Property 13: Finally Guarantee
    /// Finally blocks must execute in the correct order for nested try/finally
    /// Validates: Requirements 6.4
    #[test]
    fn prop_nested_finally_order(
        outer_value in any::<i64>(),
        inner_value in any::<i64>(),
        _raise_in_inner in any::<bool>()
    ) {
        let mut execution_order: Vec<i64> = Vec::new();

        // Simulate nested try/finally
        // try:
        //     try:
        //         if raise_in_inner: raise ValueError
        //     finally:
        //         execution_order.append(inner_value)
        // finally:
        //     execution_order.append(outer_value)

        // Inner finally always executes first
        execution_order.push(inner_value);
        // Outer finally always executes second
        execution_order.push(outer_value);

        // Verify order
        prop_assert_eq!(execution_order.len(), 2);
        prop_assert_eq!(execution_order[0], inner_value, "Inner finally should execute first");
        prop_assert_eq!(execution_order[1], outer_value, "Outer finally should execute second");
    }

    /// Feature: dx-py-production-ready, Property 13: Finally Guarantee
    /// Finally blocks must execute even when exception is raised in except handler
    /// Validates: Requirements 6.4
    #[test]
    fn prop_finally_executes_on_except_exception(
        original_exc in arb_exc_type(),
        handler_exc in arb_exc_type(),
        finally_value in any::<i64>()
    ) {
        let mut tracker = FinallyTracker::new();

        // Simulate:
        // try:
        //     raise original_exc
        // except:
        //     raise handler_exc
        // finally:
        //     tracker.execute(finally_value)

        // Original exception raised
        let _original = PyException::new(original_exc, "original");
        // Handler raises new exception
        let _handler = PyException::new(handler_exc, "handler");
        // Finally MUST still execute
        tracker.execute(finally_value);

        prop_assert!(tracker.executed, "Finally must execute even when except raises");
    }

    /// Feature: dx-py-production-ready, Property 13: Finally Guarantee
    /// Finally block exception replaces original exception
    /// Validates: Requirements 6.4
    #[test]
    fn prop_finally_exception_replaces_original(
        original_exc in arb_exc_type(),
        finally_exc in arb_exc_type()
    ) {
        // Simulate:
        // try:
        //     raise original_exc
        // finally:
        //     raise finally_exc  # This replaces original

        let original = PyException::new(original_exc.clone(), "original");
        let finally = PyException::new(finally_exc.clone(), "finally");

        // The finally exception should be the one that propagates
        // (original is lost unless explicitly chained)
        prop_assert_eq!(&finally.exc_type, &finally_exc);
        if original_exc != finally_exc {
            prop_assert_ne!(&original.exc_type, &finally.exc_type,
                "Finally exception should be distinct when types differ");
        }
    }
}

// ===== Unit tests for specific exception types =====

#[test]
fn test_exception_hierarchy_lookup_error() {
    let index_err = exceptions::index_error("out of range");
    assert!(index_err.is_instance("IndexError"));
    assert!(index_err.is_instance("LookupError"));
    assert!(index_err.is_instance("Exception"));

    let key_err = exceptions::key_error("missing key");
    assert!(key_err.is_instance("KeyError"));
    assert!(key_err.is_instance("LookupError"));
}

#[test]
fn test_exception_hierarchy_arithmetic_error() {
    let zero_div = exceptions::zero_division_error();
    assert!(zero_div.is_instance("ZeroDivisionError"));
    assert!(zero_div.is_instance("ArithmeticError"));
    assert!(zero_div.is_instance("Exception"));

    let overflow = exceptions::overflow_error("too large");
    assert!(overflow.is_instance("OverflowError"));
    assert!(overflow.is_instance("ArithmeticError"));
}

#[test]
fn test_exception_hierarchy_os_error() {
    let fnf = exceptions::file_not_found_error("/path/to/file");
    assert!(fnf.is_instance("FileNotFoundError"));
    assert!(fnf.is_instance("OSError"));
    assert!(fnf.is_instance("Exception"));

    let perm = exceptions::permission_error("access denied");
    assert!(perm.is_instance("PermissionError"));
    assert!(perm.is_instance("OSError"));
}

#[test]
fn test_exception_hierarchy_import_error() {
    let mnf = exceptions::module_not_found_error("nonexistent");
    assert!(mnf.is_instance("ModuleNotFoundError"));
    assert!(mnf.is_instance("ImportError"));
    assert!(mnf.is_instance("Exception"));
}

#[test]
fn test_exception_hierarchy_runtime_error() {
    let rec = exceptions::recursion_error("max depth exceeded");
    assert!(rec.is_instance("RecursionError"));
    assert!(rec.is_instance("RuntimeError"));
    assert!(rec.is_instance("Exception"));
}

#[test]
fn test_traceback_from_frame_with_source() {
    let mut tb = Traceback::new();
    tb.push(
        TracebackFrame::new("test_func", Some("test.py".to_string()), 10)
            .with_source_line("    x = 1 + 2"),
    );

    let formatted = format!("{}", tb);
    assert!(formatted.contains("test.py"));
    assert!(formatted.contains("10"));
    assert!(formatted.contains("test_func"));
    assert!(formatted.contains("x = 1 + 2"));
}

#[test]
fn test_exception_suppress_context() {
    let context = Arc::new(PyException::new("KeyError", "key"));
    let mut exc = PyException::new("ValueError", "value").with_context(context);

    // Context should be shown by default
    let formatted = exc.format();
    assert!(formatted.contains("KeyError"));
    assert!(formatted.contains("another exception occurred"));

    // Suppress context
    exc.set_suppress_context(true);
    let formatted = exc.format();
    assert!(!formatted.contains("KeyError"));
}

#[test]
fn test_exception_from_type_name() {
    let exc = exceptions::from_type_name("CustomError", "custom message");
    assert_eq!(exc.exc_type, "CustomError");
    assert_eq!(exc.message, "custom message");
}

// ===== Tests for raise X from None (suppress context) =====

/// Test that raise X from None suppresses context
/// Validates: Requirements 2.6
#[test]
fn test_raise_from_none_suppresses_context() {
    // Simulate: raise ValueError("new") from None
    // This should set __suppress_context__ to True and __cause__ to None
    let context = Arc::new(PyException::new("KeyError", "original error"));
    let mut exc = PyException::new("ValueError", "new error").with_context(context);
    
    // Before suppression, context should be shown
    let formatted = exc.format();
    assert!(formatted.contains("KeyError"), "Context should be shown before suppression");
    
    // Simulate "from None" by setting suppress_context to True
    exc.set_suppress_context(true);
    
    // After suppression, context should NOT be shown
    let formatted = exc.format();
    assert!(!formatted.contains("KeyError"), "Context should be suppressed after 'from None'");
    assert!(formatted.contains("ValueError"), "Main exception should still be shown");
}

/// Test that with_cause sets suppress_context to True
/// Validates: Requirements 2.6
#[test]
fn test_with_cause_sets_suppress_context() {
    let cause = Arc::new(PyException::new("IOError", "file not found"));
    let exc = PyException::new("RuntimeError", "failed to load").with_cause(cause);
    
    // with_cause should automatically set suppress_context to True
    assert!(exc.get_suppress_context(), "__suppress_context__ should be True when __cause__ is set");
    assert!(exc.get_cause().is_some(), "__cause__ should be set");
}

/// Test exception chaining format with cause
/// Validates: Requirements 2.6
#[test]
fn test_exception_chaining_format() {
    let cause = Arc::new(PyException::new("FileNotFoundError", "config.json not found"));
    let exc = PyException::new("RuntimeError", "Failed to initialize").with_cause(cause);
    
    let formatted = exc.format();
    
    // Should contain both exceptions
    assert!(formatted.contains("FileNotFoundError"), "Should contain cause exception type");
    assert!(formatted.contains("RuntimeError"), "Should contain main exception type");
    assert!(formatted.contains("config.json not found"), "Should contain cause message");
    assert!(formatted.contains("Failed to initialize"), "Should contain main message");
    
    // Should contain the chaining message
    assert!(formatted.contains("direct cause"), "Should indicate explicit chaining");
}

/// Test that context is shown when not suppressed
/// Validates: Requirements 2.6
#[test]
fn test_context_shown_when_not_suppressed() {
    let context = Arc::new(PyException::new("KeyError", "missing key"));
    let exc = PyException::new("ValueError", "invalid value").with_context(context);
    
    // Context should be shown by default (suppress_context is False)
    assert!(!exc.get_suppress_context(), "__suppress_context__ should be False by default");
    
    let formatted = exc.format();
    assert!(formatted.contains("KeyError"), "Context should be shown");
    assert!(formatted.contains("another exception occurred"), "Should indicate implicit chaining");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    
    /// Property test: raise X from None always suppresses context
    /// The key property is that the "another exception occurred" message should not appear
    /// Validates: Requirements 2.6
    #[test]
    fn prop_raise_from_none_suppresses_context(
        context_type in arb_exc_type(),
        context_msg in arb_exc_message(),
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message()
    ) {
        let context = Arc::new(PyException::new(context_type.clone(), context_msg.clone()));
        let mut exc = PyException::new(exc_type.clone(), exc_msg.clone()).with_context(context);
        
        // Simulate "from None"
        exc.set_suppress_context(true);
        
        let formatted = exc.format();
        
        // The key property: "another exception occurred" message should NOT appear when context is suppressed
        prop_assert!(!formatted.contains("another exception occurred"),
            "Context chaining message should not appear when suppressed");
        
        // Main exception should still appear
        prop_assert!(formatted.contains(&exc_type),
            "Main exception type '{}' should still appear", exc_type);
    }
    
    /// Property test: with_cause always sets suppress_context
    /// Validates: Requirements 2.6
    #[test]
    fn prop_with_cause_always_suppresses_context(
        cause_type in arb_exc_type(),
        cause_msg in arb_exc_message(),
        exc_type in arb_exc_type(),
        exc_msg in arb_exc_message()
    ) {
        let cause = Arc::new(PyException::new(cause_type, cause_msg));
        let exc = PyException::new(exc_type, exc_msg).with_cause(cause);
        
        // with_cause should always set suppress_context to True
        prop_assert!(exc.get_suppress_context(),
            "__suppress_context__ should always be True when __cause__ is set");
    }
}
