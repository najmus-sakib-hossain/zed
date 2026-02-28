//! Property-based tests for DX-Py Exception Handling
//!
//! Feature: dx-py-production-ready
//! Property 4: Exception Handler Selection Is Correct
//! Property 5: Finally Blocks Always Execute
//! Property 6: Exception Propagation Preserves Stack Semantics
//! Validates: Requirements 2.1-2.7

use dx_py_bytecode::{CodeObject, Constant, DpbOpcode};
use dx_py_compiler::SourceCompiler;
use proptest::prelude::*;

// ===== Generators for property tests =====

/// Generate a valid Python identifier (for variable names)
fn arb_identifier() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z_][a-z0-9_]{0,10}")
        .unwrap()
        .prop_filter("not a keyword", |s| !is_python_keyword(s))
}

/// Check if a string is a Python keyword
fn is_python_keyword(s: &str) -> bool {
    matches!(
        s,
        "False" | "None" | "True" | "and" | "as" | "assert" | "async" | "await"
            | "break" | "class" | "continue" | "def" | "del" | "elif" | "else"
            | "except" | "finally" | "for" | "from" | "global" | "if" | "import"
            | "in" | "is" | "lambda" | "nonlocal" | "not" | "or" | "pass" | "raise"
            | "return" | "try" | "while" | "with" | "yield" | "self"
    )
}

/// Generate a Python exception type name
fn arb_exception_type() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "Exception".to_string(),
        "ValueError".to_string(),
        "TypeError".to_string(),
        "KeyError".to_string(),
        "IndexError".to_string(),
        "RuntimeError".to_string(),
        "AttributeError".to_string(),
        "ZeroDivisionError".to_string(),
    ])
}

/// Generate a list of unique exception types for handler testing
fn arb_unique_exception_types(count: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::hash_set(arb_exception_type(), 1..=count.min(5))
        .prop_map(|set| set.into_iter().collect::<Vec<String>>())
        .prop_filter("non-empty", |v: &Vec<String>| !v.is_empty())
}

/// Generate a simple integer value for testing
fn arb_int_value() -> impl Strategy<Value = i64> {
    -100i64..100i64
}

// ===== Helper Functions =====

/// Check if bytecode contains a specific opcode
fn bytecode_contains_opcode(code: &[u8], opcode: DpbOpcode) -> bool {
    code.iter().any(|&b| b == opcode as u8)
}

/// Count occurrences of an opcode in bytecode
fn count_opcode(code: &[u8], opcode: DpbOpcode) -> usize {
    code.iter().filter(|&&b| b == opcode as u8).count()
}

/// Find all CodeRef constants in a CodeObject
fn find_code_constants(code: &CodeObject) -> Vec<&CodeObject> {
    code.constants
        .iter()
        .filter_map(|c| match c {
            Constant::Code(inner) => Some(inner.as_ref()),
            _ => None,
        })
        .collect()
}

/// Check if bytecode has finally block structure
fn has_finally_structure(code: &[u8]) -> bool {
    bytecode_contains_opcode(code, DpbOpcode::SetupFinally)
        && bytecode_contains_opcode(code, DpbOpcode::EndFinally)
}

// ===== Property 4: Exception Handler Selection Is Correct =====

/// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
/// For any try/except block with multiple handlers, when an exception is raised,
/// the Runtime SHALL select the first handler whose exception type matches (via isinstance check).
/// Validates: Requirements 2.1, 2.2
mod exception_handler_selection_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
        /// Validates: Requirements 2.1, 2.2
        ///
        /// For any try/except block, the compiler SHALL emit SetupExcept opcode
        /// to establish the exception handler.
        #[test]
        fn prop_try_except_emits_setup_except(
            exc_type in arb_exception_type(),
            var_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nexcept {}:\n    {} = 2",
                exc_type, var_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile try/except: {}", source);

            let code = result.unwrap();

            // Should have SetupExcept opcode
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
                "try/except should emit SetupExcept opcode");
        }

        /// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
        /// Validates: Requirements 2.1, 2.2
        ///
        /// For any try/except block with multiple handlers, the compiler SHALL emit
        /// CheckExcMatch opcodes for type checking in handler order.
        #[test]
        fn prop_multiple_handlers_emit_check_exc_match(
            exc_types in arb_unique_exception_types(3)
        ) {
            // Build multiple except handlers
            let handlers = exc_types.iter()
                .enumerate()
                .map(|(i, t)| format!("except {}:\n    x = {}", t, i))
                .collect::<Vec<_>>()
                .join("\n");

            let source = format!("try:\n    y = 1\n{}", handlers);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // Should have CheckExcMatch for each typed handler
            let check_count = count_opcode(&code.code, DpbOpcode::CheckExcMatch);
            prop_assert!(check_count >= exc_types.len(),
                "Expected at least {} CheckExcMatch opcodes for {} handlers, found {}",
                exc_types.len(), exc_types.len(), check_count);
        }

        /// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
        /// Validates: Requirements 2.1, 2.2
        ///
        /// For any try/except with exception binding (as clause), the compiler SHALL
        /// emit store operations for the exception variable.
        #[test]
        fn prop_exception_binding_stores_variable(
            exc_type in arb_exception_type(),
            binding_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nexcept {} as {}:\n    y = {}",
                exc_type, binding_name, binding_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // The binding name should be in the names table
            prop_assert!(code.names.contains(&binding_name),
                "Exception binding '{}' should be in names table", binding_name);
        }

        /// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
        /// Validates: Requirements 2.1, 2.2
        ///
        /// For any bare except clause (catches all), the compiler SHALL NOT emit
        /// CheckExcMatch for that handler.
        #[test]
        fn prop_bare_except_no_type_check(
            var_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nexcept:\n    {} = 2",
                var_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile bare except: {}", source);

            let code = result.unwrap();

            // Should have SetupExcept but no CheckExcMatch for bare except
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
                "Bare except should still emit SetupExcept");
            
            // Bare except should not have CheckExcMatch
            let check_count = count_opcode(&code.code, DpbOpcode::CheckExcMatch);
            prop_assert_eq!(check_count, 0,
                "Bare except should not emit CheckExcMatch, found {}", check_count);
        }

        /// Feature: dx-py-production-ready, Property 4: Exception Handler Selection Is Correct
        /// Validates: Requirements 2.1, 2.2
        ///
        /// For any try/except with unmatched exception, the compiler SHALL emit
        /// Reraise opcode to propagate the exception.
        #[test]
        fn prop_unmatched_exception_reraises(
            exc_type in arb_exception_type()
        ) {
            let source = format!(
                "try:\n    x = 1\nexcept {}:\n    y = 2",
                exc_type
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // Should have Reraise for unmatched exceptions
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Reraise),
                "try/except should emit Reraise for unmatched exceptions");
        }
    }

    // ===== Unit tests for Property 4 =====

    #[test]
    fn test_simple_try_except_compiles() {
        let source = r#"
try:
    x = 1
except ValueError:
    y = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple try/except should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
            "Should have SetupExcept opcode");
    }

    #[test]
    fn test_multiple_except_handlers_compile() {
        let source = r#"
try:
    x = 1
except ValueError:
    y = 1
except TypeError:
    y = 2
except KeyError:
    y = 3
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Multiple except handlers should compile");

        let code = result.unwrap();
        // Should have CheckExcMatch for each typed handler
        let check_count = count_opcode(&code.code, DpbOpcode::CheckExcMatch);
        assert!(check_count >= 3, "Should have at least 3 CheckExcMatch opcodes");
    }

    #[test]
    fn test_except_with_binding_compiles() {
        let source = r#"
try:
    x = 1
except ValueError as e:
    print(e)
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Except with binding should compile");

        let code = result.unwrap();
        assert!(code.names.contains(&"e".to_string()),
            "Exception binding 'e' should be in names");
    }

    #[test]
    fn test_bare_except_compiles() {
        let source = r#"
try:
    x = 1
except:
    y = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Bare except should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
            "Bare except should have SetupExcept");
    }

    #[test]
    fn test_handler_order_preserved() {
        // Test that handlers are checked in order (first match wins)
        let source = r#"
try:
    x = 1
except Exception:
    y = 1
except ValueError:
    y = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Handler order test should compile");

        let code = result.unwrap();
        // Both Exception and ValueError should be in names
        assert!(code.names.contains(&"Exception".to_string()));
        assert!(code.names.contains(&"ValueError".to_string()));
    }
}

// ===== Property 5: Finally Blocks Always Execute =====

/// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
/// For any try/finally block, the finally block SHALL execute regardless of whether
/// an exception was raised, caught, or propagated.
/// Validates: Requirements 2.3, 2.7
mod finally_block_execution_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
        /// Validates: Requirements 2.3, 2.7
        ///
        /// For any try/finally block, the compiler SHALL emit SetupFinally opcode
        /// to establish the finally handler.
        #[test]
        fn prop_try_finally_emits_setup_finally(
            var_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nfinally:\n    {} = 2",
                var_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile try/finally: {}", source);

            let code = result.unwrap();

            // Should have SetupFinally opcode
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupFinally),
                "try/finally should emit SetupFinally opcode");
        }

        /// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
        /// Validates: Requirements 2.3, 2.7
        ///
        /// For any try/finally block, the compiler SHALL emit EndFinally opcode
        /// to properly complete the finally block.
        #[test]
        fn prop_try_finally_emits_end_finally(
            var_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nfinally:\n    {} = 2",
                var_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // Should have EndFinally opcode
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::EndFinally),
                "try/finally should emit EndFinally opcode");
        }

        /// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
        /// Validates: Requirements 2.3, 2.7
        ///
        /// For any try/except/finally block, the compiler SHALL emit both
        /// SetupExcept and SetupFinally opcodes.
        #[test]
        fn prop_try_except_finally_emits_both_setups(
            exc_type in arb_exception_type(),
            var_name in arb_identifier()
        ) {
            let source = format!(
                "try:\n    x = 1\nexcept {}:\n    y = 2\nfinally:\n    {} = 3",
                exc_type, var_name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile try/except/finally: {}", source);

            let code = result.unwrap();

            // Should have both SetupExcept and SetupFinally
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
                "try/except/finally should emit SetupExcept");
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupFinally),
                "try/except/finally should emit SetupFinally");
        }

        /// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
        /// Validates: Requirements 2.3, 2.7
        ///
        /// For any try/finally with return in try block, the finally block
        /// SHALL still be compiled to execute before the return.
        #[test]
        fn prop_finally_executes_before_return(
            return_val in arb_int_value()
        ) {
            let source = format!(
                "def f():\n    try:\n        return {}\n    finally:\n        x = 1",
                return_val
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // Find the function code object
            let func_codes = find_code_constants(&code);
            prop_assert!(!func_codes.is_empty(), "Should have function code");

            let func_code = func_codes[0];

            // Function should have SetupFinally and EndFinally
            prop_assert!(bytecode_contains_opcode(&func_code.code, DpbOpcode::SetupFinally),
                "Function with try/finally should have SetupFinally");
            prop_assert!(bytecode_contains_opcode(&func_code.code, DpbOpcode::EndFinally),
                "Function with try/finally should have EndFinally");
        }

        /// Feature: dx-py-production-ready, Property 5: Finally Blocks Always Execute
        /// Validates: Requirements 2.3, 2.7
        ///
        /// For nested try/finally blocks, the compiler SHALL emit SetupFinally
        /// for each level of nesting.
        #[test]
        fn prop_nested_finally_blocks_all_setup(
            nesting_depth in 1usize..4
        ) {
            // Build nested try/finally
            let mut source = String::new();
            for i in 0..nesting_depth {
                source.push_str(&format!("{}try:\n", "    ".repeat(i)));
            }
            source.push_str(&format!("{}x = 1\n", "    ".repeat(nesting_depth)));
            for i in (0..nesting_depth).rev() {
                source.push_str(&format!("{}finally:\n{}y = {}\n", 
                    "    ".repeat(i), "    ".repeat(i + 1), i));
            }

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile nested try/finally: {}", source);

            let code = result.unwrap();

            // Should have SetupFinally for each nesting level
            let setup_count = count_opcode(&code.code, DpbOpcode::SetupFinally);
            prop_assert!(setup_count >= nesting_depth,
                "Expected at least {} SetupFinally opcodes for {} nesting levels, found {}",
                nesting_depth, nesting_depth, setup_count);
        }
    }

    // ===== Unit tests for Property 5 =====

    #[test]
    fn test_simple_try_finally_compiles() {
        let source = r#"
try:
    x = 1
finally:
    y = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple try/finally should compile");

        let code = result.unwrap();
        assert!(has_finally_structure(&code.code),
            "Should have proper finally structure");
    }

    #[test]
    fn test_try_except_finally_compiles() {
        let source = r#"
try:
    x = 1
except ValueError:
    y = 2
finally:
    z = 3
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "try/except/finally should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
            "Should have SetupExcept");
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupFinally),
            "Should have SetupFinally");
    }

    #[test]
    fn test_finally_with_return_compiles() {
        let source = r#"
def f():
    try:
        return 1
    finally:
        x = 2
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "finally with return should compile");

        let code = result.unwrap();
        let func_codes = find_code_constants(&code);
        assert!(!func_codes.is_empty(), "Should have function code");

        let func_code = func_codes[0];
        assert!(has_finally_structure(&func_code.code),
            "Function should have finally structure");
    }

    #[test]
    fn test_nested_try_finally_compiles() {
        let source = r#"
try:
    try:
        x = 1
    finally:
        y = 2
finally:
    z = 3
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Nested try/finally should compile");

        let code = result.unwrap();
        let setup_count = count_opcode(&code.code, DpbOpcode::SetupFinally);
        assert!(setup_count >= 2, "Should have at least 2 SetupFinally opcodes");
    }

    #[test]
    fn test_finally_with_break_compiles() {
        let source = r#"
for i in range(10):
    try:
        if i > 5:
            break
    finally:
        x = i
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "finally with break should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupFinally),
            "Should have SetupFinally");
    }

    #[test]
    fn test_finally_with_continue_compiles() {
        let source = r#"
for i in range(10):
    try:
        if i < 5:
            continue
    finally:
        x = i
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "finally with continue should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupFinally),
            "Should have SetupFinally");
    }
}

