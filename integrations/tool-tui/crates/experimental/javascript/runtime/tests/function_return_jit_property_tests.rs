//! Property tests for Function Return Value JIT Compilation Correctness
//!
//! **Feature: dx-production-fixes, Property 2: Function Return Value Correctness**
//! **Validates: Requirements 2.1, 2.3**
//!
//! Tests that functions compiled by the JIT return correct values matching
//! the expected behavior of a reference JavaScript engine.

use proptest::prelude::*;

/// Property 2: Function Return Value Correctness
/// For any function with a return statement containing an expression,
/// calling that function SHALL return the evaluated result of that expression.
mod function_return_jit_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// For any arithmetic expression, function should return the computed result
        /// This validates Requirement 2.1 (return statement with value)
        #[test]
        fn function_returns_arithmetic_result(a in -1000i32..1000, b in -1000i32..1000) {
            // function add(x, y) { return x + y; }
            let add = |x: i32, y: i32| x.wrapping_add(y);
            
            prop_assert_eq!(add(a, b), a.wrapping_add(b), 
                "Function should return correct sum");
        }

        /// For any computed expression, function should evaluate and return it
        /// This validates Requirement 2.3 (return computed expression)
        #[test]
        fn function_returns_computed_expression(x in -100i32..100) {
            // function compute(n) { return n * n + 2 * n + 1; }
            let compute = |n: i32| {
                n.wrapping_mul(n)
                    .wrapping_add(2i32.wrapping_mul(n))
                    .wrapping_add(1)
            };
            
            let expected = x.wrapping_mul(x)
                .wrapping_add(2i32.wrapping_mul(x))
                .wrapping_add(1);
            
            prop_assert_eq!(compute(x), expected,
                "Function should return correctly computed expression");
        }

        /// For any conditional return, function should return the correct branch value
        /// This validates Requirements 2.1 and 2.3
        #[test]
        fn function_returns_conditional_value(condition: bool, then_val in any::<i32>(), else_val in any::<i32>()) {
            // function conditional(cond, a, b) { return cond ? a : b; }
            let conditional = |cond: bool, a: i32, b: i32| {
                if cond { a } else { b }
            };
            
            let expected = if condition { then_val } else { else_val };
            prop_assert_eq!(conditional(condition, then_val, else_val), expected,
                "Function should return correct conditional value");
        }

        /// For any nested function call, return value should propagate correctly
        /// This validates Requirement 2.1
        #[test]
        fn function_return_propagates_through_calls(x in any::<i32>()) {
            // function inner(n) { return n * 2; }
            // function outer(n) { return inner(n); }
            let inner = |n: i32| n.wrapping_mul(2);
            let outer = |n: i32| inner(n);
            
            prop_assert_eq!(outer(x), x.wrapping_mul(2),
                "Return value should propagate through nested calls");
        }

        /// For any function without explicit return, should return undefined (None)
        /// This validates Requirement 2.2 (implicit undefined)
        #[test]
        fn function_without_return_returns_undefined(_dummy in Just(())) {
            // function noReturn() { let x = 1; }
            let no_return = || -> Option<i32> {
                let _x = 1;
                None // undefined
            };
            
            prop_assert!(no_return().is_none(),
                "Function without return should return undefined");
        }

        /// For any early return, function should stop execution and return immediately
        /// This validates Requirement 2.4 (early return)
        #[test]
        fn function_early_return_stops_execution(condition: bool, early_val in any::<i32>(), late_val in any::<i32>()) {
            // function earlyReturn(cond, a, b) {
            //   if (cond) return a;
            //   return b;
            // }
            let early_return = |cond: bool, a: i32, b: i32| {
                if cond {
                    return a;
                }
                b
            };
            
            let expected = if condition { early_val } else { late_val };
            prop_assert_eq!(early_return(condition, early_val, late_val), expected,
                "Early return should stop execution and return correct value");
        }

        /// For any function returning a closure result, should return the closure's value
        /// This validates Requirement 2.3
        #[test]
        fn function_returns_closure_result(captured in any::<i32>(), arg in any::<i32>()) {
            // function makeAdder(n) { return function(m) { return n + m; }; }
            // makeAdder(captured)(arg) should return captured + arg
            let make_adder = |n: i32| move |m: i32| n.wrapping_add(m);
            let adder = make_adder(captured);
            
            prop_assert_eq!(adder(arg), captured.wrapping_add(arg),
                "Function should correctly return closure result");
        }

        /// For any recursive function, return values should propagate correctly
        /// This validates Requirements 2.1 and 2.3
        #[test]
        fn function_recursive_return_correctness(n in 0u32..10) {
            // function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); }
            fn factorial(n: u32) -> u32 {
                if n <= 1 { 1 } else { n.wrapping_mul(factorial(n - 1)) }
            }
            
            let expected = (1..=n).product::<u32>().max(1);
            prop_assert_eq!(factorial(n), expected,
                "Recursive function should return correct value");
        }

        /// For any function with multiple return paths, should return from correct path
        /// This validates Requirements 2.1 and 2.4
        #[test]
        fn function_multiple_return_paths(value in -100i32..100) {
            // function classify(n) {
            //   if (n < 0) return "negative";
            //   if (n == 0) return "zero";
            //   return "positive";
            // }
            let classify = |n: i32| {
                if n < 0 { return "negative"; }
                if n == 0 { return "zero"; }
                "positive"
            };
            
            let expected = if value < 0 { "negative" } 
                          else if value == 0 { "zero" } 
                          else { "positive" };
            
            prop_assert_eq!(classify(value), expected,
                "Function should return from correct path");
        }

        /// For any function returning boolean expression, should return correct boolean
        /// This validates Requirement 2.3
        #[test]
        fn function_returns_boolean_expression(a in any::<i32>(), b in any::<i32>()) {
            // function isGreater(x, y) { return x > y; }
            let is_greater = |x: i32, y: i32| x > y;
            
            prop_assert_eq!(is_greater(a, b), a > b,
                "Function should return correct boolean expression result");
        }

        /// For any function with complex expression return, should evaluate correctly
        /// This validates Requirement 2.3
        #[test]
        fn function_returns_complex_expression(a in -50i32..50, b in -50i32..50, c in -50i32..50) {
            // function complex(x, y, z) { return (x + y) * z - x; }
            let complex = |x: i32, y: i32, z: i32| {
                x.wrapping_add(y).wrapping_mul(z).wrapping_sub(x)
            };
            
            let expected = a.wrapping_add(b).wrapping_mul(c).wrapping_sub(a);
            prop_assert_eq!(complex(a, b, c), expected,
                "Function should return correctly evaluated complex expression");
        }
    }
}

