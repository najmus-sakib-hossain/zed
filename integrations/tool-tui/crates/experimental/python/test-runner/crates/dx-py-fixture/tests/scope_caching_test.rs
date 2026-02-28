//! Tests for fixture scope caching
//!
//! Validates Requirements 11.2, 11.3:
//! - Fixtures with scope="module" are created once per module
//! - Fixtures with scope="session" are created once per test session
//! - Fixtures with scope="class" are created once per test class
//! - Fixtures with scope="function" are created for each test

use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope, ScopeInstance};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_function_scope_never_caches() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a function-scoped fixture
    let fixture = FixtureDefinition::new("data", "tests/test_module.py", 10)
        .with_scope(FixtureScope::Function);
    manager.register(fixture);

    let module_path = PathBuf::from("tests/test_module.py");

    // First test
    let resolved1 = manager
        .resolve_fixtures_for_test_with_context(&["data".to_string()], &module_path, None)
        .unwrap();
    assert_eq!(resolved1.len(), 1);
    assert!(resolved1[0].needs_setup, "Function scope should always need setup");
    assert!(resolved1[0].cached_value.is_none(), "Function scope should not have cached value");

    // Cache a value (this should not actually cache for function scope)
    let scope_instance = ScopeInstance::from_test_context(FixtureScope::Function, &module_path, None);
    manager.cache_for_scope("data", scope_instance.clone(), vec![1, 2, 3]);

    // Second test - should still need setup
    let resolved2 = manager
        .resolve_fixtures_for_test_with_context(&["data".to_string()], &module_path, None)
        .unwrap();
    assert_eq!(resolved2.len(), 1);
    assert!(resolved2[0].needs_setup, "Function scope should always need setup");
    assert!(resolved2[0].cached_value.is_none(), "Function scope should not cache");
}

#[test]
fn test_module_scope_caches_per_module() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a module-scoped fixture
    let fixture = FixtureDefinition::new("db", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Module);
    manager.register(fixture);

    let module1 = PathBuf::from("tests/test_api.py");
    let module2 = PathBuf::from("tests/test_db.py");

    // First test in module1
    let resolved1 = manager
        .resolve_fixtures_for_test_with_context(&["db".to_string()], &module1, None)
        .unwrap();
    assert_eq!(resolved1.len(), 1);
    assert!(resolved1[0].needs_setup, "First use should need setup");
    assert!(resolved1[0].cached_value.is_none());

    // Simulate fixture setup - cache the value
    let scope_instance1 = ScopeInstance::from_test_context(FixtureScope::Module, &module1, None);
    manager.cache_for_scope("db", scope_instance1, vec![10, 20, 30]);

    // Second test in same module - should use cached value
    let resolved2 = manager
        .resolve_fixtures_for_test_with_context(&["db".to_string()], &module1, None)
        .unwrap();
    assert_eq!(resolved2.len(), 1);
    assert!(!resolved2[0].needs_setup, "Should use cached value");
    assert_eq!(resolved2[0].cached_value, Some(vec![10, 20, 30]));

    // First test in different module - should need setup again
    let resolved3 = manager
        .resolve_fixtures_for_test_with_context(&["db".to_string()], &module2, None)
        .unwrap();
    assert_eq!(resolved3.len(), 1);
    assert!(resolved3[0].needs_setup, "Different module should need setup");
    assert!(resolved3[0].cached_value.is_none());
}

#[test]
fn test_class_scope_caches_per_class() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a class-scoped fixture
    let fixture = FixtureDefinition::new("client", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Class);
    manager.register(fixture);

    let module = PathBuf::from("tests/test_api.py");
    let class1 = "TestAPI".to_string();
    let class2 = "TestDatabase".to_string();

    // First test in class1
    let resolved1 = manager
        .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class1))
        .unwrap();
    assert_eq!(resolved1.len(), 1);
    assert!(resolved1[0].needs_setup, "First use should need setup");

    // Cache the value for class1
    let scope_instance1 = ScopeInstance::from_test_context(FixtureScope::Class, &module, Some(&class1));
    manager.cache_for_scope("client", scope_instance1, vec![1, 2, 3, 4]);

    // Second test in same class - should use cached value
    let resolved2 = manager
        .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class1))
        .unwrap();
    assert_eq!(resolved2.len(), 1);
    assert!(!resolved2[0].needs_setup, "Should use cached value");
    assert_eq!(resolved2[0].cached_value, Some(vec![1, 2, 3, 4]));

    // First test in different class - should need setup again
    let resolved3 = manager
        .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class2))
        .unwrap();
    assert_eq!(resolved3.len(), 1);
    assert!(resolved3[0].needs_setup, "Different class should need setup");
    assert!(resolved3[0].cached_value.is_none());
}

