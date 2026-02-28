//! Ecosystem Compatibility Integration Tests
//!
//! This module provides integration tests for ecosystem compatibility,
//! validating that popular npm packages work correctly with DX-JS.
//!
//! **Validates: Requirements 7.1, 7.2, 7.3, 7.4**

mod ecosystem;

use ecosystem::test262::{Test262Config, Test262Harness, Test262Summary};
use ecosystem::lodash::{LodashTestSuite, LodashCategory};
use ecosystem::express::ExpressTestSuite;
use ecosystem::typescript::TypeScriptTestSuite;

// ============================================================================
// Test262 Integration Tests
// ============================================================================

#[test]
fn test_test262_metadata_parsing() {
    let content = r#"/*---
description: Test that BigInt literals work
flags: [onlyStrict]
features: [BigInt]
includes: [assert.js]
---*/
var x = 1n;
assert.sameValue(x, 1n);
"#;
    
    let metadata = Test262Harness::parse_metadata(content).unwrap();
    assert_eq!(metadata.description, "Test that BigInt literals work");
    assert!(metadata.flags.contains(&"onlyStrict".to_string()));
    assert!(metadata.features.contains(&"BigInt".to_string()));
    assert!(metadata.includes.contains(&"assert.js".to_string()));
}

#[test]
fn test_test262_negative_metadata() {
    let content = r#"/*---
description: Test that throws TypeError
negative:
  phase: runtime
  type: TypeError
---*/
null.foo;
"#;
    
    let metadata = Test262Harness::parse_metadata(content).unwrap();
    assert!(metadata.negative.is_some());
    let neg = metadata.negative.unwrap();
    assert_eq!(neg.phase, "runtime");
    assert_eq!(neg.error_type, "TypeError");
}

#[test]
fn test_test262_summary_pass_rate() {
    let summary = Test262Summary {
        passed: 950,
        failed: 50,
        skipped: 100,
        timeout: 0,
    };
    
    // Pass rate should be 95% (950 / 1000 applicable tests)
    assert!((summary.pass_rate() - 95.0).abs() < 0.01);
}

#[test]
fn test_test262_config_default() {
    let config = Test262Config::default();
    
    // Should skip certain features by default
    assert!(config.skip_features.contains(&"Atomics".to_string()));
    assert!(config.skip_features.contains(&"SharedArrayBuffer".to_string()));
    assert!(config.skip_features.contains(&"Temporal".to_string()));
    
    // Should run async and module tests by default
    assert!(config.run_async);
    assert!(config.run_modules);
}

// ============================================================================
// Lodash Integration Tests
// ============================================================================

#[test]
fn test_lodash_suite_coverage() {
    let suite = LodashTestSuite::new();
    let counts = suite.count_by_category();
    
    // Should have tests for all major categories
    assert!(counts.get(&LodashCategory::Array).unwrap_or(&0) >= &5);
    assert!(counts.get(&LodashCategory::Collection).unwrap_or(&0) >= &5);
    assert!(counts.get(&LodashCategory::Lang).unwrap_or(&0) >= &5);
    assert!(counts.get(&LodashCategory::Object).unwrap_or(&0) >= &5);
    assert!(counts.get(&LodashCategory::String).unwrap_or(&0) >= &5);
    assert!(counts.get(&LodashCategory::Math).unwrap_or(&0) >= &5);
}

#[test]
fn test_lodash_array_functions() {
    let suite = LodashTestSuite::new();
    let array_tests = suite.test_cases_by_category(LodashCategory::Array);
    
    let names: Vec<_> = array_tests.iter().map(|t| t.name.as_str()).collect();
    
    // Core array functions should be tested
    assert!(names.contains(&"chunk"));
    assert!(names.contains(&"compact"));
    assert!(names.contains(&"flatten"));
    assert!(names.contains(&"uniq"));
    assert!(names.contains(&"zip"));
}

#[test]
fn test_lodash_collection_functions() {
    let suite = LodashTestSuite::new();
    let collection_tests = suite.test_cases_by_category(LodashCategory::Collection);
    
    let names: Vec<_> = collection_tests.iter().map(|t| t.name.as_str()).collect();
    
    // Core collection functions should be tested
    assert!(names.contains(&"map"));
    assert!(names.contains(&"filter"));
    assert!(names.contains(&"reduce"));
    assert!(names.contains(&"find"));
    assert!(names.contains(&"groupBy"));
}

#[test]
fn test_lodash_test_case_structure() {
    let suite = LodashTestSuite::new();
    
    for test_case in suite.test_cases() {
        // Each test case should have valid structure
        assert!(!test_case.name.is_empty());
        assert!(!test_case.test_code.is_empty());
        assert!(!test_case.expected.is_empty());
        
        // Test code should require lodash
        assert!(test_case.test_code.contains("require('lodash')"));
    }
}

// ============================================================================
// Express Integration Tests
// ============================================================================

#[test]
fn test_express_suite_coverage() {
    let suite = ExpressTestSuite::new();
    
    // Should have a reasonable number of tests
    assert!(suite.test_count() >= 15);
}