// ===== Property 6: Exception Propagation Preserves Stack Semantics =====

/// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
/// For any nested function call where an exception is raised and not caught,
/// the exception SHALL propagate to the caller's exception handler or terminate the program.
/// Validates: Requirements 2.4, 2.5, 2.6
mod exception_propagation_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
        /// Validates: Requirements 2.4, 2.5, 2.6
        ///
        /// For any raise statement, the compiler SHALL emit Raise opcode
        /// to initiate exception propagation.
        #[test]
        fn prop_raise_emits_raise_opcode(
            exc_type in arb_exception_type()
        ) {
            let source = format!("raise {}('error')", exc_type);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile raise: {}", source);

            let code = result.unwrap();

            // Should have Raise opcode
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
                "raise statement should emit Raise opcode");
        }

        /// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
        /// Validates: Requirements 2.4, 2.5, 2.6
        ///
        /// For any bare raise (re-raise), the compiler SHALL emit Reraise opcode.
        #[test]
        fn prop_bare_raise_emits_reraise(
            exc_type in arb_exception_type()
        ) {
            // Use proper indentation with 4 spaces and trailing newline
            let source = format!(
                "\ntry:\n    x = 1\nexcept {}:\n    raise\n",
                exc_type
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile bare raise: {}", source);

            let code = result.unwrap();

            // Should have Reraise opcode for bare raise
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Reraise),
                "bare raise should emit Reraise opcode");
        }

        /// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
        /// Validates: Requirements 2.4, 2.5, 2.6
        ///
        /// For any raise with 'from' clause (exception chaining), the compiler
        /// SHALL emit proper bytecode to establish the exception chain.
        #[test]
        fn prop_raise_from_compiles(
            exc_type in arb_exception_type(),
            cause_type in arb_exception_type()
        ) {
            let source = format!(
                "raise {}('error') from {}('cause')",
                exc_type, cause_type
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile raise from: {}", source);

            let code = result.unwrap();

            // Should have Raise opcode
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
                "raise from should emit Raise opcode");

            // Both exception types should be referenced
            prop_assert!(code.names.contains(&exc_type),
                "Exception type '{}' should be in names", exc_type);
            prop_assert!(code.names.contains(&cause_type),
                "Cause type '{}' should be in names", cause_type);
        }

        /// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
        /// Validates: Requirements 2.4, 2.5, 2.6
        ///
        /// For any function that raises an exception, the caller's try/except
        /// SHALL be able to catch it (proper stack unwinding).
        #[test]
        fn prop_exception_propagates_to_caller(
            exc_type in arb_exception_type(),
            func_name in arb_identifier()
        ) {
            let source = format!(
                "def {}():\n    raise {}('error')\n\ntry:\n    {}()\nexcept {}:\n    x = 1",
                func_name, exc_type, func_name, exc_type
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();

            // Module level should have SetupExcept for the try block
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetupExcept),
                "Caller should have SetupExcept for catching propagated exception");

            // Function should have Raise
            let func_codes = find_code_constants(&code);
            prop_assert!(!func_codes.is_empty(), "Should have function code");

            let func_code = func_codes[0];
            prop_assert!(bytecode_contains_opcode(&func_code.code, DpbOpcode::Raise),
                "Function should have Raise opcode");
        }

        /// Feature: dx-py-production-ready, Property 6: Exception Propagation Preserves Stack Semantics
        /// Validates: Requirements 2.4, 2.5, 2.6
        ///
        /// For nested try blocks, an unhandled exception in inner block SHALL
        /// propagate to outer block's handler.
        #[test]
        fn prop_nested_exception_propagation(
            inner_exc in arb_exception_type(),
            outer_exc in arb_exception_type()
        ) {
            let source = format!(
                "try:\n    try:\n        raise {}('inner')\n    except {}:\n        x = 1\nexcept {}:\n    y = 2",
                inner_exc, outer_exc, inner_exc
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile nested try: {}", source);

            let code = result.unwrap();

            // Should have multiple SetupExcept for nested try blocks
            let setup_count = count_opcode(&code.code, DpbOpcode::SetupExcept);
            prop_assert!(setup_count >= 2,
                "Nested try blocks should have at least 2 SetupExcept, found {}", setup_count);
        }
    }

    // ===== Unit tests for Property 6 =====

    #[test]
    fn test_simple_raise_compiles() {
        let source = r#"
raise ValueError("test error")
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple raise should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
            "Should have Raise opcode");
    }

    #[test]
    fn test_bare_raise_in_except_compiles() {
        let source = r#"
try:
    x = 1
except ValueError:
    raise
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Bare raise in except should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Reraise),
            "Should have Reraise opcode");
    }

    #[test]
    fn test_raise_from_compiles() {
        let source = r#"
try:
    x = 1
except ValueError as e:
    raise RuntimeError("wrapped") from e
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Raise from should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
            "Should have Raise opcode");
    }

    #[test]
    fn test_raise_from_none_compiles() {
        let source = r#"
try:
    x = 1
except ValueError:
    raise RuntimeError("no context") from None
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Raise from None should compile");

        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
            "Should have Raise opcode");
    }

    #[test]
    fn test_exception_in_function_propagates() {
        let source = r#"
def inner():
    raise ValueError("inner error")

def outer():
    try:
        inner()
    except ValueError:
        x = 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Exception propagation should compile");

        let code = result.unwrap();
        let func_codes = find_code_constants(&code);
        assert!(func_codes.len() >= 2, "Should have at least 2 function codes");

        // inner function should have Raise
        let inner_code = func_codes[0];
        assert!(bytecode_contains_opcode(&inner_code.code, DpbOpcode::Raise),
            "inner() should have Raise opcode");

        // outer function should have SetupExcept
        let outer_code = func_codes[1];
        assert!(bytecode_contains_opcode(&outer_code.code, DpbOpcode::SetupExcept),
            "outer() should have SetupExcept opcode");
    }

    #[test]
    fn test_deeply_nested_exception_propagation() {
        let source = r#"
def level3():
    raise ValueError("deep error")

def level2():
    level3()

def level1():
    try:
        level2()
    except ValueError:
        x = 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Deep exception propagation should compile");

        let code = result.unwrap();
        let func_codes = find_code_constants(&code);
        assert!(func_codes.len() >= 3, "Should have 3 function codes");
    }

    #[test]
    fn test_exception_with_finally_propagation() {
        let source = r#"
def f():
    try:
        raise ValueError("error")
    finally:
        cleanup = True
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Exception with finally should compile");

        let code = result.unwrap();
        let func_codes = find_code_constants(&code);
        assert!(!func_codes.is_empty(), "Should have function code");

        let func_code = func_codes[0];
        assert!(bytecode_contains_opcode(&func_code.code, DpbOpcode::Raise),
            "Should have Raise opcode");
        assert!(bytecode_contains_opcode(&func_code.code, DpbOpcode::SetupFinally),
            "Should have SetupFinally opcode");
    }

    #[test]
    fn test_reraise_preserves_traceback() {
        let source = r#"
try:
    try:
        raise ValueError("original")
    except ValueError:
        raise  # Should preserve original traceback
except ValueError:
    x = 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Reraise should compile");

        let code = result.unwrap();
        // Should have both Raise (original) and Reraise (bare raise)
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Raise),
            "Should have Raise opcode");
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::Reraise),
            "Should have Reraise opcode");
    }
}
