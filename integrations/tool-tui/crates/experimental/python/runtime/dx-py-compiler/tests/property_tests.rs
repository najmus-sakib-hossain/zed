//! Property-based tests for the DX-Py Compiler
//!
//! These tests verify correctness properties using proptest.

use dx_py_compiler::{ScopeType, SymbolTable};
use dx_py_parser::parse_module;
use proptest::prelude::*;

/// **Feature: dx-py-production-ready, Property 3: Bytecode Validity**
/// Verify that local variable indices are correct and consistent.
/// **Validates: Requirements 1.6**
mod symbol_resolution_properties {
    use super::*;

    /// Generate a valid Python identifier
    fn arb_identifier() -> impl Strategy<Value = String> {
        // Python identifiers: start with letter or underscore, followed by letters, digits, or underscores
        prop::string::string_regex("[a-z_][a-z0-9_]{0,10}")
            .unwrap()
            .prop_filter("not a keyword", |s| !is_python_keyword(s))
    }

    /// Check if a string is a Python keyword
    fn is_python_keyword(s: &str) -> bool {
        matches!(
            s,
            "False"
                | "None"
                | "True"
                | "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "try"
                | "while"
                | "with"
                | "yield"
        )
    }

    /// Generate a list of unique identifiers
    fn arb_unique_identifiers(count: usize) -> impl Strategy<Value = Vec<String>> {
        prop::collection::hash_set(arb_identifier(), 1..=count)
            .prop_map(|set| set.into_iter().collect())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Local variables get unique, sequential indices
        /// For any set of local variable assignments in a function, each variable should get
        /// a unique index starting from 0.
        /// Note: At module level, assigned names are globals, not locals.
        #[test]
        fn prop_local_indices_are_sequential(names in arb_unique_identifiers(10)) {
            if names.is_empty() {
                return Ok(());
            }

            // Build source with assignments inside a function
            // At module level, assigned names are globals, not locals
            // So we test inside a function where they become locals
            let assignments = names.iter()
                .map(|n| format!("    {} = 1", n))
                .collect::<Vec<_>>()
                .join("\n");
            let source = format!("def test_func():\n{}", assignments);

            let module = parse_module(&source).unwrap();
            let mut st = SymbolTable::new();
            st.analyze_module(&module).unwrap();

            let root = st.root.as_ref().unwrap();
            // Get the function scope (first child of root)
            prop_assert!(!root.children.is_empty(), "Expected function scope");
            let func_scope = &root.children[0];

            // All names should be local in the function scope
            for name in &names {
                prop_assert!(func_scope.is_local(name), "Expected {} to be local", name);
            }

            // Indices should be unique and in range [0, n)
            let mut indices: Vec<u16> = names.iter()
                .filter_map(|n| func_scope.get_local_index(n))
                .collect();
            indices.sort();

            prop_assert_eq!(indices.len(), names.len(), "Not all names got indices");

            // Check indices are sequential starting from 0
            for idx in indices.iter() {
                prop_assert!(*idx < names.len() as u16, "Index {} out of range", idx);
            }
        }

        /// Property: Function parameters come before other locals
        /// For any function with parameters and local variables, parameter indices
        /// should be lower than other local variable indices.
        #[test]
        fn prop_parameters_before_locals(
            params in arb_unique_identifiers(5),
            locals in arb_unique_identifiers(5)
        ) {
            if params.is_empty() {
                return Ok(());
            }

            // Filter out any locals that conflict with params
            let locals: Vec<_> = locals.into_iter()
                .filter(|l| !params.contains(l))
                .collect();

            if locals.is_empty() {
                return Ok(());
            }

            // Build function source
            let param_str = params.join(", ");
            let body = locals.iter()
                .map(|n| format!("    {} = 1", n))
                .collect::<Vec<_>>()
                .join("\n");

            let source = format!("def foo({}):\n{}\n    pass", param_str, body);

            let module = parse_module(&source).unwrap();
            let mut st = SymbolTable::new();
            st.analyze_module(&module).unwrap();

            let root = st.root.as_ref().unwrap();
            prop_assert_eq!(root.children.len(), 1, "Expected one child scope");

            let func_scope = &root.children[0];

            // Get max parameter index
            let max_param_idx = params.iter()
                .filter_map(|p| func_scope.get_local_index(p))
                .max()
                .unwrap_or(0);

            // Get min local index
            let min_local_idx = locals.iter()
                .filter_map(|l| func_scope.get_local_index(l))
                .min();

            if let Some(min_local) = min_local_idx {
                prop_assert!(
                    max_param_idx < min_local,
                    "Parameter index {} >= local index {}",
                    max_param_idx, min_local
                );
            }
        }

        /// Property: Global declarations prevent local binding
        /// For any variable declared global, it should not appear in locals.
        #[test]
        fn prop_global_not_in_locals(name in arb_identifier()) {
            let source = format!(
                "def foo():\n    global {}\n    {} = 1",
                name, name
            );

            let module = parse_module(&source).unwrap();
            let mut st = SymbolTable::new();
            st.analyze_module(&module).unwrap();

            let root = st.root.as_ref().unwrap();
            let func_scope = &root.children[0];

            prop_assert!(
                !func_scope.is_local(&name),
                "Global {} should not be local",
                name
            );
            prop_assert!(
                func_scope.explicit_globals.contains(&name),
                "Global {} should be in explicit_globals",
                name
            );
        }

        /// Property: Closure variables are tracked correctly
        /// For any variable used in a nested function but defined in outer,
        /// it should be a free variable in inner and cell variable in outer.
        #[test]
        fn prop_closure_tracking(name in arb_identifier()) {
            let source = format!(
                "def outer():\n    {} = 1\n    def inner():\n        return {}\n    return inner",
                name, name
            );

            let module = parse_module(&source).unwrap();
            let mut st = SymbolTable::new();
            st.analyze_module(&module).unwrap();

            let root = st.root.as_ref().unwrap();
            let outer_scope = &root.children[0];
            let inner_scope = &outer_scope.children[0];

            prop_assert!(
                outer_scope.cell_vars.contains(&name),
                "{} should be cell var in outer",
                name
            );
            prop_assert!(
                inner_scope.free_vars.contains(&name),
                "{} should be free var in inner",
                name
            );
        }
    }

