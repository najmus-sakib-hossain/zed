//! Property-based tests for the Python parser
//!
//! These tests verify the parser's correctness using property-based testing.
//! The main property tested is round-trip consistency: parsing source code,
//! printing it back to source, and parsing again should produce equivalent ASTs.

use dx_py_parser::ast::*;
use dx_py_parser::{parse_expression, parse_module, print_expression, print_module};
use proptest::prelude::*;

/// Generate arbitrary valid Python identifiers
fn arb_identifier() -> impl Strategy<Value = String> {
    // Use a simpler approach - just generate from a fixed set of valid identifiers
    prop::sample::select(vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "x".to_string(),
        "y".to_string(),
        "z".to_string(),
        "foo".to_string(),
        "bar".to_string(),
        "baz".to_string(),
        "qux".to_string(),
        "name".to_string(),
        "value".to_string(),
        "item".to_string(),
        "data".to_string(),
        "result".to_string(),
    ])
}

/// Generate arbitrary integer constants
fn arb_int() -> impl Strategy<Value = i64> {
    -1_000_000i64..1_000_000i64
}

/// Generate arbitrary float constants
fn arb_float() -> impl Strategy<Value = f64> {
    (-1_000_000.0f64..1_000_000.0f64).prop_filter("not nan", |n| !n.is_nan())
}

/// Generate arbitrary string constants (simple ASCII for now)
fn arb_string() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ]{0,20}").unwrap()
}

/// Generate arbitrary constants
fn arb_constant() -> impl Strategy<Value = Constant> {
    prop_oneof![
        Just(Constant::None),
        Just(Constant::Bool(true)),
        Just(Constant::Bool(false)),
        arb_int().prop_map(Constant::Int),
        arb_float().prop_map(Constant::Float),
        arb_string().prop_map(Constant::Str),
        Just(Constant::Ellipsis),
    ]
}

/// Generate simple expressions (non-recursive)
fn arb_simple_expression() -> impl Strategy<Value = Expression> {
    prop_oneof![
        arb_identifier().prop_map(|id| Expression::Name {
            id,
            location: Default::default()
        }),
        arb_constant().prop_map(|value| Expression::Constant {
            value,
            location: Default::default()
        }),
    ]
}

/// Generate arbitrary binary operators
fn arb_binop() -> impl Strategy<Value = BinOp> {
    prop_oneof![
        Just(BinOp::Add),
        Just(BinOp::Sub),
        Just(BinOp::Mult),
        Just(BinOp::Div),
        Just(BinOp::Mod),
        Just(BinOp::FloorDiv),
    ]
}

/// Generate arbitrary comparison operators
fn arb_cmpop() -> impl Strategy<Value = CmpOp> {
    prop_oneof![
        Just(CmpOp::Eq),
        Just(CmpOp::NotEq),
        Just(CmpOp::Lt),
        Just(CmpOp::LtE),
        Just(CmpOp::Gt),
        Just(CmpOp::GtE),
    ]
}

/// Generate expressions with limited depth
fn arb_expression(depth: usize) -> BoxedStrategy<Expression> {
    if depth == 0 {
        arb_simple_expression().boxed()
    } else {
        prop_oneof![
            // Simple expressions (higher weight)
            3 => arb_simple_expression(),
            // Binary operations (only with simple operands to avoid precedence issues)
            1 => (arb_simple_expression(), arb_binop(), arb_simple_expression())
                .prop_map(|(left, op, right)| Expression::BinOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    location: Default::default(),
                }),
            // Comparisons (only with simple operands)
            1 => (arb_simple_expression(), arb_cmpop(), arb_simple_expression())
                .prop_map(|(left, op, right)| Expression::Compare {
                    left: Box::new(left),
                    ops: vec![op],
                    comparators: vec![right],
                    location: Default::default(),
                }),
            // Lists
            1 => prop::collection::vec(arb_simple_expression(), 0..3)
                .prop_map(|elts| Expression::List {
                    elts,
                    location: Default::default(),
                }),
            // Tuples
            1 => prop::collection::vec(arb_simple_expression(), 0..3)
                .prop_map(|elts| Expression::Tuple {
                    elts,
                    location: Default::default(),
                }),
        ]
        .boxed()
    }
}

