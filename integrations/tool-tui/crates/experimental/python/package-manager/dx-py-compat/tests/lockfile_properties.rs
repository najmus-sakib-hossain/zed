//! Property-based tests for lock file compatibility
//!
//! Property 6: Lock File Round-Trip
//! For any lock file, parsing and re-serializing SHALL preserve all
//! dependency information (package names, versions, and constraints).
//!
//! Validates: Requirements 2.4.1-2.4.3, 2.4.7

use proptest::prelude::*;
use std::collections::HashSet;

use dx_py_compat::lockfiles::{
    LockFile, LockFileFormat, LockMetadata, LockedDependency, LockedPackage, PackageSource,
    PipfileLockFormat, PoetryLockFormat, RequirementsTxtFormat, UvLockFormat,
};

/// Generate arbitrary valid package names
fn arb_package_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_-]{2,15}")
        .unwrap()
        .prop_filter("valid package name", |s| !s.is_empty() && s.len() >= 3)
}

/// Generate arbitrary version strings
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate arbitrary package source
fn arb_source() -> impl Strategy<Value = Option<PackageSource>> {
    prop_oneof![
        Just(None),
        Just(Some(PackageSource::Registry {
            url: "https://pypi.org/simple".to_string()
        })),
        arb_package_name().prop_map(|name| Some(PackageSource::Git {
            url: format!("https://github.com/test/{}.git", name),
            rev: Some("main".to_string()),
        })),
    ]
}

/// Generate arbitrary locked dependency
fn arb_locked_dependency() -> impl Strategy<Value = LockedDependency> {
    (arb_package_name(), prop::option::of(arb_version())).prop_map(|(name, version)| {
        LockedDependency {
            name,
            version,
            markers: None,
            extras: Vec::new(),
        }
    })
}

/// Generate arbitrary locked package
fn arb_locked_package() -> impl Strategy<Value = LockedPackage> {
    (
        arb_package_name(),
        arb_version(),
        arb_source(),
        prop::collection::vec(arb_locked_dependency(), 0..3),
    )
        .prop_map(|(name, version, source, dependencies)| LockedPackage {
            name,
            version,
            source,
            dependencies,
            extras: Vec::new(),
            markers: None,
            hashes: Vec::new(),
        })
}

