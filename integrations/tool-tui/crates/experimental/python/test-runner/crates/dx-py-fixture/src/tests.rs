//! Tests for fixture management

use super::*;
use tempfile::tempdir;

#[test]
fn test_fixture_scope_priority() {
    assert!(FixtureScope::Function.priority() < FixtureScope::Class.priority());
    assert!(FixtureScope::Class.priority() < FixtureScope::Module.priority());
    assert!(FixtureScope::Module.priority() < FixtureScope::Session.priority());
}

#[test]
fn test_fixture_scope_default() {
    assert_eq!(FixtureScope::default(), FixtureScope::Function);
}

#[test]
fn test_fixture_definition_new() {
    let fixture = FixtureDefinition::new("my_fixture", "tests/conftest.py", 10);
    assert_eq!(fixture.name, "my_fixture");
    assert_eq!(fixture.scope, FixtureScope::Function);
    assert!(!fixture.autouse);
    assert!(fixture.dependencies.is_empty());
    assert!(!fixture.is_generator);
}

#[test]
fn test_fixture_definition_builder() {
    let fixture = FixtureDefinition::new("db_connection", "tests/conftest.py", 20)
        .with_scope(FixtureScope::Module)
        .with_autouse(true)
        .with_dependencies(vec!["config".to_string()])
        .with_generator(true);

    assert_eq!(fixture.name, "db_connection");
    assert_eq!(fixture.scope, FixtureScope::Module);
    assert!(fixture.autouse);
    assert_eq!(fixture.dependencies, vec!["config".to_string()]);
    assert!(fixture.is_generator);
}

#[test]
fn test_fixture_manager_new() {
    let temp_dir = tempdir().unwrap();
    let manager = FixtureManager::new(temp_dir.path()).unwrap();
    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_fixture_manager_register() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    let fixture = FixtureDefinition::new("test_fixture", "tests/conftest.py", 10);
    manager.register(fixture);

    assert_eq!(manager.len(), 1);
    assert!(manager.get("test_fixture").is_some());
    assert!(manager.get("nonexistent").is_none());
}

#[test]
fn test_fixture_manager_resolve_simple() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    let fixture = FixtureDefinition::new("simple_fixture", "tests/conftest.py", 10);
    manager.register(fixture);

    let resolved = manager.resolve_fixtures(&["simple_fixture".to_string()]).unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "simple_fixture");
}

#[test]
fn test_fixture_manager_resolve_with_dependencies() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with dependencies
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10);
    let db = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_dependencies(vec!["config".to_string()]);
    let app = FixtureDefinition::new("app", "tests/conftest.py", 30)
        .with_dependencies(vec!["db".to_string(), "config".to_string()]);

    manager.register(config);
    manager.register(db);
    manager.register(app);

    let resolved = manager.resolve_fixtures(&["app".to_string()]).unwrap();

    // Should have all three fixtures in dependency order
    assert_eq!(resolved.len(), 3);

    // config should come before db and app
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let db_idx = resolved.iter().position(|f| f.definition.name == "db").unwrap();
    let app_idx = resolved.iter().position(|f| f.definition.name == "app").unwrap();

    assert!(config_idx < db_idx, "config should come before db");
    assert!(db_idx < app_idx, "db should come before app");
}

