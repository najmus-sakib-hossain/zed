//! Property-based tests for DX-Py Comprehensions
//!
//! Feature: dx-py-production-ready
//! Property 7: List Comprehension Equivalence
//! Property 8: Dict Comprehension Equivalence
//! Property 9: Set Comprehension Equivalence
//! Validates: Requirements 3.1-3.5, 4.1-4.5

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
            | "return" | "try" | "while" | "with" | "yield" | "self" | "x" | "i"
            | "items" | "range" | "result" | "lst"
    )
}

/// Generate a simple integer value for testing
fn arb_int_value() -> impl Strategy<Value = i64> {
    0i64..100i64
}

/// Generate a small positive integer for range bounds
fn arb_range_bound() -> impl Strategy<Value = usize> {
    1usize..20
}

/// Generate a list of unique identifiers for nested comprehensions
fn arb_unique_loop_vars(count: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::hash_set(arb_identifier(), count..=count)
        .prop_map(|set| set.into_iter().collect::<Vec<String>>())
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
#[allow(dead_code)]
fn find_code_constants(code: &CodeObject) -> Vec<&CodeObject> {
    code.constants
        .iter()
        .filter_map(|c| match c {
            Constant::Code(inner) => Some(inner.as_ref()),
            _ => None,
        })
        .collect()
}

/// Check if bytecode has proper list comprehension structure
fn has_list_comp_structure(code: &[u8]) -> bool {
    bytecode_contains_opcode(code, DpbOpcode::BuildList)
        && bytecode_contains_opcode(code, DpbOpcode::GetIter)
        && bytecode_contains_opcode(code, DpbOpcode::ForIter)
        && bytecode_contains_opcode(code, DpbOpcode::ListAppend)
}

/// Check if bytecode has proper dict comprehension structure
fn has_dict_comp_structure(code: &[u8]) -> bool {
    bytecode_contains_opcode(code, DpbOpcode::BuildDict)
        && bytecode_contains_opcode(code, DpbOpcode::GetIter)
        && bytecode_contains_opcode(code, DpbOpcode::ForIter)
        && bytecode_contains_opcode(code, DpbOpcode::MapAdd)
}

/// Check if bytecode has proper set comprehension structure
fn has_set_comp_structure(code: &[u8]) -> bool {
    bytecode_contains_opcode(code, DpbOpcode::BuildSet)
        && bytecode_contains_opcode(code, DpbOpcode::GetIter)
        && bytecode_contains_opcode(code, DpbOpcode::ForIter)
        && bytecode_contains_opcode(code, DpbOpcode::SetAdd)
}

// ===== Property 7: List Comprehension Equivalence =====

/// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
/// Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5
mod list_comprehension_equivalence_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
        /// Validates: Requirements 3.1
        #[test]
        fn prop_simple_list_comp_emits_correct_opcodes(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("[{} for {} in range({})]", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(has_list_comp_structure(&code.code),
                "List comprehension should emit BuildList, GetIter, ForIter, ListAppend opcodes");
        }

        /// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
        /// Validates: Requirements 3.2
        #[test]
        fn prop_filtered_comp_emits_condition_jump(
            var in arb_identifier(),
            range_end in arb_range_bound(),
            threshold in arb_int_value()
        ) {
            let source = format!("[{} for {} in range({}) if {} > {}]", var, var, range_end, var, threshold);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse),
                "Filtered comprehension should emit PopJumpIfFalse for condition");
        }

        /// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
        /// Validates: Requirements 3.3
        #[test]
        fn prop_nested_comp_emits_multiple_for_iter(
            vars in arb_unique_loop_vars(2),
            range1 in arb_range_bound(),
            range2 in arb_range_bound()
        ) {
            if vars.len() < 2 { return Ok(()); }
            let var1 = &vars[0];
            let var2 = &vars[1];
            let source = format!("[{} + {} for {} in range({}) for {} in range({})]",
                var1, var2, var1, range1, var2, range2);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
            prop_assert!(for_iter_count >= 2, "Nested comprehension should have at least 2 ForIter opcodes");
        }

        /// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
        /// Validates: Requirements 3.4
        #[test]
        fn prop_range_iterable_uses_get_iter(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("[{} * 2 for {} in range({})]", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::GetIter),
                "Range iterable should use GetIter opcode");
        }

        /// Feature: dx-py-production-ready, Property 7: List Comprehension Equivalence
        /// Validates: Requirements 3.5
        #[test]
        fn prop_list_iterable_uses_get_iter(var in arb_identifier()) {
            let source = format!("[{} for {} in [1, 2, 3, 4, 5]]", var, var);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::GetIter),
                "List iterable should use GetIter opcode");
        }
    }

    #[test]
    fn test_simple_list_comprehension_compiles() {
        let source = "[x for x in range(10)]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple list comprehension should compile");
        let code = result.unwrap();
        assert!(has_list_comp_structure(&code.code));
    }

    #[test]
    fn test_filtered_list_comprehension() {
        let source = "[x for x in range(10) if x > 5]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse));
    }

    #[test]
    fn test_nested_list_comprehension() {
        let source = "[x + y for x in range(3) for y in range(3)]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
        assert!(for_iter_count >= 2);
    }
}


