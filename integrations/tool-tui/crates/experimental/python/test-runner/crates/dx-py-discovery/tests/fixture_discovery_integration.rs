//! Integration tests for fixture discovery

use dx_py_discovery::FixtureDiscovery;
use dx_py_fixture::FixtureScope;
use std::path::PathBuf;

#[test]
fn test_discover_conftest_fixtures() {
    let conftest_path = PathBuf::from("../../../runtime/tests/sample_pytest_tests/conftest.py");
    
    if !conftest_path.exists() {
        // Skip test if file doesn't exist
        return;
    }

    let mut discovery = FixtureDiscovery::new().unwrap();
    let fixtures = discovery.discover_file(&conftest_path).unwrap();

    // Should find multiple fixtures
    assert!(!fixtures.is_empty(), "Should discover fixtures from conftest.py");

    // Check for specific fixtures
    let module_data = fixtures.iter().find(|f| f.name == "module_data");
    assert!(module_data.is_some(), "Should find module_data fixture");
    assert_eq!(module_data.unwrap().scope, FixtureScope::Module);

    let session_config = fixtures.iter().find(|f| f.name == "session_config");
    assert!(session_config.is_some(), "Should find session_config fixture");
    assert_eq!(session_config.unwrap().scope, FixtureScope::Session);

    let counter = fixtures.iter().find(|f| f.name == "counter");
    assert!(counter.is_some(), "Should find counter fixture");
    assert_eq!(counter.unwrap().scope, FixtureScope::Function);

    let temp_file = fixtures.iter().find(|f| f.name == "temp_file");
    assert!(temp_file.is_some(), "Should find temp_file fixture");
    assert!(temp_file.unwrap().is_generator, "temp_file should be a generator (uses yield)");
    assert!(temp_file.unwrap().dependencies.contains(&"tmp_path".to_string()), 
            "temp_file should depend on tmp_path");
}

#[test]
fn test_discover_test_file_fixtures() {
    let test_path = PathBuf::from("../../../runtime/tests/sample_pytest_tests/test_sample.py");
    
    if !test_path.exists() {
        // Skip test if file doesn't exist
        return;
    }

    let mut discovery = FixtureDiscovery::new().unwrap();
    let fixtures = discovery.discover_file(&test_path).unwrap();

    // Should find fixtures defined in test file
    assert!(!fixtures.is_empty(), "Should discover fixtures from test file");

    // Check for specific fixtures
    let sample_list = fixtures.iter().find(|f| f.name == "sample_list");
    assert!(sample_list.is_some(), "Should find sample_list fixture");

    let sample_dict = fixtures.iter().find(|f| f.name == "sample_dict");
    assert!(sample_dict.is_some(), "Should find sample_dict fixture");
}
