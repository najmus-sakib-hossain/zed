//! Integration tests for the module CLI commands
//!
//! These tests verify the module management CLI commands work correctly
//! end-to-end.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

// Import from the crate using its library name
use driven::modules::ModuleManager;

/// Helper to create a test module directory with manifest
fn create_test_module(dir: &Path, id: &str, name: &str, version: &str) {
    fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        r#"# Test Module
id|{}
nm|{}
v|{}
desc|A test module for integration testing
agent.0|test-agent
workflow.0|test-workflow
"#,
        id, name, version
    );
    fs::write(dir.join("module.dx"), manifest).unwrap();
}

#[test]
fn test_module_manager_install_and_list() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_path = temp_dir.path().join("test-module");

    // Create a test module
    create_test_module(&module_path, "test-module", "Test Module", "1.0.0");

    // Create manager and install
    let mut manager = ModuleManager::new(&registry_path);
    manager.install(&module_path).unwrap();

    // Verify installation
    let installed = manager.list_installed();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "test-module");
    assert_eq!(installed[0].name, "Test Module");
    assert_eq!(installed[0].version, "1.0.0");
}

#[test]
fn test_module_manager_uninstall() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_path = temp_dir.path().join("test-module");

    // Create and install a test module
    create_test_module(&module_path, "test-module", "Test Module", "1.0.0");
    let mut manager = ModuleManager::new(&registry_path);
    manager.install(&module_path).unwrap();

    // Verify installed
    assert_eq!(manager.list_installed().len(), 1);

    // Uninstall
    manager.uninstall("test-module").unwrap();

    // Verify uninstalled
    assert_eq!(manager.list_installed().len(), 0);
}

#[test]
fn test_module_manager_update() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_v1_path = temp_dir.path().join("test-module-v1");
    let module_v2_path = temp_dir.path().join("test-module-v2");

    // Create v1 module
    create_test_module(&module_v1_path, "test-module", "Test Module", "1.0.0");

    // Create v2 module
    create_test_module(&module_v2_path, "test-module", "Test Module", "2.0.0");

    // Install v1
    let mut manager = ModuleManager::new(&registry_path);
    manager.install(&module_v1_path).unwrap();

    // Verify v1 installed
    let module = manager.get("test-module").unwrap();
    assert_eq!(module.version, "1.0.0");

    // Update to v2
    manager.update("test-module", &module_v2_path).unwrap();

    // Verify v2 installed
    let module = manager.get("test-module").unwrap();
    assert_eq!(module.version, "2.0.0");
}

#[test]
fn test_module_manager_dependency_check() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let base_module_path = temp_dir.path().join("base-module");
    let dependent_module_path = temp_dir.path().join("dependent-module");

    // Create base module
    create_test_module(&base_module_path, "base-module", "Base Module", "1.0.0");

    // Create dependent module with a manifest that has a dependency
    fs::create_dir_all(&dependent_module_path).unwrap();
    let manifest = r#"# Dependent Module
id|dependent-module
nm|Dependent Module
v|1.0.0
desc|A module that depends on base-module
dep.base-module|^1.0.0
"#;
    fs::write(dependent_module_path.join("module.dx"), manifest).unwrap();

    let mut manager = ModuleManager::new(&registry_path);

    // Should fail because dependency is not installed
    let result = manager.install(&dependent_module_path);
    assert!(result.is_err());

    // Install the base module first
    manager.install(&base_module_path).unwrap();

    // Now installing dependent should succeed
    let result = manager.install(&dependent_module_path);
    assert!(result.is_ok());
}

#[test]
fn test_module_namespacing() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_path = temp_dir.path().join("test-module");

    // Create a test module with agents and workflows
    create_test_module(&module_path, "my-module", "My Module", "1.0.0");

    let mut manager = ModuleManager::new(&registry_path);
    manager.install(&module_path).unwrap();

    // Get namespaced agents
    let agents = manager.get_all_agents();
    assert!(agents.iter().any(|a| a == "my-module:test-agent"));

    // Get namespaced workflows
    let workflows = manager.get_all_workflows();
    assert!(workflows.iter().any(|w| w == "my-module:test-workflow"));
}

#[test]
fn test_module_isolation_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_path = temp_dir.path().join("test-module");

    create_test_module(&module_path, "my-module", "My Module", "1.0.0");

    let mut manager = ModuleManager::new(&registry_path);
    manager.disable_isolation();
    manager.install(&module_path).unwrap();

    // Without isolation, agents should not be namespaced
    let agents = manager.get_all_agents();
    assert!(agents.iter().any(|a| a == "test-agent"));
    assert!(!agents.iter().any(|a| a.contains(':')));
}

#[test]
fn test_module_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let registry_path = temp_dir.path().join("modules");
    let module_path = temp_dir.path().join("test-module");

    // Create and install a module
    create_test_module(&module_path, "test-module", "Test Module", "1.0.0");
    {
        let mut manager = ModuleManager::new(&registry_path);
        manager.install(&module_path).unwrap();
    }

    // Create a new manager and load
    let mut manager2 = ModuleManager::new(&registry_path);
    manager2.load().unwrap();

    // Verify the module is still there
    let installed = manager2.list_installed();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "test-module");
}