#[test]
fn test_session_scope_caches_globally() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a session-scoped fixture
    let fixture = FixtureDefinition::new("config", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Session);
    manager.register(fixture);

    let module1 = PathBuf::from("tests/test_api.py");
    let module2 = PathBuf::from("tests/test_db.py");
    let class1 = "TestAPI".to_string();
    let class2 = "TestDatabase".to_string();

    // First test
    let resolved1 = manager
        .resolve_fixtures_for_test_with_context(&["config".to_string()], &module1, Some(&class1))
        .unwrap();
    assert_eq!(resolved1.len(), 1);
    assert!(resolved1[0].needs_setup, "First use should need setup");

    // Cache the value
    let scope_instance = ScopeInstance::from_test_context(FixtureScope::Session, &module1, Some(&class1));
    manager.cache_for_scope("config", scope_instance, vec![99, 88, 77]);

    // Test in different module and class - should still use cached value
    let resolved2 = manager
        .resolve_fixtures_for_test_with_context(&["config".to_string()], &module2, Some(&class2))
        .unwrap();
    assert_eq!(resolved2.len(), 1);
    assert!(!resolved2[0].needs_setup, "Session scope should be cached globally");
    assert_eq!(resolved2[0].cached_value, Some(vec![99, 88, 77]));

    // Test in same module, no class - should still use cached value
    let resolved3 = manager
        .resolve_fixtures_for_test_with_context(&["config".to_string()], &module1, None)
        .unwrap();
    assert_eq!(resolved3.len(), 1);
    assert!(!resolved3[0].needs_setup, "Session scope should be cached globally");
    assert_eq!(resolved3[0].cached_value, Some(vec![99, 88, 77]));
}

#[test]
fn test_clear_scope_cache() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with different scopes
    let module_fixture = FixtureDefinition::new("db", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Module);
    let class_fixture = FixtureDefinition::new("client", "tests/conftest.py", 20)
        .with_scope(FixtureScope::Class);

    manager.register(module_fixture);
    manager.register(class_fixture);

    let module = PathBuf::from("tests/test_api.py");
    let class_name = "TestAPI".to_string();

    // Cache values
    let module_scope = ScopeInstance::from_test_context(FixtureScope::Module, &module, None);
    let class_scope = ScopeInstance::from_test_context(FixtureScope::Class, &module, Some(&class_name));

    manager.cache_for_scope("db", module_scope.clone(), vec![1, 2, 3]);
    manager.cache_for_scope("client", class_scope.clone(), vec![4, 5, 6]);

    // Verify both are cached
    assert_eq!(manager.get_cached_for_scope("db", &module_scope), Some(vec![1, 2, 3]));
    assert_eq!(manager.get_cached_for_scope("client", &class_scope), Some(vec![4, 5, 6]));

    // Clear class scope
    manager.clear_scope_cache(FixtureScope::Class, &module, Some(&class_name));

    // Class cache should be cleared, module cache should remain
    assert_eq!(manager.get_cached_for_scope("db", &module_scope), Some(vec![1, 2, 3]));
    assert_eq!(manager.get_cached_for_scope("client", &class_scope), None);

    // Clear module scope
    manager.clear_scope_cache(FixtureScope::Module, &module, None);

    // Both should be cleared now
    assert_eq!(manager.get_cached_for_scope("db", &module_scope), None);
}

