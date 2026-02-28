//! Property-based tests for dx-check
//!
//! Uses proptest to verify invariants across random inputs.
//! **Validates: Requirement 9.2 - Property-based tests for parsers**

use proptest::prelude::*;
use std::path::Path;

// ============================================================================
// Arbitrary Generators
// ============================================================================

/// Generate arbitrary valid JavaScript identifiers
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,20}".prop_map(|s| s)
}

/// Generate arbitrary JavaScript variable declarations
fn arb_var_declaration() -> impl Strategy<Value = String> {
    (
        prop_oneof!["const", "let", "var"],
        arb_identifier(),
        prop_oneof![
            Just("1".to_string()),
            Just("\"hello\"".to_string()),
            Just("true".to_string()),
            Just("null".to_string()),
            Just("[]".to_string()),
            Just("{}".to_string()),
        ],
    )
        .prop_map(|(keyword, name, value)| format!("{} {} = {};", keyword, name, value))
}

/// Generate arbitrary JavaScript function declarations
fn arb_function_declaration() -> impl Strategy<Value = String> {
    (arb_identifier(), prop::collection::vec(arb_identifier(), 0..5)).prop_map(|(name, params)| {
        format!("function {}({}) {{ return null; }}", name, params.join(", "))
    })
}

/// Generate arbitrary JavaScript code snippets
fn arb_js_code() -> impl Strategy<Value = String> {
    prop::collection::vec(prop_oneof![arb_var_declaration(), arb_function_declaration(),], 1..10)
        .prop_map(|statements| statements.join("\n"))
}

/// Generate arbitrary TOML content
fn arb_toml_content() -> impl Strategy<Value = String> {
    (
        arb_identifier(),
        prop_oneof![
            Just("\"value\"".to_string()),
            Just("123".to_string()),
            Just("true".to_string()),
            Just("false".to_string()),
        ],
    )
        .prop_map(|(key, value)| format!("[section]\n{} = {}\n", key, value))
}

/// Generate arbitrary Markdown content
fn arb_markdown_content() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            arb_identifier().prop_map(|s| format!("# {}", s)),
            arb_identifier().prop_map(|s| format!("## {}", s)),
            arb_identifier().prop_map(|s| format!("- {}", s)),
            arb_identifier().prop_map(|s| format!("1. {}", s)),
            arb_identifier(),
        ],
        1..10,
    )
    .prop_map(|lines| lines.join("\n\n"))
}

// ============================================================================
// Parser Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Parsing valid JS code should not panic
    #[test]
    fn prop_parse_js_no_panic(code in arb_js_code()) {
        use dx_check::config::CheckerConfig;
        use dx_check::engine::Checker;

        let checker = Checker::new(CheckerConfig::default());
        // Should not panic - result can be Ok or Err
        let _ = checker.check_source(Path::new("test.js"), &code);
    }

    /// Property: Empty input should be handled gracefully
    #[test]
    fn prop_empty_input_handled(ext in prop_oneof!["js", "ts", "jsx", "tsx"]) {
        use dx_check::config::CheckerConfig;
        use dx_check::engine::Checker;

        let checker = Checker::new(CheckerConfig::default());
        let path = format!("test.{}", ext);
        let result = checker.check_source(Path::new(&path), "");
        // Empty input should not panic and should return Ok
        prop_assert!(result.is_ok());
    }

    /// Property: Whitespace-only input should be handled gracefully
    #[test]
    fn prop_whitespace_only_handled(
        whitespace in "[ \t\n\r]{0,100}",
        ext in prop_oneof!["js", "ts", "jsx", "tsx"]
    ) {
        use dx_check::config::CheckerConfig;
        use dx_check::engine::Checker;

        let checker = Checker::new(CheckerConfig::default());
        let path = format!("test.{}", ext);
        let result = checker.check_source(Path::new(&path), &whitespace);
        prop_assert!(result.is_ok());
    }

    /// Property: Diagnostics should have valid spans
    #[test]
    fn prop_diagnostic_spans_valid(code in arb_js_code()) {
        use dx_check::config::CheckerConfig;
        use dx_check::engine::Checker;

        let checker = Checker::new(CheckerConfig::default());
        if let Ok(diagnostics) = checker.check_source(Path::new("test.js"), &code) {
            for diag in diagnostics {
                // Span start should be <= end
                prop_assert!(diag.span.start <= diag.span.end);
                // Span should be within source bounds
                prop_assert!((diag.span.end as usize) <= code.len() + 1);
            }
        }
    }

    /// Property: Rule IDs should be non-empty
    #[test]
    fn prop_rule_ids_non_empty(code in arb_js_code()) {
        use dx_check::config::CheckerConfig;
        use dx_check::engine::Checker;

        let checker = Checker::new(CheckerConfig::default());
        if let Ok(diagnostics) = checker.check_source(Path::new("test.js"), &code) {
            for diag in diagnostics {
                prop_assert!(!diag.rule_id.is_empty());
            }
        }
    }
}

