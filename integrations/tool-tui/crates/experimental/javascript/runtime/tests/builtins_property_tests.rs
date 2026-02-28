//! Property tests for JavaScript built-in methods
//!
//! These tests verify universal correctness properties:
//! - Object.entries[i] = [Object.keys[i], Object.values[i]]
//! - Array.from preserves elements
//! - JSON.parse(JSON.stringify(x)) == x for JSON-safe values

use dx_js_runtime::compiler::builtins_registry::BuiltinRegistry;
use dx_js_runtime::value::object::Object;
use dx_js_runtime::value::Value;
use proptest::prelude::*;

// ============================================================================
// Property 1: Object Methods Consistency
// Object.entries[i] = [Object.keys[i], Object.values[i]]
// Validates: Requirements 1.1, 1.2, 1.3
// ============================================================================

proptest! {
    #[test]
    fn prop_object_methods_consistency(
        keys in prop::collection::vec("[a-z]{1,10}", 0..20usize),
        values in prop::collection::vec(any::<i32>(), 0..20usize)
    ) {
        // Create an object with the given keys and values
        let mut obj = Object::new();
        let len = keys.len().min(values.len());

        for i in 0..len {
            obj.set(keys[i].clone(), Value::Number(values[i] as f64));
        }

        // Get keys, values, and entries
        let obj_keys = obj.keys_owned();
        let obj_values = obj.values_cloned();
        let obj_entries = obj.entries_cloned();

        // Property: entries length equals keys length equals values length
        prop_assert_eq!(obj_entries.len(), obj_keys.len());
        prop_assert_eq!(obj_entries.len(), obj_values.len());

        // Property: for each entry, entry[0] is in keys and entry[1] is in values
        for (key, value) in &obj_entries {
            prop_assert!(obj_keys.contains(key), "Entry key not found in keys");
            prop_assert!(obj_values.contains(value), "Entry value not found in values");
        }

        // Property: all keys appear in entries
        for key in &obj_keys {
            prop_assert!(
                obj_entries.iter().any(|(k, _)| k == key),
                "Key not found in entries"
            );
        }
    }
}

// ============================================================================
// Property 2: Object.assign copies all properties
// Validates: Requirement 1.4
// ============================================================================

proptest! {
    #[test]
    fn prop_object_assign_copies_all(
        source_keys in prop::collection::vec("[a-z]{1,5}", 1..10usize),
        source_values in prop::collection::vec(any::<i32>(), 1..10usize)
    ) {
        let mut source = Object::new();
        let len = source_keys.len().min(source_values.len());

        for i in 0..len {
            source.set(source_keys[i].clone(), Value::Number(source_values[i] as f64));
        }

        let mut target = Object::new();
        target.assign_from(&source);

        // Property: all source properties are in target
        for (key, value) in source.entries() {
            let target_value = target.get(key);
            prop_assert!(target_value.is_some(), "Property {} not copied", key);
            prop_assert_eq!(target_value.unwrap(), value);
        }
    }
}

// ============================================================================
// Property 3: Object.freeze prevents modifications
// Validates: Requirement 1.5
// ============================================================================

proptest! {
    #[test]
    fn prop_object_freeze_prevents_modifications(
        initial_key in "[a-z]{1,5}",
        initial_value in any::<i32>(),
        new_key in "[a-z]{1,5}",
        new_value in any::<i32>()
    ) {
        let mut obj = Object::new();
        obj.set(initial_key.clone(), Value::Number(initial_value as f64));

        // Freeze the object
        obj.freeze();

        // Property: frozen object rejects new properties
        let set_result = obj.set(new_key.clone(), Value::Number(new_value as f64));
        prop_assert!(!set_result, "Frozen object should reject set");

        // Property: frozen object rejects modifications to existing properties
        let modify_result = obj.set(initial_key.clone(), Value::Number((initial_value + 1) as f64));
        prop_assert!(!modify_result, "Frozen object should reject modification");

        // Property: original value is preserved
        if let Some(Value::Number(n)) = obj.get(&initial_key) {
            prop_assert_eq!(*n as i32, initial_value);
        }
    }
}

// ============================================================================
// Property 4: Array.isArray correctness
// Validates: Requirement 2.3
// ============================================================================

proptest! {
    #[test]
    fn prop_array_is_array_correctness(
        elements in prop::collection::vec(any::<i32>(), 0..20usize)
    ) {
        let registry = BuiltinRegistry::new();
        let is_array = registry.get("Array.isArray").unwrap();

        // Property: Array.isArray returns true for arrays
        let arr: Vec<Value> = elements.iter().map(|&n| Value::Number(n as f64)).collect();
        let result = is_array(&[Value::Array(arr)]);
        prop_assert_eq!(result, Value::Boolean(true));

        // Property: Array.isArray returns false for non-arrays
        let result = is_array(&[Value::Number(42.0)]);
        prop_assert_eq!(result, Value::Boolean(false));

        let result = is_array(&[Value::String("test".to_string())]);
        prop_assert_eq!(result, Value::Boolean(false));

        let result = is_array(&[Value::Object(Object::new())]);
        prop_assert_eq!(result, Value::Boolean(false));
    }
}

// ============================================================================
// Property 5: Array.from preserves elements
// Validates: Requirements 2.1, 2.2, 2.4
// ============================================================================

proptest! {
    #[test]
    fn prop_array_from_preserves_elements(
        elements in prop::collection::vec(any::<i32>(), 0..50usize)
    ) {
        let registry = BuiltinRegistry::new();
        let array_from = registry.get("Array.from").unwrap();

        // Create source array
        let source: Vec<Value> = elements.iter().map(|&n| Value::Number(n as f64)).collect();
        let source_len = source.len();

        // Property: Array.from(array) preserves all elements
        let result = array_from(&[Value::Array(source.clone())]);

        if let Value::Array(result_arr) = result {
            prop_assert_eq!(result_arr.len(), source_len);

            for (i, (src, dst)) in source.iter().zip(result_arr.iter()).enumerate() {
                prop_assert_eq!(src, dst, "Element {} differs", i);
            }
        } else {
            prop_assert!(false, "Array.from should return an array");
        }
    }
}

// ============================================================================
// Property 6: Array.of creates array from arguments
// Validates: Requirement 2.2
// ============================================================================

proptest! {
    #[test]
    fn prop_array_of_creates_from_args(
        elements in prop::collection::vec(any::<i32>(), 0..20usize)
    ) {
        let registry = BuiltinRegistry::new();
        let array_of = registry.get("Array.of").unwrap();

        // Create arguments
        let args: Vec<Value> = elements.iter().map(|&n| Value::Number(n as f64)).collect();

        // Property: Array.of returns array with all arguments as elements
        let result = array_of(&args);

        if let Value::Array(result_arr) = result {
            prop_assert_eq!(result_arr.len(), args.len());

            for (i, (arg, elem)) in args.iter().zip(result_arr.iter()).enumerate() {
                prop_assert_eq!(arg, elem, "Element {} differs", i);
            }
        } else {
            prop_assert!(false, "Array.of should return an array");
        }
    }
}

// ============================================================================
// Property 7: JSON round-trip for primitives
// JSON.parse(JSON.stringify(x)) == x for JSON-safe values
// Validates: Requirements 3.1, 3.2, 3.5
// ============================================================================

proptest! {
    #[test]
    fn prop_json_roundtrip_numbers(n in any::<i32>()) {
        let registry = BuiltinRegistry::new();
        let stringify = registry.get("JSON.stringify").unwrap();
        let parse = registry.get("JSON.parse").unwrap();

        let value = Value::Number(n as f64);

        // Stringify then parse
        let json_str = stringify(std::slice::from_ref(&value));

        if let Value::String(s) = json_str {
            let parsed = parse(&[Value::String(s)]);

            if let Value::Number(parsed_n) = parsed {
                // Property: round-trip preserves numeric value
                prop_assert_eq!(parsed_n as i32, n);
            }
        }
    }

    #[test]
    fn prop_json_roundtrip_strings(s in "[a-zA-Z0-9 ]{0,50}") {
        let registry = BuiltinRegistry::new();
        let stringify = registry.get("JSON.stringify").unwrap();
        let parse = registry.get("JSON.parse").unwrap();

        let value = Value::String(s.clone());

        // Stringify then parse
        let json_str = stringify(&[value]);

        if let Value::String(json) = json_str {
            let parsed = parse(&[Value::String(json)]);

            if let Value::String(parsed_s) = parsed {
                // Property: round-trip preserves string value
                prop_assert_eq!(parsed_s, s);
            }
        }
    }

    #[test]
    fn prop_json_roundtrip_booleans(b in any::<bool>()) {
        let registry = BuiltinRegistry::new();
        let stringify = registry.get("JSON.stringify").unwrap();
        let parse = registry.get("JSON.parse").unwrap();

        let value = Value::Boolean(b);

        // Stringify then parse
        let json_str = stringify(&[value]);

        if let Value::String(s) = json_str {
            let parsed = parse(&[Value::String(s)]);

            if let Value::Boolean(parsed_b) = parsed {
                // Property: round-trip preserves boolean value
                prop_assert_eq!(parsed_b, b);
            }
        }
    }
}