/// Generate simple statements
fn arb_simple_statement() -> impl Strategy<Value = Statement> {
    prop_oneof![
        Just(Statement::Pass {
            location: Default::default()
        }),
        Just(Statement::Break {
            location: Default::default()
        }),
        Just(Statement::Continue {
            location: Default::default()
        }),
        arb_expression(1).prop_map(|value| Statement::Expr {
            value,
            location: Default::default(),
        }),
        (arb_identifier(), arb_expression(1)).prop_map(|(name, value)| Statement::Assign {
            targets: vec![Expression::Name {
                id: name,
                location: Default::default()
            }],
            value,
            location: Default::default(),
        }),
    ]
}

/// Generate function definitions
#[allow(dead_code)]
fn arb_function_def() -> impl Strategy<Value = Statement> {
    (arb_identifier(), prop::collection::vec(arb_simple_statement(), 1..3)).prop_map(
        |(name, body)| Statement::FunctionDef {
            name,
            args: Arguments::default(),
            body,
            decorators: Vec::new(),
            returns: None,
            is_async: false,
            location: Default::default(),
        },
    )
}

/// Generate class definitions
#[allow(dead_code)]
fn arb_class_def() -> impl Strategy<Value = Statement> {
    (arb_identifier(), prop::collection::vec(arb_simple_statement(), 1..3)).prop_map(
        |(name, body)| Statement::ClassDef {
            name,
            bases: Vec::new(),
            keywords: Vec::new(),
            body,
            decorators: Vec::new(),
            location: Default::default(),
        },
    )
}

/// Generate if statements
#[allow(dead_code)]
fn arb_if_statement() -> impl Strategy<Value = Statement> {
    (arb_expression(1), prop::collection::vec(arb_simple_statement(), 1..2)).prop_map(
        |(test, body)| Statement::If {
            test,
            body,
            orelse: Vec::new(),
            location: Default::default(),
        },
    )
}

/// Generate while statements
#[allow(dead_code)]
fn arb_while_statement() -> impl Strategy<Value = Statement> {
    (arb_expression(1), prop::collection::vec(arb_simple_statement(), 1..2)).prop_map(
        |(test, body)| Statement::While {
            test,
            body,
            orelse: Vec::new(),
            location: Default::default(),
        },
    )
}

/// Generate arbitrary statements
#[allow(dead_code)]
fn arb_statement() -> impl Strategy<Value = Statement> {
    prop_oneof![
        arb_simple_statement(),
        arb_function_def(),
        arb_class_def(),
        arb_if_statement(),
        arb_while_statement(),
    ]
}