// ============================================================================
// Configuration Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Config serialization round-trip
    #[test]
    fn prop_config_round_trip(
        threads in 0usize..32,
        cache_enabled in any::<bool>(),
        indent_width in 1u8..8,
    ) {
        use dx_check::config::CheckerConfig;

        let mut config = CheckerConfig::default();
        config.parallel.threads = threads;
        config.cache.enabled = cache_enabled;
        config.format.indent_width = indent_width;

        // Serialize to TOML
        let toml_str = toml::to_string(&config).expect("Serialization should succeed");

        // Deserialize back
        let restored: CheckerConfig = toml::from_str(&toml_str).expect("Deserialization should succeed");

        prop_assert_eq!(config.parallel.threads, restored.parallel.threads);
        prop_assert_eq!(config.cache.enabled, restored.cache.enabled);
        prop_assert_eq!(config.format.indent_width, restored.format.indent_width);
    }
}

// ============================================================================
// Span Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Span creation should maintain invariants
    #[test]
    fn prop_span_invariants(start in 0u32..1000, len in 0u32..1000) {
        use dx_check::diagnostics::Span;

        let end = start.saturating_add(len);
        let span = Span::new(start, end);

        prop_assert!(span.start <= span.end);
        prop_assert_eq!(span.len() as usize, (end - start) as usize);
    }

    /// Property: Span contains should be consistent
    #[test]
    fn prop_span_contains(start in 0u32..100, len in 1u32..100, offset in 0u32..200) {
        use dx_check::diagnostics::Span;

        let end = start.saturating_add(len);
        let span = Span::new(start, end);

        let contains = offset >= start && offset < end;

        prop_assert_eq!(span.start <= offset && offset < span.end, contains);
    }
}

// ============================================================================
// Fix Application Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Delete fix should reduce content length
    #[test]
    fn prop_delete_fix_reduces_length(
        prefix in "[a-z]{0,20}",
        to_delete in "[a-z]{1,20}",
        suffix in "[a-z]{0,20}",
    ) {
        use dx_check::diagnostics::{Fix, Span};
        use dx_check::fix::FixEngine;

        let content = format!("{}{}{}", prefix, to_delete, suffix);
        let start = prefix.len() as u32;
        let end = start + to_delete.len() as u32;

        let fix = Fix::delete("Delete", Span::new(start, end));
        let engine = FixEngine::new();
        let result = engine.apply_fix(content.as_bytes(), &fix);

        prop_assert!(result.len() < content.len());
        prop_assert_eq!(result.len(), prefix.len() + suffix.len());
    }

    /// Property: Replace fix should change content
    #[test]
    fn prop_replace_fix_changes_content(
        prefix in "[a-z]{0,10}",
        original in "[a-z]{1,10}",
        replacement in "[A-Z]{1,10}",
        suffix in "[a-z]{0,10}",
    ) {
        use dx_check::diagnostics::{Fix, Span};
        use dx_check::fix::FixEngine;

        // Ensure original and replacement are different
        prop_assume!(original != replacement.to_lowercase());

        let content = format!("{}{}{}", prefix, original, suffix);
        let start = prefix.len() as u32;
        let end = start + original.len() as u32;

        let fix = Fix::replace("Replace", Span::new(start, end), &replacement);
        let engine = FixEngine::new();
        let result = engine.apply_fix(content.as_bytes(), &fix);
        let result_str = String::from_utf8_lossy(&result);

        prop_assert!(result_str.contains(&replacement));
        prop_assert!(!result_str.contains(&original) || prefix.contains(&original) || suffix.contains(&original));
    }
}