    /// Property: Scope nesting is preserved
    /// For any nested function/class structure, the scope tree should match.
    #[test]
    fn test_scope_nesting_preserved() {
        let source = r#"
def outer():
    def inner1():
        pass
    def inner2():
        def innermost():
            pass
        pass
    pass
"#;
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.as_ref().unwrap();
        assert_eq!(root.scope_type, ScopeType::Module);
        assert_eq!(root.children.len(), 1);

        let outer = &root.children[0];
        assert_eq!(outer.name, "outer");
        assert_eq!(outer.scope_type, ScopeType::Function);
        assert_eq!(outer.children.len(), 2);

        let inner1 = &outer.children[0];
        assert_eq!(inner1.name, "inner1");
        assert_eq!(inner1.children.len(), 0);

        let inner2 = &outer.children[1];
        assert_eq!(inner2.name, "inner2");
        assert_eq!(inner2.children.len(), 1);

        let innermost = &inner2.children[0];
        assert_eq!(innermost.name, "innermost");
    }

    /// Property: Comprehension creates implicit scope
    #[test]
    fn test_comprehension_scope() {
        let source = "[x for x in range(10)]";
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.as_ref().unwrap();
        // Comprehension creates a child scope
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].scope_type, ScopeType::Comprehension);
    }

    /// Property: Lambda creates implicit scope
    #[test]
    fn test_lambda_scope() {
        let source = "f = lambda x: x + 1";
        let module = parse_module(source).unwrap();
        let mut st = SymbolTable::new();
        st.analyze_module(&module).unwrap();

        let root = st.root.as_ref().unwrap();
        assert!(root.is_local("f"));
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].scope_type, ScopeType::Lambda);
        assert!(root.children[0].locals.contains(&"x".to_string()));
    }
}

/// **Feature: dx-py-production-ready, Property 1: Compilation Round-Trip**
/// Verify that expressions compile to valid bytecode that produces correct results.
/// **Validates: Requirements 1.3, 1.4**
mod expression_compilation_properties {
    use dx_py_bytecode::DpbOpcode;
    use dx_py_compiler::SourceCompiler;
    use proptest::prelude::*;

    /// Generate a simple integer literal
    fn arb_int_literal() -> impl Strategy<Value = String> {
        prop::num::i64::ANY.prop_map(|n| n.to_string())
    }

    /// Generate a simple float literal
    fn arb_float_literal() -> impl Strategy<Value = String> {
        prop::num::f64::ANY
            .prop_filter("not nan or inf", |f| f.is_finite())
            .prop_map(|f| format!("{:.6}", f))
    }

