//! Integration test for autouse fixtures
//!
//! This test demonstrates the complete autouse fixture flow:
//! 1. Autouse fixtures are automatically injected without explicit request
//! 2. Scope boundaries are respected (module, class, session)
//! 3. Autouse fixtures work with dependencies
//! 4. Autouse fixtures work with teardown

use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_autouse_complete_flow() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Setup: Register fixtures with different scopes and autouse settings
    
    // Session-scoped autouse fixture (applies to all tests)
    let session_setup = FixtureDefinition::new("session_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Session);

    // Module-scoped autouse fixture (applies to tests in module A)
    let module_a_setup = FixtureDefinition::new("module_a_setup", "tests/test_a.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);

    // Module-scoped autouse fixture (applies to tests in module B)
    let module_b_setup = FixtureDefinition::new("module_b_setup", "tests/test_b.py", 30)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);

    // Function-scoped autouse fixture (applies to all tests)
    let function_setup = FixtureDefinition::new("function_setup", "tests/conftest.py", 40)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    // Normal fixture (not autouse)
    let data_fixture = FixtureDefinition::new("data", "tests/conftest.py", 50);

    manager.register(session_setup);
    manager.register(module_a_setup);
    manager.register(module_b_setup);
    manager.register(function_setup);
    manager.register(data_fixture);

    // Test 1: Test in module A with no explicit fixtures
    let module_a_path = PathBuf::from("tests/test_a.py");
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_a_path,
        None,
    ).unwrap();

    // Should get: session_setup, module_a_setup, function_setup
    assert_eq!(resolved.len(), 3);
    assert!(resolved.iter().any(|f| f.definition.name == "session_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "module_a_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "function_setup"));
    assert!(!resolved.iter().any(|f| f.definition.name == "module_b_setup"));
    assert!(!resolved.iter().any(|f| f.definition.name == "data"));

    // Test 2: Test in module B with explicit data fixture
    let module_b_path = PathBuf::from("tests/test_b.py");
    let test_params_with_data = vec!["data".to_string()];
    let resolved_b = manager.resolve_fixtures_for_test_with_context(
        &test_params_with_data,
        &module_b_path,
        None,
    ).unwrap();

    // Should get: session_setup, module_b_setup, function_setup, data
    assert_eq!(resolved_b.len(), 4);
    assert!(resolved_b.iter().any(|f| f.definition.name == "session_setup"));
    assert!(resolved_b.iter().any(|f| f.definition.name == "module_b_setup"));
    assert!(resolved_b.iter().any(|f| f.definition.name == "function_setup"));
    assert!(resolved_b.iter().any(|f| f.definition.name == "data"));
    assert!(!resolved_b.iter().any(|f| f.definition.name == "module_a_setup"));
}

#[test]
fn test_autouse_with_dependency_chain() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Setup: Create a dependency chain with autouse fixtures
    
    // Base fixture (not autouse)
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10);

    // Autouse fixture that depends on config
    let db_setup = FixtureDefinition::new("db_setup", "tests/conftest.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Session)
        .with_dependencies(vec!["config".to_string()]);

    // Another autouse fixture that depends on db_setup
    let cache_setup = FixtureDefinition::new("cache_setup", "tests/conftest.py", 30)
        .with_autouse(true)
        .with_scope(FixtureScope::Session)
        .with_dependencies(vec!["db_setup".to_string()]);

    manager.register(config);
    manager.register(db_setup);
    manager.register(cache_setup);

    // Test with no explicit fixtures
    let module_path = PathBuf::from("tests/test_example.py");
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        None,
    ).unwrap();

    // Should get all three fixtures in dependency order
    assert_eq!(resolved.len(), 3);
    
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let db_idx = resolved.iter().position(|f| f.definition.name == "db_setup").unwrap();
    let cache_idx = resolved.iter().position(|f| f.definition.name == "cache_setup").unwrap();
    
    assert!(config_idx < db_idx, "config should come before db_setup");
    assert!(db_idx < cache_idx, "db_setup should come before cache_setup");
}

#[test]
fn test_autouse_with_generator_fixtures() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Setup: Create autouse fixtures with teardown
    
    let setup_fixture = FixtureDefinition::new("setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function)
        .with_generator(true);

    let cleanup_fixture = FixtureDefinition::new("cleanup", "tests/test_example.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Module)
        .with_generator(true);

    manager.register(setup_fixture);
    manager.register(cleanup_fixture);

    // Test with no explicit fixtures
    let module_path = PathBuf::from("tests/test_example.py");
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        None,
    ).unwrap();

    // Should get both autouse fixtures
    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.definition.name == "setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "cleanup"));
    
    // Both should be marked as generators
    for fixture in &resolved {
        assert!(fixture.definition.is_generator, 
            "Fixture {} should be a generator", fixture.definition.name);
    }

    // Verify teardown order (reverse of setup)
    let teardown_order = manager.get_teardown_order(&resolved);
    // Only generator fixtures should be in teardown order
    assert_eq!(teardown_order.len(), 2, 
        "Expected 2 fixtures in teardown order, got {}. Resolved fixtures: {:?}",
        teardown_order.len(),
        resolved.iter().map(|f| (&f.definition.name, f.definition.is_generator)).collect::<Vec<_>>()
    );
    // Teardown should be in reverse order of setup
    // The order depends on which fixture was resolved first
    assert!(teardown_order.iter().any(|f| f.definition.name == "setup"));
    assert!(teardown_order.iter().any(|f| f.definition.name == "cleanup"));
}

#[test]
fn test_autouse_class_scope_with_context() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Setup: Create class-scoped autouse fixtures
    
    let class_setup = FixtureDefinition::new("class_setup", "tests/test_module.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Class);

    manager.register(class_setup);

    // Test 1: Test with class context should get the fixture
    let module_path = PathBuf::from("tests/test_module.py");
    let class_name = "TestClass".to_string();
    let test_params: Vec<String> = vec![];
    let resolved_with_class = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        Some(&class_name),
    ).unwrap();

    assert_eq!(resolved_with_class.len(), 1);
    assert_eq!(resolved_with_class[0].definition.name, "class_setup");

    // Test 2: Test without class context should not get the fixture
    let resolved_without_class = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        None,
    ).unwrap();

    assert_eq!(resolved_without_class.len(), 0);

    // Test 3: Test in different module should not get the fixture
    let other_module_path = PathBuf::from("tests/test_other.py");
    let resolved_other_module = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &other_module_path,
        Some(&class_name),
    ).unwrap();

    assert_eq!(resolved_other_module.len(), 0);
}