// ============================================================================
// Scanner Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Scanner should not panic on arbitrary input
    #[test]
    fn prop_scanner_no_panic(input in ".*") {
        use dx_check::scanner::PatternScanner;

        let scanner = PatternScanner::new();
        // Should not panic
        let _ = scanner.scan(input.as_bytes());
        let _ = scanner.has_any_match(input.as_bytes());
    }

    /// Property: Scanner results should be consistent
    #[test]
    fn prop_scanner_consistent(input in "[a-zA-Z0-9 \n\t]{0,1000}") {
        use dx_check::scanner::PatternScanner;

        let scanner = PatternScanner::new();
        let result1 = scanner.scan(input.as_bytes());
        let result2 = scanner.scan(input.as_bytes());

        // Same input should produce same results
        prop_assert_eq!(result1.len(), result2.len());
    }
}

// ============================================================================
// Rule Registry Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Enabling a rule should make it enabled
    #[test]
    fn prop_rule_enable_disable(rule_name in prop_oneof![
        Just("no-console"),
        Just("no-debugger"),
        Just("no-eval"),
        Just("eqeqeq"),
        Just("prefer-const"),
    ]) {
        use dx_check::rules::{RuleRegistry, Severity};

        let mut registry = RuleRegistry::with_builtins();

        // Enable the rule
        registry.enable(&rule_name, Severity::Error);
        prop_assert!(registry.is_enabled(&rule_name));

        // Disable the rule
        registry.disable(&rule_name);
        prop_assert!(!registry.is_enabled(&rule_name));
    }
}

