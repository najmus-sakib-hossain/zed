//! Property tests for Module Resolution
//!
//! These tests verify universal correctness properties for the module system:
//! - Module resolution determinism
//! - Package.json exports resolution
//! - Module graph construction
//! - Topological ordering
//!
//! **Feature: ES Module Loading, Property 4: Module Resolution Determinism**
//! **Validates: Requirements 1.7**

use dx_js_runtime::compiler::modules::{
    CommonJSParser, ESModuleParser, ModuleGraph, ModuleResolver, ModuleType, PackageJson,
};
use proptest::prelude::*;
use std::path::PathBuf;

// ============================================================================
// Property 1: Module Resolution Determinism
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_relative_resolution_deterministic(
        filename in "[a-z]{1,10}",
        ext in prop::sample::select(vec!["js", "ts", "mjs", "cjs"]),
        depth in 0usize..5usize
    ) {
        let prefix = "../".repeat(depth);
        let specifier = format!("{}{}.{}", prefix, filename, ext);

        let mut resolver1 = ModuleResolver::new();
        let mut resolver2 = ModuleResolver::new();

        let from = PathBuf::from("/project/src/deep/nested/file.js");

        let result1 = resolver1.resolve(&specifier, &from);
        let result2 = resolver2.resolve(&specifier, &from);

        match (&result1, &result2) {
            (Ok(path1), Ok(path2)) => {
                prop_assert_eq!(path1, path2, "Resolution should be deterministic");
            }
            (Err(_), Err(_)) => {}
            _ => {
                prop_assert!(false, "Resolution results should be consistent");
            }
        }
    }
}

// ============================================================================
// Property 2: Package.json Exports Resolution
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_exports_string_resolution(
        filename in "[a-z]{1,10}",
        ext in prop::sample::select(vec!["js", "mjs", "cjs"])
    ) {
        let entry_path = format!("./{}.{}", filename, ext);
        let json = format!(r#"{{"name": "test", "exports": "{}"}}"#, entry_path);
        let pkg = PackageJson::parse(&json).unwrap();
        let resolved = pkg.resolve_entry(".", true);
        prop_assert_eq!(resolved, Some(entry_path));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_exports_conditional_esm_vs_cjs(
        esm_name in "[a-z]{1,10}",
        cjs_name in "[a-z]{1,10}"
    ) {
        let esm_path = format!("./{}.mjs", esm_name);
        let cjs_path = format!("./{}.cjs", cjs_name);
        let json = format!(
            r#"{{"name": "test", "exports": {{".": {{"import": "{}", "require": "{}"}}}}}}"#,
            esm_path, cjs_path
        );
        let pkg = PackageJson::parse(&json).unwrap();

        let esm_resolved = pkg.resolve_entry(".", true);
        prop_assert_eq!(esm_resolved, Some(esm_path));

        let cjs_resolved = pkg.resolve_entry(".", false);
        prop_assert_eq!(cjs_resolved, Some(cjs_path));
    }
}

// ============================================================================
// Property 3: Module Graph Construction
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_module_graph_preserves_all_modules(num_modules in 1usize..20usize) {
        let mut graph = ModuleGraph::new();

        for i in 0..num_modules {
            graph.add_module(PathBuf::from(format!("m{}.js", i)), ModuleType::ESModule);
        }

        prop_assert_eq!(graph.modules().len(), num_modules);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_module_graph_idempotent_add(filename in "[a-z]{1,10}") {
        let mut graph = ModuleGraph::new();
        let path = PathBuf::from(format!("{}.js", filename));

        let id1 = graph.add_module(path.clone(), ModuleType::ESModule);
        let id2 = graph.add_module(path.clone(), ModuleType::ESModule);
        let id3 = graph.add_module(path, ModuleType::ESModule);

        prop_assert_eq!(id1, id2);
        prop_assert_eq!(id2, id3);
        prop_assert_eq!(graph.modules().len(), 1);
    }
}

// ============================================================================
// Property 4: Topological Ordering
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_topological_order_dependencies_first(chain_length in 2usize..10usize) {
        let mut graph = ModuleGraph::new();

        for i in 0..chain_length {
            graph.add_module(PathBuf::from(format!("m{}.js", i)), ModuleType::ESModule);
        }

        for i in 0..chain_length - 1 {
            graph.add_dependency(i, i + 1);
        }

        graph.set_entry_point(0);
        let order = graph.topological_order().unwrap();

        for i in 0..chain_length - 1 {
            let dep_pos = order.iter().position(|&x| x == i + 1).unwrap();
            let dependent_pos = order.iter().position(|&x| x == i).unwrap();
            prop_assert!(dep_pos < dependent_pos);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_topological_order_connected_graph(num_modules in 2usize..15usize) {
        let mut graph = ModuleGraph::new();

        // Add modules
        for i in 0..num_modules {
            graph.add_module(PathBuf::from(format!("m{}.js", i)), ModuleType::ESModule);
        }

        // Create a connected graph: 0 depends on all others
        for i in 1..num_modules {
            graph.add_dependency(0, i);
        }

        graph.set_entry_point(0);
        let order = graph.topological_order().unwrap();

        // All modules should be in the order
        prop_assert_eq!(order.len(), num_modules);

        // Entry point (0) should be last since it depends on all others
        prop_assert_eq!(*order.last().unwrap(), 0);
    }
}

// ============================================================================
// Property 5: Import/Export Parsing
// **Validates: Requirements 1.7**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_import_parsing_extracts_specifier(
        module_name in "[a-z][a-z0-9]{0,15}",
        local_name in "[a-zA-Z_][a-zA-Z0-9_]{0,10}"
    ) {
        let source = format!("import {} from '{}';", local_name, module_name);
        let imports = ESModuleParser::extract_imports(&source);

        prop_assert_eq!(imports.len(), 1);
        prop_assert_eq!(&imports[0].specifier, &module_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_commonjs_require_parsing(module_name in "[a-z][a-z0-9]{0,15}") {
        let source = format!("const mod = require('{}');", module_name);
        let requires = CommonJSParser::extract_requires(&source);

        prop_assert_eq!(requires.len(), 1);
        prop_assert_eq!(&requires[0], &module_name);
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_package_json_esm_detection() {
    let json = r#"{"type": "module"}"#;
    let pkg = PackageJson::parse(json).unwrap();
    assert!(pkg.is_esm());
}

#[test]
fn test_package_json_cjs_detection() {
    let json = r#"{"type": "commonjs"}"#;
    let pkg = PackageJson::parse(json).unwrap();
    assert!(!pkg.is_esm());
}

#[test]
fn test_cycle_detection_simple() {
    let mut graph = ModuleGraph::new();

    let a = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
    let b = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);

    graph.add_dependency(a, b);
    graph.add_dependency(b, a);

    let cycles = graph.find_cycles();
    assert!(!cycles.is_empty());
}

#[test]
fn test_no_cycle_linear() {
    let mut graph = ModuleGraph::new();

    let a = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
    let b = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);
    let c = graph.add_module(PathBuf::from("c.js"), ModuleType::ESModule);

    graph.add_dependency(a, b);
    graph.add_dependency(b, c);

    // Linear chain has no cycles
    let order = graph.topological_order().unwrap();
    assert_eq!(order.len(), 3);
}