// ===== Property 8: Dict Comprehension Equivalence =====

/// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
/// Validates: Requirements 4.1, 4.3, 4.5
mod dict_comprehension_equivalence_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.1
        #[test]
        fn prop_simple_dict_comp_emits_correct_opcodes(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{}: {} * 2 for {} in range({})}}", var, var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(has_dict_comp_structure(&code.code),
                "Dict comprehension should emit BuildDict, GetIter, ForIter, MapAdd opcodes");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.3
        #[test]
        fn prop_filtered_dict_comp_emits_condition_jump(
            var in arb_identifier(),
            range_end in arb_range_bound(),
            threshold in arb_int_value()
        ) {
            let source = format!("{{{}: {} * 2 for {} in range({}) if {} > {}}}",
                var, var, var, range_end, var, threshold);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse),
                "Filtered dict comprehension should emit PopJumpIfFalse for condition");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.5
        #[test]
        fn prop_nested_dict_comp_emits_multiple_for_iter(
            vars in arb_unique_loop_vars(2),
            range1 in arb_range_bound(),
            range2 in arb_range_bound()
        ) {
            if vars.len() < 2 { return Ok(()); }
            let var1 = &vars[0];
            let var2 = &vars[1];
            let source = format!("{{({}, {}): {} + {} for {} in range({}) for {} in range({})}}",
                var1, var2, var1, var2, var1, range1, var2, range2);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
            prop_assert!(for_iter_count >= 2, "Nested dict comprehension should have at least 2 ForIter opcodes");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.1
        #[test]
        fn prop_dict_comp_has_map_add(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{}: {} * 3 for {} in range({})}}", var, var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::MapAdd),
                "Dict comprehension should emit MapAdd opcode");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.1
        #[test]
        fn prop_dict_comp_result_starts_empty(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{}: {} for {} in range({})}}", var, var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::BuildDict),
                "Dict comprehension should start with BuildDict");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.3
        #[test]
        fn prop_dict_comp_multiple_conditions_emit_multiple_jumps(
            var in arb_identifier(),
            range_end in arb_range_bound(),
            threshold1 in arb_int_value(),
            threshold2 in arb_int_value()
        ) {
            let source = format!("{{{}: {} for {} in range({}) if {} > {} if {} < {}}}",
                var, var, var, range_end, var, threshold1, var, threshold2);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let jump_count = count_opcode(&code.code, DpbOpcode::PopJumpIfFalse);
            prop_assert!(jump_count >= 2, "Expected at least 2 PopJumpIfFalse for 2 conditions");
        }

        /// Feature: dx-py-production-ready, Property 8: Dict Comprehension Equivalence
        /// Validates: Requirements 4.5
        #[test]
        fn prop_triple_nested_dict_comp_compiles(
            vars in arb_unique_loop_vars(3),
            range1 in arb_range_bound(),
            range2 in arb_range_bound(),
            range3 in arb_range_bound()
        ) {
            if vars.len() < 3 { return Ok(()); }
            let var1 = &vars[0];
            let var2 = &vars[1];
            let var3 = &vars[2];
            let source = format!("{{({}, {}, {}): {} + {} + {} for {} in range({}) for {} in range({}) for {} in range({})}}",
                var1, var2, var3, var1, var2, var3, var1, range1, var2, range2, var3, range3);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
            prop_assert!(for_iter_count >= 3, "Triple nested dict comprehension should have at least 3 ForIter opcodes");
        }
    }

    #[test]
    fn test_simple_dict_comprehension_compiles() {
        let source = "{x: x for x in range(10)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple dict comprehension should compile");
        let code = result.unwrap();
        assert!(has_dict_comp_structure(&code.code));
    }

    #[test]
    fn test_filtered_dict_comprehension() {
        let source = "{x: x for x in range(10) if x > 5}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse));
    }

    #[test]
    fn test_nested_dict_comprehension() {
        let source = "{(x, y): x + y for x in range(3) for y in range(3)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
        assert!(for_iter_count >= 2);
    }

    #[test]
    fn test_dict_comprehension_with_tuple_key() {
        let source = "{(x, x*2): x*3 for x in range(5)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::BuildTuple));
    }
}