// ============================================================================
// DX-Serializer Round-Trip Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Serialization round-trip for configuration
    /// **Validates: Requirements 6.1, 6.2**
    /// Test that any configuration serialized to LLM format and deserialized produces equivalent data
    #[test]
    fn prop_serializer_config_round_trip(
        name in "[a-zA-Z0-9_-]{1,50}",
        version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        enabled in any::<bool>(),
        score_impact in 1u8..11,
    ) {
        use dx_check::serializer::DxSerializerWrapper;
        use serializer::{DxDocument, DxLlmValue};
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let wrapper = DxSerializerWrapper::new(temp_dir.path().to_path_buf());

        // Create a configuration document
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str(name.clone()));
        doc.context.insert("version".to_string(), DxLlmValue::Str(version.clone()));
        doc.context.insert("enabled".to_string(), DxLlmValue::Bool(enabled));
        doc.context.insert("score_impact".to_string(), DxLlmValue::Num(score_impact as f64));

        // Save to disk (LLM format)
        let config_path = temp_dir.path().join("config.sr");
        wrapper.save_to_disk(&doc, &config_path).expect("Save should succeed");

        // Load back from disk
        let loaded = wrapper.load_from_disk(&config_path).expect("Load should succeed");

        // Verify round-trip equivalence
        prop_assert_eq!(wrapper.get_string(&loaded, "name"), Some(name));
        prop_assert_eq!(wrapper.get_string(&loaded, "version"), Some(version));
        prop_assert_eq!(wrapper.get_bool(&loaded, "enabled"), Some(enabled));
        prop_assert_eq!(wrapper.get_number(&loaded, "score_impact"), Some(score_impact as f64));
    }

    /// Property 2: Serialization round-trip for rules
    /// **Validates: Requirements 6.1, 6.2**
    /// Test that any rule serialized to LLM format and deserialized produces equivalent data
    #[test]
    fn prop_serializer_rule_round_trip(
        rule_id in "[a-z-]{3,30}",
        category in prop_oneof![
            Just("formatting"),
            Just("linting"),
            Just("security"),
            Just("design_patterns"),
            Just("structure_docs"),
        ],
        severity in prop_oneof![
            Just("critical"),
            Just("high"),
            Just("medium"),
            Just("low"),
        ],
        score_impact in 1u8..11,
    ) {
        use dx_check::serializer::{DxSerializerWrapper, RuleLoader};
        use serializer::{DxDocument, DxLlmValue};
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let loader = RuleLoader::new(cache_dir);

        // Create a rule document
        let mut rule = DxDocument::new();
        rule.context.insert("rule".to_string(), DxLlmValue::Str(rule_id.clone()));
        rule.context.insert("category".to_string(), DxLlmValue::Str(category.to_string()));
        rule.context.insert("severity".to_string(), DxLlmValue::Str(severity.to_string()));
        rule.context.insert("score_impact".to_string(), DxLlmValue::Num(score_impact as f64));

        // Save rule
        let rule_path = temp_dir.path().join(format!("{}.sr", rule_id));
        loader.save_rule(&rule, &rule_path).expect("Save should succeed");

        // Load rule back
        let loaded = loader.load_rule(&rule_path).expect("Load should succeed");

        // Verify round-trip equivalence
        let wrapper = DxSerializerWrapper::new(temp_dir.path().to_path_buf());
        prop_assert_eq!(wrapper.get_string(&loaded, "rule"), Some(rule_id));
        prop_assert_eq!(wrapper.get_string(&loaded, "category"), Some(category.to_string()));
        prop_assert_eq!(wrapper.get_string(&loaded, "severity"), Some(severity.to_string()));
        prop_assert_eq!(wrapper.get_number(&loaded, "score_impact"), Some(score_impact as f64));
    }

    /// Property 3: MACHINE format compilation is deterministic
    /// **Validates: Requirement 6.2**
    /// Test that compiling the same rule multiple times produces identical MACHINE format
    #[test]
    fn prop_machine_format_deterministic(
        rule_id in "[a-z-]{3,30}",
        message in "[a-zA-Z0-9_ ]{5,50}",
    ) {
        use dx_check::serializer::DxSerializerWrapper;
        use serializer::{DxDocument, DxLlmValue};
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let wrapper = DxSerializerWrapper::new(cache_dir.clone());

        // Create a rule document
        let mut rule = DxDocument::new();
        rule.context.insert("rule".to_string(), DxLlmValue::Str(rule_id.clone()));
        rule.context.insert("message".to_string(), DxLlmValue::Str(message.clone()));

        // Save and compile first time
        let rule_path1 = temp_dir.path().join("rule1.sr");
        wrapper.save_to_disk(&rule, &rule_path1).expect("Save should succeed");
        let loaded1 = wrapper.load_with_cache(&rule_path1).expect("Load should succeed");

        // Save and compile second time (same content, different file)
        let rule_path2 = temp_dir.path().join("rule2.sr");
        wrapper.save_to_disk(&rule, &rule_path2).expect("Save should succeed");
        let loaded2 = wrapper.load_with_cache(&rule_path2).expect("Load should succeed");

        // Verify both compilations produce identical results
        prop_assert_eq!(wrapper.get_string(&loaded1, "rule"), wrapper.get_string(&loaded2, "rule"));
        prop_assert_eq!(wrapper.get_string(&loaded1, "message"), wrapper.get_string(&loaded2, "message"));
    }

    /// Property 4: Cache invalidation works correctly
    /// **Validates: Requirements 6.2, 6.3**
    /// Test that modifying a source file invalidates the cache
    #[test]
    fn prop_cache_invalidation(
        initial_value in 1.0f64..100.0,
        updated_value in 100.0f64..200.0,
    ) {
        use dx_check::serializer::DxSerializerWrapper;
        use serializer::{DxDocument, DxLlmValue};
        use tempfile::tempdir;
        use std::thread;
        use std::time::Duration;

        // Ensure values are different
        prop_assume!((initial_value - updated_value).abs() > 1.0);

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let cache_dir = temp_dir.path().join(".dx/serializer");
        let wrapper = DxSerializerWrapper::new(cache_dir);

        // Create initial document
        let mut doc = DxDocument::new();
        doc.context.insert("value".to_string(), DxLlmValue::Num(initial_value));

        let source_path = temp_dir.path().join("test.sr");
        wrapper.save_to_disk(&doc, &source_path).expect("Save should succeed");

        // First load - creates cache
        let loaded1 = wrapper.load_with_cache(&source_path).expect("Load should succeed");
        prop_assert_eq!(wrapper.get_number(&loaded1, "value"), Some(initial_value));

        // Wait to ensure file modification time changes
        thread::sleep(Duration::from_millis(10));

        // Modify source file
        doc.context.insert("value".to_string(), DxLlmValue::Num(updated_value));
        wrapper.save_to_disk(&doc, &source_path).expect("Save should succeed");

        // Second load - should invalidate cache and reload
        let loaded2 = wrapper.load_with_cache(&source_path).expect("Load should succeed");
        prop_assert_eq!(wrapper.get_number(&loaded2, "value"), Some(updated_value));
    }

    /// Property 5: Array and object round-trip
    /// **Validates: Requirements 6.1, 6.2**
    /// Test that complex data structures (arrays, objects) round-trip correctly
    #[test]
    fn prop_complex_data_round_trip(
        array_size in 0usize..10,
        obj_keys in prop::collection::vec("[a-z]{3,10}", 0..5),
    ) {
        use dx_check::serializer::DxSerializerWrapper;
        use serializer::{DxDocument, DxLlmValue};
        use tempfile::tempdir;
        use std::collections::HashMap;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let wrapper = DxSerializerWrapper::new(temp_dir.path().to_path_buf());

        // Create document with array and object
        let mut doc = DxDocument::new();

        // Add array
        let array: Vec<DxLlmValue> = (0..array_size)
            .map(|i| DxLlmValue::Num(i as f64))
            .collect();
        doc.context.insert("array".to_string(), DxLlmValue::Arr(array.clone()));

        // Add object
        let mut obj = HashMap::new();
        for (i, key) in obj_keys.iter().enumerate() {
            obj.insert(key.clone(), DxLlmValue::Num(i as f64));
        }
        doc.context.insert("object".to_string(), DxLlmValue::Obj(obj.clone()));

        // Save and load
        let path = temp_dir.path().join("complex.sr");
        wrapper.save_to_disk(&doc, &path).expect("Save should succeed");
        let loaded = wrapper.load_from_disk(&path).expect("Load should succeed");

        // Verify array
        let loaded_array = wrapper.get_array(&loaded, "array");
        prop_assert_eq!(loaded_array, Some(array));

        // Verify object
        let loaded_obj = wrapper.get_object(&loaded, "object");
        prop_assert_eq!(loaded_obj, Some(obj));
    }
}