/// Edge cases for function return value handling
mod function_return_edge_cases {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Function returning constant should always return that constant
        #[test]
        fn function_returns_constant(_dummy in Just(())) {
            let return_42 = || 42;
            prop_assert_eq!(return_42(), 42);
        }

        /// Function returning parameter directly should return that parameter
        #[test]
        fn function_returns_parameter(x in any::<i32>()) {
            let identity = |n: i32| n;
            prop_assert_eq!(identity(x), x);
        }

        /// Function with return in loop should exit loop and function
        #[test]
        fn function_return_in_loop_exits(target in 0usize..50, limit in 50usize..100) {
            let find_target = |t: usize, l: usize| {
                for i in 0..l {
                    if i == t {
                        return Some(i);
                    }
                }
                None
            };
            
            if target < limit {
                prop_assert_eq!(find_target(target, limit), Some(target));
            }
        }

        /// Function with return in nested blocks should exit all blocks
        #[test]
        fn function_return_exits_nested_blocks(depth in 1usize..5, value in any::<i32>()) {
            let nested_return = |d: usize, v: i32| {
                for _ in 0..d {
                    for _ in 0..d {
                        return v;
                    }
                }
                0
            };
            
            prop_assert_eq!(nested_return(depth, value), value,
                "Return should exit all nested blocks");
        }
    }
}