// ===== Property 9: Set Comprehension Equivalence =====

/// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
/// Validates: Requirements 4.2, 4.4
mod set_comprehension_equivalence_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_simple_set_comp_emits_correct_opcodes(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{} for {} in range({})}}", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(has_set_comp_structure(&code.code),
                "Set comprehension should emit BuildSet, GetIter, ForIter, SetAdd opcodes");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.4
        #[test]
        fn prop_filtered_set_comp_emits_condition_jump(
            var in arb_identifier(),
            range_end in arb_range_bound(),
            threshold in arb_int_value()
        ) {
            let source = format!("{{{} for {} in range({}) if {} > {}}}",
                var, var, range_end, var, threshold);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse),
                "Filtered set comprehension should emit PopJumpIfFalse for condition");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_nested_set_comp_emits_multiple_for_iter(
            vars in arb_unique_loop_vars(2),
            range1 in arb_range_bound(),
            range2 in arb_range_bound()
        ) {
            if vars.len() < 2 { return Ok(()); }
            let var1 = &vars[0];
            let var2 = &vars[1];
            let source = format!("{{{} + {} for {} in range({}) for {} in range({})}}",
                var1, var2, var1, range1, var2, range2);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
            prop_assert!(for_iter_count >= 2, "Nested set comprehension should have at least 2 ForIter opcodes");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_set_comp_has_set_add(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{} * 3 for {} in range({})}}", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetAdd),
                "Set comprehension should emit SetAdd opcode");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_set_comp_result_starts_empty(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{} for {} in range({})}}", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::BuildSet),
                "Set comprehension should start with BuildSet");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.4
        #[test]
        fn prop_set_comp_multiple_conditions_emit_multiple_jumps(
            var in arb_identifier(),
            range_end in arb_range_bound(),
            threshold1 in arb_int_value(),
            threshold2 in arb_int_value()
        ) {
            let source = format!("{{{} for {} in range({}) if {} > {} if {} < {}}}",
                var, var, range_end, var, threshold1, var, threshold2);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let jump_count = count_opcode(&code.code, DpbOpcode::PopJumpIfFalse);
            prop_assert!(jump_count >= 2, "Expected at least 2 PopJumpIfFalse for 2 conditions");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_triple_nested_set_comp_compiles(
            vars in arb_unique_loop_vars(3),
            range1 in arb_range_bound(),
            range2 in arb_range_bound(),
            range3 in arb_range_bound()
        ) {
            if vars.len() < 3 { return Ok(()); }
            let var1 = &vars[0];
            let var2 = &vars[1];
            let var3 = &vars[2];
            let source = format!("{{{} + {} + {} for {} in range({}) for {} in range({}) for {} in range({})}}",
                var1, var2, var3, var1, range1, var2, range2, var3, range3);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
            prop_assert!(for_iter_count >= 3, "Triple nested set comprehension should have at least 3 ForIter opcodes");
        }

        /// Feature: dx-py-production-ready, Property 9: Set Comprehension Equivalence
        /// Validates: Requirements 4.2
        #[test]
        fn prop_set_comp_range_iterable_uses_get_iter(
            var in arb_identifier(),
            range_end in arb_range_bound()
        ) {
            let source = format!("{{{} for {} in range({})}}", var, var, range_end);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
            let code = result.unwrap();
            prop_assert!(bytecode_contains_opcode(&code.code, DpbOpcode::GetIter),
                "Range iterable should use GetIter opcode");
        }
    }

    #[test]
    fn test_simple_set_comprehension_compiles() {
        let source = "{x for x in range(10)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Simple set comprehension should compile");
        let code = result.unwrap();
        assert!(has_set_comp_structure(&code.code));
    }

    #[test]
    fn test_filtered_set_comprehension() {
        let source = "{x for x in range(10) if x > 5}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::PopJumpIfFalse));
    }

    #[test]
    fn test_nested_set_comprehension() {
        let source = "{x + y for x in range(3) for y in range(3)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        let for_iter_count = count_opcode(&code.code, DpbOpcode::ForIter);
        assert!(for_iter_count >= 2);
    }

    #[test]
    fn test_set_comprehension_with_tuple_expression() {
        let source = "{(x, x*2) for x in range(5)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::BuildTuple));
    }

    #[test]
    fn test_set_comprehension_deduplication_structure() {
        // Set comprehension should use SetAdd which handles deduplication
        let source = "{x % 3 for x in range(10)}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::SetAdd));
        assert!(bytecode_contains_opcode(&code.code, DpbOpcode::BinaryMod));
    }
}