#[test]
fn test_fixture_manager_resolve_not_found() {
    let temp_dir = tempdir().unwrap();
    let manager = FixtureManager::new(temp_dir.path()).unwrap();

    let result = manager.resolve_fixtures(&["nonexistent".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_fixture_manager_autouse() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    let autouse_fixture = FixtureDefinition::new("auto", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    let normal_fixture = FixtureDefinition::new("normal", "tests/conftest.py", 20);

    manager.register(autouse_fixture);
    manager.register(normal_fixture);

    let autouse = manager.get_autouse_fixtures(FixtureScope::Function);
    assert_eq!(autouse.len(), 1);
    assert_eq!(autouse[0].name, "auto");
}

#[test]
fn test_fixture_manager_teardown_order() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    let fixture1 = FixtureDefinition::new("fixture1", "tests/conftest.py", 10).with_generator(true);
    let fixture2 = FixtureDefinition::new("fixture2", "tests/conftest.py", 20).with_generator(true);
    let fixture3 =
        FixtureDefinition::new("fixture3", "tests/conftest.py", 30).with_generator(false);

    manager.register(fixture1);
    manager.register(fixture2);
    manager.register(fixture3);

    let resolved = manager
        .resolve_fixtures(&[
            "fixture1".to_string(),
            "fixture2".to_string(),
            "fixture3".to_string(),
        ])
        .unwrap();

    let teardown = manager.get_teardown_order(&resolved);

    // Only generator fixtures should be in teardown
    assert_eq!(teardown.len(), 2);

    // Should be in reverse order
    assert_eq!(teardown[0].definition.name, "fixture2");
    assert_eq!(teardown[1].definition.name, "fixture1");
}

#[test]
fn test_fixture_manager_activation() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    let fixture = FixtureDefinition::new("test_fixture", "tests/conftest.py", 10);
    manager.register(fixture);

    assert!(!manager.is_active("test_fixture"));

    manager.activate_fixture("test_fixture", FixtureScope::Function);
    assert!(manager.is_active("test_fixture"));

    manager.deactivate_scope(FixtureScope::Function);
    assert!(!manager.is_active("test_fixture"));
}

#[test]
fn test_fixture_cache_new() {
    let temp_dir = tempdir().unwrap();
    let cache = FixtureCache::new(temp_dir.path()).unwrap();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_fixture_cache_store_and_load() {
    let temp_dir = tempdir().unwrap();
    let mut cache = FixtureCache::new(temp_dir.path()).unwrap();

    let id = FixtureId::new(12345);
    let source_hash = FixtureCache::hash_source("def my_fixture(): return 42");
    let value: i32 = 42;

    cache.store(id, source_hash, &value).unwrap();
    assert!(cache.contains(id));

    let loaded: i32 = cache.load(id).unwrap();
    assert_eq!(loaded, 42);
}

#[test]
fn test_fixture_cache_invalidation() {
    let temp_dir = tempdir().unwrap();
    let mut cache = FixtureCache::new(temp_dir.path()).unwrap();

    let id = FixtureId::new(12345);
    let source_hash = FixtureCache::hash_source("def my_fixture(): return 42");
    let value: i32 = 42;

    cache.store(id, source_hash, &value).unwrap();
    assert!(cache.contains(id));

    cache.invalidate(id).unwrap();
    assert!(!cache.contains(id));
}

#[test]
fn test_fixture_cache_clear() {
    let temp_dir = tempdir().unwrap();
    let mut cache = FixtureCache::new(temp_dir.path()).unwrap();

    let id1 = FixtureId::new(12345);
    let id2 = FixtureId::new(67890);
    let source_hash = FixtureCache::hash_source("def fixture(): pass");

    cache.store(id1, source_hash, &1i32).unwrap();
    cache.store(id2, source_hash, &2i32).unwrap();

    assert_eq!(cache.len(), 2);

    cache.clear().unwrap();
    assert!(cache.is_empty());
}

#[test]
fn test_fixture_cache_get_or_create() {
    let temp_dir = tempdir().unwrap();
    let mut cache = FixtureCache::new(temp_dir.path()).unwrap();

    let id = FixtureId::new(12345);
    let source = "def my_fixture(): return 42";

    // First call should create
    let value1: i32 = cache.get_or_create(id, source, || 42).unwrap();
    assert_eq!(value1, 42);

    // Second call should use cache
    let value2: i32 = cache.get_or_create(id, source, || 100).unwrap();
    assert_eq!(value2, 42); // Should still be 42, not 100
}

#[test]
fn test_fixture_cache_source_change_invalidates() {
    let temp_dir = tempdir().unwrap();
    let mut cache = FixtureCache::new(temp_dir.path()).unwrap();

    let id = FixtureId::new(12345);
    let source1 = "def my_fixture(): return 42";
    let source2 = "def my_fixture(): return 100";

    // Create with first source
    let value1: i32 = cache.get_or_create(id, source1, || 42).unwrap();
    assert_eq!(value1, 42);

    // Change source - should invalidate and recreate
    let value2: i32 = cache.get_or_create(id, source2, || 100).unwrap();
    assert_eq!(value2, 100);
}

#[test]
fn test_fixture_injection_simple() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a fixture
    let fixture = FixtureDefinition::new("sample_data", "tests/conftest.py", 10);
    manager.register(fixture);

    // Test with parameter matching fixture name
    let test_params = vec!["sample_data".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "sample_data");
}

#[test]
fn test_fixture_injection_with_dependencies() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures with dependencies
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10);
    let db = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_dependencies(vec!["config".to_string()]);

    manager.register(config);
    manager.register(db);

    // Test requests only 'db', but should get 'config' too
    let test_params = vec!["db".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    assert_eq!(resolved.len(), 2);
    
    // config should come before db
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let db_idx = resolved.iter().position(|f| f.definition.name == "db").unwrap();
    assert!(config_idx < db_idx, "config should come before db");
}