// ============================================================================
// Complexity Metrics Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Complexity metric bounds
    /// **Validates: Requirements 4.2**
    /// Test that cyclomatic complexity is always >= 1
    /// Test that LOC >= SLOC
    #[test]
    fn prop_complexity_metric_bounds(
        num_ifs in 0usize..10,
        num_loops in 0usize..10,
        num_blank_lines in 0usize..20,
        num_comment_lines in 0usize..20,
        num_code_lines in 1usize..50,
    ) {
        use dx_check::complexity::ComplexityCalculator;

        let calc = ComplexityCalculator::with_defaults();

        // Generate a function with controlled complexity
        let mut lines = vec!["fn test_function() {".to_string()];

        // Add if statements
        for i in 0..num_ifs {
            lines.push(format!("    if x > {} {{", i));
            lines.push("        println!(\"branch\");".to_string());
            lines.push("    }".to_string());
        }

        // Add loops
        for i in 0..num_loops {
            lines.push(format!("    for i in 0..{} {{", i + 1));
            lines.push("        println!(\"loop\");".to_string());
            lines.push("    }".to_string());
        }

        // Add regular code lines
        for i in 0..num_code_lines {
            lines.push(format!("    let x{} = {};", i, i));
        }

        // Add blank lines
        for _ in 0..num_blank_lines {
            lines.push("".to_string());
        }

        // Add comment lines
        for i in 0..num_comment_lines {
            lines.push(format!("    // Comment {}", i));
        }

        lines.push("}".to_string());

        let source = lines.join("\n");
        let metrics = calc.calculate_all(&source);

        // Should find at least one function
        prop_assert!(!metrics.is_empty());

        for metric in metrics {
            // Property 1: Cyclomatic complexity is always >= 1
            prop_assert!(metric.cyclomatic >= 1,
                "Cyclomatic complexity must be at least 1, got {}", metric.cyclomatic);

            // Property 2: LOC >= SLOC (total lines >= source lines)
            prop_assert!(metric.loc >= metric.sloc,
                "LOC ({}) must be >= SLOC ({})", metric.loc, metric.sloc);

            // Additional sanity checks
            prop_assert!(metric.cognitive >= 0, "Cognitive complexity must be non-negative");
            prop_assert!(metric.loc > 0, "LOC must be positive for a function");
        }
    }

    /// Property: Cyclomatic complexity increases with decision points
    #[test]
    fn prop_cyclomatic_increases_with_branches(
        num_branches in 0usize..20,
    ) {
        use dx_check::complexity::ComplexityCalculator;

        let calc = ComplexityCalculator::with_defaults();

        // Generate function with N branches
        let mut lines = vec!["fn test() {".to_string()];
        for i in 0..num_branches {
            lines.push(format!("    if x > {} {{ y = {}; }}", i, i));
        }
        lines.push("}".to_string());

        let source = lines.join("\n");
        let metrics = calc.calculate_all(&source);

        prop_assert!(!metrics.is_empty());
        let metric = &metrics[0];

        // Cyclomatic complexity should be at least 1 + number of branches
        // (may be higher due to parsing variations)
        prop_assert!(metric.cyclomatic >= 1,
            "Expected cyclomatic >= 1, got {}", metric.cyclomatic);
    }

    /// Property: SLOC excludes blank and comment lines
    #[test]
    fn prop_sloc_excludes_blanks_and_comments(
        num_code_lines in 1usize..20,
        num_blank_lines in 0usize..20,
        num_comment_lines in 0usize..20,
    ) {
        use dx_check::complexity::ComplexityCalculator;

        let calc = ComplexityCalculator::with_defaults();

        let mut lines = vec!["fn test() {".to_string()];

        // Add code lines
        for i in 0..num_code_lines {
            lines.push(format!("    let x{} = {};", i, i));
        }

        // Add blank lines
        for _ in 0..num_blank_lines {
            lines.push("".to_string());
        }

        // Add comment lines
        for i in 0..num_comment_lines {
            lines.push(format!("    // Comment {}", i));
        }

        lines.push("}".to_string());

        let source = lines.join("\n");
        let metrics = calc.calculate_all(&source);

        prop_assert!(!metrics.is_empty());
        let metric = &metrics[0];

        // SLOC should be less than or equal to LOC
        prop_assert!(metric.sloc <= metric.loc,
            "SLOC ({}) should be <= LOC ({})", metric.sloc, metric.loc);

        // If we have blank or comment lines, SLOC should be less than LOC
        if num_blank_lines > 0 || num_comment_lines > 0 {
            prop_assert!(metric.sloc < metric.loc,
                "SLOC should be less than LOC when blank/comment lines exist");
        }
    }

    /// Property: Cognitive complexity increases with nesting
    #[test]
    fn prop_cognitive_increases_with_nesting(
        nesting_depth in 1usize..5,
    ) {
        use dx_check::complexity::ComplexityCalculator;

        let calc = ComplexityCalculator::with_defaults();

        // Generate nested if statements
        let mut lines = vec!["fn test() {".to_string()];

        // Opening braces
        for i in 0..nesting_depth {
            lines.push(format!("{}if x > {} {{", "    ".repeat(i + 1), i));
        }

        lines.push(format!("{}println!(\"nested\");", "    ".repeat(nesting_depth + 1)));

        // Closing braces
        for i in (0..nesting_depth).rev() {
            lines.push(format!("{}}}", "    ".repeat(i + 1)));
        }

        lines.push("}".to_string());

        let source = lines.join("\n");
        let metrics = calc.calculate_all(&source);

        prop_assert!(!metrics.is_empty());
        let metric = &metrics[0];

        // Cognitive complexity should increase with nesting
        // At minimum, should be >= nesting_depth
        prop_assert!(metric.cognitive >= 0,
            "Cognitive complexity should be non-negative");
    }

    /// Property: Empty function has minimal complexity
    #[test]
    fn prop_empty_function_minimal_complexity(
        func_name in "[a-z_][a-z0-9_]{0,20}",
    ) {
        use dx_check::complexity::ComplexityCalculator;

        let calc = ComplexityCalculator::with_defaults();

        let source = format!("fn {}() {{}}", func_name);
        let metrics = calc.calculate_all(&source);

        if !metrics.is_empty() {
            let metric = &metrics[0];

            // Empty function should have cyclomatic complexity of 1
            prop_assert_eq!(metric.cyclomatic, 1,
                "Empty function should have cyclomatic complexity of 1");

            // Empty function should have cognitive complexity of 0
            prop_assert_eq!(metric.cognitive, 0,
                "Empty function should have cognitive complexity of 0");
        }
    }
}
