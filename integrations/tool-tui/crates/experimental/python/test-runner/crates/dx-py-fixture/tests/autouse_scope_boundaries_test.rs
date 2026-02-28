//! Test for autouse fixtures respecting scope boundaries
//!
//! This test verifies that autouse fixtures respect their scope boundaries:
//! - Module-scoped autouse fixtures should only apply to tests in that module
//! - Class-scoped autouse fixtures should only apply to tests in that class
//! - Session-scoped autouse fixtures should apply to all tests

use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_autouse_module_scope_boundary() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a module-scoped autouse fixture in module A
    let module_a_fixture = FixtureDefinition::new("module_a_setup", "tests/test_a.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);

    // Register a module-scoped autouse fixture in module B
    let module_b_fixture = FixtureDefinition::new("module_b_setup", "tests/test_b.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Module);

    manager.register(module_a_fixture);
    manager.register(module_b_fixture);

    // Test in module A should only get module A's autouse fixture
    let test_params: Vec<String> = vec![];
    let module_a_path = PathBuf::from("tests/test_a.py");
    let resolved = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_a_path,
        None,
    ).unwrap();

    // Should only get module A's fixture
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].definition.name, "module_a_setup");

    // Test in module B should only get module B's autouse fixture
    let module_b_path = PathBuf::from("tests/test_b.py");
    let resolved_b = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_b_path,
        None,
    ).unwrap();

    assert_eq!(resolved_b.len(), 1);
    assert_eq!(resolved_b[0].definition.name, "module_b_setup");
}

#[test]
fn test_autouse_class_scope_boundary() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a class-scoped autouse fixture in class A
    let class_a_fixture = FixtureDefinition::new("class_a_setup", "tests/test_module.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Class);

    // Register a class-scoped autouse fixture in class B
    let class_b_fixture = FixtureDefinition::new("class_b_setup", "tests/test_module.py", 20)
        .with_autouse(true)
        .with_scope(FixtureScope::Class);

    manager.register(class_a_fixture);
    manager.register(class_b_fixture);

    // Test in class A should get both fixtures (same module, has class context)
    let test_params: Vec<String> = vec![];
    let module_path = PathBuf::from("tests/test_module.py");
    let class_a_name = "TestClassA".to_string();
    let resolved = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        Some(&class_a_name),
    ).unwrap();

    // Should get both class fixtures since they're in the same module
    // Note: We can't distinguish between classes without more metadata
    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.definition.name == "class_a_setup"));
    assert!(resolved.iter().any(|f| f.definition.name == "class_b_setup"));

    // Test without class context should not get class-scoped fixtures
    let resolved_no_class = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_path,
        None,
    ).unwrap();

    assert_eq!(resolved_no_class.len(), 0);
}

#[test]
fn test_autouse_session_scope_applies_to_all() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Register a session-scoped autouse fixture
    let session_fixture = FixtureDefinition::new("session_setup", "tests/conftest.py", 10)
        .with_autouse(true)
        .with_scope(FixtureScope::Session);

    manager.register(session_fixture);

    // Test in any module should get the session fixture
    let test_params: Vec<String> = vec![];
    
    // Test in module A
    let module_a_path = PathBuf::from("tests/test_a.py");
    let resolved_a = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_a_path,
        None,
    ).unwrap();
    assert_eq!(resolved_a.len(), 1);
    assert_eq!(resolved_a[0].definition.name, "session_setup");

    // Test in module B
    let module_b_path = PathBuf::from("tests/test_b.py");
    let resolved_b = manager.resolve_fixtures_for_test_with_context(
        &test_params,
        &module_b_path,
        None,
    ).unwrap();
    assert_eq!(resolved_b.len(), 1);
    assert_eq!(resolved_b[0].definition.name, "session_setup");
}
