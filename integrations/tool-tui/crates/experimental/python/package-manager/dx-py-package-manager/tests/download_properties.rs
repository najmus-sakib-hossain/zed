//! Property-based tests for package downloads
//!
//! **Feature: dx-py-production-ready, Property 15: Package Download Hash Verification**
//! **Feature: dx-py-production-ready, Property 16: Dependency Resolution Completeness**
//! **Validates: Requirements 8.1-8.6**

use proptest::prelude::*;
use std::collections::HashSet;

use dx_py_core::version::PackedVersion;
use dx_py_package_manager::{
    compute_sha256, verify_sha256, Dependency, InMemoryProvider, PubGrubResolver, Resolver,
    VersionConstraint,
};

// ============================================================================
// Property 15: Package Download Hash Verification
// Validates: Requirements 8.2
//
// *For any* downloaded content, if the computed SHA256 hash does not match
// the expected hash, the download SHALL be rejected.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 15a: Correct hash is always accepted
    /// *For any* downloaded content, computing its SHA256 hash and verifying
    /// against that same hash SHALL succeed.
    #[test]
    fn prop_download_correct_hash_accepted(data in prop::collection::vec(any::<u8>(), 0..2048)) {
        let hash = compute_sha256(&data);
        // Correct hash should always be accepted
        prop_assert!(verify_sha256(&data, &hash).is_ok(),
            "Verification should succeed for correct hash");
    }

    /// Property 15b: Wrong hash is always rejected
    /// *For any* downloaded content and any different hash, verification SHALL fail.
    #[test]
    fn prop_download_wrong_hash_rejected(
        data in prop::collection::vec(any::<u8>(), 1..1024),
        wrong_hash in "[0-9a-f]{64}"
    ) {
        let correct_hash = compute_sha256(&data);
        // Only test if the wrong hash is actually different
        if wrong_hash.to_lowercase() != correct_hash {
            prop_assert!(verify_sha256(&data, &wrong_hash).is_err(),
                "Verification should fail for incorrect hash");
        }
    }

    /// Property 15c: Hash verification is case-insensitive
    /// *For any* downloaded content, hash verification SHALL accept both
    /// uppercase and lowercase hex representations.
    #[test]
    fn prop_download_hash_case_insensitive(data in prop::collection::vec(any::<u8>(), 0..512)) {
        let hash = compute_sha256(&data);
        let upper_hash = hash.to_uppercase();
        let lower_hash = hash.to_lowercase();

        prop_assert!(verify_sha256(&data, &upper_hash).is_ok(),
            "Uppercase hash should be accepted");
        prop_assert!(verify_sha256(&data, &lower_hash).is_ok(),
            "Lowercase hash should be accepted");
    }

    /// Property 15d: Hash computation is deterministic
    /// *For any* downloaded content, computing the hash multiple times
    /// SHALL produce identical results.
    #[test]
    fn prop_download_hash_deterministic(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let hash1 = compute_sha256(&data);
        let hash2 = compute_sha256(&data);
        let hash3 = compute_sha256(&data);

        prop_assert_eq!(&hash1, &hash2, "Hash should be deterministic (1 vs 2)");
        prop_assert_eq!(&hash2, &hash3, "Hash should be deterministic (2 vs 3)");
    }

    /// Property 15e: Different content produces different hashes
    /// *For any* two different data inputs, the hashes SHALL be different
    /// (collision resistance).
    #[test]
    fn prop_download_hash_collision_resistant(
        data1 in prop::collection::vec(any::<u8>(), 1..512),
        data2 in prop::collection::vec(any::<u8>(), 1..512)
    ) {
        if data1 != data2 {
            let hash1 = compute_sha256(&data1);
            let hash2 = compute_sha256(&data2);
            prop_assert_ne!(hash1, hash2,
                "Different data should produce different hashes");
        }
    }

    /// Property 15f: Single byte modification invalidates hash
    /// *For any* downloaded content, modifying a single byte SHALL cause
    /// hash verification to fail.
    #[test]
    fn prop_download_single_byte_modification_detected(
        data in prop::collection::vec(any::<u8>(), 1..512),
        modify_index in any::<usize>(),
        modify_value in any::<u8>()
    ) {
        let original_hash = compute_sha256(&data);
        let mut modified_data = data.clone();

        // Modify a single byte
        let idx = modify_index % modified_data.len();
        let original_byte = modified_data[idx];

        // Only test if we're actually changing the byte
        if modify_value != original_byte {
            modified_data[idx] = modify_value;

            // Verification should fail for modified data
            prop_assert!(verify_sha256(&modified_data, &original_hash).is_err(),
                "Modified data should fail hash verification");
        }
    }
}

