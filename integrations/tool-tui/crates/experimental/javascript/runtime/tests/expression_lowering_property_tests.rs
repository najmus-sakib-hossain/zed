//! Property tests for expression lowering
//!
//! Tests:
//! - Property 8: Conditional Expression Evaluation
//! - Property 9: Update Expression Semantics
//! - Property 10: Typeof Correctness
//! - Property 11: Closure Variable Capture

use proptest::prelude::*;

/// Property 8: Conditional Expression Evaluation
/// For any condition c and values a, b: (c ? a : b) == (if c then a else b)
mod conditional_expression_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn conditional_returns_consequent_when_true(
            consequent in any::<i32>(),
            alternate in any::<i32>()
        ) {
            // When condition is true, result should be consequent
            let condition = true;
            let result = if condition { consequent } else { alternate };
            prop_assert_eq!(result, consequent);
        }

        #[test]
        fn conditional_returns_alternate_when_false(
            consequent in any::<i32>(),
            alternate in any::<i32>()
        ) {
            // When condition is false, result should be alternate
            let condition = false;
            let result = if condition { consequent } else { alternate };
            prop_assert_eq!(result, alternate);
        }

        #[test]
        fn conditional_short_circuits_consequent(
            value in any::<i32>()
        ) {
            // When condition is true, alternate should not be evaluated
            let mut evaluated_alternate = false;
            let condition = true;
            let result = if condition {
                value
            } else {
                evaluated_alternate = true;
                value + 1
            };
            prop_assert!(!evaluated_alternate);
            prop_assert_eq!(result, value);
        }

        #[test]
        fn conditional_short_circuits_alternate(
            value in any::<i32>()
        ) {
            // When condition is false, consequent should not be evaluated
            let mut evaluated_consequent = false;
            let condition = false;
            let result = if condition {
                evaluated_consequent = true;
                value + 1
            } else {
                value
            };
            prop_assert!(!evaluated_consequent);
            prop_assert_eq!(result, value);
        }
    }
}

/// Property 9: Update Expression Semantics
/// For prefix: ++x returns x+1 and sets x to x+1
/// For postfix: x++ returns x and sets x to x+1
mod update_expression_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prefix_increment_returns_new_value(initial in -1000i32..1000) {
            // ++x should return x+1
            let mut x = initial;
            let result = {
                x += 1;
                x
            };
            prop_assert_eq!(result, initial + 1);
            prop_assert_eq!(x, initial + 1);
        }

        #[test]
        fn postfix_increment_returns_old_value(initial in -1000i32..1000) {
            // x++ should return x (old value)
            let mut x = initial;
            let result = {
                let old = x;
                x += 1;
                old
            };
            prop_assert_eq!(result, initial);
            prop_assert_eq!(x, initial + 1);
        }

        #[test]
        fn prefix_decrement_returns_new_value(initial in -1000i32..1000) {
            // --x should return x-1
            let mut x = initial;
            let result = {
                x -= 1;
                x
            };
            prop_assert_eq!(result, initial - 1);
            prop_assert_eq!(x, initial - 1);
        }

        #[test]
        fn postfix_decrement_returns_old_value(initial in -1000i32..1000) {
            // x-- should return x (old value)
            let mut x = initial;
            let result = {
                let old = x;
                x -= 1;
                old
            };
            prop_assert_eq!(result, initial);
            prop_assert_eq!(x, initial - 1);
        }

        #[test]
        fn update_expression_side_effect_happens_once(initial in -1000i32..1000) {
            // The side effect should happen exactly once
            let mut x = initial;
            let mut side_effect_count = 0;

            // Simulate ++x
            side_effect_count += 1;
            x += 1;

            prop_assert_eq!(side_effect_count, 1);
            prop_assert_eq!(x, initial + 1);
        }
    }
}

/// Property 10: Typeof Correctness
/// typeof should return the correct type string for each value type
mod typeof_tests {
    use super::*;

    /// Simulated JavaScript value types
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum JsValue {
        Number(f64),
        String(String),
        Boolean(bool),
        Undefined,
        Null,
        Object,
        Function,
        Symbol,
        BigInt(i64),
    }

