//! Test for autouse fixtures with different scopes
//!
//! This test verifies that autouse fixtures are automatically injected
//! for all tests within their scope, as per requirement 11.6.

use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope};
use tempfile::tempdir;

#[test]
fn test_autouse_function_scope() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a function-scoped autouse fixture
    let autouse = FixtureDefinition::new("setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    manager.register(autouse);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should automatically get the autouse fixture
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "setup");
}

#[test]
fn test_autouse_module_scope() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a module-scoped autouse fixture
    let autouse = FixtureDefinition::new("module_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);

    manager.register(autouse);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should automatically get the autouse fixture
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "module_setup");
}

#[test]
fn test_autouse_class_scope() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a class-scoped autouse fixture
    let autouse = FixtureDefinition::new("class_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Class);

    manager.register(autouse);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should automatically get the autouse fixture
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "class_setup");
}

#[test]
fn test_autouse_session_scope() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a session-scoped autouse fixture
    let autouse = FixtureDefinition::new("session_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Session);

    manager.register(autouse);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should automatically get the autouse fixture
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "session_setup");
}

#[test]
fn test_autouse_multiple_scopes() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register autouse fixtures with different scopes
    let session = FixtureDefinition::new("session_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Session);
    
    let module = FixtureDefinition::new("module_setup", "tests/conftest.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);
    
    let class = FixtureDefinition::new("class_setup", "tests/conftest.py", 30)
        .with_autouse(true)
        .with_scope(FixtureScope::Class);
    
    let function = FixtureDefinition::new("function_setup", "tests/conftest.py", 40)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    manager.register(session);
    manager.register(module);
    manager.register(class);
    manager.register(function);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should get all autouse fixtures
    assert_eq!(resolved.len(), 4);
    assert!(resolved.iter().any(|f| f.definition.name == "session_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "module_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "class_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "function_setup"));
}

#[test]
fn test_autouse_with_explicit_fixtures() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register autouse fixtures
    let autouse_module = FixtureDefinition::new("auto_module", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);
    
    let autouse_function = FixtureDefinition::new("auto_func", "tests/conftest.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    // Register normal fixtures
    let normal = FixtureDefinition::new("data", "tests/conftest.py", 30);

    manager.register(autouse_module);
    manager.register(autouse_function);
    manager.register(normal);

    // Test explicitly requests 'data'
    let test_params = vec!["data".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should get both autouse fixtures plus the explicit one
    assert_eq!(resolved.len(), 3);
    assert!(resolved.iter().any(|f| f.definition.name == "auto_module"));
    assert!(resolved.iter().any(|f| f.definition.name == "auto_func"));
    assert!(resolved.iter().any(|f| f.definition.name == "data"));
}

#[test]
fn test_autouse_with_dependencies() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a base fixture
    let base = FixtureDefinition::new("config", "tests/conftest.py", 10);

    // Register an autouse fixture that depends on the base
    let autouse = FixtureDefinition::new("setup", "tests/conftest.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Function)
        .with_dependencies(vec!["config".to_string()]);

    manager.register(base);
    manager.register(autouse);

    // Test with no explicit fixture parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should get both fixtures in dependency order
    assert_eq!(resolved.len(), 2);
    
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let setup_idx = resolved.iter().position(|f| f.definition.name == "setup").unwrap();
    
    assert!(config_idx < setup_idx, "config should come before setup");
}

#[test]
fn test_autouse_no_duplicates() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register an autouse fixture
    let autouse = FixtureDefinition::new("setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    manager.register(autouse);

    // Test explicitly requests the autouse fixture
    let test_params = vec!["setup".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should only get the fixture once
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "setup");
}
