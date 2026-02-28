//! Integration tests for dx-py-compability
//!
//! These tests verify the full integration of all modules working together.

use dx_py_compability::{
    validate_config,
    Architecture,
    // Config
    DxPyConfig,
    InstallationSource,
    // Markers
    MarkerEnvironment,
    MarkerEvaluator,
    // Platform
    Platform,
    PlatformDetector,
    PythonRuntime,
    PythonVersion,
    RuntimeCapabilities,
    // Runtime
    RuntimeDetector,
    // Uv
    UvConfig,
    UvConfigLoader,
    // Venv
    VenvBuilder,
    VenvOptions,
    WheelTagGenerator,
};
use tempfile::TempDir;

// =============================================================================
// Runtime Detection Integration Tests
// =============================================================================

#[test]
fn test_runtime_detector_creation() {
    let _detector = RuntimeDetector::new();
    // Just verify it can be created without panicking
}

#[test]
fn test_python_version_supported_range() {
    // Test all supported versions
    for minor in 8..=13 {
        let version = PythonVersion::new(3, minor, 0);
        assert!(version.is_supported(), "Python 3.{} should be supported", minor);
    }

    // Test unsupported versions
    assert!(!PythonVersion::new(2, 7, 0).is_supported());
    assert!(!PythonVersion::new(3, 7, 0).is_supported());
    assert!(!PythonVersion::new(3, 14, 0).is_supported());
}

#[test]
fn test_python_version_parsing() {
    let v = PythonVersion::parse("3.12.0").unwrap();
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 12);
    assert_eq!(v.patch, 0);

    let v2 = PythonVersion::parse("3.13.0a1").unwrap();
    assert_eq!(v2.major, 3);
    assert_eq!(v2.minor, 13);
    assert!(v2.pre_release.is_some());
}

// =============================================================================
// Platform Detection Integration Tests
// =============================================================================

#[test]
fn test_platform_detection() {
    let platform = PlatformDetector::detect();

    // Platform should have valid OS
    match platform.os {
        dx_py_compability::Os::Linux
        | dx_py_compability::Os::Windows
        | dx_py_compability::Os::MacOs
        | dx_py_compability::Os::FreeBsd
        | dx_py_compability::Os::Other(_) => {}
    }

    // Architecture should be detected
    match platform.arch {
        Architecture::X86_64
        | Architecture::X86
        | Architecture::Aarch64
        | Architecture::Arm
        | Architecture::Other(_) => {}
    }
}

#[test]
fn test_wheel_tag_generation() {
    let platform = Platform::default();
    let generator = WheelTagGenerator::with_version(platform, 3, 12);
    let tags = generator.generate_tags();

    // Should generate at least some tags
    assert!(!tags.is_empty(), "Should generate wheel tags");

    // Tags should be in priority order
    for window in tags.windows(2) {
        assert!(window[0].priority <= window[1].priority, "Tags should be in priority order");
    }
}

// =============================================================================
// Marker Evaluation Integration Tests
// =============================================================================

#[test]
fn test_marker_environment_creation() {
    let env = MarkerEnvironment::default();

    // Should have valid values
    assert!(!env.python_version.is_empty());
    assert!(!env.sys_platform.is_empty());
}

#[test]
fn test_marker_evaluation_basic() {
    let env = MarkerEnvironment {
        python_version: "3.12".to_string(),
        sys_platform: "linux".to_string(),
        ..Default::default()
    };

    let mut evaluator = MarkerEvaluator::new(env);

    // Test basic comparisons
    assert!(evaluator.evaluate("python_version >= '3.8'").unwrap());
    assert!(evaluator.evaluate("sys_platform == 'linux'").unwrap());
    assert!(!evaluator.evaluate("sys_platform == 'win32'").unwrap());
}

#[test]
fn test_marker_evaluation_complex() {
    let env = MarkerEnvironment {
        python_version: "3.12".to_string(),
        sys_platform: "linux".to_string(),
        ..Default::default()
    };

    let mut evaluator = MarkerEvaluator::new(env);

    // Test AND
    assert!(evaluator
        .evaluate("python_version >= '3.8' and sys_platform == 'linux'")
        .unwrap());
    assert!(!evaluator
        .evaluate("python_version >= '3.8' and sys_platform == 'win32'")
        .unwrap());

    // Test OR
    assert!(evaluator
        .evaluate("sys_platform == 'linux' or sys_platform == 'win32'")
        .unwrap());
}

// =============================================================================
// Virtual Environment Integration Tests
// =============================================================================

#[test]
fn test_venv_options_builder() {
    let options = VenvOptions::new()
        .system_site_packages(true)
        .copies(true)
        .clear(true)
        .with_pip(false);

    assert!(options.system_site_packages);
    assert!(options.copies);
    assert!(options.clear);
    assert!(!options.with_pip);
}