/// Generate arbitrary lock file
fn arb_lock_file() -> impl Strategy<Value = LockFile> {
    prop::collection::vec(arb_locked_package(), 1..10).prop_map(|packages| {
        // Ensure unique package names
        let mut seen = HashSet::new();
        let unique_packages: Vec<_> =
            packages.into_iter().filter(|p| seen.insert(p.name.clone())).collect();

        LockFile {
            packages: unique_packages,
            metadata: LockMetadata {
                version: Some(1),
                requires_python: Some(">=3.8".to_string()),
                content_hash: None,
                created_by: Some("dx-py".to_string()),
            },
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Lock File Round-Trip (uv.lock format)
    /// Validates: Requirements 2.4.1, 2.4.7
    ///
    /// For any lock file, parsing and re-serializing in uv.lock format
    /// SHALL preserve all package names and versions.
    #[test]
    fn prop_uv_lock_roundtrip(lock in arb_lock_file()) {
        // Serialize
        let serialized = UvLockFormat::serialize(&lock).unwrap();

        // Parse back
        let parsed = UvLockFormat::parse(&serialized).unwrap();

        // Verify package count
        prop_assert_eq!(
            parsed.packages.len(),
            lock.packages.len(),
            "Package count should be preserved"
        );

        // Verify all packages are present with correct versions
        for original in &lock.packages {
            let found = parsed.get_package(&original.name);
            prop_assert!(
                found.is_some(),
                "Package {} should be present after round-trip",
                original.name
            );

            let found = found.unwrap();
            prop_assert_eq!(
                &found.version,
                &original.version,
                "Version of {} should be preserved",
                original.name
            );
        }

        // Verify metadata
        prop_assert_eq!(
            parsed.metadata.version,
            lock.metadata.version,
            "Lock file version should be preserved"
        );
    }

    /// Property 6: Lock File Round-Trip (poetry.lock format)
    /// Validates: Requirements 2.4.2, 2.4.7
    #[test]
    fn prop_poetry_lock_roundtrip(lock in arb_lock_file()) {
        // Serialize
        let serialized = PoetryLockFormat::serialize(&lock).unwrap();

        // Parse back
        let parsed = PoetryLockFormat::parse(&serialized).unwrap();

        // Verify all packages are present with correct versions
        for original in &lock.packages {
            let found = parsed.get_package(&original.name);
            prop_assert!(
                found.is_some(),
                "Package {} should be present after round-trip",
                original.name
            );

            let found = found.unwrap();
            prop_assert_eq!(
                &found.version,
                &original.version,
                "Version of {} should be preserved",
                original.name
            );
        }
    }

    /// Property 6: Lock File Round-Trip (requirements.txt format)
    /// Validates: Requirements 2.4.4, 2.4.5, 2.4.7
    #[test]
    fn prop_requirements_txt_roundtrip(lock in arb_lock_file()) {
        // Serialize
        let serialized = RequirementsTxtFormat::serialize(&lock).unwrap();

        // Parse back
        let parsed = RequirementsTxtFormat::parse(&serialized).unwrap();

        // Verify all packages are present with correct versions
        for original in &lock.packages {
            let found = parsed.get_package(&original.name);
            prop_assert!(
                found.is_some(),
                "Package {} should be present after round-trip",
                original.name
            );

            let found = found.unwrap();
            prop_assert_eq!(
                &found.version,
                &original.version,
                "Version of {} should be preserved",
                original.name
            );
        }
    }

    /// Property: Lock file package lookup is case-insensitive and dash/underscore normalized
    #[test]
    fn prop_package_lookup_normalized(
        name in arb_package_name(),
        version in arb_version(),
    ) {
        let lock = LockFile {
            packages: vec![LockedPackage {
                name: name.clone(),
                version: version.clone(),
                source: None,
                dependencies: Vec::new(),
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            }],
            metadata: LockMetadata::default(),
        };

        // Original name should work
        prop_assert!(lock.contains(&name));

        // Uppercase should work
        prop_assert!(lock.contains(&name.to_uppercase()));

        // Dash/underscore variants should work
        let with_dashes = name.replace('_', "-");
        let with_underscores = name.replace('-', "_");
        prop_assert!(lock.contains(&with_dashes));
        prop_assert!(lock.contains(&with_underscores));
    }

    /// Property: Dependencies are preserved in round-trip
    #[test]
    fn prop_dependencies_preserved(
        pkg_name in arb_package_name(),
        pkg_version in arb_version(),
        deps in prop::collection::vec(arb_locked_dependency(), 1..5),
    ) {
        let lock = LockFile {
            packages: vec![LockedPackage {
                name: pkg_name.clone(),
                version: pkg_version,
                source: None,
                dependencies: deps.clone(),
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            }],
            metadata: LockMetadata::default(),
        };

        // Round-trip through uv.lock format
        let serialized = UvLockFormat::serialize(&lock).unwrap();
        let parsed = UvLockFormat::parse(&serialized).unwrap();

        let pkg = parsed.get_package(&pkg_name).unwrap();
        prop_assert_eq!(
            pkg.dependencies.len(),
            deps.len(),
            "Dependency count should be preserved"
        );

        // Verify dependency names are preserved
        let original_dep_names: HashSet<_> = deps.iter().map(|d| &d.name).collect();
        let parsed_dep_names: HashSet<_> = pkg.dependencies.iter().map(|d| &d.name).collect();
        prop_assert_eq!(
            original_dep_names,
            parsed_dep_names,
            "Dependency names should be preserved"
        );
    }

    /// Property: Empty lock file round-trips correctly
    #[test]
    fn prop_empty_lock_roundtrip(_seed in any::<u64>()) {
        let lock = LockFile::new();

        // uv.lock
        let serialized = UvLockFormat::serialize(&lock).unwrap();
        let parsed = UvLockFormat::parse(&serialized).unwrap();
        prop_assert!(parsed.packages.is_empty());

        // requirements.txt
        let serialized = RequirementsTxtFormat::serialize(&lock).unwrap();
        let parsed = RequirementsTxtFormat::parse(&serialized).unwrap();
        prop_assert!(parsed.packages.is_empty());
    }

    /// Property: Pipfile.lock preserves dev dependencies
    #[test]
    fn prop_pipfile_dev_deps(
        main_pkg in arb_package_name(),
        dev_pkg in arb_package_name(),
        version in arb_version(),
    ) {
        prop_assume!(main_pkg != dev_pkg);

        let lock = LockFile {
            packages: vec![
                LockedPackage {
                    name: main_pkg.clone(),
                    version: version.clone(),
                    source: None,
                    dependencies: Vec::new(),
                    extras: Vec::new(),
                    markers: None,
                    hashes: Vec::new(),
                },
                LockedPackage {
                    name: dev_pkg.clone(),
                    version: version.clone(),
                    source: None,
                    dependencies: Vec::new(),
                    extras: vec!["dev".to_string()],
                    markers: None,
                    hashes: Vec::new(),
                },
            ],
            metadata: LockMetadata::default(),
        };

        let serialized = PipfileLockFormat::serialize(&lock).unwrap();
        let parsed = PipfileLockFormat::parse(&serialized).unwrap();

        // Both packages should be present
        prop_assert!(parsed.contains(&main_pkg));
        prop_assert!(parsed.contains(&dev_pkg));

        // Dev package should have dev extra
        let dev = parsed.get_package(&dev_pkg).unwrap();
        prop_assert!(
            dev.extras.contains(&"dev".to_string()),
            "Dev package should have dev extra"
        );
    }
}

/// Test cross-format conversion preserves essential data
#[test]
fn test_cross_format_conversion() {
    let lock = LockFile {
        packages: vec![
            LockedPackage {
                name: "requests".to_string(),
                version: "2.31.0".to_string(),
                source: Some(PackageSource::Registry {
                    url: "https://pypi.org/simple".to_string(),
                }),
                dependencies: vec![LockedDependency {
                    name: "urllib3".to_string(),
                    version: Some(">=1.21".to_string()),
                    markers: None,
                    extras: Vec::new(),
                }],
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            },
            LockedPackage {
                name: "urllib3".to_string(),
                version: "2.0.0".to_string(),
                source: None,
                dependencies: Vec::new(),
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            },
        ],
        metadata: LockMetadata {
            version: Some(1),
            requires_python: Some(">=3.8".to_string()),
            content_hash: None,
            created_by: None,
        },
    };

    // Convert to each format and back
    let uv_serialized = UvLockFormat::serialize(&lock).unwrap();
    let uv_parsed = UvLockFormat::parse(&uv_serialized).unwrap();

    let poetry_serialized = PoetryLockFormat::serialize(&lock).unwrap();
    let poetry_parsed = PoetryLockFormat::parse(&poetry_serialized).unwrap();

    let req_serialized = RequirementsTxtFormat::serialize(&lock).unwrap();
    let req_parsed = RequirementsTxtFormat::parse(&req_serialized).unwrap();

    // All formats should preserve package names and versions
    for format_name in ["uv", "poetry", "requirements"] {
        let parsed = match format_name {
            "uv" => &uv_parsed,
            "poetry" => &poetry_parsed,
            "requirements" => &req_parsed,
            _ => unreachable!(),
        };

        assert!(parsed.contains("requests"), "{} format should contain requests", format_name);
        assert!(parsed.contains("urllib3"), "{} format should contain urllib3", format_name);

        let requests = parsed.get_package("requests").unwrap();
        assert_eq!(
            requests.version, "2.31.0",
            "{} format should preserve requests version",
            format_name
        );
    }
}

/// Test that markers are preserved in formats that support them
#[test]
fn test_markers_preserved() {
    let lock = LockFile {
        packages: vec![LockedPackage {
            name: "colorama".to_string(),
            version: "0.4.6".to_string(),
            source: None,
            dependencies: Vec::new(),
            extras: Vec::new(),
            markers: Some("sys_platform == \"win32\"".to_string()),
            hashes: Vec::new(),
        }],
        metadata: LockMetadata::default(),
    };

    // requirements.txt should preserve markers
    let serialized = RequirementsTxtFormat::serialize(&lock).unwrap();
    assert!(serialized.contains("sys_platform"));

    let parsed = RequirementsTxtFormat::parse(&serialized).unwrap();
    let colorama = parsed.get_package("colorama").unwrap();
    assert!(colorama.markers.is_some());
}

/// Test that hashes are preserved in formats that support them
#[test]
fn test_hashes_preserved() {
    let lock = LockFile {
        packages: vec![LockedPackage {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
            source: None,
            dependencies: Vec::new(),
            extras: Vec::new(),
            markers: None,
            hashes: vec!["sha256:abc123".to_string()],
        }],
        metadata: LockMetadata::default(),
    };

    // requirements.txt should preserve hashes
    let serialized = RequirementsTxtFormat::serialize(&lock).unwrap();
    assert!(serialized.contains("--hash=sha256:abc123"));
}
