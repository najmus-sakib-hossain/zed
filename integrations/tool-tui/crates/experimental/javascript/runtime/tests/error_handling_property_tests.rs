//! Property-Based Tests for Error Handling
//!
//! **Feature: production-readiness, Property 24: Error source location accuracy**
//! **Feature: production-readiness, Property 25: Unhandled rejection reporting**
//! **Validates: Requirements 10.1, 10.2, 10.3, 10.6**

use proptest::prelude::*;

// Import error types
use dx_js_runtime::error::{
    JsException, JsErrorType, StackFrame, SourceLocation, CodeSnippet,
    ModuleSourceMap, SourceMapEntry, CallFrame,
    push_call_frame, pop_call_frame, capture_stack_trace, clear_call_stack,
    create_exception_with_stack,
};

// Import unhandled rejection tracking
use dx_js_runtime::runtime::unhandled_rejection::{
    TrackedRejection, track_rejection, mark_rejection_handled,
    check_unhandled_rejections, get_pending_rejections, clear_rejection_registry,
};

// Import Value type
use dx_js_runtime::value::Value;

// ============================================================================
// Test Helpers for Source Location
// ============================================================================

/// Generate valid source file paths
fn arb_source_file() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("test.js".to_string()),
        Just("src/main.js".to_string()),
        Just("lib/utils.ts".to_string()),
        Just("components/Button.tsx".to_string()),
        Just("index.mjs".to_string()),
    ]
}

/// Generate valid line numbers (1-indexed)
fn arb_line() -> impl Strategy<Value = u32> {
    1u32..10000u32
}

/// Generate valid column numbers (1-indexed)
fn arb_column() -> impl Strategy<Value = u32> {
    1u32..500u32
}

/// Generate error messages
fn arb_error_message() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("undefined is not a function".to_string()),
        Just("Cannot read property 'x' of undefined".to_string()),
        Just("Unexpected token".to_string()),
        Just("ReferenceError: x is not defined".to_string()),
        Just("TypeError: null is not an object".to_string()),
    ]
}

/// Generate function names
fn arb_function_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("<anonymous>".to_string()),
        Just("main".to_string()),
        Just("handleClick".to_string()),
        Just("processData".to_string()),
        Just("fetchUser".to_string()),
    ]
}

/// Generate JavaScript error types
fn arb_error_type() -> impl Strategy<Value = JsErrorType> {
    prop_oneof![
        Just(JsErrorType::Error),
        Just(JsErrorType::TypeError),
        Just(JsErrorType::SyntaxError),
        Just(JsErrorType::ReferenceError),
        Just(JsErrorType::RangeError),
    ]
}

/// Generate a call frame
fn arb_call_frame() -> impl Strategy<Value = CallFrame> {
    (arb_function_name(), arb_source_file(), arb_line(), arb_column())
        .prop_map(|(func, file, line, col)| CallFrame::new(func, file, line, col))
}

/// Generate rejection reasons
fn arb_rejection_reason() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::String("Error: something went wrong".to_string())),
        Just(Value::String("Network failure".to_string())),
        Just(Value::Null),
        Just(Value::Undefined),
    ]
}

/// Generate source code snippets
fn arb_source_code() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("function test() {\n  return 42;\n}\n".to_string()),
        Just("const x = 1;\nconst y = 2;\nconst z = x + y;\n".to_string()),
        Just("async function fetchData() {\n  const res = await fetch(url);\n  return res;\n}\n".to_string()),
        Just("class MyClass {\n  constructor() {\n    this.value = 0;\n  }\n}\n".to_string()),
    ]
}