#[test]
fn test_venv_creation_with_mock_python() {
    let temp_dir = TempDir::new().unwrap();
    let python_path = temp_dir.path().join("python");

    // Create a mock Python executable
    #[cfg(unix)]
    {
        std::fs::write(&python_path, "#!/bin/sh\necho 'Python 3.12.0'").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&python_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::write(&python_path, "").unwrap();
    }

    let python = PythonRuntime {
        executable: python_path,
        version: PythonVersion::new(3, 12, 0),
        architecture: Architecture::default(),
        source: InstallationSource::System,
        capabilities: RuntimeCapabilities::default(),
    };

    let options = VenvOptions::new().system_site_packages(false).copies(true).clear(true);

    let venv_path = temp_dir.path().join("test_venv");
    let builder = VenvBuilder::new(python).with_options(options);
    let result = builder.build(&venv_path);

    assert!(result.is_ok(), "Venv creation should succeed");

    let venv = result.unwrap();
    assert!(venv.path.exists());
    assert!(venv.path.join("pyvenv.cfg").exists());
}

// =============================================================================
// Configuration Integration Tests
// =============================================================================

#[test]
fn test_config_creation_and_validation() {
    let config = DxPyConfig::new()
        .with_python_version(PythonVersion::new(3, 12, 0))
        .with_index_url("https://pypi.org/simple")
        .with_max_concurrent_downloads(10);

    assert!(validate_config(&config).is_ok());
}

#[test]
fn test_config_round_trip() {
    let config = DxPyConfig {
        python_version: Some(PythonVersion::new(3, 12, 0)),
        index_url: Some("https://pypi.org/simple".to_string()),
        extra_index_urls: vec!["https://extra.pypi.org".to_string()],
        cache_dir: None,
        max_concurrent_downloads: Some(10),
        uv_compat: None,
    };

    let toml = config.to_toml().unwrap();
    let parsed = DxPyConfig::from_toml(&toml).unwrap();

    assert_eq!(config, parsed);
}

#[test]
fn test_config_validation_errors() {
    // Invalid index URL
    let config = DxPyConfig {
        index_url: Some("ftp://invalid.com".to_string()),
        ..Default::default()
    };
    assert!(validate_config(&config).is_err());

    // Invalid max downloads
    let config = DxPyConfig {
        max_concurrent_downloads: Some(0),
        ..Default::default()
    };
    assert!(validate_config(&config).is_err());

    // Unsupported Python version
    let config = DxPyConfig {
        python_version: Some(PythonVersion::new(2, 7, 0)),
        ..Default::default()
    };
    assert!(validate_config(&config).is_err());
}

// =============================================================================
// uv Configuration Integration Tests
// =============================================================================

#[test]
fn test_uv_config_creation() {
    let config = UvConfig {
        index_url: Some("https://pypi.org/simple".to_string()),
        extra_index_url: vec![],
        find_links: vec![],
        no_binary: vec![],
        only_binary: vec![],
        python_version: Some("3.12".to_string()),
        python_preference: None,
        cache_dir: None,
        compile_bytecode: None,
    };

    assert!(!config.is_empty());
}

#[test]
fn test_uv_config_merge_precedence() {
    let uv_config = UvConfig {
        index_url: Some("https://uv.pypi.org/simple".to_string()),
        ..Default::default()
    };

    let dxpy_config = DxPyConfig {
        index_url: Some("https://dxpy.pypi.org/simple".to_string()),
        ..Default::default()
    };

    let merged = UvConfigLoader::merge_with_dxpy(uv_config, &dxpy_config);

    // dx-py should take precedence
    assert_eq!(merged.config.index_url, Some("https://dxpy.pypi.org/simple".to_string()));
    // Should have a warning about the override
    assert!(!merged.warnings.is_empty());
}

// =============================================================================
// Full Integration Flow Test
// =============================================================================

#[test]
fn test_full_integration_flow() {
    // 1. Create a configuration
    let config = DxPyConfig::new()
        .with_python_version(PythonVersion::new(3, 12, 0))
        .with_index_url("https://pypi.org/simple")
        .with_max_concurrent_downloads(10);

    // 2. Validate the configuration
    assert!(validate_config(&config).is_ok());

    // 3. Serialize and deserialize
    let toml = config.to_toml().unwrap();
    let parsed = DxPyConfig::from_toml(&toml).unwrap();
    assert_eq!(config, parsed);

    // 4. Detect platform
    let platform = PlatformDetector::detect();

    // 5. Generate wheel tags
    let generator = WheelTagGenerator::with_version(platform, 3, 12);
    let tags = generator.generate_tags();
    assert!(!tags.is_empty());

    // 6. Create marker environment
    let env = MarkerEnvironment::default();
    let mut evaluator = MarkerEvaluator::new(env);

    // 7. Evaluate markers
    let result = evaluator.evaluate("python_version >= '3.8'");
    assert!(result.is_ok());
}