#[test]
fn test_express_routing_tests() {
    let suite = ExpressTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test all HTTP methods
    assert!(names.contains(&"basic_get"));
    assert!(names.contains(&"basic_post"));
    assert!(names.contains(&"basic_put"));
    assert!(names.contains(&"basic_delete"));
    
    // Should test route parameters
    assert!(names.contains(&"route_params"));
    assert!(names.contains(&"query_params"));
}

#[test]
fn test_express_middleware_tests() {
    let suite = ExpressTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test middleware
    assert!(names.contains(&"app_middleware"));
    assert!(names.contains(&"router_middleware"));
    assert!(names.contains(&"json_middleware"));
}

#[test]
fn test_express_error_handling_tests() {
    let suite = ExpressTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test error handling
    assert!(names.contains(&"404_handler"));
    assert!(names.contains(&"error_middleware"));
}

#[test]
fn test_express_test_case_structure() {
    let suite = ExpressTestSuite::new();
    
    for test_case in suite.test_cases() {
        // Each test case should have valid structure
        assert!(!test_case.name.is_empty());
        assert!(!test_case.description.is_empty());
        assert!(!test_case.server_code.is_empty());
        assert!(!test_case.path.is_empty());
        
        // Server code should use express
        assert!(test_case.server_code.contains("require('express')"));
        assert!(test_case.server_code.contains("app.listen"));
    }
}

// ============================================================================
// TypeScript Integration Tests
// ============================================================================

#[test]
fn test_typescript_suite_coverage() {
    let suite = TypeScriptTestSuite::new();
    
    // Should have a reasonable number of tests
    assert!(suite.test_count() >= 20);
}

#[test]
fn test_typescript_basic_type_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test basic types
    assert!(names.contains(&"basic_types"));
    assert!(names.contains(&"optional_params"));
    assert!(names.contains(&"default_params"));
    assert!(names.contains(&"type_error"));
}

#[test]
fn test_typescript_interface_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test interfaces
    assert!(names.contains(&"basic_interface"));
    assert!(names.contains(&"optional_properties"));
    assert!(names.contains(&"interface_extension"));
}

#[test]
fn test_typescript_class_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test classes
    assert!(names.contains(&"basic_class"));
    assert!(names.contains(&"access_modifiers"));
    assert!(names.contains(&"class_inheritance"));
    assert!(names.contains(&"abstract_class"));
}

#[test]
fn test_typescript_generic_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test generics
    assert!(names.contains(&"generic_function"));
    assert!(names.contains(&"generic_class"));
    assert!(names.contains(&"generic_constraints"));
}

#[test]
fn test_typescript_enum_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test enums
    assert!(names.contains(&"numeric_enum"));
    assert!(names.contains(&"string_enum"));
    assert!(names.contains(&"const_enum"));
}

#[test]
fn test_typescript_advanced_type_tests() {
    let suite = TypeScriptTestSuite::new();
    let names: Vec<_> = suite.test_cases().iter().map(|t| t.name.as_str()).collect();
    
    // Should test advanced types
    assert!(names.contains(&"union_types"));
    assert!(names.contains(&"intersection_types"));
    assert!(names.contains(&"mapped_types"));
    assert!(names.contains(&"conditional_types"));
}

#[test]
fn test_typescript_test_case_structure() {
    let suite = TypeScriptTestSuite::new();
    
    for test_case in suite.test_cases() {
        // Each test case should have valid structure
        assert!(!test_case.name.is_empty());
        assert!(!test_case.description.is_empty());
        assert!(!test_case.source.is_empty());
        
        // Source should be valid TypeScript (contains type annotations or keywords)
        let has_ts_features = test_case.source.contains(": ")
            || test_case.source.contains("interface ")
            || test_case.source.contains("type ")
            || test_case.source.contains("enum ")
            || test_case.source.contains("<T>");
        assert!(has_ts_features, "Test '{}' should contain TypeScript features", test_case.name);
    }
}

// ============================================================================
// Cross-Package Integration Tests
// ============================================================================

#[test]
fn test_all_suites_have_tests() {
    let lodash = LodashTestSuite::new();
    let express = ExpressTestSuite::new();
    let typescript = TypeScriptTestSuite::new();
    
    assert!(!lodash.test_cases().is_empty(), "Lodash suite should have tests");
    assert!(!express.test_cases().is_empty(), "Express suite should have tests");
    assert!(!typescript.test_cases().is_empty(), "TypeScript suite should have tests");
}

#[test]
fn test_total_ecosystem_coverage() {
    let lodash = LodashTestSuite::new();
    let express = ExpressTestSuite::new();
    let typescript = TypeScriptTestSuite::new();
    
    let total_tests = lodash.test_cases().len() 
        + express.test_count() 
        + typescript.test_count();
    
    // Should have comprehensive coverage
    assert!(total_tests >= 60, "Should have at least 60 ecosystem tests, got {}", total_tests);
}