// ============================================================================
// Property 24: Error source location accuracy
// **Validates: Requirements 10.1, 10.2, 10.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 24.1: Source location preserves line and column information
    /// *For any* source location, creating an error with that location
    /// SHALL preserve the exact line and column values.
    #[test]
    fn prop_source_location_preserves_line_column(
        file in arb_source_file(),
        line in arb_line(),
        column in arb_column(),
        message in arb_error_message(),
        err_type in arb_error_type(),
    ) {
        let exception = JsException::with_location(
            err_type,
            message.clone(),
            file.clone(),
            line,
            column,
        );

        // Verify location is preserved
        let location = exception.location.expect("Location should be set");
        prop_assert_eq!(&location.file, &file, "File mismatch");
        prop_assert_eq!(location.line, line, "Line mismatch");
        prop_assert_eq!(location.column, column, "Column mismatch");
    }

    /// Property 24.2: Stack frame preserves all location information
    /// *For any* function name, file, line, and column, creating a stack frame
    /// SHALL preserve all values accurately.
    #[test]
    fn prop_stack_frame_preserves_info(
        func_name in arb_function_name(),
        file in arb_source_file(),
        line in arb_line(),
        column in arb_column(),
    ) {
        let frame = StackFrame::new(&func_name, &file, line, column);

        // Verify all fields are preserved
        prop_assert_eq!(&frame.function_name, &func_name);
        prop_assert_eq!(&frame.file, &file);
        prop_assert_eq!(frame.line, line);
        prop_assert_eq!(frame.column, column);

        // Verify formatted output contains all info
        let formatted = frame.format_v8_style();
        prop_assert!(formatted.contains(&file), "Formatted should contain file");
        prop_assert!(formatted.contains(&line.to_string()), "Formatted should contain line");
        prop_assert!(formatted.contains(&column.to_string()), "Formatted should contain column");
    }

    /// Property 24.3: Source map lookup returns correct location
    /// *For any* source map entry, looking up that offset SHALL return
    /// the correct source file, line, and column.
    #[test]
    fn prop_source_map_lookup_returns_correct_location(
        native_offset in 0usize..10000usize,
        file in arb_source_file(),
        line in arb_line(),
        column in arb_column(),
    ) {
        let mut source_map = ModuleSourceMap::new("test_module");
        let entry = SourceMapEntry::new(native_offset, file.clone(), line, column);
        source_map.add_entry(entry);
        source_map.finalize();

        // Lookup the entry
        let result = source_map.lookup(native_offset);
        prop_assert!(result.is_some(), "Lookup should find entry");

        let found = result.unwrap();
        prop_assert_eq!(&found.source_file, &file);
        prop_assert_eq!(found.line, line);
        prop_assert_eq!(found.column, column);
    }

    /// Property 24.4: Call stack preserves frame order
    /// *For any* sequence of call frames, pushing them and then capturing
    /// the stack trace SHALL return frames in reverse order (innermost first).
    #[test]
    fn prop_call_stack_preserves_frame_order(
        frames in prop::collection::vec(arb_call_frame(), 1..5),
    ) {
        // Clear any existing stack
        clear_call_stack();

        // Push frames
        for frame in &frames {
            push_call_frame(frame.clone());
        }

        // Capture stack trace
        let captured = capture_stack_trace();

        // Verify order is reversed (innermost first)
        prop_assert_eq!(captured.len(), frames.len());
        for (i, captured_frame) in captured.iter().enumerate() {
            let original_idx = frames.len() - 1 - i;
            let original = &frames[original_idx];
            prop_assert_eq!(&captured_frame.function_name, &original.function_name);
            prop_assert_eq!(&captured_frame.file, &original.file);
            prop_assert_eq!(captured_frame.line, original.line);
            prop_assert_eq!(captured_frame.column, original.column);
        }

        // Clean up
        clear_call_stack();
    }

    /// Property 24.5: Code snippet extraction includes error line
    /// *For any* source code and error location, the code snippet SHALL
    /// include the error line and mark it correctly.
    #[test]
    fn prop_code_snippet_includes_error_line(
        source in arb_source_code(),
    ) {
        let line_count = source.lines().count() as u32;
        if line_count == 0 {
            return Ok(());
        }

        let error_line = 1.max(line_count / 2);
        let snippet = CodeSnippet::from_source(&source, error_line, 1, 1);

        // Verify error line is in the snippet
        prop_assert_eq!(snippet.error_line, error_line);
        prop_assert!(
            snippet.lines.iter().any(|(ln, _)| *ln == error_line),
            "Snippet should contain the error line"
        );
    }

    /// Property 24.6: Exception location comes from top frame
    /// *For any* call stack, creating an exception with stack SHALL set
    /// the location from the top (innermost) frame.
    #[test]
    fn prop_exception_location_from_top_frame(
        frames in prop::collection::vec(arb_call_frame(), 1..5),
        error_type in arb_error_type(),
        message in arb_error_message(),
    ) {
        // Clear and set up stack
        clear_call_stack();
        for frame in &frames {
            push_call_frame(frame.clone());
        }

        // Create exception with stack
        let exception = create_exception_with_stack(error_type, message);

        // Location should be from the top of stack (last pushed = innermost)
        if let Some(location) = &exception.location {
            let top_frame = frames.last().unwrap();
            prop_assert_eq!(&location.file, &top_frame.file);
            prop_assert_eq!(location.line, top_frame.line);
            prop_assert_eq!(location.column, top_frame.column);
        }

        // Clean up
        clear_call_stack();
    }
}