// ============================================================================
// Property 16: Dependency Resolution Completeness
// Validates: Requirements 8.3, 8.4
//
// *For any* package with dependencies, the resolver SHALL include all
// transitive dependencies in the resolution, selecting the highest
// compatible version for each.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 16a: All transitive dependencies are included
    /// *For any* dependency chain a -> b -> c -> ..., resolving 'a' SHALL
    /// include all packages in the chain.
    #[test]
    fn prop_resolution_includes_all_transitive_deps(
        depth in 2usize..6,
        seed in any::<u64>()
    ) {
        // Create a deterministic chain based on seed
        let mut provider = InMemoryProvider::new();
        let pkg_names: Vec<String> = (0..depth).map(|i| format!("pkg_{}", i)).collect();

        for i in 0..depth {
            let version = ((seed >> (i * 8)) % 10 + 1) as u32;
            let version_str = format!("{}.0.0", version);

            let deps = if i + 1 < depth {
                vec![Dependency::new(&pkg_names[i + 1], VersionConstraint::Any)]
            } else {
                vec![]
            };

            provider.add_package(&pkg_names[i], &version_str, deps);
        }

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new(&pkg_names[0], VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps);
        prop_assert!(resolution.is_ok(), "Resolution should succeed");

        let resolution = resolution.unwrap();
        let resolved_names: HashSet<_> = resolution.packages.iter()
            .map(|p| p.name.as_str())
            .collect();

        // All packages in the chain should be resolved
        for name in &pkg_names {
            prop_assert!(resolved_names.contains(name.as_str()),
                "Package '{}' should be in resolution", name);
        }

        prop_assert_eq!(resolution.packages.len(), depth,
            "Resolution should contain exactly {} packages", depth);
    }

    /// Property 16b: Diamond dependencies are resolved correctly
    /// *For any* diamond dependency pattern (a -> b, a -> c, b -> d, c -> d),
    /// resolving 'a' SHALL include all four packages exactly once.
    #[test]
    fn prop_resolution_diamond_completeness(
        v_a in 1u32..10,
        v_b in 1u32..10,
        v_c in 1u32..10,
        v_d in 1u32..10,
    ) {
        let mut provider = InMemoryProvider::new();

        provider.add_package(
            "a",
            &format!("{}.0.0", v_a),
            vec![
                Dependency::new("b", VersionConstraint::Any),
                Dependency::new("c", VersionConstraint::Any),
            ],
        );
        provider.add_package(
            "b",
            &format!("{}.0.0", v_b),
            vec![Dependency::new("d", VersionConstraint::Any)],
        );
        provider.add_package(
            "c",
            &format!("{}.0.0", v_c),
            vec![Dependency::new("d", VersionConstraint::Any)],
        );
        provider.add_package("d", &format!("{}.0.0", v_d), vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("a", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();

        // Should have exactly 4 packages
        prop_assert_eq!(resolution.packages.len(), 4,
            "Diamond should resolve to exactly 4 packages");

        let resolved_names: HashSet<_> = resolution.packages.iter()
            .map(|p| p.name.as_str())
            .collect();

        prop_assert!(resolved_names.contains("a"), "Package 'a' should be resolved");
        prop_assert!(resolved_names.contains("b"), "Package 'b' should be resolved");
        prop_assert!(resolved_names.contains("c"), "Package 'c' should be resolved");
        prop_assert!(resolved_names.contains("d"), "Package 'd' should be resolved");
    }

    /// Property 16c: Highest compatible version is selected
    /// *For any* package with multiple versions, the resolver SHALL select
    /// the highest version satisfying all constraints.
    #[test]
    fn prop_resolution_selects_highest_version(
        v1 in 1u32..10,
        v2 in 11u32..20,
        v3 in 21u32..30,
    ) {
        let mut provider = InMemoryProvider::new();

        // Add multiple versions
        provider.add_package("pkg", &format!("{}.0.0", v1), vec![]);
        provider.add_package("pkg", &format!("{}.0.0", v2), vec![]);
        provider.add_package("pkg", &format!("{}.0.0", v3), vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("pkg", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();

        prop_assert_eq!(resolution.packages.len(), 1);
        prop_assert_eq!(resolution.packages[0].version.major, v3,
            "Should select highest version ({}), got {}",
            v3, resolution.packages[0].version.major);
    }

    /// Property 16d: Constrained resolution selects highest within constraint
    /// *For any* version constraint, the resolver SHALL select the highest
    /// version that satisfies the constraint.
    #[test]
    fn prop_resolution_highest_within_constraint(
        min_version in 1u32..5,
        max_version in 6u32..10,
    ) {
        let mut provider = InMemoryProvider::new();

        // Add versions from 1 to 10
        for v in 1u32..=10 {
            provider.add_package("pkg", &format!("{}.0.0", v), vec![]);
        }

        let mut resolver = Resolver::new(provider);

        // Request versions in range [min, max)
        let deps = vec![Dependency::new(
            "pkg",
            VersionConstraint::Range {
                min: PackedVersion::new(min_version, 0, 0),
                max: PackedVersion::new(max_version, 0, 0),
            },
        )];

        let resolution = resolver.resolve(&deps).unwrap();

        prop_assert_eq!(resolution.packages.len(), 1);

        // Should be highest version less than max
        let expected_version = max_version - 1;
        prop_assert_eq!(resolution.packages[0].version.major, expected_version,
            "Should select highest version in range [{}, {}), expected {}, got {}",
            min_version, max_version, expected_version, resolution.packages[0].version.major);
    }

    /// Property 16e: Transitive constraint satisfaction
    /// *For any* dependency graph where a -> b with constraint, the resolved
    /// version of b SHALL satisfy a's constraint.
    #[test]
    fn prop_resolution_transitive_constraint_satisfaction(
        min_b_version in 2u32..5,
    ) {
        let mut provider = InMemoryProvider::new();

        // a requires b >= min_b_version
        provider.add_package(
            "a",
            "1.0.0",
            vec![Dependency::new(
                "b",
                VersionConstraint::Gte(PackedVersion::new(min_b_version, 0, 0)),
            )],
        );

        // Add multiple versions of b
        for v in 1u32..=10 {
            provider.add_package("b", &format!("{}.0.0", v), vec![]);
        }

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("a", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();

        // Find package b
        let b_pkg = resolution.packages.iter().find(|p| p.name == "b").unwrap();

        // b's version should satisfy a's constraint (>= min_b_version)
        prop_assert!(b_pkg.version.major >= min_b_version,
            "b's version ({}) should be >= {}", b_pkg.version.major, min_b_version);

        // Should be highest available (10)
        prop_assert_eq!(b_pkg.version.major, 10,
            "Should select highest version satisfying constraint");
    }

    /// Property 16f: Multiple root dependencies are all resolved
    /// *For any* set of root dependencies, all SHALL be included in resolution.
    #[test]
    fn prop_resolution_multiple_roots(
        num_roots in 2usize..5,
        seed in any::<u64>(),
    ) {
        let mut provider = InMemoryProvider::new();
        let root_names: Vec<String> = (0..num_roots).map(|i| format!("root_{}", i)).collect();

        for (i, name) in root_names.iter().enumerate() {
            let version = ((seed >> (i * 8)) % 10 + 1) as u32;
            provider.add_package(name, &format!("{}.0.0", version), vec![]);
        }

        let mut resolver = Resolver::new(provider);
        let deps: Vec<Dependency> = root_names.iter()
            .map(|name| Dependency::new(name, VersionConstraint::Any))
            .collect();

        let resolution = resolver.resolve(&deps).unwrap();

        let resolved_names: HashSet<_> = resolution.packages.iter()
            .map(|p| p.name.as_str())
            .collect();

        for name in &root_names {
            prop_assert!(resolved_names.contains(name.as_str()),
                "Root package '{}' should be in resolution", name);
        }

        prop_assert_eq!(resolution.packages.len(), num_roots,
            "Should resolve exactly {} root packages", num_roots);
    }

    /// Property 16g: Shared transitive dependencies are resolved once
    /// *For any* graph where multiple packages depend on the same package,
    /// that package SHALL appear exactly once in the resolution.
    #[test]
    fn prop_resolution_shared_deps_resolved_once(
        num_dependents in 2usize..5,
        shared_version in 1u32..10,
    ) {
        let mut provider = InMemoryProvider::new();

        // Create multiple packages that all depend on "shared"
        for i in 0..num_dependents {
            provider.add_package(
                &format!("dep_{}", i),
                "1.0.0",
                vec![Dependency::new("shared", VersionConstraint::Any)],
            );
        }

        provider.add_package("shared", &format!("{}.0.0", shared_version), vec![]);

        let mut resolver = Resolver::new(provider);
        let deps: Vec<Dependency> = (0..num_dependents)
            .map(|i| Dependency::new(&format!("dep_{}", i), VersionConstraint::Any))
            .collect();

        let resolution = resolver.resolve(&deps).unwrap();

        // Count occurrences of "shared"
        let shared_count = resolution.packages.iter()
            .filter(|p| p.name == "shared")
            .count();

        prop_assert_eq!(shared_count, 1,
            "Shared dependency should appear exactly once, found {}", shared_count);

        // Total packages should be num_dependents + 1 (shared)
        prop_assert_eq!(resolution.packages.len(), num_dependents + 1,
            "Should have {} dependents + 1 shared = {} packages",
            num_dependents, num_dependents + 1);
    }
}

// ============================================================================
// Additional unit tests for edge cases
// ============================================================================

#[test]
fn test_hash_empty_data() {
    let data: &[u8] = b"";
    let hash = compute_sha256(data);
    // SHA256 of empty string is well-known
    assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert!(verify_sha256(data, &hash).is_ok());
}

#[test]
fn test_hash_known_value() {
    let data = b"hello world";
    let hash = compute_sha256(data);
    assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    assert!(verify_sha256(data, &hash).is_ok());
}

#[test]
fn test_resolution_deep_chain() {
    let mut provider = InMemoryProvider::new();

    // Create a deep chain: a -> b -> c -> d -> e -> f
    provider.add_package("a", "1.0.0", vec![Dependency::new("b", VersionConstraint::Any)]);
    provider.add_package("b", "1.0.0", vec![Dependency::new("c", VersionConstraint::Any)]);
    provider.add_package("c", "1.0.0", vec![Dependency::new("d", VersionConstraint::Any)]);
    provider.add_package("d", "1.0.0", vec![Dependency::new("e", VersionConstraint::Any)]);
    provider.add_package("e", "1.0.0", vec![Dependency::new("f", VersionConstraint::Any)]);
    provider.add_package("f", "1.0.0", vec![]);

    let mut resolver = Resolver::new(provider);
    let deps = vec![Dependency::new("a", VersionConstraint::Any)];

    let resolution = resolver.resolve(&deps).unwrap();

    assert_eq!(resolution.packages.len(), 6);

    let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains("a"));
    assert!(names.contains("b"));
    assert!(names.contains("c"));
    assert!(names.contains("d"));
    assert!(names.contains("e"));
    assert!(names.contains("f"));
}

#[test]
fn test_resolution_with_version_constraints() {
    let mut provider = InMemoryProvider::new();

    // requests depends on urllib3>=1.21.0 and certifi
    provider.add_package(
        "requests",
        "2.30.0",
        vec![
            Dependency::new(
                "urllib3",
                VersionConstraint::Gte(PackedVersion::new(1, 21, 0)),
            ),
            Dependency::new("certifi", VersionConstraint::Any),
        ],
    );

    // Multiple versions of urllib3
    provider.add_package("urllib3", "1.20.0", vec![]);
    provider.add_package("urllib3", "1.26.0", vec![]);
    provider.add_package("urllib3", "2.0.0", vec![]);

    provider.add_package("certifi", "2023.5.7", vec![]);

    let mut resolver = Resolver::new(provider);
    let deps = vec![Dependency::new("requests", VersionConstraint::Any)];

    let resolution = resolver.resolve(&deps).unwrap();

    // Should resolve all 3 packages
    assert_eq!(resolution.packages.len(), 3);

    // urllib3 should be 2.0.0 (highest satisfying >=1.21.0)
    let urllib3 = resolution.packages.iter().find(|p| p.name == "urllib3").unwrap();
    assert_eq!(urllib3.version, PackedVersion::new(2, 0, 0));
}

#[test]
fn test_resolution_combined_constraints() {
    let mut provider = InMemoryProvider::new();

    // a requires c>=1.0.0
    // b requires c<2.0.0
    // Should resolve c to highest in [1.0.0, 2.0.0)
    provider.add_package(
        "a",
        "1.0.0",
        vec![Dependency::new(
            "c",
            VersionConstraint::Gte(PackedVersion::new(1, 0, 0)),
        )],
    );
    provider.add_package(
        "b",
        "1.0.0",
        vec![Dependency::new(
            "c",
            VersionConstraint::Lt(PackedVersion::new(2, 0, 0)),
        )],
    );

    provider.add_package("c", "0.5.0", vec![]);
    provider.add_package("c", "1.0.0", vec![]);
    provider.add_package("c", "1.5.0", vec![]);
    provider.add_package("c", "2.0.0", vec![]);

    // Use PubGrubResolver for proper constraint combination
    let mut resolver = PubGrubResolver::new(provider);
    let deps = vec![
        Dependency::new("a", VersionConstraint::Any),
        Dependency::new("b", VersionConstraint::Any),
    ];

    let resolution = resolver.resolve(&deps).unwrap();

    let c_pkg = resolution.packages.iter().find(|p| p.name == "c").unwrap();
    assert_eq!(c_pkg.version, PackedVersion::new(1, 5, 0),
        "Should select 1.5.0 (highest in [1.0.0, 2.0.0))");
}
