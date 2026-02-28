//! Integration test for fixture injection
//!
//! This test demonstrates the complete fixture injection flow:
//! 1. Discover test functions with parameters
//! 2. Match parameters to fixture names
//! 3. Resolve fixture dependency chains
//! 4. Inject values as test arguments

use dx_py_core::TestCase;
use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_complete_fixture_injection_flow() {
    // Setup: Create a fixture manager and register fixtures
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with dependencies
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10);
    let db = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_dependencies(vec!["config".to_string()]);
    let api = FixtureDefinition::new("api", "tests/conftest.py", 30)
        .with_dependencies(vec!["db".to_string()]);

    manager.register(config);
    manager.register(db);
    manager.register(api);

    // Simulate a test function with parameters
    let test = TestCase::new("test_api_endpoint", PathBuf::from("tests/test_api.py"), 50)
        .with_parameters(vec!["api".to_string(), "config".to_string()]);

    // Resolve fixtures for the test
    let resolved = manager.resolve_fixtures_for_test(&test.parameters).unwrap();

    // Verify: All fixtures in dependency order
    assert_eq!(resolved.len(), 3);

    // Check that fixtures are in correct dependency order
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let db_idx = resolved.iter().position(|f| f.definition.name == "db").unwrap();
    let api_idx = resolved.iter().position(|f| f.definition.name == "api").unwrap();

    assert!(config_idx < db_idx, "config should come before db");
    assert!(db_idx < api_idx, "db should come before api");

    // Verify that all fixtures need setup (function scope)
    for fixture in &resolved {
        assert!(fixture.needs_setup, "Function-scoped fixtures should need setup");
    }
}

#[test]
fn test_fixture_injection_with_scopes() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with different scopes
    let session_fixture = FixtureDefinition::new("session_data", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Session);
    let module_fixture = FixtureDefinition::new("module_data", "tests/conftest.py", 20)
        .with_scope(FixtureScope::Module);
    let function_fixture = FixtureDefinition::new("function_data", "tests/conftest.py", 30)
        .with_scope(FixtureScope::Function);

    manager.register(session_fixture);
    manager.register(module_fixture);
    manager.register(function_fixture);

    // Test requests all three fixtures
    let test = TestCase::new("test_with_scopes", PathBuf::from("tests/test_scopes.py"), 40)
        .with_parameters(vec![
            "session_data".to_string(),
            "module_data".to_string(),
            "function_data".to_string(),
        ]);

    let resolved = manager.resolve_fixtures_for_test(&test.parameters).unwrap();

    assert_eq!(resolved.len(), 3);

    // Verify scopes are preserved
    let session = resolved.iter().find(|f| f.definition.name == "session_data").unwrap();
    let module = resolved.iter().find(|f| f.definition.name == "module_data").unwrap();
    let function = resolved.iter().find(|f| f.definition.name == "function_data").unwrap();

    assert_eq!(session.definition.scope, FixtureScope::Session);
    assert_eq!(module.definition.scope, FixtureScope::Module);
    assert_eq!(function.definition.scope, FixtureScope::Function);
}

#[test]
fn test_fixture_injection_with_autouse() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register an autouse fixture
    let autouse = FixtureDefinition::new("auto_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    // Register a normal fixture
    let normal = FixtureDefinition::new("data", "tests/conftest.py", 20);

    manager.register(autouse);
    manager.register(normal);

    // Test only explicitly requests 'data'
    let test = TestCase::new("test_with_autouse", PathBuf::from("tests/test_auto.py"), 30)
        .with_parameters(vec!["data".to_string()]);

    let resolved = manager.resolve_fixtures_for_test(&test.parameters).unwrap();

    // Should get both fixtures
    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.definition.name == "auto_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "data"));
}

#[test]
fn test_fixture_injection_complex_dependency_graph() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Create a complex dependency graph:
    //     test
    //    /    \
    //   a      b
    //   |    / | \
    //   c   d  e  f
    //    \ /
    //     g

    let g = FixtureDefinition::new("g", "tests/conftest.py", 10);
    let c = FixtureDefinition::new("c", "tests/conftest.py", 20)
        .with_dependencies(vec!["g".to_string()]);
    let d = FixtureDefinition::new("d", "tests/conftest.py", 30)
        .with_dependencies(vec!["g".to_string()]);
    let e = FixtureDefinition::new("e", "tests/conftest.py", 40);
    let f = FixtureDefinition::new("f", "tests/conftest.py", 50);
    let a = FixtureDefinition::new("a", "tests/conftest.py", 60)
        .with_dependencies(vec!["c".to_string()]);
    let b = FixtureDefinition::new("b", "tests/conftest.py", 70)
        .with_dependencies(vec!["d".to_string(), "e".to_string(), "f".to_string()]);

    manager.register(g);
    manager.register(c);
    manager.register(d);
    manager.register(e);
    manager.register(f);
    manager.register(a);
    manager.register(b);

    // Test requests a and b
    let test = TestCase::new("test_complex", PathBuf::from("tests/test_complex.py"), 80)
        .with_parameters(vec!["a".to_string(), "b".to_string()]);

    let resolved = manager.resolve_fixtures_for_test(&test.parameters).unwrap();

    // Should resolve all fixtures
    assert_eq!(resolved.len(), 7);

    // Verify dependency order constraints
    let get_idx = |name: &str| resolved.iter().position(|f| f.definition.name == name).unwrap();

    let g_idx = get_idx("g");
    let c_idx = get_idx("c");
    let d_idx = get_idx("d");
    let e_idx = get_idx("e");
    let f_idx = get_idx("f");
    let a_idx = get_idx("a");
    let b_idx = get_idx("b");

    // g must come before c and d
    assert!(g_idx < c_idx, "g should come before c");
    assert!(g_idx < d_idx, "g should come before d");

    // c must come before a
    assert!(c_idx < a_idx, "c should come before a");

    // d, e, f must come before b
    assert!(d_idx < b_idx, "d should come before b");
    assert!(e_idx < b_idx, "e should come before b");
    assert!(f_idx < b_idx, "f should come before b");
}
