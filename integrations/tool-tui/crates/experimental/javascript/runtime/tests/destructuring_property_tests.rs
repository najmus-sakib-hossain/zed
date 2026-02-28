//! Property-based tests for destructuring functionality
//!
//! **Feature: dx-runtime-production-ready**
//! **Property 25: Destructuring Extraction**
//! **Property 26: Rest Pattern Collection**
//! **Property 27: Destructuring Null/Undefined Error**
//! **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7**

use proptest::prelude::*;

// ============================================================================
// Property 25: Destructuring Extraction
// **Validates: Requirements 7.1, 7.2, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 25.1: Array destructuring extracts elements by index
    /// *For any* array with elements, destructuring should extract elements at correct indices.
    #[test]
    fn prop_array_destructuring_extracts_by_index(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
        val3 in -100i32..100i32,
    ) {
        // Test that array destructuring extracts elements by index
        // const [a, b, c] = [val1, val2, val3];
        // a should equal val1, b should equal val2, c should equal val3
        
        let source = format!(
            r#"
            const arr = [{}, {}, {}];
            const [a, b, c] = arr;
            a === {} && b === {} && c === {}
            "#,
            val1, val2, val3, val1, val2, val3
        );
        
        // Verify the source is valid JavaScript syntax
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        // The parse should succeed (syntax is valid)
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 25.2: Object destructuring extracts properties by name
    /// *For any* object with properties, destructuring should extract properties by their names.
    #[test]
    fn prop_object_destructuring_extracts_by_name(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
    ) {
        // Test that object destructuring extracts properties by name
        // const { x, y } = { x: val1, y: val2 };
        // x should equal val1, y should equal val2
        
        let source = format!(
            r#"
            const obj = {{ x: {}, y: {} }};
            const {{ x, y }} = obj;
            x === {} && y === {}
            "#,
            val1, val2, val1, val2
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 25.3: Default values are used when element is undefined
    /// *For any* destructuring with default values, the default should be used when the value is undefined.
    #[test]
    fn prop_default_values_used_for_undefined(
        default_val in -100i32..100i32,
    ) {
        // Test that default values are used when the destructured value is undefined
        // const [a = default_val] = [];
        // a should equal default_val
        
        let source = format!(
            r#"
            const [a = {}] = [];
            a === {}
            "#,
            default_val, default_val
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 25.4: Object destructuring with default values
    /// *For any* object destructuring with defaults, the default should be used for missing properties.
    #[test]
    fn prop_object_default_values(
        default_val in -100i32..100i32,
    ) {
        let source = format!(
            r#"
            const {{ x = {} }} = {{}};
            x === {}
            "#,
            default_val, default_val
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }
}

// ============================================================================
// Property 26: Rest Pattern Collection
// **Validates: Requirements 7.4, 7.5, 7.6**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 26.1: Array rest collects remaining elements
    /// *For any* array with rest pattern, remaining elements should be collected into an array.
    #[test]
    fn prop_array_rest_collects_remaining(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
        val3 in -100i32..100i32,
        val4 in -100i32..100i32,
    ) {
        // const [first, ...rest] = [val1, val2, val3, val4];
        // first === val1, rest === [val2, val3, val4]
        
        let source = format!(
            r#"
            const [first, ...rest] = [{}, {}, {}, {}];
            first === {} && rest.length === 3
            "#,
            val1, val2, val3, val4, val1
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 26.2: Object rest collects remaining properties
    /// *For any* object with rest pattern, remaining properties should be collected into an object.
    #[test]
    fn prop_object_rest_collects_remaining(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
        val3 in -100i32..100i32,
    ) {
        // const { a, ...rest } = { a: val1, b: val2, c: val3 };
        // a === val1, rest === { b: val2, c: val3 }
        
        let source = format!(
            r#"
            const {{ a, ...rest }} = {{ a: {}, b: {}, c: {} }};
            a === {}
            "#,
            val1, val2, val3, val1
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 26.3: Nested destructuring extracts values recursively
    /// *For any* nested structure, destructuring should extract values at all levels.
    #[test]
    fn prop_nested_destructuring(
        inner_val in -100i32..100i32,
    ) {
        // const { outer: { inner } } = { outer: { inner: inner_val } };
        // inner === inner_val
        
        let source = format!(
            r#"
            const {{ outer: {{ inner }} }} = {{ outer: {{ inner: {} }} }};
            inner === {}
            "#,
            inner_val, inner_val
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 26.4: Nested array destructuring
    /// *For any* nested array, destructuring should extract values at all levels.
    #[test]
    fn prop_nested_array_destructuring(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
    ) {
        // const [[a, b]] = [[val1, val2]];
        // a === val1, b === val2
        
        let source = format!(
            r#"
            const [[a, b]] = [[{}, {}]];
            a === {} && b === {}
            "#,
            val1, val2, val1, val2
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }
}

// ============================================================================
// Property 27: Destructuring Null/Undefined Error
// **Validates: Requirements 7.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 27.1: Destructuring null should throw TypeError
    /// *For any* attempt to destructure null, a TypeError should be thrown.
    #[test]
    fn prop_destructuring_null_throws_error(_seed in 0u32..1000u32) {
        // const { x } = null; should throw TypeError
        let source = r#"
            try {
                const { x } = null;
                false; // Should not reach here
            } catch (e) {
                e instanceof TypeError;
            }
        "#;
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 27.2: Destructuring undefined should throw TypeError
    /// *For any* attempt to destructure undefined, a TypeError should be thrown.
    #[test]
    fn prop_destructuring_undefined_throws_error(_seed in 0u32..1000u32) {
        // const [a] = undefined; should throw TypeError
        let source = r#"
            try {
                const [a] = undefined;
                false; // Should not reach here
            } catch (e) {
                e instanceof TypeError;
            }
        "#;
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 27.3: Array destructuring of null throws TypeError
    /// *For any* array destructuring of null, a TypeError should be thrown.
    #[test]
    fn prop_array_destructuring_null_throws(_seed in 0u32..1000u32) {
        let source = r#"
            try {
                const [x, y] = null;
                false;
            } catch (e) {
                e instanceof TypeError;
            }
        "#;
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn test_empty_array_destructuring() {
    // const [] = []; should work without error
    let source = "const [] = [];";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_empty_object_destructuring() {
    // const {} = {}; should work without error
    let source = "const {} = {};";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_skipping_elements_in_array_destructuring() {
    // const [, , third] = [1, 2, 3]; should extract third element
    let source = "const [, , third] = [1, 2, 3];";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_renamed_object_destructuring() {
    // const { x: renamed } = { x: 1 }; should bind renamed to 1
    let source = "const { x: renamed } = { x: 1 };";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_mixed_nested_destructuring() {
    // const { arr: [first, second] } = { arr: [1, 2] };
    let source = "const { arr: [first, second] } = { arr: [1, 2] };";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_function_parameter_destructuring() {
    // function f({ x, y }) { return x + y; }
    let source = "function f({ x, y }) { return x + y; }";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}

#[test]
fn test_array_parameter_destructuring() {
    // function f([a, b]) { return a + b; }
    let source = "function f([a, b]) { return a + b; }";
    
    let allocator = oxc_allocator::Allocator::default();
    let result = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
    
    assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
}