#[test]
fn test_mixed_scopes_in_single_test() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with different scopes
    let session = FixtureDefinition::new("config", "tests/conftest.py", 10)
        .with_scope(FixtureScope::Session);
    let module = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_scope(FixtureScope::Module);
    let class = FixtureDefinition::new("client", "tests/conftest.py", 30)
        .with_scope(FixtureScope::Class);
    let function = FixtureDefinition::new("request", "tests/conftest.py", 40)
        .with_scope(FixtureScope::Function);

    manager.register(session);
    manager.register(module);
    manager.register(class);
    manager.register(function);

    let module_path = PathBuf::from("tests/test_api.py");
    let class_name = "TestAPI".to_string();

    // First test - all need setup
    let resolved1 = manager
        .resolve_fixtures_for_test_with_context(
            &["config".to_string(), "db".to_string(), "client".to_string(), "request".to_string()],
            &module_path,
            Some(&class_name),
        )
        .unwrap();

    assert_eq!(resolved1.len(), 4);
    for fixture in &resolved1 {
        assert!(fixture.needs_setup, "First use should need setup for {}", fixture.definition.name);
    }

    // Cache all values
    let session_scope = ScopeInstance::from_test_context(FixtureScope::Session, &module_path, Some(&class_name));
    let module_scope = ScopeInstance::from_test_context(FixtureScope::Module, &module_path, Some(&class_name));
    let class_scope = ScopeInstance::from_test_context(FixtureScope::Class, &module_path, Some(&class_name));
    let function_scope = ScopeInstance::from_test_context(FixtureScope::Function, &module_path, Some(&class_name));

    manager.cache_for_scope("config", session_scope, vec![1]);
    manager.cache_for_scope("db", module_scope, vec![2]);
    manager.cache_for_scope("client", class_scope, vec![3]);
    manager.cache_for_scope("request", function_scope, vec![4]); // Should not actually cache

    // Second test in same context
    let resolved2 = manager
        .resolve_fixtures_for_test_with_context(
            &["config".to_string(), "db".to_string(), "client".to_string(), "request".to_string()],
            &module_path,
            Some(&class_name),
        )
        .unwrap();

    assert_eq!(resolved2.len(), 4);

    // Check caching behavior
    let config = resolved2.iter().find(|f| f.definition.name == "config").unwrap();
    let db = resolved2.iter().find(|f| f.definition.name == "db").unwrap();
    let client = resolved2.iter().find(|f| f.definition.name == "client").unwrap();
    let request = resolved2.iter().find(|f| f.definition.name == "request").unwrap();

    assert!(!config.needs_setup, "Session scope should be cached");
    assert_eq!(config.cached_value, Some(vec![1]));

    assert!(!db.needs_setup, "Module scope should be cached");
    assert_eq!(db.cached_value, Some(vec![2]));

    assert!(!client.needs_setup, "Class scope should be cached");
    assert_eq!(client.cached_value, Some(vec![3]));

    assert!(request.needs_setup, "Function scope should always need setup");
    assert_eq!(request.cached_value, None, "Function scope should not cache");
}

#[test]
fn test_scope_instance_equality() {
    let module1 = PathBuf::from("tests/test_api.py");
    let module2 = PathBuf::from("tests/test_db.py");
    let class1 = "TestAPI".to_string();
    let class2 = "TestDatabase".to_string();

    // Same module scope instances should be equal
    let scope1 = ScopeInstance::from_test_context(FixtureScope::Module, &module1, None);
    let scope2 = ScopeInstance::from_test_context(FixtureScope::Module, &module1, None);
    assert_eq!(scope1, scope2);

    // Different module scope instances should not be equal
    let scope3 = ScopeInstance::from_test_context(FixtureScope::Module, &module2, None);
    assert_ne!(scope1, scope3);

    // Same class scope instances should be equal
    let scope4 = ScopeInstance::from_test_context(FixtureScope::Class, &module1, Some(&class1));
    let scope5 = ScopeInstance::from_test_context(FixtureScope::Class, &module1, Some(&class1));
    assert_eq!(scope4, scope5);

    // Different class scope instances should not be equal
    let scope6 = ScopeInstance::from_test_context(FixtureScope::Class, &module1, Some(&class2));
    assert_ne!(scope4, scope6);

    // Session scope instances should always be equal
    let scope7 = ScopeInstance::from_test_context(FixtureScope::Session, &module1, Some(&class1));
    let scope8 = ScopeInstance::from_test_context(FixtureScope::Session, &module2, Some(&class2));
    assert_eq!(scope7, scope8);
}