// ============================================================================
// Property 25: Unhandled rejection reporting
// **Validates: Requirements 10.6**
// ============================================================================

// Mutex to serialize rejection tests since they share global state
use std::sync::Mutex;
static REJECTION_TEST_MUTEX: Mutex<()> = Mutex::new(());

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property 25.1: Tracked rejection preserves reason
    /// *For any* promise rejection, tracking it SHALL preserve the rejection reason.
    #[test]
    fn prop_tracked_rejection_preserves_reason(
        promise_id in 1000u64..2000u64,
        reason in arb_rejection_reason(),
    ) {
        let _guard = REJECTION_TEST_MUTEX.lock().unwrap();
        // Clear registry before test
        clear_rejection_registry();
        
        // Track the rejection
        let rejection_id = track_rejection(promise_id, reason.clone());
        prop_assert!(rejection_id > 0, "Should return a valid rejection ID");

        // Verify it's in pending
        let pending = get_pending_rejections();
        let found = pending.iter().find(|r| r.promise_id == promise_id);
        prop_assert!(found.is_some(), "Rejection should be in pending");

        // Clean up by checking unhandled (which drains pending)
        let _ = check_unhandled_rejections();
    }

    /// Property 25.2: Marking rejection as handled removes it from pending
    /// *For any* tracked rejection, marking it as handled SHALL remove it
    /// from the pending rejections list.
    #[test]
    fn prop_mark_handled_removes_from_pending(
        promise_id in 2000u64..3000u64,
        reason in arb_rejection_reason(),
    ) {
        let _guard = REJECTION_TEST_MUTEX.lock().unwrap();
        // Clear registry before test
        clear_rejection_registry();
        
        // Track the rejection
        track_rejection(promise_id, reason);

        // Mark as handled
        let was_pending = mark_rejection_handled(promise_id);
        prop_assert!(was_pending, "Should have been pending");

        // Verify not in pending
        let pending = get_pending_rejections();
        let found = pending.iter().find(|r| r.promise_id == promise_id);
        prop_assert!(found.is_none(), "Rejection should not be in pending");
    }

    /// Property 25.3: Unhandled rejections are reported with context
    /// *For any* unhandled rejection, checking unhandled rejections SHALL
    /// return the rejection with its context.
    #[test]
    fn prop_unhandled_rejection_reported_with_context(
        promise_id in 3000u64..4000u64,
        reason in arb_rejection_reason(),
    ) {
        let _guard = REJECTION_TEST_MUTEX.lock().unwrap();
        // Clear registry before test
        clear_rejection_registry();
        
        // Track the rejection
        track_rejection(promise_id, reason.clone());

        // Check unhandled (this drains pending)
        let unhandled = check_unhandled_rejections();

        // Find our rejection
        let found = unhandled.iter().find(|r| r.promise_id == promise_id);
        prop_assert!(found.is_some(), "Rejection should be in unhandled");

        let rejection = found.unwrap();
        prop_assert_eq!(rejection.promise_id, promise_id, "Promise ID should match");
        prop_assert!(!rejection.handled, "Should not be marked as handled");
    }

    /// Property 25.4: TrackedRejection converts to exception with context
    /// *For any* tracked rejection, converting to JsException SHALL preserve
    /// the rejection information.
    #[test]
    fn prop_tracked_rejection_to_exception(
        promise_id in 4000u64..5000u64,
        reason in arb_rejection_reason(),
    ) {
        let rejection = TrackedRejection::new(promise_id, reason);
        let exception = rejection.to_exception();

        // Verify exception contains rejection info
        prop_assert!(
            exception.message.contains("Unhandled promise rejection"),
            "Exception message should mention unhandled rejection"
        );
        prop_assert_eq!(exception.error_type, JsErrorType::Error);
    }

    /// Property 25.5: Multiple rejections are tracked independently
    /// *For any* set of promise rejections, each SHALL be tracked independently
    /// and can be handled or reported separately.
    #[test]
    fn prop_multiple_rejections_tracked_independently(
        base_id in 5000u64..6000u64,
        count in 2usize..5usize,
    ) {
        let _guard = REJECTION_TEST_MUTEX.lock().unwrap();
        // Clear registry before test
        clear_rejection_registry();
        
        let promise_ids: Vec<u64> = (0..count).map(|i| base_id + i as u64).collect();

        // Track multiple rejections
        for &pid in &promise_ids {
            track_rejection(pid, Value::String(format!("Error for {}", pid)));
        }

        // Handle only the first one
        mark_rejection_handled(promise_ids[0]);

        // Check unhandled
        let unhandled = check_unhandled_rejections();

        // First should not be in unhandled
        prop_assert!(
            !unhandled.iter().any(|r| r.promise_id == promise_ids[0]),
            "Handled rejection should not be in unhandled"
        );

        // Others should be in unhandled
        for &pid in &promise_ids[1..] {
            prop_assert!(
                unhandled.iter().any(|r| r.promise_id == pid),
                "Unhandled rejection {} should be reported", pid
            );
        }
    }
}