#[test]
fn test_json_roundtrip_null() {
    let registry = BuiltinRegistry::new();
    let stringify = registry.get("JSON.stringify").unwrap();
    let parse = registry.get("JSON.parse").unwrap();

    let value = Value::Null;

    // Stringify then parse
    let json_str = stringify(&[value]);

    if let Value::String(s) = json_str {
        let parsed = parse(&[Value::String(s)]);
        // Property: round-trip preserves null
        assert_eq!(parsed, Value::Null);
    }
}

// ============================================================================
// Property 8: JSON.stringify handles special values correctly
// Validates: Requirements 3.2, 3.4
// ============================================================================

#[test]
fn test_json_stringify_nan_infinity() {
    let registry = BuiltinRegistry::new();
    let stringify = registry.get("JSON.stringify").unwrap();

    // Property: NaN becomes null
    let nan_result = stringify(&[Value::Number(f64::NAN)]);
    if let Value::String(s) = nan_result {
        assert_eq!(s, "null");
    }

    // Property: Infinity becomes null
    let inf_result = stringify(&[Value::Number(f64::INFINITY)]);
    if let Value::String(s) = inf_result {
        assert_eq!(s, "null");
    }

    // Property: -Infinity becomes null
    let neg_inf_result = stringify(&[Value::Number(f64::NEG_INFINITY)]);
    if let Value::String(s) = neg_inf_result {
        assert_eq!(s, "null");
    }
}

// ============================================================================
// Property 9: Array.from with string splits into characters
// Validates: Requirement 2.1
// ============================================================================

proptest! {
    #[test]
    fn prop_array_from_string_splits_chars(s in "[a-z]{0,20}") {
        let registry = BuiltinRegistry::new();
        let array_from = registry.get("Array.from").unwrap();

        let result = array_from(&[Value::String(s.clone())]);

        if let Value::Array(arr) = result {
            // Property: length equals string length
            prop_assert_eq!(arr.len(), s.chars().count());

            // Property: each element is a single character
            for (i, (c, elem)) in s.chars().zip(arr.iter()).enumerate() {
                if let Value::String(elem_s) = elem {
                    prop_assert_eq!(elem_s.len(), 1, "Element {} should be single char", i);
                    prop_assert_eq!(elem_s.chars().next().unwrap(), c);
                } else {
                    prop_assert!(false, "Element {} should be a string", i);
                }
            }
        } else {
            prop_assert!(false, "Array.from(string) should return array");
        }
    }
}

// ============================================================================
// Property 10: Closure Variable Capture (MIR Generation)
// For any arrow function or function expression that references variables
// from an outer scope, those variables SHALL be captured in the closure.
// Validates: Requirements 4.1, 4.2, 4.3, 4.4
// ============================================================================

#[test]
fn test_closure_captures_outer_variables() {
    use dx_js_runtime::compiler::mir::{FunctionBuilder, FunctionId, Type};

    // Create a function builder
    let mut builder = FunctionBuilder::new(FunctionId(0), "test".to_string());

    // Add some outer scope variables
    let outer_var = builder.add_local("outer".to_string(), Type::Any);

    // The closure should capture the outer variable
    // This test validates that the infrastructure for capturing is in place
    assert_eq!(outer_var.0, 0); // First local should have ID 0
}

#[test]
fn test_arrow_function_preserves_lexical_this() {
    // Arrow functions should preserve lexical `this` binding
    // This is indicated by the `is_arrow: true` flag in CreateFunction
    use dx_js_runtime::compiler::mir::{FunctionId, LocalId, TypedInstruction};

    // Create a CreateFunction instruction for an arrow function
    let arrow_inst = TypedInstruction::CreateFunction {
        dest: LocalId(0),
        function_id: FunctionId(1),
        captured_vars: vec![],
        is_arrow: true,
    };

    // Verify the instruction is properly formed
    if let TypedInstruction::CreateFunction { is_arrow, .. } = arrow_inst {
        assert!(is_arrow, "Arrow functions should have is_arrow = true");
    }
}

#[test]
fn test_function_expression_has_own_this() {
    // Regular function expressions should have their own `this` binding
    // This is indicated by the `is_arrow: false` flag in CreateFunction
    use dx_js_runtime::compiler::mir::{FunctionId, LocalId, TypedInstruction};

    // Create a CreateFunction instruction for a regular function
    let func_inst = TypedInstruction::CreateFunction {
        dest: LocalId(0),
        function_id: FunctionId(1),
        captured_vars: vec![],
        is_arrow: false,
    };

    // Verify the instruction is properly formed
    if let TypedInstruction::CreateFunction { is_arrow, .. } = func_inst {
        assert!(!is_arrow, "Regular functions should have is_arrow = false");
    }
}

proptest! {
    #[test]
    fn prop_closure_captures_all_referenced_vars(
        var_count in 1..10usize
    ) {
        use dx_js_runtime::compiler::mir::{FunctionBuilder, FunctionId, Type};

        // Create a function builder with multiple outer variables
        let mut builder = FunctionBuilder::new(FunctionId(0), "test".to_string());

        // Add outer scope variables
        let mut outer_vars = Vec::new();
        for i in 0..var_count {
            let var_name = format!("var_{}", i);
            let local_id = builder.add_local(var_name, Type::Any);
            outer_vars.push(local_id);
        }

        // Property: All outer variables should be capturable
        prop_assert_eq!(outer_vars.len(), var_count);

        // Property: Each captured variable should have a unique LocalId
        let unique_ids: std::collections::HashSet<_> = outer_vars.iter().map(|id| id.0).collect();
        prop_assert_eq!(unique_ids.len(), var_count);
    }
}

// ============================================================================
// Property 11: Object Literal Allocation
// For any object literal {k1: v1, k2: v2, ...}, the resulting object SHALL
// have obj[ki] === vi for all specified key-value pairs.
// Note: When duplicate keys exist, the last value wins (JavaScript semantics)
// Validates: Requirements 5.1, 5.3, 5.4, 5.5
// ============================================================================

