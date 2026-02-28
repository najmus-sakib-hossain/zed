//! Property-based tests for System Information Provider
//!
//! These tests validate the correctness properties defined in the design document.

use super::*;
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Generate a random project type
fn arb_project_type() -> impl Strategy<Value = ProjectType> {
    prop_oneof![
        Just(ProjectType::Rust),
        Just(ProjectType::Node),
        Just(ProjectType::Python),
        Just(ProjectType::Go),
        Just(ProjectType::Java),
        Just(ProjectType::Ruby),
    ]
}

/// Generate a random TTL duration (1ms to 10s)
fn arb_ttl() -> impl Strategy<Value = Duration> {
    (1u64..10000u64).prop_map(Duration::from_millis)
}

/// Create a project directory with marker files for a given project type
fn create_project_dir(project_type: ProjectType) -> TempDir {
    let temp = TempDir::new().unwrap();

    match project_type {
        ProjectType::Rust => {
            fs::write(
                temp.path().join("Cargo.toml"),
                r#"[package]
name = "test-project"
version = "0.1.0"
"#,
            )
            .unwrap();
            fs::create_dir(temp.path().join("src")).unwrap();
        }
        ProjectType::Node => {
            fs::write(
                temp.path().join("package.json"),
                r#"{"name": "test-project", "version": "1.0.0"}"#,
            )
            .unwrap();
        }
        ProjectType::Python => {
            fs::write(
                temp.path().join("pyproject.toml"),
                r#"[project]
name = "test-project"
"#,
            )
            .unwrap();
        }
        ProjectType::Go => {
            fs::write(temp.path().join("go.mod"), "module test-project\n\ngo 1.21\n").unwrap();
        }
        ProjectType::Java => {
            fs::write(
                temp.path().join("pom.xml"),
                r#"<project><artifactId>test-project</artifactId></project>"#,
            )
            .unwrap();
        }
        ProjectType::Ruby => {
            fs::write(temp.path().join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
        }
        _ => {}
    }

    temp
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 17: Project Type Detection Accuracy**
    /// *For any* project with standard structure markers (Cargo.toml, package.json, etc.),
    /// the project type SHALL be correctly detected.
    /// **Validates: Requirements 5.5**
    #[test]
    fn prop_project_type_detection_accuracy(project_type in arb_project_type()) {
        let temp = create_project_dir(project_type);

        let provider = SystemInfoProvider::default()
            .with_project_root(temp.path());

        let detected = provider.detect_project();

        prop_assert!(detected.is_some(), "Project should be detected");
        let project_info = detected.unwrap();
        prop_assert_eq!(
            project_info.project_type,
            project_type,
            "Detected project type should match expected"
        );
    }

    /// **Property 15: System Information Caching**
    /// *For any* system information request within TTL, the cached value SHALL be
    /// returned without re-detection.
    /// **Validates: Requirements 5.9**
    #[test]
    fn prop_system_info_caching(ttl_ms in 100u64..5000u64) {
        let ttl = Duration::from_millis(ttl_ms);
        let mut provider = SystemInfoProvider::new(ttl);

        // First call populates cache
        let info1 = provider.get().unwrap();
        let collected_at1 = info1.collected_at;

        // Verify cache is valid
        prop_assert!(provider.is_cache_valid(), "Cache should be valid after first call");

        // Second call within TTL should return cached value
        let info2 = provider.get().unwrap();
        let collected_at2 = info2.collected_at;

        // The collected_at timestamp should be the same (cached)
        prop_assert_eq!(
            collected_at1,
            collected_at2,
            "Cached value should have same timestamp"
        );

        // OS info should be identical
        prop_assert_eq!(info1.os, info2.os, "Cached OS info should match");
        prop_assert_eq!(info1.shell, info2.shell, "Cached shell info should match");
    }

    /// **Property 16: System Information Cache Invalidation**
    /// *For any* cache invalidation, the next request SHALL re-detect system information.
    /// **Validates: Requirements 5.10**
    #[test]
    fn prop_cache_invalidation(ttl_ms in 100u64..5000u64) {
        let ttl = Duration::from_millis(ttl_ms);
        let mut provider = SystemInfoProvider::new(ttl);

        // Populate cache
        let _info1 = provider.get().unwrap();
        prop_assert!(provider.is_cache_valid(), "Cache should be valid");

        // Invalidate cache
        provider.invalidate();
        prop_assert!(!provider.is_cache_valid(), "Cache should be invalid after invalidation");

        // Next call should re-detect (cache should become valid again)
        let _info2 = provider.get().unwrap();
        prop_assert!(provider.is_cache_valid(), "Cache should be valid after re-detection");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Additional property: OS detection consistency
    /// *For any* number of consecutive calls, OS detection should return consistent results.
    #[test]
    fn prop_os_detection_consistency(num_calls in 1usize..10usize) {
        let provider = SystemInfoProvider::default();

        let first_os = provider.detect_os().unwrap();

        for _ in 0..num_calls {
            let os = provider.detect_os().unwrap();
            prop_assert_eq!(&os.name, &first_os.name, "OS name should be consistent");
            prop_assert_eq!(&os.arch, &first_os.arch, "OS arch should be consistent");
            prop_assert_eq!(&os.family, &first_os.family, "OS family should be consistent");
        }
    }

    /// Additional property: Shell detection consistency
    /// *For any* number of consecutive calls, shell detection should return consistent results.
    #[test]
    fn prop_shell_detection_consistency(num_calls in 1usize..10usize) {
        let provider = SystemInfoProvider::default();

        let first_shell = provider.detect_shell().unwrap();

        for _ in 0..num_calls {
            let shell = provider.detect_shell().unwrap();
            prop_assert_eq!(&shell.name, &first_shell.name, "Shell name should be consistent");
            prop_assert_eq!(&shell.path, &first_shell.path, "Shell path should be consistent");
        }
    }

    /// Additional property: Cache TTL expiry
    /// *For any* TTL, cache should expire after the TTL duration.
    #[test]
    fn prop_cache_ttl_expiry(ttl_ms in 1u64..50u64) {
        let ttl = Duration::from_millis(ttl_ms);
        let mut cache = SystemInfoCache::new();

        // Create minimal test info
        let info = SystemInfo {
            os: OsInfo {
                name: "test".to_string(),
                version: "1.0".to_string(),
                arch: "x86_64".to_string(),
                family: "unix".to_string(),
            },
            shell: ShellInfo {
                name: "bash".to_string(),
                version: None,
                path: PathBuf::from("/bin/bash"),
            },
            languages: vec![],
            package_managers: vec![],
            project: None,
            git: None,
            build_tools: vec![],
            test_frameworks: vec![],
            collected_at: std::time::SystemTime::now(),
        };

        cache.set(info);
        prop_assert!(cache.is_valid(ttl), "Cache should be valid immediately after set");

        // Wait for TTL to expire
        std::thread::sleep(ttl + Duration::from_millis(10));

        prop_assert!(!cache.is_valid(ttl), "Cache should be invalid after TTL expires");
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_project_markers_exist() {
        // Verify that get_project_markers returns non-empty for known types
        let rust_markers = detectors::get_project_markers(ProjectType::Rust);
        assert!(!rust_markers.is_empty());
        assert!(rust_markers.contains(&"Cargo.toml"));

        let node_markers = detectors::get_project_markers(ProjectType::Node);
        assert!(!node_markers.is_empty());
        assert!(node_markers.contains(&"package.json"));
    }

    #[test]
    fn test_mixed_project_detection() {
        let temp = TempDir::new().unwrap();

        // Create both Rust and Node markers
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();

        let detected = detect_project_type(temp.path());
        assert_eq!(detected, ProjectType::Mixed);
    }

    #[test]
    fn test_unknown_project_detection() {
        let temp = TempDir::new().unwrap();

        // No marker files
        let detected = detect_project_type(temp.path());
        assert_eq!(detected, ProjectType::Unknown);
    }
}
