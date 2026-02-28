//! Property tests for TaggedValue heap object type identification
//!
//! Feature: dx-js-warnings-fixes
//! Property 1: Heap Object Type Identification
//! Validates: Requirements 1.1, 1.2, 1.3
//!
//! For any heap object pointer and object type (string, object, array, function),
//! creating a TaggedValue with that type and then checking is_<type>() SHALL return true,
//! and checking other type methods SHALL return false.

use dx_js_runtime::value::tagged::{HeapObjectType, TaggedValue};
use proptest::prelude::*;

// ============================================================================
// Property 1: Heap Object Type Identification
// For any heap object pointer and object type, type checking is correct
// Validates: Requirements 1.1, 1.2, 1.3
// ============================================================================

// Note: For heap objects with subtypes (object, array, function), the pointer
// payload is limited to 32 bits (bits 0-31) because bits 32-47 are used for
// the subtype. Strings use the full 48-bit payload.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Object type identification is mutually exclusive
    /// For any 32-bit pointer, creating an object TaggedValue should:
    /// - Return true for is_object()
    /// - Return false for is_array() and is_function()
    #[test]
    fn prop_object_type_identification(ptr in 1u32..u32::MAX) {
        let ptr = ptr as *const u8;
        let value = TaggedValue::from_object_ptr(ptr);

        // Property: is_object() returns true
        prop_assert!(value.is_object(), "Object should be identified as object");

        // Property: is_array() returns false
        prop_assert!(!value.is_array(), "Object should not be identified as array");

        // Property: is_function() returns false
        prop_assert!(!value.is_function(), "Object should not be identified as function");

        // Property: is_string() returns false
        prop_assert!(!value.is_string(), "Object should not be identified as string");

        // Property: is_heap_object() returns true
        prop_assert!(value.is_heap_object(), "Object should be a heap object");

        // Property: type_of() returns "object"
        prop_assert_eq!(value.type_of(), "object", "typeof object should be 'object'");
    }

    /// Property: Array type identification is mutually exclusive
    /// For any 32-bit pointer, creating an array TaggedValue should:
    /// - Return true for is_array()
    /// - Return false for is_object() and is_function()
    #[test]
    fn prop_array_type_identification(ptr in 1u32..u32::MAX) {
        let ptr = ptr as *const u8;
        let value = TaggedValue::from_array_ptr(ptr);

        // Property: is_array() returns true
        prop_assert!(value.is_array(), "Array should be identified as array");

        // Property: is_object() returns false
        prop_assert!(!value.is_object(), "Array should not be identified as object");

        // Property: is_function() returns false
        prop_assert!(!value.is_function(), "Array should not be identified as function");

        // Property: is_string() returns false
        prop_assert!(!value.is_string(), "Array should not be identified as string");

        // Property: is_heap_object() returns true
        prop_assert!(value.is_heap_object(), "Array should be a heap object");

        // Property: type_of() returns "object" (arrays are objects in JS)
        prop_assert_eq!(value.type_of(), "object", "typeof array should be 'object'");
    }

    /// Property: Function type identification is mutually exclusive
    /// For any 32-bit pointer, creating a function TaggedValue should:
    /// - Return true for is_function()
    /// - Return false for is_object() and is_array()
    #[test]
    fn prop_function_type_identification(ptr in 1u32..u32::MAX) {
        let ptr = ptr as *const u8;
        let value = TaggedValue::from_function_ptr(ptr);

        // Property: is_function() returns true
        prop_assert!(value.is_function(), "Function should be identified as function");

        // Property: is_object() returns false
        prop_assert!(!value.is_object(), "Function should not be identified as object");

        // Property: is_array() returns false
        prop_assert!(!value.is_array(), "Function should not be identified as array");

        // Property: is_string() returns false
        prop_assert!(!value.is_string(), "Function should not be identified as string");

        // Property: is_heap_object() returns true
        prop_assert!(value.is_heap_object(), "Function should be a heap object");

        // Property: type_of() returns "function"
        prop_assert_eq!(value.type_of(), "function", "typeof function should be 'function'");
    }

    /// Property: String type identification
    /// For any 32-bit pointer, creating a string TaggedValue should:
    /// - Return true for is_string()
    /// - Return false for is_object(), is_array(), and is_function()
    /// Note: Strings also use 32-bit pointers because bits 32-47 are reserved for subtypes
    #[test]
    fn prop_string_type_identification(ptr in 1u32..u32::MAX) {
        let ptr = ptr as *const u8;
        let value = TaggedValue::from_string_ptr(ptr);

        // Property: is_string() returns true
        prop_assert!(value.is_string(), "String should be identified as string");

        // Property: is_object() returns false
        prop_assert!(!value.is_object(), "String should not be identified as object");

        // Property: is_array() returns false
        prop_assert!(!value.is_array(), "String should not be identified as array");

        // Property: is_function() returns false
        prop_assert!(!value.is_function(), "String should not be identified as function");

        // Property: is_heap_object() returns true
        prop_assert!(value.is_heap_object(), "String should be a heap object");

        // Property: type_of() returns "string"
        prop_assert_eq!(value.type_of(), "string", "typeof string should be 'string'");
    }

    /// Property: Pointer extraction preserves address for 32-bit pointers
    /// For heap objects with subtypes, the extracted pointer should match the original
    #[test]
    fn prop_pointer_extraction_preserves_address(ptr in 1u32..u32::MAX) {
        let original_ptr = ptr as *const u8;

        // Test for each heap object type with subtypes
        let object_value = TaggedValue::from_object_ptr(original_ptr);
        let array_value = TaggedValue::from_array_ptr(original_ptr);
        let function_value = TaggedValue::from_function_ptr(original_ptr);

        // Property: Extracted pointer matches original (masked to 32 bits for subtyped objects)
        // Note: The subtype bits occupy bits 32-47, so only bits 0-31 are preserved
        let mask = 0x0000_0000_FFFF_FFFFu64;
        let expected = ((ptr as u64) & mask) as *const u8;

        prop_assert_eq!(object_value.as_ptr(), Some(expected), "Object pointer should be preserved");
        prop_assert_eq!(array_value.as_ptr(), Some(expected), "Array pointer should be preserved");
        prop_assert_eq!(function_value.as_ptr(), Some(expected), "Function pointer should be preserved");
    }

    /// Property: String pointer extraction preserves 32-bit address
    /// Note: Despite the comment about 48-bit pointers, strings also use 32-bit pointers
    /// because bits 32-47 are reserved for subtypes (strings have subtype 0)
    #[test]
    fn prop_string_pointer_extraction_preserves_address(ptr in 1u32..u32::MAX) {
        let original_ptr = ptr as *const u8;
        let string_value = TaggedValue::from_string_ptr(original_ptr);

        // Property: Extracted pointer matches original (32 bits for strings too)
        let mask = 0x0000_0000_FFFF_FFFFu64;
        let expected = ((ptr as u64) & mask) as *const u8;

        prop_assert_eq!(string_value.as_ptr(), Some(expected), "String pointer should be preserved");
    }
}