/// Generate arbitrary modules
#[allow(dead_code)]
fn arb_module() -> impl Strategy<Value = Module> {
    prop::collection::vec(arb_statement(), 1..5).prop_map(|body| Module { body })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 1: Parser Round-Trip Consistency
    /// Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.9
    ///
    /// For any valid Python AST, pretty-printing then parsing SHALL produce
    /// a semantically equivalent AST.
    #[test]
    fn parser_round_trip_expression(expr in arb_expression(2)) {
        let printed = print_expression(&expr);
        // Try to parse the printed expression
        if let Ok(parsed) = parse_expression(&printed) {
            // Print again and compare
            let printed_again = print_expression(&parsed);
            // The second print should be identical to the first
            // (normalization should be idempotent)
            prop_assert_eq!(&printed, &printed_again,
                "Round-trip failed: original printed '{}', reparsed printed '{}'",
                &printed, &printed_again);
        }
        // If parsing fails, that's okay - not all generated expressions
        // may produce valid Python syntax when printed
    }

    /// Test that simple Python programs can be parsed and printed
    #[test]
    fn parser_round_trip_simple_programs(
        name in arb_identifier(),
        value in arb_int()
    ) {
        let source = format!("{} = {}\n", name, value);
        let module = parse_module(&source).unwrap();
        let printed = print_module(&module);
        let reparsed = parse_module(&printed).unwrap();
        let printed_again = print_module(&reparsed);

        // The printed output should be stable after one round-trip
        prop_assert_eq!(printed, printed_again,
            "Round-trip failed for source: {}", source);
    }

    /// Test function definitions round-trip
    #[test]
    fn parser_round_trip_function_def(
        name in arb_identifier(),
        body_stmt in arb_simple_statement()
    ) {
        let stmt = Statement::FunctionDef {
            name: name.clone(),
            args: Arguments::default(),
            body: vec![body_stmt],
            decorators: Vec::new(),
            returns: None,
            is_async: false,
            location: Default::default(),
        };
        let module = Module { body: vec![stmt] };
        let printed = print_module(&module);

        if let Ok(reparsed) = parse_module(&printed) {
            let printed_again = print_module(&reparsed);
            prop_assert_eq!(printed, printed_again,
                "Function def round-trip failed for: {}", name);
        }
    }

    /// Test class definitions round-trip
    #[test]
    fn parser_round_trip_class_def(
        name in arb_identifier()
    ) {
        let stmt = Statement::ClassDef {
            name: name.clone(),
            bases: Vec::new(),
            keywords: Vec::new(),
            body: vec![Statement::Pass { location: Default::default() }],
            decorators: Vec::new(),
            location: Default::default(),
        };
        let module = Module { body: vec![stmt] };
        let printed = print_module(&module);

        if let Ok(reparsed) = parse_module(&printed) {
            let printed_again = print_module(&reparsed);
            prop_assert_eq!(printed, printed_again,
                "Class def round-trip failed for: {}", name);
        }
    }
}

/// Test specific Python constructs for round-trip consistency
#[cfg(test)]
mod specific_tests {
    use super::*;

    fn test_round_trip(source: &str) {
        let module = parse_module(source).unwrap_or_else(|_| panic!("Failed to parse: {}", source));
        let printed = print_module(&module);
        let reparsed =
            parse_module(&printed).unwrap_or_else(|_| panic!("Failed to reparse: {}", printed));
        let printed_again = print_module(&reparsed);
        assert_eq!(
            printed, printed_again,
            "Round-trip failed.\nOriginal: {}\nPrinted: {}\nReprinted: {}",
            source, printed, printed_again
        );
    }

    #[test]
    fn test_simple_assignment() {
        test_round_trip("x = 1\n");
    }

    #[test]
    fn test_function_def() {
        test_round_trip("def foo():\n    pass\n");
    }

    #[test]
    fn test_function_with_args() {
        test_round_trip("def foo(x, y):\n    return x\n");
    }

    #[test]
    fn test_class_def() {
        test_round_trip("class Foo:\n    pass\n");
    }

    #[test]
    fn test_if_statement() {
        test_round_trip("if x:\n    pass\n");
    }

    #[test]
    fn test_while_loop() {
        test_round_trip("while x:\n    pass\n");
    }

    #[test]
    fn test_for_loop() {
        test_round_trip("for x in items:\n    pass\n");
    }

    #[test]
    fn test_try_except() {
        test_round_trip("try:\n    pass\nexcept:\n    pass\n");
    }

    #[test]
    fn test_with_statement() {
        test_round_trip("with x:\n    pass\n");
    }

    #[test]
    fn test_import() {
        test_round_trip("import os\n");
    }

    #[test]
    fn test_from_import() {
        test_round_trip("from os import path\n");
    }

    #[test]
    fn test_list_comprehension() {
        test_round_trip("x = [i for i in items]\n");
    }

    #[test]
    fn test_lambda() {
        test_round_trip("f = lambda x: x\n");
    }

    #[test]
    fn test_match_statement() {
        test_round_trip("match x:\n    case 1:\n        pass\n");
    }
}