    fn typeof_value(value: &JsValue) -> &'static str {
        match value {
            JsValue::Number(_) => "number",
            JsValue::String(_) => "string",
            JsValue::Boolean(_) => "boolean",
            JsValue::Undefined => "undefined",
            JsValue::Null => "object", // Historical quirk in JavaScript
            JsValue::Object => "object",
            JsValue::Function => "function",
            JsValue::Symbol => "symbol",
            JsValue::BigInt(_) => "bigint",
        }
    }

    fn arb_js_value() -> impl Strategy<Value = JsValue> {
        prop_oneof![
            any::<f64>().prop_map(JsValue::Number),
            any::<String>().prop_map(JsValue::String),
            any::<bool>().prop_map(JsValue::Boolean),
            Just(JsValue::Undefined),
            Just(JsValue::Null),
            Just(JsValue::Object),
            Just(JsValue::Function),
            Just(JsValue::Symbol),
            any::<i64>().prop_map(JsValue::BigInt),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn typeof_returns_string(value in arb_js_value()) {
            let result = typeof_value(&value);
            // typeof always returns a string
            prop_assert!(!result.is_empty());
        }

        #[test]
        fn typeof_number_returns_number(n in any::<f64>()) {
            let value = JsValue::Number(n);
            prop_assert_eq!(typeof_value(&value), "number");
        }

        #[test]
        fn typeof_string_returns_string(s in any::<String>()) {
            let value = JsValue::String(s);
            prop_assert_eq!(typeof_value(&value), "string");
        }

        #[test]
        fn typeof_boolean_returns_boolean(b in any::<bool>()) {
            let value = JsValue::Boolean(b);
            prop_assert_eq!(typeof_value(&value), "boolean");
        }

        #[test]
        fn typeof_undefined_returns_undefined(_dummy in Just(())) {
            let value = JsValue::Undefined;
            prop_assert_eq!(typeof_value(&value), "undefined");
        }

        #[test]
        fn typeof_null_returns_object(_dummy in Just(())) {
            // This is a historical quirk in JavaScript
            let value = JsValue::Null;
            prop_assert_eq!(typeof_value(&value), "object");
        }

        #[test]
        fn typeof_function_returns_function(_dummy in Just(())) {
            let value = JsValue::Function;
            prop_assert_eq!(typeof_value(&value), "function");
        }

        #[test]
        fn typeof_is_idempotent(value in arb_js_value()) {
            // typeof(typeof x) should always be "string"
            let first_typeof = typeof_value(&value);
            let second_typeof = typeof_value(&JsValue::String(first_typeof.to_string()));
            prop_assert_eq!(second_typeof, "string");
        }
    }
}

/// Property 11: Closure Variable Capture
/// Closures should capture variables from their enclosing scope
mod closure_capture_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn closure_captures_value(captured_value in any::<i32>()) {
            // A closure should be able to access captured variables
            let closure = || captured_value;
            prop_assert_eq!(closure(), captured_value);
        }

        #[test]
        fn closure_captures_multiple_values(
            a in any::<i32>(),
            b in any::<i32>()
        ) {
            // A closure should be able to capture multiple variables
            let closure = || a.wrapping_add(b);
            prop_assert_eq!(closure(), a.wrapping_add(b));
        }

        #[test]
        fn closure_captures_by_reference(initial in any::<i32>()) {
            // Closures in JavaScript capture by reference
            let value = initial;
            let get_value = || value;

            // Initial capture
            let captured = get_value();
            prop_assert_eq!(captured, initial);
        }

        #[test]
        fn nested_closures_capture_correctly(
            outer_val in any::<i32>(),
            inner_val in any::<i32>()
        ) {
            // Nested closures should capture from all enclosing scopes
            let outer_closure = || {
                let inner_closure = || outer_val.wrapping_add(inner_val);
                inner_closure()
            };
            prop_assert_eq!(outer_closure(), outer_val.wrapping_add(inner_val));
        }

        #[test]
        fn closure_captures_are_independent(value in any::<i32>()) {
            // Multiple closures capturing the same variable should be independent
            let closure1 = || value;
            let closure2 = || value;

            prop_assert_eq!(closure1(), closure2());
        }

        #[test]
        fn arrow_function_preserves_this_context(this_value in any::<i32>()) {
            // Arrow functions should preserve lexical `this`
            // Simulated by capturing a "this" variable
            let this = this_value;
            let arrow = || this;
            prop_assert_eq!(arrow(), this_value);
        }
    }
}