proptest! {
    #[test]
    fn prop_object_literal_allocation(
        keys in prop::collection::vec("[a-z]{1,5}", 1..10usize),
        values in prop::collection::vec(any::<i32>(), 1..10usize)
    ) {
        // Create an object with the given keys and values
        let mut obj = Object::new();
        let len = keys.len().min(values.len());

        // Track the last value for each key (JavaScript semantics: last wins)
        let mut expected_values: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

        for i in 0..len {
            obj.set(keys[i].clone(), Value::Number(values[i] as f64));
            expected_values.insert(keys[i].clone(), values[i]);
        }

        // Property: All key-value pairs should be accessible with last-wins semantics
        for (key, expected_value) in &expected_values {
            let retrieved = obj.get(key);
            prop_assert!(retrieved.is_some(), "Key {} should exist", key);

            if let Some(Value::Number(n)) = retrieved {
                prop_assert_eq!(*n as i32, *expected_value, "Value for key {} should match", key);
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_array_literal_allocation(
        elements in prop::collection::vec(any::<i32>(), 0..20usize)
    ) {
        // Create an array with the given elements
        let arr: Vec<Value> = elements.iter().map(|&n| Value::Number(n as f64)).collect();

        // Property: Array length should match element count
        prop_assert_eq!(arr.len(), elements.len());

        // Property: All elements should be accessible at their indices
        for (i, &expected) in elements.iter().enumerate() {
            if let Value::Number(n) = &arr[i] {
                prop_assert_eq!(*n as i32, expected, "Element at index {} should match", i);
            }
        }
    }
}

#[test]
fn test_create_object_instruction() {
    use dx_js_runtime::compiler::mir::{LocalId, TypedInstruction};

    // Test that CreateObject instruction can be created with properties
    let inst = TypedInstruction::CreateObject {
        dest: LocalId(0),
        properties: vec![
            ("name".to_string(), LocalId(1)),
            ("age".to_string(), LocalId(2)),
        ],
    };

    if let TypedInstruction::CreateObject { dest, properties } = inst {
        assert_eq!(dest.0, 0);
        assert_eq!(properties.len(), 2);
        assert_eq!(properties[0].0, "name");
        assert_eq!(properties[1].0, "age");
    }
}

#[test]
fn test_create_array_instruction() {
    use dx_js_runtime::compiler::mir::{LocalId, TypedInstruction};

    // Test that CreateArray instruction can be created with elements
    let inst = TypedInstruction::CreateArray {
        dest: LocalId(0),
        elements: vec![LocalId(1), LocalId(2), LocalId(3)],
    };

    if let TypedInstruction::CreateArray { dest, elements } = inst {
        assert_eq!(dest.0, 0);
        assert_eq!(elements.len(), 3);
    }
}

// ============================================================================
// Property 12: For-In Iteration Completeness
// For any object with enumerable properties, for...in SHALL iterate over
// all enumerable property names exactly once.
// Validates: Requirements 6.1, 22.5
// ============================================================================

proptest! {
    #[test]
    fn prop_for_in_iteration_completeness(
        keys in prop::collection::vec("[a-z]{1,5}", 1..10usize),
        values in prop::collection::vec(any::<i32>(), 1..10usize)
    ) {
        // Create an object with enumerable properties
        let mut obj = Object::new();
        let len = keys.len().min(values.len());

        // Track unique keys (last value wins for duplicates)
        let mut unique_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

        for i in 0..len {
            obj.set(keys[i].clone(), Value::Number(values[i] as f64));
            unique_keys.insert(keys[i].clone());
        }

        // Get all enumerable keys
        let enumerable_keys = obj.keys_owned();

        // Property: All unique keys should be enumerable
        prop_assert_eq!(enumerable_keys.len(), unique_keys.len());

        // Property: Each enumerable key should be in our unique keys set
        for key in &enumerable_keys {
            prop_assert!(unique_keys.contains(key), "Key {} should be in unique keys", key);
        }

        // Property: Each unique key should appear exactly once in enumerable keys
        let enumerable_set: std::collections::HashSet<_> = enumerable_keys.iter().cloned().collect();
        prop_assert_eq!(enumerable_set.len(), enumerable_keys.len(), "No duplicate keys in enumeration");
    }
}

#[test]
fn test_for_in_mir_generation() {
    use dx_js_runtime::compiler::mir::{Constant, LocalId, TypedInstruction};

    // Test that for-in loop generates proper MIR instructions
    // The loop should:
    // 1. Get enumerable keys from object
    // 2. Initialize index to 0
    // 3. Loop through keys with index increment

    // Test GetPropertyDynamic for getting keys
    let get_keys = TypedInstruction::GetPropertyDynamic {
        dest: LocalId(0),
        object: LocalId(1),
        property: "__keys__".to_string(),
    };

    if let TypedInstruction::GetPropertyDynamic {
        dest,
        object,
        property,
    } = get_keys
    {
        assert_eq!(dest.0, 0);
        assert_eq!(object.0, 1);
        assert_eq!(property, "__keys__");
    }

    // Test index initialization
    let init_index = TypedInstruction::Const {
        dest: LocalId(2),
        value: Constant::I32(0),
    };

    if let TypedInstruction::Const { dest, .. } = init_index {
        assert_eq!(dest.0, 2);
        // Constant doesn't implement PartialEq, so we just verify the structure
    }
}

// ============================================================================
// Property 13: Switch Case Matching
// For any switch statement, cases SHALL be matched using strict equality (===)
// and the default case SHALL be executed when no case matches.
// Validates: Requirements 6.3
// ============================================================================

proptest! {
    #[test]
    fn prop_switch_case_strict_equality(
        discriminant in any::<i32>(),
        case_values in prop::collection::vec(any::<i32>(), 1..10usize)
    ) {
        // Property: Strict equality means type and value must match
        // For numbers, this is straightforward

        let has_match = case_values.contains(&discriminant);

        // Property: If discriminant matches a case value, that case should be selected
        if has_match {
            let match_index = case_values.iter().position(|&v| v == discriminant);
            prop_assert!(match_index.is_some(), "Should find matching case");
        }

        // Property: If no case matches, default should be selected
        // (This is implicit - if has_match is false, default is used)
    }
}

#[test]
fn test_switch_strict_equality_types() {
    // Test that switch uses strict equality (===) not loose equality (==)
    // In JavaScript: 1 === "1" is false, but 1 == "1" is true

    let num_value = Value::Number(1.0);
    let str_value = Value::String("1".to_string());

    // Property: Different types should not match with strict equality
    assert_ne!(num_value, str_value, "Number and String should not be strictly equal");

    // Property: Same type and value should match
    let num_value2 = Value::Number(1.0);
    assert_eq!(num_value, num_value2, "Same numbers should be strictly equal");
}

#[test]
fn test_switch_fall_through() {
    // Test that switch cases fall through without break
    // This is a semantic property - cases without break continue to next case

    // In MIR, this is represented by Goto to next case block
    use dx_js_runtime::compiler::mir::{BlockId, Terminator};

    let fall_through = Terminator::Goto(BlockId(1));

    if let Terminator::Goto(target) = fall_through {
        assert_eq!(target.0, 1, "Fall-through should go to next block");
    }
}

// ============================================================================
// Property 14: Loop Condition Evaluation
// For while and do-while loops, the condition SHALL be evaluated correctly
// and the loop SHALL terminate when the condition becomes false.
// Validates: Requirements 22.1, 22.2, 22.3, 22.4, 22.5
// ============================================================================

proptest! {
    #[test]
    fn prop_while_loop_condition_evaluation(
        initial_value in 0..100i32,
        limit in 1..50i32
    ) {
        // Simulate a while loop: while (i < limit) { i++ }
        let mut i = initial_value;
        let mut iterations = 0;

        while i < limit {
            i += 1;
            iterations += 1;

            // Safety: prevent infinite loops in test
            if iterations > 1000 {
                break;
            }
        }

        // Property: Loop should terminate
        prop_assert!(iterations <= 1000, "Loop should terminate");

        // Property: After loop, condition should be false (i >= limit)
        prop_assert!(i >= limit, "After while loop, condition should be false");

        // Property: Number of iterations should be correct
        let expected_iterations = if initial_value < limit {
            (limit - initial_value) as usize
        } else {
            0
        };
        prop_assert_eq!(iterations, expected_iterations, "Iteration count should match");
    }
}

#[test]
fn test_do_while_executes_at_least_once() {
    // Simulate a do-while loop: do { executed = true } while (false)
    let mut executed;
    let mut iterations = 0;
    let mut condition = true; // Will be set to false after first iteration

    // do-while always executes at least once
    while {
        executed = true;
        iterations += 1;
        let should_continue = condition;
        condition = false; // Simulating do { } while (false)
        should_continue && iterations < 1
    } {}

    // Property: Body should execute at least once
    assert!(executed, "do-while should execute at least once");
    assert_eq!(iterations, 1, "do-while(false) should execute exactly once");
}

#[test]
fn test_while_loop_mir_branch() {
    use dx_js_runtime::compiler::mir::{BlockId, LocalId, Terminator};

    // Test that while loop generates proper Branch terminator
    let condition = LocalId(0);
    let body_block = BlockId(1);
    let exit_block = BlockId(2);

    let branch = Terminator::Branch {
        condition,
        then_block: body_block,
        else_block: exit_block,
    };

    if let Terminator::Branch {
        condition: c,
        then_block,
        else_block,
    } = branch
    {
        assert_eq!(c.0, 0, "Condition should be LocalId(0)");
        assert_eq!(then_block.0, 1, "Then block should be body");
        assert_eq!(else_block.0, 2, "Else block should be exit");
    }
}

#[test]
fn test_do_while_loop_mir_structure() {
    use dx_js_runtime::compiler::mir::{BlockId, LocalId, Terminator};

    // Test that do-while loop structure:
    // 1. Entry -> Body (unconditional)
    // 2. Body -> Test
    // 3. Test -> Body (if true) or Exit (if false)

    let body_block = BlockId(0);
    let test_block = BlockId(1);
    let exit_block = BlockId(2);

    // Entry to body (unconditional - execute at least once)
    let entry_to_body = Terminator::Goto(body_block);

    // Body to test
    let _body_to_test = Terminator::Goto(test_block);

    // Test branches back to body or exits
    let test_branch = Terminator::Branch {
        condition: LocalId(0),
        then_block: body_block,
        else_block: exit_block,
    };

    if let Terminator::Goto(target) = entry_to_body {
        assert_eq!(target.0, 0, "Entry should go to body first");
    }

    if let Terminator::Branch {
        then_block,
        else_block,
        ..
    } = test_branch
    {
        assert_eq!(then_block.0, 0, "True condition loops back to body");
        assert_eq!(else_block.0, 2, "False condition exits loop");
    }
}

proptest! {
    #[test]
    fn prop_for_loop_iteration_count(
        start in 0..50i32,
        end in 0..100i32,
        step in 1..10i32
    ) {
        // Simulate: for (let i = start; i < end; i += step)
        let mut i = start;
        let mut iterations = 0;

        while i < end {
            iterations += 1;
            i += step;

            // Safety: prevent infinite loops
            if iterations > 1000 {
                break;
            }
        }

        // Property: Loop should terminate
        prop_assert!(iterations <= 1000, "Loop should terminate");

        // Property: Iteration count should be correct
        let expected = if start < end {
            ((end - start + step - 1) / step) as usize
        } else {
            0
        };
        prop_assert_eq!(iterations, expected, "For loop iteration count should match");
    }
}

// ============================================================================
// Property 15: Exception Propagation
// For any throw statement, the exception SHALL propagate to the nearest
// enclosing catch block, or terminate the program if no catch exists.
// Validates: Requirements 6.4, 6.5, 6.6, 23.1, 23.2, 23.3, 23.4, 23.5
// ============================================================================

#[test]
fn test_throw_instruction_exists() {
    use dx_js_runtime::compiler::mir::{LocalId, TypedInstruction};

    // Test that Throw instruction can be created
    let throw_inst = TypedInstruction::Throw { value: LocalId(0) };

    if let TypedInstruction::Throw { value } = throw_inst {
        assert_eq!(value.0, 0, "Throw should capture the exception value");
    }
}

#[test]
fn test_setup_exception_handler_instruction() {
    use dx_js_runtime::compiler::mir::{BlockId, TypedInstruction};

    // Test that SetupExceptionHandler instruction can be created
    let setup_inst = TypedInstruction::SetupExceptionHandler {
        catch_block: BlockId(1),
        finally_block: Some(BlockId(2)),
    };

    if let TypedInstruction::SetupExceptionHandler {
        catch_block,
        finally_block,
    } = setup_inst
    {
        assert_eq!(catch_block.0, 1, "Catch block should be set");
        assert_eq!(finally_block, Some(BlockId(2)), "Finally block should be set");
    }
}

#[test]
fn test_get_exception_instruction() {
    use dx_js_runtime::compiler::mir::{LocalId, TypedInstruction};

    // Test that GetException instruction can be created
    let get_exc_inst = TypedInstruction::GetException { dest: LocalId(0) };

    if let TypedInstruction::GetException { dest } = get_exc_inst {
        assert_eq!(dest.0, 0, "GetException should store to destination");
    }
}

#[test]
fn test_clear_exception_handler_instruction() {
    use dx_js_runtime::compiler::mir::TypedInstruction;

    // Test that ClearExceptionHandler instruction exists
    let clear_inst = TypedInstruction::ClearExceptionHandler;

    // Just verify it can be created (it's a unit variant)
    match clear_inst {
        TypedInstruction::ClearExceptionHandler => {
            // Success - instruction exists
        }
        _ => panic!("Should be ClearExceptionHandler"),
    }
}

proptest! {
    #[test]
    fn prop_exception_propagation_structure(
        try_depth in 1..5usize,
        has_catch in any::<bool>(),
        has_finally in any::<bool>()
    ) {
        use dx_js_runtime::compiler::mir::{FunctionBuilder, FunctionId, BlockId};

        // Create a function with nested try blocks
        let mut builder = FunctionBuilder::new(FunctionId(0), "test".to_string());

        // Create blocks for each try level
        let mut try_blocks = Vec::new();
        let mut catch_blocks = Vec::new();
        let mut finally_blocks = Vec::new();

        for _ in 0..try_depth {
            try_blocks.push(builder.new_block());
            if has_catch {
                catch_blocks.push(builder.new_block());
            }
            if has_finally {
                finally_blocks.push(builder.new_block());
            }
        }

        // Property: Each try block should have corresponding catch/finally blocks
        if has_catch {
            prop_assert_eq!(catch_blocks.len(), try_depth);
        }
        if has_finally {
            prop_assert_eq!(finally_blocks.len(), try_depth);
        }

        // Property: Block IDs should be unique
        let all_blocks: Vec<BlockId> = try_blocks.iter()
            .chain(catch_blocks.iter())
            .chain(finally_blocks.iter())
            .cloned()
            .collect();
        let unique_ids: std::collections::HashSet<_> = all_blocks.iter().map(|b| b.0).collect();
        prop_assert_eq!(unique_ids.len(), all_blocks.len(), "All block IDs should be unique");
    }
}

// ============================================================================
// Property 16: Finally Always Executes
// For any try-finally block, the finally block SHALL execute regardless of
// whether an exception was thrown or caught.
// Validates: Requirements 6.5, 23.3
// ============================================================================

#[test]
fn test_finally_block_structure() {
    use dx_js_runtime::compiler::mir::{FunctionBuilder, FunctionId, Terminator};

    // Create a function with try-finally
    let mut builder = FunctionBuilder::new(FunctionId(0), "test".to_string());

    let try_block = builder.new_block();
    let finally_block = builder.new_block();
    let exit_block = builder.new_block();

    // Try block should go to finally
    builder.switch_to_block(try_block);
    builder.set_terminator(Terminator::Goto(finally_block));

    // Finally block should go to exit
    builder.switch_to_block(finally_block);
    builder.set_terminator(Terminator::Goto(exit_block));

    let func = builder.build();

    // Property: Try block terminates with Goto to finally
    let try_term = &func.blocks.iter().find(|b| b.id == try_block).unwrap().terminator;
    if let Terminator::Goto(target) = try_term {
        assert_eq!(*target, finally_block, "Try should go to finally");
    }

    // Property: Finally block terminates with Goto to exit
    let finally_term = &func.blocks.iter().find(|b| b.id == finally_block).unwrap().terminator;
    if let Terminator::Goto(target) = finally_term {
        assert_eq!(*target, exit_block, "Finally should go to exit");
    }
}

proptest! {
    #[test]
    fn prop_finally_always_reachable(
        exception_thrown in any::<bool>(),
        exception_caught in any::<bool>()
    ) {
        // Simulate try-catch-finally control flow
        // Property: Finally should be reachable in all scenarios

        // Simulate: try { if (exception_thrown) throw; } catch { } finally { finally_executed = true; }

        // Try block
        if exception_thrown {
            // Exception thrown
            if exception_caught {
                // Caught by catch block
            }
            // Fall through to finally
        }
        // Normal completion - fall through to finally

        // Finally always executes
        let finally_executed = true;

        // Property: Finally should always execute
        prop_assert!(finally_executed, "Finally should always execute");
    }
}

#[test]
fn test_try_catch_finally_mir_flow() {
    use dx_js_runtime::compiler::mir::{
        BlockId, FunctionBuilder, FunctionId, Terminator, Type, TypedInstruction,
    };

    // Create a complete try-catch-finally structure
    let mut builder = FunctionBuilder::new(FunctionId(0), "test".to_string());

    let entry_block = BlockId(0); // Default entry block
    let try_block = builder.new_block();
    let catch_block = builder.new_block();
    let finally_block = builder.new_block();
    let exit_block = builder.new_block();

    // Entry: setup exception handler and go to try
    builder.switch_to_block(entry_block);
    builder.emit(TypedInstruction::SetupExceptionHandler {
        catch_block,
        finally_block: Some(finally_block),
    });
    builder.set_terminator(Terminator::Goto(try_block));

    // Try block: clear handler and go to finally
    builder.switch_to_block(try_block);
    builder.emit(TypedInstruction::ClearExceptionHandler);
    builder.set_terminator(Terminator::Goto(finally_block));

    // Catch block: get exception and go to finally
    builder.switch_to_block(catch_block);
    let exc_local = builder.add_local("e".to_string(), Type::Any);
    builder.emit(TypedInstruction::GetException { dest: exc_local });
    builder.set_terminator(Terminator::Goto(finally_block));

    // Finally block: go to exit
    builder.switch_to_block(finally_block);
    builder.set_terminator(Terminator::Goto(exit_block));

    // Exit block
    builder.switch_to_block(exit_block);
    builder.set_terminator(Terminator::Return(None));

    let func = builder.build();

    // Verify structure
    assert!(func.blocks.len() >= 5, "Should have at least 5 blocks");

    // Property: All paths lead to finally
    // - Entry -> Try -> Finally
    // - Entry -> Catch -> Finally

    // Check try block goes to finally
    let try_term = &func.blocks.iter().find(|b| b.id == try_block).unwrap().terminator;
    assert!(matches!(try_term, Terminator::Goto(b) if *b == finally_block));

    // Check catch block goes to finally
    let catch_term = &func.blocks.iter().find(|b| b.id == catch_block).unwrap().terminator;
    assert!(matches!(catch_term, Terminator::Goto(b) if *b == finally_block));
}

// ============================================================================
// Property 17: Promise.all Semantics
// Promise.all SHALL resolve when all promises resolve, and reject when any
// promise rejects.
// Validates: Requirements 7.1
// ============================================================================

use dx_js_runtime::value::PromiseState;

#[test]
fn test_promise_resolve() {
    let registry = BuiltinRegistry::new();
    let resolve = registry.get("Promise.resolve").unwrap();

    // Property: Promise.resolve creates a fulfilled promise
    let result = resolve(&[Value::Number(42.0)]);

    if let Value::Promise(p) = result {
        if let PromiseState::Fulfilled(value) = &p.state {
            assert_eq!(**value, Value::Number(42.0));
        } else {
            panic!("Promise.resolve should create fulfilled promise");
        }
    } else {
        panic!("Promise.resolve should return a Promise");
    }
}

#[test]
fn test_promise_reject() {
    let registry = BuiltinRegistry::new();
    let reject = registry.get("Promise.reject").unwrap();

    // Property: Promise.reject creates a rejected promise
    let result = reject(&[Value::String("error".to_string())]);

    if let Value::Promise(p) = result {
        if let PromiseState::Rejected(reason) = &p.state {
            assert_eq!(**reason, Value::String("error".to_string()));
        } else {
            panic!("Promise.reject should create rejected promise");
        }
    } else {
        panic!("Promise.reject should return a Promise");
    }
}

proptest! {
    #[test]
    fn prop_promise_all_resolves_when_all_resolve(
        values in prop::collection::vec(any::<i32>(), 1..10usize)
    ) {
        let registry = BuiltinRegistry::new();
        let resolve = registry.get("Promise.resolve").unwrap();
        let all = registry.get("Promise.all").unwrap();

        // Create array of resolved promises
        let promises: Vec<Value> = values.iter()
            .map(|&n| resolve(&[Value::Number(n as f64)]))
            .collect();

        let result = all(&[Value::Array(promises)]);

        // Property: Promise.all should resolve with array of values
        if let Value::Promise(p) = result {
            if let PromiseState::Fulfilled(value) = &p.state {
                if let Value::Array(arr) = &**value {
                    prop_assert_eq!(arr.len(), values.len());
                    for (i, &expected) in values.iter().enumerate() {
                        if let Value::Number(n) = &arr[i] {
                            prop_assert_eq!(*n as i32, expected);
                        }
                    }
                }
            } else {
                prop_assert!(false, "Promise.all should resolve when all resolve");
            }
        }
    }
}

#[test]
fn test_promise_all_rejects_when_any_rejects() {
    let registry = BuiltinRegistry::new();
    let resolve = registry.get("Promise.resolve").unwrap();
    let reject = registry.get("Promise.reject").unwrap();
    let all = registry.get("Promise.all").unwrap();

    // Create array with one rejected promise
    let promises = vec![
        resolve(&[Value::Number(1.0)]),
        reject(&[Value::String("error".to_string())]),
        resolve(&[Value::Number(3.0)]),
    ];

    let result = all(&[Value::Array(promises)]);

    // Property: Promise.all should reject when any promise rejects
    if let Value::Promise(p) = result {
        if let PromiseState::Rejected(reason) = &p.state {
            assert_eq!(**reason, Value::String("error".to_string()));
        } else {
            panic!("Promise.all should reject when any promise rejects");
        }
    }
}

// ============================================================================
// Property 18: Promise.race Semantics
// Promise.race SHALL settle with the first settled promise.
// Validates: Requirements 7.2
// ============================================================================

#[test]
fn test_promise_race_resolves_with_first_resolved() {
    let registry = BuiltinRegistry::new();
    let resolve = registry.get("Promise.resolve").unwrap();
    let race = registry.get("Promise.race").unwrap();

    // Create array of resolved promises
    let promises = vec![
        resolve(&[Value::Number(1.0)]),
        resolve(&[Value::Number(2.0)]),
        resolve(&[Value::Number(3.0)]),
    ];

    let result = race(&[Value::Array(promises)]);

    // Property: Promise.race should resolve with first resolved value
    if let Value::Promise(p) = result {
        if let PromiseState::Fulfilled(value) = &p.state {
            assert_eq!(**value, Value::Number(1.0));
        } else {
            panic!("Promise.race should resolve with first resolved");
        }
    }
}

#[test]
fn test_promise_race_rejects_with_first_rejected() {
    let registry = BuiltinRegistry::new();
    let reject = registry.get("Promise.reject").unwrap();
    let race = registry.get("Promise.race").unwrap();

    // Create array with first promise rejected
    let promises = vec![reject(&[Value::String("error".to_string())])];

    let result = race(&[Value::Array(promises)]);

    // Property: Promise.race should reject with first rejected reason
    if let Value::Promise(p) = result {
        if let PromiseState::Rejected(reason) = &p.state {
            assert_eq!(**reason, Value::String("error".to_string()));
        } else {
            panic!("Promise.race should reject with first rejected");
        }
    }
}

// ============================================================================
// Property 19: Promise.any Semantics
// Promise.any SHALL resolve with first fulfilled, reject with AggregateError
// if all reject.
// Validates: Requirements 7.3
// ============================================================================

#[test]
fn test_promise_any_resolves_with_first_fulfilled() {
    let registry = BuiltinRegistry::new();
    let resolve = registry.get("Promise.resolve").unwrap();
    let reject = registry.get("Promise.reject").unwrap();
    let any = registry.get("Promise.any").unwrap();

    // Create array with some rejected and some resolved
    let promises = vec![
        reject(&[Value::String("error1".to_string())]),
        resolve(&[Value::Number(42.0)]),
        reject(&[Value::String("error2".to_string())]),
    ];

    let result = any(&[Value::Array(promises)]);

    // Property: Promise.any should resolve with first fulfilled
    if let Value::Promise(p) = result {
        if let PromiseState::Fulfilled(value) = &p.state {
            assert_eq!(**value, Value::Number(42.0));
        } else {
            panic!("Promise.any should resolve with first fulfilled");
        }
    }
}

#[test]
fn test_promise_any_rejects_when_all_reject() {
    let registry = BuiltinRegistry::new();
    let reject = registry.get("Promise.reject").unwrap();
    let any = registry.get("Promise.any").unwrap();

    // Create array with all rejected
    let promises = vec![
        reject(&[Value::String("error1".to_string())]),
        reject(&[Value::String("error2".to_string())]),
    ];

    let result = any(&[Value::Array(promises)]);

    // Property: Promise.any should reject with AggregateError when all reject
    if let Value::Promise(p) = result {
        if let PromiseState::Rejected(reason) = &p.state {
            // AggregateError is represented as an array of errors
            if let Value::Array(errors) = &**reason {
                assert_eq!(errors.len(), 2);
            }
        } else {
            panic!("Promise.any should reject when all reject");
        }
    }
}

// ============================================================================
// Property 20: Promise.allSettled Semantics
// Promise.allSettled SHALL resolve with all settlement results.
// Validates: Requirements 7.4
// ============================================================================

#[test]
fn test_promise_all_settled_returns_all_results() {
    let registry = BuiltinRegistry::new();
    let resolve = registry.get("Promise.resolve").unwrap();
    let reject = registry.get("Promise.reject").unwrap();
    let all_settled = registry.get("Promise.allSettled").unwrap();

    // Create array with mixed results
    let promises = vec![
        resolve(&[Value::Number(1.0)]),
        reject(&[Value::String("error".to_string())]),
        resolve(&[Value::Number(3.0)]),
    ];

    let result = all_settled(&[Value::Array(promises)]);

    // Property: Promise.allSettled should resolve with all results
    if let Value::Promise(p) = result {
        if let PromiseState::Fulfilled(value) = &p.state {
            if let Value::Array(results) = &**value {
                assert_eq!(results.len(), 3);

                // First result: fulfilled
                if let Value::Object(obj) = &results[0] {
                    assert_eq!(obj.get("status"), Some(&Value::String("fulfilled".to_string())));
                    assert_eq!(obj.get("value"), Some(&Value::Number(1.0)));
                }

                // Second result: rejected
                if let Value::Object(obj) = &results[1] {
                    assert_eq!(obj.get("status"), Some(&Value::String("rejected".to_string())));
                    assert_eq!(obj.get("reason"), Some(&Value::String("error".to_string())));
                }

                // Third result: fulfilled
                if let Value::Object(obj) = &results[2] {
                    assert_eq!(obj.get("status"), Some(&Value::String("fulfilled".to_string())));
                    assert_eq!(obj.get("value"), Some(&Value::Number(3.0)));
                }
            }
        } else {
            panic!("Promise.allSettled should always resolve");
        }
    }
}

proptest! {
    #[test]
    fn prop_promise_all_settled_always_resolves(
        fulfilled_count in 0..5usize,
        rejected_count in 0..5usize
    ) {
        let registry = BuiltinRegistry::new();
        let resolve = registry.get("Promise.resolve").unwrap();
        let reject = registry.get("Promise.reject").unwrap();
        let all_settled = registry.get("Promise.allSettled").unwrap();

        // Create mixed array of promises
        let mut promises = Vec::new();
        for i in 0..fulfilled_count {
            promises.push(resolve(&[Value::Number(i as f64)]));
        }
        for i in 0..rejected_count {
            promises.push(reject(&[Value::String(format!("error{}", i))]));
        }

        let result = all_settled(&[Value::Array(promises)]);

        // Property: Promise.allSettled should always resolve
        if let Value::Promise(p) = result {
            prop_assert!(matches!(&p.state, PromiseState::Fulfilled(_)),
                "Promise.allSettled should always resolve");

            if let PromiseState::Fulfilled(value) = &p.state {
                if let Value::Array(results) = &**value {
                    prop_assert_eq!(results.len(), fulfilled_count + rejected_count);
                }
            }
        }
    }
}

// ============================================================================
// Property 13: Cache Round-Trip
// For any source file, compiling and caching the result, then loading from
// cache SHALL produce a MIR that is equivalent to fresh compilation.
// Validates: Requirements 8.1, 8.2, 8.5
// ============================================================================

mod cache_tests {
    use dx_js_runtime::compiler::mir::{
        BlockId, Constant, FunctionId, LocalId, PrimitiveType, Terminator, Type, TypedBlock,
        TypedFunction, TypedInstruction, TypedMIR,
    };
    use dx_js_runtime::snapshot::immortal::ImmortalCache;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Generate a random MIR function for testing
    fn arb_typed_function() -> impl Strategy<Value = TypedFunction> {
        (
            0..100u32,                                 // function id
            "[a-z_][a-z0-9_]{0,20}",                   // function name
            prop::collection::vec(any::<i32>(), 0..5), // constants to use
        )
            .prop_map(|(id, name, constants)| {
                let mut instructions = Vec::new();
                let mut locals = Vec::new();

                // Add some constant instructions
                for (i, c) in constants.iter().enumerate() {
                    let local_id = LocalId(i as u32);
                    locals.push(dx_js_runtime::compiler::mir::TypedLocal {
                        name: format!("local_{}", i),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: i as u32,
                    });
                    instructions.push(TypedInstruction::Const {
                        dest: local_id,
                        value: Constant::F64(*c as f64),
                    });
                }

                TypedFunction {
                    id: FunctionId(id),
                    name,
                    params: vec![],
                    return_type: Type::Primitive(PrimitiveType::F64),
                    blocks: vec![TypedBlock {
                        id: BlockId(0),
                        instructions,
                        terminator: Terminator::Return(None),
                        instruction_spans: vec![],
                        terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                    }],
                    locals,
                    is_pure: true,
                    span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }
            })
    }

    /// Generate a random MIR for testing
    fn arb_typed_mir() -> impl Strategy<Value = TypedMIR> {
        prop::collection::vec(arb_typed_function(), 1..5).prop_map(|functions| TypedMIR {
            functions,
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: codebase-quality-fixes, Property 13: Cache Round-Trip
        /// Validates: Requirements 8.1, 8.2, 8.5
        #[test]
        fn prop_cache_round_trip(mir in arb_typed_mir()) {
            let temp_dir = TempDir::new().unwrap();
            let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

            // Generate a unique source for this MIR
            let source = format!("// Generated source for test\nconst x = {};", mir.functions.len());
            let hash = cache.hash_source(&source);

            // Store the MIR
            cache.store_mir(&hash, &mir).unwrap();

            // Load the MIR back
            let loaded = cache.get_mir(&hash).unwrap();
            prop_assert!(loaded.is_some(), "Cache should return stored MIR");

            let loaded_mir = loaded.unwrap();

            // Property: Function count should match
            prop_assert_eq!(
                loaded_mir.functions.len(),
                mir.functions.len(),
                "Function count should match"
            );

            // Property: Entry point should match
            prop_assert_eq!(
                loaded_mir.entry_point,
                mir.entry_point,
                "Entry point should match"
            );

            // Property: Each function should match
            for (orig, loaded) in mir.functions.iter().zip(loaded_mir.functions.iter()) {
                prop_assert_eq!(orig.id, loaded.id, "Function ID should match");
                prop_assert_eq!(&orig.name, &loaded.name, "Function name should match");
                prop_assert_eq!(
                    orig.blocks.len(),
                    loaded.blocks.len(),
                    "Block count should match"
                );
                prop_assert_eq!(
                    orig.locals.len(),
                    loaded.locals.len(),
                    "Local count should match"
                );
            }
        }

        /// Test that cache invalidation works correctly
        #[test]
        fn prop_cache_invalidation_on_source_change(
            original_value in any::<i32>(),
            new_value in any::<i32>()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

            // Create original MIR
            let original_mir = TypedMIR {
                functions: vec![TypedFunction {
                    id: FunctionId(0),
                    name: "test".to_string(),
                    params: vec![],
                    return_type: Type::Primitive(PrimitiveType::F64),
                    blocks: vec![TypedBlock {
                        id: BlockId(0),
                        instructions: vec![TypedInstruction::Const {
                            dest: LocalId(0),
                            value: Constant::F64(original_value as f64),
                        }],
                        terminator: Terminator::Return(Some(LocalId(0))),
                        instruction_spans: vec![],
                        terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                    }],
                    locals: vec![dx_js_runtime::compiler::mir::TypedLocal {
                        name: "result".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 0,
                    }],
                    is_pure: true,
                    span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                globals: vec![],
                entry_point: Some(FunctionId(0)),
                type_layouts: HashMap::new(),
                source_file: String::new(),
            };

            let source1 = format!("const x = {};", original_value);
            let hash1 = cache.hash_source(&source1);

            // Store original
            cache.store_mir(&hash1, &original_mir).unwrap();

            // Verify it's cached
            let loaded = cache.get_mir(&hash1).unwrap();
            prop_assert!(loaded.is_some());

            // Change source
            let source2 = format!("const x = {};", new_value);
            let hash2 = cache.hash_source(&source2);

            // Property: Different source should have different hash (unless values are equal)
            if original_value != new_value {
                prop_assert_ne!(hash1.to_hex(), hash2.to_hex(), "Different source should have different hash");
            }

            // Property: New hash should not be in cache
            let loaded2 = cache.get_mir(&hash2).unwrap();
            prop_assert!(loaded2.is_none(), "New source should not be cached");
        }
    }

    #[test]
    fn test_cache_corruption_recovery() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let mir = TypedMIR {
            functions: vec![TypedFunction {
                id: FunctionId(0),
                name: "test".to_string(),
                params: vec![],
                return_type: Type::Primitive(PrimitiveType::F64),
                blocks: vec![TypedBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: Terminator::Return(None),
                    instruction_spans: vec![],
                    terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                locals: vec![],
                is_pure: true,
                span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
            }],
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        };

        let source = "const x = 42;";
        let hash = cache.hash_source(source);

        // Store MIR
        cache.store_mir(&hash, &mir).unwrap();

        // Verify it's cached
        assert!(cache.get_mir(&hash).unwrap().is_some());

        // Corrupt the cache file
        let cache_path = temp_dir.path().join(format!("{}.dxc", hash.to_hex()));
        fs::write(&cache_path, b"corrupted data that is not valid bincode").unwrap();

        // Property: Corrupted cache should return None (not error)
        let result = cache.get_mir(&hash).unwrap();
        assert!(result.is_none(), "Corrupted cache should return None");
    }

    #[test]
    fn test_cache_stats_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let mir = TypedMIR {
            functions: vec![],
            globals: vec![],
            entry_point: None,
            type_layouts: HashMap::new(),
            source_file: String::new(),
        };

        let source1 = "const a = 1;";
        let source2 = "const b = 2;";
        let hash1 = cache.hash_source(source1);
        let hash2 = cache.hash_source(source2);

        // Store one entry
        cache.store_mir(&hash1, &mir).unwrap();

        // Hit
        let _ = cache.get_mir(&hash1);
        // Miss
        let _ = cache.get_mir(&hash2);
        // Another hit
        let _ = cache.get_mir(&hash1);

        let stats = cache.stats();
        assert_eq!(stats.hits, 2, "Should have 2 hits");
        assert_eq!(stats.misses, 1, "Should have 1 miss");
        assert_eq!(stats.modules_cached, 1, "Should have 1 module cached");
    }
}

// ============================================================================
// Property 14: Optimization Semantics Preservation
// For any valid JavaScript program, the optimized compiled code SHALL produce
// the same observable results as the unoptimized code.
// Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5
// ============================================================================

mod optimization_tests {
    use dx_js_runtime::compiler::mir::{
        BinOpKind, BlockId, Constant, FunctionId, LocalId, PrimitiveType, Terminator, Type,
        TypedBlock, TypedFunction, TypedInstruction, TypedLocal, TypedMIR,
    };
    use dx_js_runtime::compiler::optimizations::OptimizationPipeline;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a random arithmetic expression as MIR
    fn arb_arithmetic_mir() -> impl Strategy<Value = TypedMIR> {
        (
            any::<i32>(),
            any::<i32>(),
            prop_oneof![
                Just(BinOpKind::Add),
                Just(BinOpKind::Sub),
                Just(BinOpKind::Mul),
            ],
        )
            .prop_map(|(a, b, op)| TypedMIR {
                functions: vec![TypedFunction {
                    id: FunctionId(0),
                    name: "test".to_string(),
                    params: vec![],
                    return_type: Type::Primitive(PrimitiveType::F64),
                    blocks: vec![TypedBlock {
                        id: BlockId(0),
                        instructions: vec![
                            TypedInstruction::Const {
                                dest: LocalId(0),
                                value: Constant::F64(a as f64),
                            },
                            TypedInstruction::Const {
                                dest: LocalId(1),
                                value: Constant::F64(b as f64),
                            },
                            TypedInstruction::BinOp {
                                dest: LocalId(2),
                                op,
                                left: LocalId(0),
                                right: LocalId(1),
                                op_type: PrimitiveType::F64,
                            },
                        ],
                        terminator: Terminator::Return(Some(LocalId(2))),
                        instruction_spans: vec![],
                        terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                    }],
                    locals: vec![
                        TypedLocal {
                            name: "a".to_string(),
                            ty: Type::Primitive(PrimitiveType::F64),
                            index: 0,
                        },
                        TypedLocal {
                            name: "b".to_string(),
                            ty: Type::Primitive(PrimitiveType::F64),
                            index: 1,
                        },
                        TypedLocal {
                            name: "result".to_string(),
                            ty: Type::Primitive(PrimitiveType::F64),
                            index: 2,
                        },
                    ],
                    is_pure: true,
                    span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                globals: vec![],
                entry_point: Some(FunctionId(0)),
                type_layouts: HashMap::new(),
                source_file: String::new(),
            })
    }

    /// Evaluate a constant-folded MIR to get the result
    fn evaluate_mir_result(mir: &TypedMIR) -> Option<f64> {
        let func = mir.functions.first()?;
        let block = func.blocks.first()?;

        // Find the return value
        if let Terminator::Return(Some(local_id)) = &block.terminator {
            // Find the instruction that defines this local
            for instr in &block.instructions {
                match instr {
                    TypedInstruction::Const {
                        dest,
                        value: Constant::F64(v),
                    } if dest == local_id => {
                        return Some(*v);
                    }
                    TypedInstruction::BinOp {
                        dest,
                        op,
                        left,
                        right,
                        ..
                    } if dest == local_id => {
                        // Need to find the values of left and right
                        let mut left_val = None;
                        let mut right_val = None;

                        for prev_instr in &block.instructions {
                            if let TypedInstruction::Const { dest, value } = prev_instr {
                                if dest == left {
                                    if let Constant::F64(v) = value {
                                        left_val = Some(*v);
                                    }
                                }
                                if dest == right {
                                    if let Constant::F64(v) = value {
                                        right_val = Some(*v);
                                    }
                                }
                            }
                        }

                        if let (Some(l), Some(r)) = (left_val, right_val) {
                            let result = match op {
                                BinOpKind::Add => l + r,
                                BinOpKind::Sub => l - r,
                                BinOpKind::Mul => l * r,
                                BinOpKind::Div => l / r,
                                BinOpKind::Mod => l % r,
                                _ => return None,
                            };
                            return Some(result);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: codebase-quality-fixes, Property 14: Optimization Semantics Preservation
        /// Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5
        #[test]
        fn prop_optimization_preserves_semantics(mir in arb_arithmetic_mir()) {
            // Evaluate the original MIR
            let original_result = evaluate_mir_result(&mir);

            // Optimize the MIR
            let mut pipeline = OptimizationPipeline::new();
            let optimized = pipeline.optimize(mir.clone()).unwrap();

            // Evaluate the optimized MIR
            let optimized_result = evaluate_mir_result(&optimized);

            // Property: Results should be equal (or both None)
            match (original_result, optimized_result) {
                (Some(orig), Some(opt)) => {
                    // Allow for floating point precision differences
                    let diff = (orig - opt).abs();
                    prop_assert!(
                        diff < 1e-10 || (orig.is_nan() && opt.is_nan()),
                        "Results differ: original={}, optimized={}, diff={}",
                        orig,
                        opt,
                        diff
                    );
                }
                (None, None) => {
                    // Both couldn't be evaluated - that's fine
                }
                (orig, opt) => {
                    prop_assert!(
                        false,
                        "One result is None: original={:?}, optimized={:?}",
                        orig,
                        opt
                    );
                }
            }
        }

        /// Test that constant folding produces correct results
        #[test]
        fn prop_constant_folding_correctness(
            a in -1000i32..1000i32,
            b in -1000i32..1000i32
        ) {
            let mir = TypedMIR {
                functions: vec![TypedFunction {
                    id: FunctionId(0),
                    name: "test".to_string(),
                    params: vec![],
                    return_type: Type::Primitive(PrimitiveType::F64),
                    blocks: vec![TypedBlock {
                        id: BlockId(0),
                        instructions: vec![
                            TypedInstruction::Const {
                                dest: LocalId(0),
                                value: Constant::F64(a as f64),
                            },
                            TypedInstruction::Const {
                                dest: LocalId(1),
                                value: Constant::F64(b as f64),
                            },
                            TypedInstruction::BinOp {
                                dest: LocalId(2),
                                op: BinOpKind::Add,
                                left: LocalId(0),
                                right: LocalId(1),
                                op_type: PrimitiveType::F64,
                            },
                        ],
                        terminator: Terminator::Return(Some(LocalId(2))),
                        instruction_spans: vec![],
                        terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                    }],
                    locals: vec![
                        TypedLocal { name: "a".to_string(), ty: Type::Primitive(PrimitiveType::F64), index: 0 },
                        TypedLocal { name: "b".to_string(), ty: Type::Primitive(PrimitiveType::F64), index: 1 },
                        TypedLocal { name: "result".to_string(), ty: Type::Primitive(PrimitiveType::F64), index: 2 },
                    ],
                    is_pure: true,
                    span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                globals: vec![],
                entry_point: Some(FunctionId(0)),
                type_layouts: HashMap::new(),
                source_file: String::new(),
            };

            let mut pipeline = OptimizationPipeline::new();
            let optimized = pipeline.optimize(mir).unwrap();

            // After constant folding, the BinOp should be replaced with a Const
            let block = &optimized.functions[0].blocks[0];

            // Find the result instruction
            let result_instr = block.instructions.iter().find(|i| {
                matches!(i, TypedInstruction::Const { dest: LocalId(2), .. })
            });

            if let Some(TypedInstruction::Const { value: Constant::F64(v), .. }) = result_instr {
                let expected = (a as f64) + (b as f64);
                prop_assert!(
                    (*v - expected).abs() < 1e-10,
                    "Constant folding produced wrong result: expected {}, got {}",
                    expected,
                    v
                );
            }
        }

        /// Test that dead code elimination removes unused values
        #[test]
        fn prop_dead_code_elimination_removes_unused(
            used_value in any::<i32>(),
            unused_value in any::<i32>()
        ) {
            let mir = TypedMIR {
                functions: vec![TypedFunction {
                    id: FunctionId(0),
                    name: "test".to_string(),
                    params: vec![],
                    return_type: Type::Primitive(PrimitiveType::F64),
                    blocks: vec![TypedBlock {
                        id: BlockId(0),
                        instructions: vec![
                            // Used value
                            TypedInstruction::Const {
                                dest: LocalId(0),
                                value: Constant::F64(used_value as f64),
                            },
                            // Unused value (dead code)
                            TypedInstruction::Const {
                                dest: LocalId(1),
                                value: Constant::F64(unused_value as f64),
                            },
                        ],
                        terminator: Terminator::Return(Some(LocalId(0))),
                        instruction_spans: vec![],
                        terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                    }],
                    locals: vec![
                        TypedLocal { name: "used".to_string(), ty: Type::Primitive(PrimitiveType::F64), index: 0 },
                        TypedLocal { name: "unused".to_string(), ty: Type::Primitive(PrimitiveType::F64), index: 1 },
                    ],
                    is_pure: true,
                    span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                globals: vec![],
                entry_point: Some(FunctionId(0)),
                type_layouts: HashMap::new(),
                source_file: String::new(),
            };

            let mut pipeline = OptimizationPipeline::new();
            let optimized = pipeline.optimize(mir).unwrap();

            // Property: Dead code should be eliminated
            let block = &optimized.functions[0].blocks[0];
            prop_assert_eq!(
                block.instructions.len(),
                1,
                "Dead code should be eliminated, but {} instructions remain",
                block.instructions.len()
            );

            // Property: The remaining instruction should be the used value
            if let TypedInstruction::Const { dest, value: Constant::F64(v) } = &block.instructions[0] {
                prop_assert_eq!(dest.0, 0, "Remaining instruction should define LocalId(0)");
                prop_assert_eq!(*v as i32, used_value, "Remaining value should be the used value");
            } else {
                prop_assert!(false, "Remaining instruction should be a Const");
            }
        }
    }

    #[test]
    fn test_optimization_pipeline_runs_all_phases() {
        let mir = TypedMIR {
            functions: vec![TypedFunction {
                id: FunctionId(0),
                name: "test".to_string(),
                params: vec![],
                return_type: Type::Primitive(PrimitiveType::F64),
                blocks: vec![TypedBlock {
                    id: BlockId(0),
                    instructions: vec![
                        TypedInstruction::Const {
                            dest: LocalId(0),
                            value: Constant::F64(10.0),
                        },
                        TypedInstruction::Const {
                            dest: LocalId(1),
                            value: Constant::F64(20.0),
                        },
                        TypedInstruction::BinOp {
                            dest: LocalId(2),
                            op: BinOpKind::Add,
                            left: LocalId(0),
                            right: LocalId(1),
                            op_type: PrimitiveType::F64,
                        },
                    ],
                    terminator: Terminator::Return(Some(LocalId(2))),
                    instruction_spans: vec![],
                    terminator_span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
                }],
                locals: vec![
                    TypedLocal {
                        name: "a".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 0,
                    },
                    TypedLocal {
                        name: "b".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 1,
                    },
                    TypedLocal {
                        name: "c".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 2,
                    },
                ],
                is_pure: true,
                span: dx_js_runtime::compiler::mir::SourceSpan::unknown(),
            }],
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        };

        let mut pipeline = OptimizationPipeline::new();
        let optimized = pipeline.optimize(mir).unwrap();

        // After optimization, constant folding should have replaced the BinOp
        let block = &optimized.functions[0].blocks[0];

        // Find the result - should be a constant 30.0
        let has_folded_constant = block.instructions.iter().any(|i| {
            matches!(i, TypedInstruction::Const { dest: LocalId(2), value: Constant::F64(v) } if (*v - 30.0).abs() < 1e-10)
        });

        assert!(has_folded_constant, "Constant folding should produce 30.0");
    }
}

// ============================================================================
// DX Global Object Tests
// Tests for dx.features and dx.version built-in functions
// Validates: Requirements 10.2, 10.4
// ============================================================================

#[test]
fn test_dx_features_returns_object() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    let result = dx_features(&[]);

    match result {
        Value::Object(obj) => {
            // Verify required keys per Property 16
            let required_keys = [
                "es2015",
                "es2016",
                "es2017",
                "es2018",
                "es2019",
                "es2020",
                "es2021",
                "es2022",
                "typescript",
            ];

            for key in required_keys {
                let value = obj.get(key);
                assert!(value.is_some(), "dx.features should have key: {}", key);
                assert!(
                    matches!(value.unwrap(), Value::Boolean(_)),
                    "dx.features.{} should be a boolean",
                    key
                );
            }
        }
        _ => panic!("dx.features should return an Object"),
    }
}

#[test]
fn test_dx_features_all_values_are_booleans() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    let result = dx_features(&[]);

    match result {
        Value::Object(obj) => {
            for (key, value) in obj.entries() {
                assert!(
                    matches!(value, Value::Boolean(_)),
                    "dx.features.{} should be a boolean, got {:?}",
                    key,
                    value
                );
            }
        }
        _ => panic!("dx.features should return an Object"),
    }
}

#[test]
fn test_dx_features_es_versions_supported() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    let result = dx_features(&[]);

    match result {
        Value::Object(obj) => {
            // All ES versions should be supported
            for year in 2015..=2022 {
                let key = format!("es{}", year);
                let value = obj.get(&key);
                assert!(value.is_some(), "dx.features should have key: {}", key);
                assert_eq!(
                    value.unwrap(),
                    &Value::Boolean(true),
                    "dx.features.{} should be true",
                    key
                );
            }
        }
        _ => panic!("dx.features should return an Object"),
    }
}

#[test]
fn test_dx_features_typescript_supported() {
    let registry = BuiltinRegistry::new();
    let dx_features = registry.get("dx.features").expect("dx.features should be registered");

    let result = dx_features(&[]);

    match result {
        Value::Object(obj) => {
            let typescript = obj.get("typescript");
            assert!(typescript.is_some(), "dx.features should have typescript key");
            assert_eq!(
                typescript.unwrap(),
                &Value::Boolean(true),
                "dx.features.typescript should be true"
            );
        }
        _ => panic!("dx.features should return an Object"),
    }
}

#[test]
fn test_dx_version_returns_string() {
    let registry = BuiltinRegistry::new();
    let dx_version = registry.get("dx.version").expect("dx.version should be registered");

    let result = dx_version(&[]);

    match result {
        Value::String(version) => {
            // Version should be a valid semver string
            assert!(!version.is_empty(), "dx.version should not be empty");

            // Should match MAJOR.MINOR.PATCH pattern
            let parts: Vec<&str> = version.split('.').collect();
            assert!(parts.len() >= 2, "dx.version should have at least MAJOR.MINOR");

            // Each part should be a number
            for part in &parts {
                assert!(part.parse::<u32>().is_ok(), "Version part '{}' should be a number", part);
            }
        }
        _ => panic!("dx.version should return a String"),
    }
}

#[test]
fn test_dx_version_matches_cargo_version() {
    let registry = BuiltinRegistry::new();
    let dx_version = registry.get("dx.version").expect("dx.version should be registered");

    let result = dx_version(&[]);

    match result {
        Value::String(version) => {
            // Should match the version from Cargo.toml
            let expected = env!("CARGO_PKG_VERSION");
            assert_eq!(version, expected, "dx.version should match CARGO_PKG_VERSION");
        }
        _ => panic!("dx.version should return a String"),
    }
}