#[test]
fn test_fixture_injection_multiple_params() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register multiple fixtures
    let fixture1 = FixtureDefinition::new("fixture1", "tests/conftest.py", 10);
    let fixture2 = FixtureDefinition::new("fixture2", "tests/conftest.py", 20);
    let fixture3 = FixtureDefinition::new("fixture3", "tests/conftest.py", 30);

    manager.register(fixture1);
    manager.register(fixture2);
    manager.register(fixture3);

    // Test with multiple parameters
    let test_params = vec!["fixture1".to_string(), "fixture3".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.definition.name == "fixture1"));
    assert!(resolved.iter().any(|f| f.definition.name == "fixture3"));
}

#[test]
fn test_fixture_injection_non_fixture_params() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a fixture
    let fixture = FixtureDefinition::new("my_fixture", "tests/conftest.py", 10);
    manager.register(fixture);

    // Test with mix of fixture and non-fixture parameters
    let test_params = vec!["my_fixture".to_string(), "regular_param".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should only resolve the fixture parameter
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "my_fixture");
}

#[test]
fn test_fixture_injection_with_autouse() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register an autouse fixture
    let autouse_fixture = FixtureDefinition::new("auto_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Function);

    // Register a normal fixture
    let normal_fixture = FixtureDefinition::new("data", "tests/conftest.py", 20);

    manager.register(autouse_fixture);
    manager.register(normal_fixture);

    // Test only requests 'data', but should also get 'auto_setup'
    let test_params = vec!["data".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.definition.name == "auto_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "data"));
}

#[test]
fn test_fixture_injection_empty_params() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register fixtures
    let fixture = FixtureDefinition::new("my_fixture", "tests/conftest.py", 10);
    manager.register(fixture);

    // Test with no parameters
    let test_params: Vec<String> = vec![];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should only get autouse fixtures (none in this case)
    assert_eq!(resolved.len(), 0);
}

#[test]
fn test_fixture_injection_chain_resolution() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Create a dependency chain: test -> app -> db -> config
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10);
    let db = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_dependencies(vec!["config".to_string()]);
    let app = FixtureDefinition::new("app", "tests/conftest.py", 30)
        .with_dependencies(vec!["db".to_string()]);

    manager.register(config);
    manager.register(db);
    manager.register(app);

    // Test only requests 'app'
    let test_params = vec!["app".to_string()];
    let resolved = manager.resolve_fixtures_for_test(&test_params).unwrap();

    // Should resolve entire chain
    assert_eq!(resolved.len(), 3);
    
    // Verify dependency order
    let config_idx = resolved.iter().position(|f| f.definition.name == "config").unwrap();
    let db_idx = resolved.iter().position(|f| f.definition.name == "db").unwrap();
    let app_idx = resolved.iter().position(|f| f.definition.name == "app").unwrap();
    
    assert!(config_idx < db_idx, "config should come before db");
    assert!(db_idx < app_idx, "db should come before app");
}