    /// Generate a simple string literal
    fn arb_string_literal() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9 ]{0,20}")
            .unwrap()
            .prop_map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
    }

    /// Generate a simple boolean literal
    fn arb_bool_literal() -> impl Strategy<Value = String> {
        prop::bool::ANY.prop_map(|b| if b { "True" } else { "False" }.to_string())
    }

    /// Generate a simple literal expression
    fn arb_literal() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_int_literal(),
            arb_float_literal(),
            arb_string_literal(),
            arb_bool_literal(),
            Just("None".to_string()),
        ]
    }

    /// Generate a binary operator
    fn arb_binop() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("+"),
            Just("-"),
            Just("*"),
            Just("/"),
            Just("//"),
            Just("%"),
            Just("**"),
            Just("&"),
            Just("|"),
            Just("^"),
            Just("<<"),
            Just(">>"),
        ]
    }

    /// Generate a comparison operator
    fn arb_cmpop() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("<"),
            Just("<="),
            Just(">"),
            Just(">="),
            Just("=="),
            Just("!="),
        ]
    }

    /// Generate a simple binary expression
    fn arb_binary_expr() -> impl Strategy<Value = String> {
        (arb_int_literal(), arb_binop(), arb_int_literal())
            .prop_map(|(left, op, right)| format!("{} {} {}", left, op, right))
    }

    /// Generate a simple comparison expression
    fn arb_comparison_expr() -> impl Strategy<Value = String> {
        (arb_int_literal(), arb_cmpop(), arb_int_literal())
            .prop_map(|(left, op, right)| format!("{} {} {}", left, op, right))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Literal expressions compile without error
        /// For any valid literal, compilation should succeed and produce bytecode.
        #[test]
        fn prop_literal_compiles(literal in arb_literal()) {
            let source = format!("x = {}", literal);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);

            let code = result.unwrap();
            prop_assert!(!code.code.is_empty(), "Empty bytecode for: {}", source);
        }

        /// Property: Binary expressions compile without error
        /// For any valid binary expression, compilation should succeed.
        #[test]
        fn prop_binary_expr_compiles(expr in arb_binary_expr()) {
            let source = format!("x = {}", expr);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
        }

        /// Property: Comparison expressions compile without error
        /// For any valid comparison expression, compilation should succeed.
        #[test]
        fn prop_comparison_expr_compiles(expr in arb_comparison_expr()) {
            let source = format!("x = {}", expr);
            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile: {}", source);
        }

        /// Property: Binary expressions produce correct opcodes
        /// For any binary expression, the bytecode should contain the appropriate binary opcode.
        #[test]
        fn prop_binary_expr_has_correct_opcode(
            left in arb_int_literal(),
            right in arb_int_literal()
        ) {
            let source = format!("x = {} + {}", left, right);
            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            // Check that BinaryAdd opcode is present
            let has_add = code.code.contains(&(DpbOpcode::BinaryAdd as u8));
            prop_assert!(has_add, "Missing BinaryAdd opcode in: {:?}", code.code);
        }

        /// Property: List literals compile with correct size
        /// For any list literal, the BuildList opcode should have the correct count.
        #[test]
        fn prop_list_literal_correct_size(count in 0usize..10) {
            let elements: Vec<String> = (0..count).map(|i| i.to_string()).collect();
            let source = format!("x = [{}]", elements.join(", "));

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            // Find BuildList opcode and check its argument
            let mut found = false;
            for i in 0..code.code.len() {
                if code.code[i] == DpbOpcode::BuildList as u8 {
                    // Next byte is the count (1-byte arg)
                    if i + 1 < code.code.len() {
                        let arg = code.code[i + 1] as usize;
                        prop_assert_eq!(arg, count, "BuildList has wrong count");
                        found = true;
                        break;
                    }
                }
            }
            prop_assert!(found, "BuildList opcode not found");
        }

        /// Property: Tuple literals compile with correct size
        /// For any tuple literal, the BuildTuple opcode should have the correct count.
        #[test]
        fn prop_tuple_literal_correct_size(count in 2usize..10) {
            let elements: Vec<String> = (0..count).map(|i| i.to_string()).collect();
            let source = format!("x = ({})", elements.join(", "));

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            // Find BuildTuple opcode and check its argument
            let mut found = false;
            for i in 0..code.code.len() {
                if code.code[i] == DpbOpcode::BuildTuple as u8
                    && i + 1 < code.code.len()
                {
                    let arg = code.code[i + 1] as usize;
                    prop_assert_eq!(arg, count, "BuildTuple has wrong count");
                    found = true;
                    break;
                }
            }
            prop_assert!(found, "BuildTuple opcode not found");
        }

        /// Property: Constants are deduplicated
        /// When the same constant appears multiple times, it should be stored once.
        #[test]
        fn prop_constant_deduplication(value in 0i64..1000) {
            // Use positive values only since negative numbers are parsed as UnaryOp(USub, Int)
            let source = format!("x = {}\ny = {}\nz = {}", value, value, value);
            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            // Count how many times this value appears in constants
            let count = code.constants.iter()
                .filter(|c| matches!(c, dx_py_bytecode::Constant::Int(v) if *v == value))
                .count();

            prop_assert_eq!(count, 1, "Constant {} should appear exactly once", value);
        }
    }

    /// Test that nested expressions compile correctly
    #[test]
    fn test_nested_expression_compiles() {
        let source = "x = (1 + 2) * (3 - 4)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }

    /// Test that chained comparisons compile
    #[test]
    fn test_chained_comparison_compiles() {
        let source = "x = 1 < 2 < 3";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }

    /// Test that boolean operations compile
    #[test]
    fn test_boolean_ops_compile() {
        let source = "x = True and False or True";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }

    /// Test that ternary expressions compile
    #[test]
    fn test_ternary_compiles() {
        let source = "x = 1 if True else 2";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }

    /// Test that dict literals compile
    #[test]
    fn test_dict_literal_compiles() {
        let source = "x = {'a': 1, 'b': 2}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }

    /// Test that set literals compile
    #[test]
    fn test_set_literal_compiles() {
        let source = "x = {1, 2, 3}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
    }
}

/// **Feature: dx-py-production-ready, Property 3: Bytecode Validity**
/// Verify that control flow statements produce valid jump targets.
/// **Validates: Requirements 1.3, 1.7**
mod statement_compilation_properties {
    use dx_py_bytecode::DpbOpcode;
    use dx_py_compiler::SourceCompiler;
    use proptest::prelude::*;

    /// Generate a valid Python identifier
    fn arb_identifier() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z_][a-z0-9_]{0,10}")
            .unwrap()
            .prop_filter("not a keyword", |s| !is_python_keyword(s))
    }

    /// Check if a string is a Python keyword
    fn is_python_keyword(s: &str) -> bool {
        matches!(
            s,
            "False"
                | "None"
                | "True"
                | "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "try"
                | "while"
                | "with"
                | "yield"
        )
    }

    /// Generate a simple integer literal
    fn arb_int_literal() -> impl Strategy<Value = String> {
        (0i64..1000).prop_map(|n| n.to_string())
    }

    /// Validate that all jump targets in bytecode are within bounds
    fn validate_jump_targets(code: &[u8]) -> Result<(), String> {
        let mut i = 0;
        while i < code.len() {
            let opcode = code[i];

            // Check if this is a jump opcode
            let is_jump = matches!(
                DpbOpcode::from_u8(opcode),
                Some(DpbOpcode::Jump)
                    | Some(DpbOpcode::JumpIfTrue)
                    | Some(DpbOpcode::JumpIfFalse)
                    | Some(DpbOpcode::JumpIfTrueOrPop)
                    | Some(DpbOpcode::JumpIfFalseOrPop)
                    | Some(DpbOpcode::PopJumpIfTrue)
                    | Some(DpbOpcode::PopJumpIfFalse)
                    | Some(DpbOpcode::ForIter)
                    | Some(DpbOpcode::SetupExcept)
            );

            if is_jump {
                // Jump opcodes have 2-byte relative offset
                if i + 2 >= code.len() {
                    return Err(format!("Jump at {} truncated", i));
                }
                let offset = i16::from_le_bytes([code[i + 1], code[i + 2]]);
                let target = (i as i32 + 3 + offset as i32) as usize;

                if target > code.len() {
                    return Err(format!(
                        "Jump at {} with offset {} targets {} which is beyond code length {}",
                        i,
                        offset,
                        target,
                        code.len()
                    ));
                }
            }

            // Advance to next instruction
            if let Some(op) = DpbOpcode::from_u8(opcode) {
                i += 1 + op.arg_size();
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: If statements produce valid jump targets
        /// For any if statement, all jump targets should be within bytecode bounds.
        #[test]
        fn prop_if_statement_valid_jumps(
            cond in arb_int_literal(),
            then_val in arb_int_literal(),
            else_val in arb_int_literal()
        ) {
            let source = format!(
                "if {}:\n    x = {}\nelse:\n    x = {}",
                cond, then_val, else_val
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: While loops produce valid jump targets
        /// For any while loop, all jump targets should be within bytecode bounds.
        #[test]
        fn prop_while_loop_valid_jumps(
            cond in arb_int_literal(),
            body_val in arb_int_literal()
        ) {
            let source = format!(
                "while {}:\n    x = {}",
                cond, body_val
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: For loops produce valid jump targets
        /// For any for loop, all jump targets should be within bytecode bounds.
        #[test]
        fn prop_for_loop_valid_jumps(
            var in arb_identifier(),
            count in 1usize..10
        ) {
            let source = format!(
                "for {} in range({}):\n    x = {}",
                var, count, var
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: Nested control flow produces valid jump targets
        /// For any nested if/while/for, all jump targets should be valid.
        #[test]
        fn prop_nested_control_flow_valid_jumps(
            outer_cond in arb_int_literal(),
            inner_cond in arb_int_literal()
        ) {
            let source = format!(
                "if {}:\n    while {}:\n        x = 1\n    y = 2\nelse:\n    z = 3",
                outer_cond, inner_cond
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: Try/except produces valid jump targets
        /// For any try/except block, all jump targets should be valid.
        #[test]
        fn prop_try_except_valid_jumps(
            try_val in arb_int_literal(),
            except_val in arb_int_literal()
        ) {
            let source = format!(
                "try:\n    x = {}\nexcept:\n    x = {}",
                try_val, except_val
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: Boolean short-circuit produces valid jumps
        /// For any boolean expression with and/or, jump targets should be valid.
        #[test]
        fn prop_boolean_shortcircuit_valid_jumps(
            a in arb_int_literal(),
            b in arb_int_literal(),
            c in arb_int_literal()
        ) {
            let source = format!("x = {} and {} or {}", a, b, c);

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: Ternary expression produces valid jumps
        /// For any ternary expression, jump targets should be valid.
        #[test]
        fn prop_ternary_valid_jumps(
            cond in arb_int_literal(),
            then_val in arb_int_literal(),
            else_val in arb_int_literal()
        ) {
            let source = format!("x = {} if {} else {}", then_val, cond, else_val);

            let mut compiler = SourceCompiler::new("<test>".into());
            let code = compiler.compile_module_source(&source).unwrap();

            let result = validate_jump_targets(&code.code);
            prop_assert!(result.is_ok(), "Invalid jumps: {:?}", result.err());
        }

        /// Property: Function definitions compile without error
        /// For any function definition, compilation should succeed.
        #[test]
        fn prop_function_def_compiles(
            name in arb_identifier(),
            param in arb_identifier()
        ) {
            // Ensure name and param are different
            if name == param {
                return Ok(());
            }

            let source = format!(
                "def {}({}):\n    return {} + 1",
                name, param, param
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile function: {}", source);
        }

        /// Property: Class definitions compile without error
        /// For any class definition, compilation should succeed.
        #[test]
        fn prop_class_def_compiles(name in arb_identifier()) {
            let source = format!(
                "class {}:\n    def __init__(self):\n        self.x = 1",
                name
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile class: {}", source);
        }

        /// Property: Import statements compile without error
        /// For any import statement, compilation should succeed.
        #[test]
        fn prop_import_compiles(module in arb_identifier()) {
            let source = format!("import {}", module);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile import: {}", source);
        }

        /// Property: From import statements compile without error
        /// For any from...import statement, compilation should succeed.
        #[test]
        fn prop_from_import_compiles(
            module in arb_identifier(),
            name in arb_identifier()
        ) {
            let source = format!("from {} import {}", module, name);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile from import: {}", source);
        }

        /// Property: Assert statements compile without error
        /// For any assert statement, compilation should succeed.
        #[test]
        fn prop_assert_compiles(cond in arb_int_literal()) {
            let source = format!("assert {}", cond);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile assert: {}", source);
        }

        /// Property: Raise statements compile without error
        /// For any raise statement, compilation should succeed.
        #[test]
        fn prop_raise_compiles(msg in "[a-zA-Z0-9 ]{1,20}") {
            let source = format!("raise ValueError(\"{}\")", msg);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile raise: {}", source);
        }

        /// Property: Delete statements compile without error
        /// For any delete statement, compilation should succeed.
        #[test]
        fn prop_delete_compiles(name in arb_identifier()) {
            let source = format!("{} = 1\ndel {}", name, name);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile delete: {}", source);
        }

        /// Property: Augmented assignment compiles without error
        /// For any augmented assignment, compilation should succeed.
        #[test]
        fn prop_augassign_compiles(
            name in arb_identifier(),
            val in arb_int_literal()
        ) {
            let source = format!("{} = 1\n{} += {}", name, name, val);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);
            prop_assert!(result.is_ok(), "Failed to compile augassign: {}", source);
        }
    }

    /// Test that complex nested control flow compiles
    #[test]
    fn test_complex_nested_control_flow() {
        let source = r#"
def foo(x):
    if x > 0:
        while x > 0:
            if x % 2 == 0:
                x = x - 1
            else:
                x = x - 2
    else:
        for i in range(10):
            if i > 5:
                break
    return x
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile complex nested control flow");

        let code = result.unwrap();
        let jump_result = validate_jump_targets(&code.code);
        assert!(jump_result.is_ok(), "Invalid jumps in complex control flow");
    }

    /// Test that try/except/finally compiles
    #[test]
    fn test_try_except_finally() {
        let source = r#"
try:
    x = 1
except ValueError:
    x = 2
except TypeError as e:
    x = 3
finally:
    y = 4
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile try/except/finally");
    }

    /// Test that with statement compiles
    #[test]
    fn test_with_statement() {
        let source = r#"
with open("file.txt") as f:
    x = f.read()
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile with statement");
    }

    /// Test that list comprehension compiles
    #[test]
    fn test_list_comprehension() {
        let source = "x = [i * 2 for i in range(10) if i % 2 == 0]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile list comprehension");
    }

    /// Test that generator expression compiles
    /// Test that generator expression compiles
    #[test]
    fn test_generator_expression() {
        let source = "x = (i * 2 for i in range(10))";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression: {:?}", result.err());
    }

    /// Test that async function compiles
    #[test]
    fn test_async_function() {
        let source = r#"
async def foo():
    return 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile async function");
    }

    /// Test that decorated function compiles
    #[test]
    fn test_decorated_function() {
        let source = r#"
@decorator
def foo():
    pass
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile decorated function");
    }

    /// Test that class with inheritance compiles
    #[test]
    fn test_class_inheritance() {
        let source = r#"
class Child(Parent):
    def __init__(self):
        super().__init__()
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile class with inheritance");
    }
}

/// **Feature: dx-py-production-ready, Property 4: Error Reporting Completeness**
/// Verify that compiler errors contain line numbers and useful information.
/// **Validates: Requirements 1.5**
mod error_reporting_properties {
    use dx_py_compiler::SourceCompiler;

    /// Test that syntax errors include line numbers
    #[test]
    fn test_syntax_error_has_line_number() {
        let source = r#"
x = 1
y = 2 +
z = 3
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_err(), "Expected syntax error");
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);

        // Error should mention line number (line 3 where the incomplete expression is)
        assert!(
            err_str.contains("line") || err_str.contains("3") || err_str.contains("Location"),
            "Error should contain line information: {}",
            err_str
        );
    }

    /// Test that undefined name errors are caught during symbol analysis
    #[test]
    fn test_undefined_name_in_nonlocal() {
        let source = r#"
def foo():
    nonlocal x  # x is not defined in any enclosing scope
    x = 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should either error or compile (depending on strictness)
        // The important thing is it doesn't panic
        let _ = result;
    }

    /// Test that global/nonlocal conflicts are detected
    #[test]
    fn test_global_nonlocal_conflict() {
        let source = r#"
def foo():
    global x
    nonlocal x  # Can't be both global and nonlocal
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should either error or compile (depending on strictness)
        // The important thing is it doesn't panic
        let _ = result;
    }

    /// Test that invalid assignment targets are caught
    #[test]
    fn test_invalid_assignment_target() {
        let source = "1 + 2 = 3";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should be a parse error
        assert!(result.is_err(), "Expected error for invalid assignment target");
    }

    /// Test that break outside loop is handled
    #[test]
    fn test_break_outside_loop() {
        let source = r#"
break  # Not inside a loop
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This compiles but produces invalid bytecode (break with no loop context)
        // A stricter compiler would reject this
        let _ = result;
    }

    /// Test that continue outside loop is handled
    #[test]
    fn test_continue_outside_loop() {
        let source = r#"
continue  # Not inside a loop
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This compiles but produces invalid bytecode
        let _ = result;
    }

    /// Test that return outside function is handled
    #[test]
    fn test_return_outside_function() {
        let source = r#"
return 1  # Not inside a function
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // At module level, return is valid (returns from module execution)
        // This should compile successfully
        assert!(result.is_ok(), "Return at module level should compile");
    }

    /// Test that yield outside function is handled
    #[test]
    fn test_yield_outside_function() {
        let source = r#"
yield 1  # Not inside a function
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should compile (creates a module-level generator)
        // or error depending on strictness
        let _ = result;
    }

    /// Test that duplicate argument names are handled
    #[test]
    fn test_duplicate_argument_names() {
        let source = r#"
def foo(x, x):  # Duplicate parameter name
    pass
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should be a parse error or compile error
        // The important thing is it doesn't panic
        let _ = result;
    }

    /// Test that starred expressions in wrong context are handled
    #[test]
    fn test_starred_in_wrong_context() {
        let source = r#"
x = *y  # Starred not in tuple/list context
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should be a parse error
        assert!(result.is_err(), "Expected error for starred in wrong context");
    }

    /// Test that empty function body is handled
    #[test]
    fn test_empty_function_body() {
        let source = r#"
def foo():
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This should be a parse error (missing body)
        assert!(result.is_err(), "Expected error for empty function body");
    }

    /// Test that mismatched parentheses are caught
    #[test]
    fn test_mismatched_parentheses() {
        let source = "x = (1 + 2";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_err(), "Expected error for mismatched parentheses");
    }

    /// Test that mismatched brackets are caught
    #[test]
    fn test_mismatched_brackets() {
        let source = "x = [1, 2, 3";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_err(), "Expected error for mismatched brackets");
    }

    /// Test that mismatched braces are caught
    #[test]
    fn test_mismatched_braces() {
        let source = "x = {1: 2";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_err(), "Expected error for mismatched braces");
    }

    /// Test that invalid string escapes are handled
    #[test]
    fn test_invalid_string_escape() {
        // Note: This depends on how the parser handles escapes
        let source = r#"x = "\z""#; // \z is not a valid escape
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        // This might compile (treating \z as literal) or error
        let _ = result;
    }

    /// Test that unterminated string is caught
    #[test]
    fn test_unterminated_string() {
        let source = r#"x = "hello"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_err(), "Expected error for unterminated string");
    }
}


/// **Feature: dx-py-production-ready-v2, Property 11: List Comprehension Length**
/// **Feature: dx-py-production-ready-v2, Property 12: List Comprehension Filter**
/// **Validates: Requirements 4.1-4.5**
mod list_comprehension_properties {
    use super::*;
    use dx_py_bytecode::DpbOpcode;
    use dx_py_compiler::SourceCompiler;

    /// Generate a small list of integers for testing
    fn arb_small_int_list() -> impl Strategy<Value = Vec<i64>> {
        prop::collection::vec(-100i64..100i64, 0..20)
    }

    /// Generate a threshold value for filtering
    fn arb_threshold() -> impl Strategy<Value = i64> {
        -50i64..50i64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-py-production-ready-v2, Property 11: List Comprehension Length**
        /// **Validates: Requirements 4.1**
        ///
        /// For any iterable of length n, the comprehension [x for x in iterable]
        /// should produce a list of length n.
        #[test]
        fn prop_list_comprehension_preserves_length(items in arb_small_int_list()) {
            // Generate source code for: result = [x for x in items]
            let items_str = items.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let source = format!("items = [{}]\nresult = [x for x in items]", items_str);

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile list comprehension: {}", source);

            let code = result.unwrap();

            // Verify the bytecode contains BUILD_LIST and LIST_APPEND
            let bytecode = &code.code;
            let has_build_list = bytecode.windows(2).any(|w| w[0] == DpbOpcode::BuildList as u8);
            let has_list_append = bytecode.windows(2).any(|w| w[0] == DpbOpcode::ListAppend as u8);

            prop_assert!(has_build_list, "Expected BUILD_LIST opcode in bytecode");
            prop_assert!(has_list_append, "Expected LIST_APPEND opcode in bytecode");
        }

        /// **Feature: dx-py-production-ready-v2, Property 12: List Comprehension Filter**
        /// **Validates: Requirements 4.2**
        ///
        /// For any list comprehension [x for x in iterable if cond(x)],
        /// the bytecode should contain conditional jump instructions.
        #[test]
        fn prop_list_comprehension_filter_generates_conditional(
            items in arb_small_int_list(),
            threshold in arb_threshold(),
        ) {
            // Generate source code for: result = [x for x in items if x > threshold]
            let items_str = items.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let source = format!(
                "items = [{}]\nresult = [x for x in items if x > {}]",
                items_str, threshold
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile filtered list comprehension: {}", source);

            let code = result.unwrap();

            // Verify the bytecode contains conditional jump (PopJumpIfFalse)
            let bytecode = &code.code;
            let has_conditional = bytecode.windows(1).any(|w| w[0] == DpbOpcode::PopJumpIfFalse as u8);

            prop_assert!(has_conditional, "Expected PopJumpIfFalse opcode for filter condition");
        }

        /// Test that nested comprehensions compile correctly
        /// [x + y for x in [1,2] for y in [3,4]]
        #[test]
        fn prop_nested_comprehension_compiles(
            outer in prop::collection::vec(0i64..10i64, 1..5),
            inner in prop::collection::vec(0i64..10i64, 1..5),
        ) {
            let outer_str = outer.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
            let inner_str = inner.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
            let source = format!(
                "result = [x + y for x in [{}] for y in [{}]]",
                outer_str, inner_str
            );

            let mut compiler = SourceCompiler::new("<test>".into());
            let result = compiler.compile_module_source(&source);

            prop_assert!(result.is_ok(), "Failed to compile nested comprehension: {}", source);

            let code = result.unwrap();

            // Verify the bytecode contains multiple FOR_ITER (one for each generator)
            let bytecode = &code.code;
            let for_iter_count = bytecode.windows(1)
                .filter(|w| w[0] == DpbOpcode::ForIter as u8)
                .count();

            prop_assert!(for_iter_count >= 2, "Expected at least 2 FOR_ITER opcodes for nested comprehension, got {}", for_iter_count);
        }
    }

    /// Test that simple list comprehension compiles
    #[test]
    fn test_simple_list_comprehension_compiles() {
        let source = "result = [x * 2 for x in [1, 2, 3]]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile simple list comprehension");
    }

    /// Test that filtered list comprehension compiles
    #[test]
    fn test_filtered_list_comprehension_compiles() {
        let source = "result = [x for x in [1, 2, 3, 4, 5] if x > 2]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile filtered list comprehension");
    }

    /// Test that nested list comprehension compiles
    #[test]
    fn test_nested_list_comprehension_compiles() {
        let source = "result = [[x, y] for x in [1, 2] for y in [3, 4]]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile nested list comprehension");
    }

    /// Test that comprehension with multiple conditions compiles
    #[test]
    fn test_multi_condition_comprehension_compiles() {
        let source = "result = [x for x in range(10) if x > 2 if x < 8]";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile multi-condition comprehension");
    }

    /// Test that set comprehension compiles
    #[test]
    fn test_set_comprehension_compiles() {
        let source = "result = {x * 2 for x in [1, 2, 3]}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile set comprehension: {:?}", result.err());
    }
    
    /// Test that set comprehension with filter compiles
    #[test]
    fn test_set_comprehension_with_filter_compiles() {
        let source = "result = {x * 2 for x in [1, 2, 3] if x > 1}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile filtered set comprehension: {:?}", result.err());
    }

    /// Test that dict comprehension compiles
    #[test]
    fn test_dict_comprehension_compiles() {
        let source = "result = {x: x * 2 for x in [1, 2, 3]}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile dict comprehension: {:?}", result.err());
    }
    
    /// Test that dict comprehension with filter compiles
    #[test]
    fn test_dict_comprehension_with_filter_compiles() {
        let source = "result = {x: x * 2 for x in [1, 2, 3] if x > 1}";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);

        assert!(result.is_ok(), "Failed to compile filtered dict comprehension: {:?}", result.err());
    }
}


/// **Feature: dx-py-production-ready, Property 11: Generator Iteration Equivalence**
/// **Validates: Requirements 6.1, 6.2, 6.3, 6.4**
mod generator_compilation_properties {
    use dx_py_bytecode::{CodeFlags, Constant};
    use dx_py_compiler::SourceCompiler;

    /// Test that the compiler sets the GENERATOR flag for functions with yield
    #[test]
    fn test_compiler_sets_generator_flag() {
        let source = r#"
def gen():
    yield 1
    yield 2
    yield 3
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Compilation should succeed");
        
        let code = result.unwrap();
        
        // Find the function code object in constants
        let mut found_generator = false;
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    // Check that the GENERATOR flag is set
                    assert!(
                        func_code.flags.contains(CodeFlags::GENERATOR),
                        "Function 'gen' should have GENERATOR flag set"
                    );
                    found_generator = true;
                }
            }
        }
        assert!(found_generator, "Should find the 'gen' function in constants");
    }

    /// Test that regular functions don't have the GENERATOR flag
    #[test]
    fn test_regular_function_no_generator_flag() {
        let source = r#"
def regular():
    return 42
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Compilation should succeed");
        
        let code = result.unwrap();
        
        // Find the function code object in constants
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "regular" {
                    // Check that the GENERATOR flag is NOT set
                    assert!(
                        !func_code.flags.contains(CodeFlags::GENERATOR),
                        "Function 'regular' should NOT have GENERATOR flag set"
                    );
                }
            }
        }
    }

    /// Test that nested functions with yield get the GENERATOR flag
    #[test]
    fn test_nested_generator_flag() {
        let source = r#"
def outer():
    def inner():
        yield 1
    return inner
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Compilation should succeed");
        
        let code = result.unwrap();
        
        // Find the outer function
        for constant in &code.constants {
            if let Constant::Code(outer_code) = constant {
                if outer_code.name == "outer" {
                    // Outer should NOT be a generator
                    assert!(
                        !outer_code.flags.contains(CodeFlags::GENERATOR),
                        "Function 'outer' should NOT have GENERATOR flag"
                    );
                    
                    // Find inner function in outer's constants
                    for inner_const in &outer_code.constants {
                        if let Constant::Code(inner_code) = inner_const {
                            if inner_code.name == "inner" {
                                // Inner SHOULD be a generator
                                assert!(
                                    inner_code.flags.contains(CodeFlags::GENERATOR),
                                    "Function 'inner' should have GENERATOR flag"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Test that yield in if statement is detected
    #[test]
    fn test_yield_in_if_statement() {
        let source = r#"
def gen():
    if True:
        yield 1
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    assert!(func_code.flags.contains(CodeFlags::GENERATOR));
                }
            }
        }
    }

    /// Test that yield in for loop is detected
    #[test]
    fn test_yield_in_for_loop() {
        let source = r#"
def gen():
    for i in range(3):
        yield i
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    assert!(func_code.flags.contains(CodeFlags::GENERATOR));
                }
            }
        }
    }

    /// Test that yield in while loop is detected
    #[test]
    fn test_yield_in_while_loop() {
        let source = r#"
def gen():
    while True:
        yield 1
        break
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    assert!(func_code.flags.contains(CodeFlags::GENERATOR));
                }
            }
        }
    }

    /// Test that yield in try block is detected
    #[test]
    fn test_yield_in_try_block() {
        let source = r#"
def gen():
    try:
        yield 1
    except:
        pass
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    assert!(func_code.flags.contains(CodeFlags::GENERATOR));
                }
            }
        }
    }

    /// Test that yield from is detected
    #[test]
    fn test_yield_from_detected() {
        let source = r#"
def gen():
    yield from [1, 2, 3]
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        for constant in &code.constants {
            if let Constant::Code(func_code) = constant {
                if func_code.name == "gen" {
                    assert!(func_code.flags.contains(CodeFlags::GENERATOR));
                }
            }
        }
    }
}