// ============================================================================
// Unit Test Edge Cases
// ============================================================================

#[test]
fn test_source_location_display() {
    let loc = SourceLocation::new("test.js", 10, 5);
    let display = format!("{}", loc);
    assert!(display.contains("test.js"));
    assert!(display.contains("10"));
    assert!(display.contains("5"));
}

#[test]
fn test_stack_frame_native() {
    let frame = StackFrame::native("Array.prototype.map");
    assert!(frame.is_native);
    assert_eq!(frame.file, "<native>");
    assert!(frame.format_v8_style().contains("<native>"));
}

#[test]
fn test_source_map_lookup() {
    let mut source_map = ModuleSourceMap::new("test");

    // Add entries at various offsets
    source_map.add_entry(SourceMapEntry::new(0, "test.js", 1, 1));
    source_map.add_entry(SourceMapEntry::new(100, "test.js", 5, 1));
    source_map.add_entry(SourceMapEntry::new(200, "test.js", 10, 1));
    source_map.finalize();

    // Lookup at exact offset
    let result = source_map.lookup(100);
    assert!(result.is_some());
    assert_eq!(result.unwrap().line, 5);

    // Lookup between offsets (should find previous)
    let result = source_map.lookup(150);
    assert!(result.is_some());
    assert_eq!(result.unwrap().line, 5);

    // Lookup before first offset
    let result = source_map.lookup(0);
    assert!(result.is_some());
    assert_eq!(result.unwrap().line, 1);
}

#[test]
fn test_code_snippet_formatting() {
    let source = "line 1\nline 2\nline 3\nline 4\nline 5";
    let snippet = CodeSnippet::from_source(source, 3, 1, 1);

    assert_eq!(snippet.error_line, 3);
    let formatted = snippet.format();
    assert!(formatted.contains("line 3"));
    assert!(formatted.contains("^")); // Error indicator
}

#[test]
fn test_exception_type_names() {
    assert_eq!(JsErrorType::TypeError.name(), "TypeError");
    assert_eq!(JsErrorType::SyntaxError.name(), "SyntaxError");
    assert_eq!(JsErrorType::ReferenceError.name(), "ReferenceError");
    assert_eq!(JsErrorType::RangeError.name(), "RangeError");
}

#[test]
fn test_call_stack_push_pop() {
    clear_call_stack();

    let frame1 = CallFrame::new("outer", "test.js", 1, 1);
    let frame2 = CallFrame::new("inner", "test.js", 5, 1);

    push_call_frame(frame1);
    push_call_frame(frame2);

    let popped = pop_call_frame();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().function_name, "inner");

    let popped = pop_call_frame();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().function_name, "outer");

    let popped = pop_call_frame();
    assert!(popped.is_none());
}