// ============================================================================
// Property: Primitive types are not heap objects
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Numbers are not heap objects
    #[test]
    fn prop_numbers_not_heap_objects(n in any::<f64>()) {
        let value = TaggedValue::from_f64(n);

        // Property: Numbers are not heap objects (unless NaN with our tag bits)
        if !n.is_nan() {
            prop_assert!(!value.is_object(), "Number should not be object");
            prop_assert!(!value.is_array(), "Number should not be array");
            prop_assert!(!value.is_function(), "Number should not be function");
        }
    }

    /// Property: Integers are not heap objects
    #[test]
    fn prop_integers_not_heap_objects(n in any::<i32>()) {
        let value = TaggedValue::from_i32(n);

        // Property: Integers are not heap objects
        prop_assert!(!value.is_object(), "Integer should not be object");
        prop_assert!(!value.is_array(), "Integer should not be array");
        prop_assert!(!value.is_function(), "Integer should not be function");
        prop_assert!(!value.is_heap_object(), "Integer should not be heap object");
    }

    /// Property: Booleans are not heap objects
    #[test]
    fn prop_booleans_not_heap_objects(b in any::<bool>()) {
        let value = TaggedValue::from_bool(b);

        // Property: Booleans are not heap objects
        prop_assert!(!value.is_object(), "Boolean should not be object");
        prop_assert!(!value.is_array(), "Boolean should not be array");
        prop_assert!(!value.is_function(), "Boolean should not be function");
        prop_assert!(!value.is_heap_object(), "Boolean should not be heap object");
    }
}

// ============================================================================
// Unit tests for edge cases
// ============================================================================

#[test]
fn test_null_undefined_not_heap_objects() {
    let null = TaggedValue::null();
    let undef = TaggedValue::undefined();

    // Null is not a heap object
    assert!(!null.is_object());
    assert!(!null.is_array());
    assert!(!null.is_function());
    assert!(!null.is_heap_object());

    // Undefined is not a heap object
    assert!(!undef.is_object());
    assert!(!undef.is_array());
    assert!(!undef.is_function());
    assert!(!undef.is_heap_object());
}

#[test]
fn test_symbol_not_heap_object_types() {
    let sym = TaggedValue::from_symbol(42);

    // Symbol is not object/array/function
    assert!(!sym.is_object());
    assert!(!sym.is_array());
    assert!(!sym.is_function());
}

#[test]
fn test_heap_object_type_enum() {
    // Test that HeapObjectType enum values are distinct
    assert_ne!(HeapObjectType::String as u8, HeapObjectType::Object as u8);
    assert_ne!(HeapObjectType::Object as u8, HeapObjectType::Array as u8);
    assert_ne!(HeapObjectType::Array as u8, HeapObjectType::Function as u8);
}