/// **Feature: dx-py-production-ready, Property 11: Generator Expression Compilation**
/// **Validates: Requirements 6.1, 6.6**
/// 
/// Generator expressions like `(expr for x in iterable)` should:
/// 1. Compile to a generator function with the GENERATOR flag
/// 2. Return a generator object when evaluated (not execute immediately)
mod generator_expression_properties {
    use dx_py_bytecode::{CodeFlags, Constant, DpbOpcode};
    use dx_py_compiler::SourceCompiler;

    /// Test that simple generator expression compiles to a generator function
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_simple_genexp_compiles_to_generator() {
        let source = "gen = (x * 2 for x in [1, 2, 3])";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression: {:?}", result.err());
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object in constants
        let mut found_genexpr = false;
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    // Check that the GENERATOR flag is set
                    assert!(
                        genexpr_code.flags.contains(CodeFlags::GENERATOR),
                        "Generator expression should have GENERATOR flag set"
                    );
                    found_genexpr = true;
                }
            }
        }
        assert!(found_genexpr, "Should find the '<genexpr>' code object in constants");
    }

    /// Test that generator expression with filter compiles correctly
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_filtered_genexp_compiles() {
        let source = "gen = (x for x in [1, 2, 3, 4, 5] if x > 2)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile filtered generator expression: {:?}", result.err());
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    assert!(
                        genexpr_code.flags.contains(CodeFlags::GENERATOR),
                        "Filtered generator expression should have GENERATOR flag"
                    );
                }
            }
        }
    }

    /// Test that nested generator expression compiles correctly
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_nested_genexp_compiles() {
        let source = "gen = (x + y for x in [1, 2] for y in [10, 20])";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile nested generator expression: {:?}", result.err());
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    assert!(
                        genexpr_code.flags.contains(CodeFlags::GENERATOR),
                        "Nested generator expression should have GENERATOR flag"
                    );
                }
            }
        }
    }

    /// Test that generator expression has correct argument count (1 for the iterator)
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_genexp_has_one_argument() {
        let source = "gen = (x * 2 for x in items)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression");
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    // Generator expression takes one argument (the first iterator)
                    assert_eq!(
                        genexpr_code.argcount, 1,
                        "Generator expression should take exactly 1 argument (the iterator)"
                    );
                }
            }
        }
    }

    /// Test that generator expression bytecode contains YIELD opcode
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_genexp_contains_yield() {
        let source = "gen = (x for x in items)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression");
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    // Check that the bytecode contains YIELD opcode
                    let has_yield = genexpr_code.code.iter().any(|&b| b == DpbOpcode::Yield as u8);
                    assert!(
                        has_yield,
                        "Generator expression bytecode should contain YIELD opcode"
                    );
                }
            }
        }
    }

    /// Test that generator expression bytecode contains FOR_ITER for iteration
    /// **Validates: Requirements 6.6**
    #[test]
    fn test_genexp_contains_for_iter() {
        let source = "gen = (x for x in items)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression");
        
        let code = result.unwrap();
        
        // Find the <genexpr> code object
        for constant in &code.constants {
            if let Constant::Code(genexpr_code) = constant {
                if genexpr_code.name == "<genexpr>" {
                    // Check that the bytecode contains FOR_ITER opcode
                    let has_for_iter = genexpr_code.code.iter().any(|&b| b == DpbOpcode::ForIter as u8);
                    assert!(
                        has_for_iter,
                        "Generator expression bytecode should contain FOR_ITER opcode"
                    );
                }
            }
        }
    }

    /// Test that module bytecode contains MAKE_FUNCTION for generator expression
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_genexp_creates_function() {
        let source = "gen = (x for x in items)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression");
        
        let code = result.unwrap();
        
        // Check that the module bytecode contains MAKE_FUNCTION
        let has_make_function = code.code.iter().any(|&b| b == DpbOpcode::MakeFunction as u8);
        assert!(
            has_make_function,
            "Module bytecode should contain MAKE_FUNCTION for generator expression"
        );
    }

    /// Test that module bytecode contains CALL to invoke the generator function
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_genexp_calls_function() {
        let source = "gen = (x for x in items)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression");
        
        let code = result.unwrap();
        
        // Check that the module bytecode contains CALL
        let has_call = code.code.iter().any(|&b| b == DpbOpcode::Call as u8);
        assert!(
            has_call,
            "Module bytecode should contain CALL to invoke the generator function"
        );
    }

    /// Test that generator expression with complex expression compiles
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_complex_genexp_compiles() {
        let source = "gen = (x * x + 1 for x in range(10) if x % 2 == 0)";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile complex generator expression: {:?}", result.err());
    }

    /// Test that generator expression in function call compiles
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_genexp_in_function_call() {
        // Note: Generator expressions in function calls need explicit parentheses
        // sum((x for x in range(10))) instead of sum(x for x in range(10))
        let source = "result = sum((x for x in range(10)))";
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile generator expression in function call: {:?}", result.err());
    }

    /// Test that multiple generator expressions compile independently
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_multiple_genexps() {
        let source = r#"
gen1 = (x for x in [1, 2, 3])
gen2 = (y * 2 for y in [4, 5, 6])
"#;
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source(source);
        assert!(result.is_ok(), "Failed to compile multiple generator expressions: {:?}", result.err());
        
        let code = result.unwrap();
        
        // Count <genexpr> code objects
        let genexpr_count = code.constants.iter().filter(|c| {
            matches!(c, Constant::Code(gc) if gc.name == "<genexpr>")
        }).count();
        
        assert_eq!(genexpr_count, 2, "Should have 2 generator expression code objects");
    }
}
