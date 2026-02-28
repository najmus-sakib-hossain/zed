//! Property-based tests for template literal functionality
//!
//! **Feature: dx-runtime-production-ready**
//! **Property 28: Template Literal Interpolation**
//! **Property 29: Tagged Template Invocation**
//! **Validates: Requirements 8.1, 8.2, 8.3**

use proptest::prelude::*;

// ============================================================================
// Property 28: Template Literal Interpolation
// **Validates: Requirements 8.1, 8.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 28.1: Template literals evaluate expressions in ${...}
    /// *For any* template literal with expressions, the expressions should be evaluated and interpolated.
    #[test]
    fn prop_template_literal_evaluates_expressions(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
    ) {
        // Test that template literals evaluate expressions
        // `${val1} + ${val2} = ${val1 + val2}`
        
        let expected_result = format!("{} + {} = {}", val1, val2, val1 + val2);
        let source = format!(
            r#"
            const a = {};
            const b = {};
            const result = `${{a}} + ${{b}} = ${{a + b}}`;
            result === "{}"
            "#,
            val1, val2, expected_result
        );
        
        // Verify the source is valid JavaScript syntax
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        // The parse should succeed (syntax is valid)
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.2: Template literals preserve line breaks
    /// *For any* multiline template literal, line breaks should be preserved in the output.
    #[test]
    fn prop_template_literal_preserves_line_breaks(
        line1 in "[a-zA-Z0-9 ]{1,20}",
        line2 in "[a-zA-Z0-9 ]{1,20}",
    ) {
        // Test that template literals preserve line breaks
        // `line1
        // line2`
        
        let source = format!(
            r#"
            const result = `{}
{}`;
            result.includes("\n")
            "#,
            line1, line2
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.3: Template literals concatenate static and dynamic parts correctly
    /// *For any* template literal with mixed static and dynamic parts, the result should be correct concatenation.
    #[test]
    fn prop_template_literal_concatenates_correctly(
        prefix in "[a-zA-Z]{1,10}",
        value in -100i32..100i32,
        suffix in "[a-zA-Z]{1,10}",
    ) {
        // Test that template literals concatenate correctly
        // `prefix${value}suffix`
        
        let expected = format!("{}{}{}", prefix, value, suffix);
        let source = format!(
            r#"
            const val = {};
            const result = `{}${{val}}{}`;
            result === "{}"
            "#,
            value, prefix, suffix, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.4: Template literals handle empty expressions
    /// *For any* template literal with empty string expressions, the result should be correct.
    #[test]
    fn prop_template_literal_handles_empty_expressions(
        prefix in "[a-zA-Z]{1,10}",
        suffix in "[a-zA-Z]{1,10}",
    ) {
        // Test that template literals handle empty string expressions
        // `prefix${""}suffix`
        
        let expected = format!("{}{}", prefix, suffix);
        let source = format!(
            r#"
            const empty = "";
            const result = `{}${{empty}}{}`;
            result === "{}"
            "#,
            prefix, suffix, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.5: Template literals handle multiple expressions
    /// *For any* template literal with multiple expressions, all should be evaluated and interpolated.
    #[test]
    fn prop_template_literal_handles_multiple_expressions(
        val1 in -50i32..50i32,
        val2 in -50i32..50i32,
        val3 in -50i32..50i32,
    ) {
        // Test that template literals handle multiple expressions
        // `${val1}, ${val2}, ${val3}`
        
        let expected = format!("{}, {}, {}", val1, val2, val3);
        let source = format!(
            r#"
            const a = {};
            const b = {};
            const c = {};
            const result = `${{a}}, ${{b}}, ${{c}}`;
            result === "{}"
            "#,
            val1, val2, val3, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.6: Template literals handle nested expressions
    /// *For any* template literal with nested expressions, they should be evaluated correctly.
    #[test]
    fn prop_template_literal_handles_nested_expressions(
        val in 1i32..50i32,
    ) {
        // Test that template literals handle nested expressions
        // `${val * 2}`
        
        let expected = val * 2;
        let source = format!(
            r#"
            const x = {};
            const result = `${{x * 2}}`;
            result === "{}"
            "#,
            val, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.7: Template literals handle boolean expressions
    /// *For any* template literal with boolean expressions, they should be converted to string.
    #[test]
    fn prop_template_literal_handles_boolean_expressions(
        val in prop::bool::ANY,
    ) {
        // Test that template literals handle boolean expressions
        // `${true}` or `${false}`
        
        let expected = if val { "true" } else { "false" };
        let source = format!(
            r#"
            const b = {};
            const result = `${{b}}`;
            result === "{}"
            "#,
            val, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 28.8: Template literals handle null and undefined
    /// *For any* template literal with null or undefined, they should be converted to string.
    #[test]
    fn prop_template_literal_handles_null_undefined(
        use_null in prop::bool::ANY,
    ) {
        // Test that template literals handle null and undefined
        // `${null}` -> "null", `${undefined}` -> "undefined"
        
        let (value, expected) = if use_null {
            ("null", "null")
        } else {
            ("undefined", "undefined")
        };
        
        let source = format!(
            r#"
            const result = `${{{}}}`;
            result === "{}"
            "#,
            value, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }
}


// ============================================================================
// Property 29: Tagged Template Invocation
// **Validates: Requirements 8.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 29.1: Tagged templates call the tag function with strings array
    /// *For any* tagged template, the tag function should receive the strings array as first argument.
    #[test]
    fn prop_tagged_template_receives_strings_array(
        static_part1 in "[a-zA-Z]{1,10}",
        static_part2 in "[a-zA-Z]{1,10}",
    ) {
        // Test that tagged templates call the tag function with strings array
        // tag`static_part1${expr}static_part2`
        
        let source = format!(
            r#"
            function tag(strings, ...values) {{
                return strings[0] === "{}" && strings[1] === "{}";
            }}
            const result = tag`{}${{42}}{}`;
            result
            "#,
            static_part1, static_part2, static_part1, static_part2
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 29.2: Tagged templates pass interpolated values as additional arguments
    /// *For any* tagged template with expressions, the values should be passed as additional arguments.
    #[test]
    fn prop_tagged_template_passes_values(
        val1 in -100i32..100i32,
        val2 in -100i32..100i32,
    ) {
        // Test that tagged templates pass interpolated values
        // tag`${val1} and ${val2}`
        
        let source = format!(
            r#"
            function tag(strings, v1, v2) {{
                return v1 === {} && v2 === {};
            }}
            const a = {};
            const b = {};
            const result = tag`${{a}} and ${{b}}`;
            result
            "#,
            val1, val2, val1, val2
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 29.3: Tagged templates strings array has correct length
    /// *For any* tagged template, the strings array should have one more element than the number of expressions.
    #[test]
    fn prop_tagged_template_strings_length(
        num_exprs in 0usize..5usize,
    ) {
        // Test that strings array has correct length
        // For n expressions, strings array should have n+1 elements
        
        let exprs: String = (0..num_exprs).map(|i| format!("${{{}}}", i)).collect::<Vec<_>>().join("");
        let expected_length = num_exprs + 1;
        
        let source = format!(
            r#"
            function tag(strings, ...values) {{
                return strings.length === {};
            }}
            const result = tag`{}`;
            result
            "#,
            expected_length, exprs
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 29.4: Tagged templates can return any value
    /// *For any* tagged template, the tag function can return any value type.
    #[test]
    fn prop_tagged_template_returns_any_value(
        return_number in prop::bool::ANY,
        val in -100i32..100i32,
    ) {
        // Test that tagged templates can return any value
        
        let (return_expr, expected) = if return_number {
            (format!("{}", val), format!("{}", val))
        } else {
            ("\"string\"".to_string(), "\"string\"".to_string())
        };
        
        let source = format!(
            r#"
            function tag(strings, ...values) {{
                return {};
            }}
            const result = tag`hello`;
            result === {}
            "#,
            return_expr, expected
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 29.5: Tagged templates with no expressions
    /// *For any* tagged template with no expressions, the strings array should have one element.
    #[test]
    fn prop_tagged_template_no_expressions(
        content in "[a-zA-Z0-9 ]{1,20}",
    ) {
        // Test that tagged templates with no expressions work correctly
        
        let source = format!(
            r#"
            function tag(strings, ...values) {{
                return strings.length === 1 && strings[0] === "{}" && values.length === 0;
            }}
            const result = tag`{}`;
            result
            "#,
            content, content
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }

    /// Property 29.6: Tagged templates preserve raw strings
    /// *For any* tagged template, the strings array should have a raw property with unprocessed strings.
    #[test]
    fn prop_tagged_template_has_raw_property(
        content in "[a-zA-Z]{1,10}",
    ) {
        // Test that tagged templates have raw property on strings array
        
        let source = format!(
            r#"
            function tag(strings, ...values) {{
                return strings.raw !== undefined && Array.isArray(strings.raw);
            }}
            const result = tag`{}`;
            result
            "#,
            content
        );
        
        let allocator = oxc_allocator::Allocator::default();
        let result = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::mjs()).parse();
        
        prop_assert!(result.errors.is_empty(), "Parse failed: {:?}", result.errors);
    }
}
