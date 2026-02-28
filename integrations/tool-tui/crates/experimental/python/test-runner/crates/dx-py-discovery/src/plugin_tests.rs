// Unit tests for plugin compatibility

use super::plugin::*;
use tempfile::TempDir;

#[test]
fn test_hook_type_function_names() {
    assert_eq!(HookType::PytestCollectionStart.function_name(), "pytest_collection_start");
    assert_eq!(HookType::PytestSessionStart.function_name(), "pytest_sessionstart");
    assert_eq!(HookType::PytestConfigure.function_name(), "pytest_configure");
}

#[test]
fn test_known_plugin_names() {
    assert_eq!(KnownPlugin::PytestCov.package_name(), "pytest-cov");
    assert_eq!(KnownPlugin::PytestAsyncio.package_name(), "pytest-asyncio");
    assert_eq!(KnownPlugin::PytestCov.import_name(), "pytest_cov");
}

#[test]
fn test_plugin_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = PluginManager::new(temp_dir.path());
    assert!(manager.is_ok());
}

#[test]
fn test_discover_conftest_simple() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple conftest.py
    let conftest_content = r#"
import pytest

@pytest.fixture
def my_fixture():
    return 42

def pytest_configure(config):
    pass
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    let conftest_files = manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    assert_eq!(conftest_files.len(), 1);
    assert!(!conftest_files[0].fixtures.is_empty());
    assert!(conftest_files[0].fixtures.contains(&"my_fixture".to_string()));
}

#[test]
fn test_discover_hooks() {
    let temp_dir = TempDir::new().unwrap();
    
    let conftest_content = r#"
def pytest_collection_start(session):
    pass

def pytest_sessionstart(session):
    pass

def pytest_configure(config):
    pass
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    let collection_hooks = manager.get_hooks(HookType::PytestCollectionStart);
    assert_eq!(collection_hooks.len(), 1);
    
    let session_hooks = manager.get_hooks(HookType::PytestSessionStart);
    assert_eq!(session_hooks.len(), 1);
    
    let configure_hooks = manager.get_hooks(HookType::PytestConfigure);
    assert_eq!(configure_hooks.len(), 1);
}

#[test]
fn test_nested_conftest_discovery() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create root conftest
    std::fs::write(
        temp_dir.path().join("conftest.py"),
        "@pytest.fixture\ndef root_fixture():\n    pass\n"
    ).unwrap();
    
    // Create nested directory with conftest
    let nested_dir = temp_dir.path().join("tests");
    std::fs::create_dir(&nested_dir).unwrap();
    std::fs::write(
        nested_dir.join("conftest.py"),
        "@pytest.fixture\ndef nested_fixture():\n    pass\n"
    ).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    let conftest_files = manager.discover_conftest_files(&nested_dir).unwrap();
    
    // Should find both root and nested conftest
    assert_eq!(conftest_files.len(), 2);
}

#[test]
fn test_pytest_plugins_parsing() {
    let temp_dir = TempDir::new().unwrap();
    
    let conftest_content = r#"
pytest_plugins = ["pytest_cov", "pytest_asyncio"]
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    let conftest_files = manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    assert_eq!(conftest_files.len(), 1);
    assert!(conftest_files[0].plugin_imports.contains(&"pytest_cov".to_string()));
    assert!(conftest_files[0].plugin_imports.contains(&"pytest_asyncio".to_string()));
}

#[test]
fn test_hook_priority_sorting() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create conftest with hooks that have different priorities
    let conftest_content = r#"
import pytest

@pytest.hookimpl(trylast=True)
def pytest_configure(config):
    pass
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    let hooks = manager.get_hooks(HookType::PytestConfigure);
    // Should have the hook with trylast marker
    assert!(!hooks.is_empty());
}

#[test]
fn test_get_all_fixtures() {
    let temp_dir = TempDir::new().unwrap();
    
    let conftest_content = r#"
import pytest

@pytest.fixture
def fixture_one():
    return 1

@pytest.fixture
def fixture_two():
    return 2
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    let fixtures = manager.get_all_fixtures();
    assert_eq!(fixtures.len(), 2);
    
    let fixture_names: Vec<_> = fixtures.iter().map(|(_, name)| *name).collect();
    assert!(fixture_names.contains(&"fixture_one"));
    assert!(fixture_names.contains(&"fixture_two"));
}

#[test]
fn test_detect_plugins() {
    let temp_dir = TempDir::new().unwrap();
    
    let conftest_content = r#"
pytest_plugins = ["pytest_cov", "pytest_asyncio"]
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    manager.discover_conftest_files(temp_dir.path()).unwrap();
    
    let plugins = manager.detect_plugins();
    assert!(plugins.contains(&KnownPlugin::PytestCov));
    assert!(plugins.contains(&KnownPlugin::PytestAsyncio));
}

#[test]
fn test_has_plugin() {
    let temp_dir = TempDir::new().unwrap();
    
    let conftest_content = r#"
pytest_plugins = ["pytest_cov"]
"#;
    
    std::fs::write(temp_dir.path().join("conftest.py"), conftest_content).unwrap();
    
    let mut manager = PluginManager::new(temp_dir.path()).unwrap();
    manager.discover_conftest_files(temp_dir.path()).unwrap();
    manager.detect_plugins();
    
    assert!(manager.has_plugin(KnownPlugin::PytestCov));
    assert!(!manager.has_plugin(KnownPlugin::PytestXdist));
}
