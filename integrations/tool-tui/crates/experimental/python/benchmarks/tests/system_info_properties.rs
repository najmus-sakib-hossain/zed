//! Property-based tests for SystemInfo
//!
//! **Feature: comparative-benchmarks**

use dx_py_benchmarks::core::SystemInfo;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: System Info Completeness**
    /// *For any* benchmark result, the system_info field SHALL contain non-empty values for:
    /// os, os_version, cpu_model, cpu_cores, memory_gb, and python_version.
    /// **Validates: Requirements 1.5**
    #[test]
    fn property_system_info_completeness_validation(
        os in "[a-zA-Z]{3,20}",
        os_version in "[0-9]{1,2}\\.[0-9]{1,2}(\\.[0-9]{1,2})?",
        cpu_model in "[a-zA-Z0-9 ]{5,30}",
        cpu_cores in 1usize..128,
        memory_gb in 0.5f64..1024.0,
        python_version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}"
    ) {
        let info = SystemInfo {
            os: os.clone(),
            os_version: os_version.clone(),
            cpu_model: cpu_model.clone(),
            cpu_cores,
            memory_gb,
            python_version: python_version.clone(),
            dxpy_version: "0.1.0".to_string(),
            uv_version: None,
            pytest_version: None,
        };

        // Verify completeness check works correctly
        prop_assert!(info.is_complete(),
            "SystemInfo with all required fields should be complete");

        // Verify no missing fields
        let missing = info.missing_fields();
        prop_assert!(missing.is_empty(),
            "Should have no missing fields, but found: {:?}", missing);
    }

    /// Test that missing fields are correctly detected
    #[test]
    fn property_missing_fields_detection(
        os in proptest::option::of("[a-zA-Z]{3,20}"),
        cpu_cores in 0usize..128
    ) {
        let info = SystemInfo {
            os: os.clone().unwrap_or_default(),
            os_version: "10.0".to_string(),
            cpu_model: "Test CPU".to_string(),
            cpu_cores,
            memory_gb: 16.0,
            python_version: "3.11.0".to_string(),
            dxpy_version: "0.1.0".to_string(),
            uv_version: None,
            pytest_version: None,
        };

        let missing = info.missing_fields();

        // Check os field
        if os.is_none() || os.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            prop_assert!(missing.contains(&"os"),
                "Should detect missing os field");
        }

        // Check cpu_cores field
        if cpu_cores == 0 {
            prop_assert!(missing.contains(&"cpu_cores"),
                "Should detect missing cpu_cores field");
        }
    }
}

/// Test that SystemInfo::collect() returns complete info on a real system
#[test]
fn test_system_info_collect_completeness() {
    let info = SystemInfo::collect();

    // On a real system, these should be populated
    assert!(!info.os.is_empty(), "OS should be detected");
    assert!(!info.os_version.is_empty(), "OS version should be detected");
    assert!(!info.cpu_model.is_empty(), "CPU model should be detected");
    assert!(info.cpu_cores > 0, "CPU cores should be > 0");
    assert!(info.memory_gb > 0.0, "Memory should be > 0");
    // Python version may be "Unknown" if Python is not installed
}

/// Test that is_complete correctly identifies incomplete SystemInfo
#[test]
fn test_incomplete_system_info() {
    let info = SystemInfo::default();

    assert!(!info.is_complete(), "Default SystemInfo should be incomplete");

    let missing = info.missing_fields();
    assert!(missing.contains(&"os"), "Should detect missing os");
    assert!(missing.contains(&"os_version"), "Should detect missing os_version");
    assert!(missing.contains(&"cpu_model"), "Should detect missing cpu_model");
    assert!(missing.contains(&"cpu_cores"), "Should detect missing cpu_cores");
    assert!(missing.contains(&"memory_gb"), "Should detect missing memory_gb");
    assert!(missing.contains(&"python_version"), "Should detect missing python_version");
}

/// Test that complete SystemInfo passes validation
#[test]
fn test_complete_system_info() {
    let info = SystemInfo {
        os: "Windows".to_string(),
        os_version: "10.0.19041".to_string(),
        cpu_model: "Intel Core i7-9700K".to_string(),
        cpu_cores: 8,
        memory_gb: 32.0,
        python_version: "3.11.0".to_string(),
        dxpy_version: "0.1.0".to_string(),
        uv_version: Some("0.1.0".to_string()),
        pytest_version: Some("7.4.0".to_string()),
    };

    assert!(info.is_complete(), "Complete SystemInfo should pass validation");
    assert!(info.missing_fields().is_empty(), "Should have no missing fields");
}

/// Test serialization round-trip
#[test]
fn test_system_info_serialization() {
    let info = SystemInfo {
        os: "Linux".to_string(),
        os_version: "5.15.0".to_string(),
        cpu_model: "AMD Ryzen 9 5900X".to_string(),
        cpu_cores: 12,
        memory_gb: 64.0,
        python_version: "3.10.0".to_string(),
        dxpy_version: "0.1.0".to_string(),
        uv_version: Some("0.2.0".to_string()),
        pytest_version: None,
    };

    let json = serde_json::to_string(&info).expect("Should serialize");
    let deserialized: SystemInfo = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(info.os, deserialized.os);
    assert_eq!(info.os_version, deserialized.os_version);
    assert_eq!(info.cpu_model, deserialized.cpu_model);
    assert_eq!(info.cpu_cores, deserialized.cpu_cores);
    assert!((info.memory_gb - deserialized.memory_gb).abs() < 0.001);
    assert_eq!(info.python_version, deserialized.python_version);
    assert_eq!(info.uv_version, deserialized.uv_version);
    assert_eq!(info.pytest_version, deserialized.pytest_version);
}
