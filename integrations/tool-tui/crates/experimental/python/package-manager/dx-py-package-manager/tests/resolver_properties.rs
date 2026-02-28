//! Property-based tests for the dependency resolver
//!
//! Property 4: Dependency Resolution Determinism
//! For any set of package requirements, running resolution multiple times
//! SHALL produce identical solutions.
//!
//! Property 9: Resolution Hint Cache Correctness
//! For any set of dependencies, if a cached resolution exists and is valid,
//! using the cached resolution SHALL produce the same installed package set
//! as performing a fresh resolution.

#![allow(dead_code)]

use proptest::prelude::*;
use std::collections::HashSet;

use dx_py_core::version::PackedVersion;
use dx_py_package_manager::resolver::{
    Dependency, HintCache, InMemoryProvider, PubGrubResolver, Resolution, ResolvedPackage,
    Resolver, VersionConstraint,
};

/// Generate arbitrary package names
fn arb_package_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_]{2,15}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// Generate arbitrary version
fn arb_version() -> impl Strategy<Value = (u32, u32, u32)> {
    (0u32..100, 0u32..100, 0u32..100)
}

/// Generate arbitrary version constraint
fn arb_constraint() -> impl Strategy<Value = VersionConstraint> {
    prop_oneof![
        Just(VersionConstraint::Any),
        arb_version().prop_map(|(major, minor, patch)| {
            VersionConstraint::Gte(PackedVersion::new(major, minor, patch))
        }),
        arb_version().prop_map(|(major, minor, patch)| {
            VersionConstraint::Exact(PackedVersion::new(major, minor, patch))
        }),
    ]
}

/// Generate a simple package registry
fn arb_registry() -> impl Strategy<Value = InMemoryProvider> {
    prop::collection::vec((arb_package_name(), prop::collection::vec(arb_version(), 1..5)), 1..10)
        .prop_map(|packages| {
            let mut provider = InMemoryProvider::new();
            for (name, versions) in packages {
                for (major, minor, patch) in versions {
                    let version_str = format!("{}.{}.{}", major, minor, patch);
                    provider.add_package(&name, &version_str, vec![]);
                }
            }
            provider
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 4: Dependency Resolution Determinism
    /// Validates: Requirements 2.1.8
    ///
    /// For any set of package requirements, running resolution multiple times
    /// SHALL produce identical solutions.
    #[test]
    fn prop_resolution_determinism(
        seed in any::<u64>(),
    ) {
        // Create a deterministic provider based on seed
        let mut provider = InMemoryProvider::new();

        // Add packages with deterministic versions based on seed
        let pkg_count = ((seed % 5) + 2) as usize;
        let mut pkg_names = Vec::new();

        for i in 0..pkg_count {
            let name = format!("pkg_{}", i);
            pkg_names.push(name.clone());

            // Add 2-4 versions per package
            let version_count = ((seed >> (i * 4)) % 3) + 2;
            for v in 0..version_count {
                let version_str = format!("{}.0.0", v + 1);
                provider.add_package(&name, &version_str, vec![]);
            }
        }

        // Create dependencies for first few packages
        let deps: Vec<Dependency> = pkg_names.iter().take(2).map(|name| {
            Dependency::new(name, VersionConstraint::Any)
        }).collect();

        // Resolve multiple times with PubGrub resolver
        let mut resolver1 = PubGrubResolver::new(provider.clone());
        let mut resolver2 = PubGrubResolver::new(provider.clone());
        let mut resolver3 = PubGrubResolver::new(provider);

        let res1 = resolver1.resolve(&deps);
        let res2 = resolver2.resolve(&deps);
        let res3 = resolver3.resolve(&deps);

        // All resolutions should succeed or fail consistently
        match (&res1, &res2, &res3) {
            (Ok(r1), Ok(r2), Ok(r3)) => {
                // Extract package names and versions
                let set1: HashSet<_> = r1.packages.iter()
                    .map(|p| (&p.name, p.version))
                    .collect();
                let set2: HashSet<_> = r2.packages.iter()
                    .map(|p| (&p.name, p.version))
                    .collect();
                let set3: HashSet<_> = r3.packages.iter()
                    .map(|p| (&p.name, p.version))
                    .collect();

                prop_assert_eq!(&set1, &set2, "Resolution 1 and 2 differ");
                prop_assert_eq!(&set2, &set3, "Resolution 2 and 3 differ");
            }
            (Err(_), Err(_), Err(_)) => {
                // All failed - that's consistent
            }
            _ => {
                prop_assert!(false, "Inconsistent resolution results");
            }
        }
    }

    /// Property 5: Dependency Resolution Correctness
    /// Validates: Requirements 2.1.1, 2.1.3-2.1.7
    ///
    /// For any dependency graph with a valid solution, the resolver SHALL find
    /// a solution where all version constraints are satisfied.
    #[test]
    fn prop_resolution_correctness(
        seed in any::<u64>(),
    ) {
        // Create a provider with packages that have a valid solution
        let mut provider = InMemoryProvider::new();

        // Create a simple dependency chain: a -> b -> c
        // All with compatible versions
        let a_version = ((seed % 3) + 1) as u32;
        let b_version = ((seed >> 4) % 3 + 1) as u32;
        let c_version = ((seed >> 8) % 3 + 1) as u32;

        // Package c has no dependencies
        provider.add_package("c", &format!("{}.0.0", c_version), vec![]);
        provider.add_package("c", &format!("{}.0.0", c_version + 1), vec![]);

        // Package b depends on c with a range constraint
        provider.add_package(
            "b",
            &format!("{}.0.0", b_version),
            vec![Dependency::new(
                "c",
                VersionConstraint::Gte(PackedVersion::new(c_version, 0, 0)),
            )],
        );

        // Package a depends on b with a range constraint
        provider.add_package(
            "a",
            &format!("{}.0.0", a_version),
            vec![Dependency::new(
                "b",
                VersionConstraint::Gte(PackedVersion::new(b_version, 0, 0)),
            )],
        );

        // Request package a
        let deps = vec![Dependency::new("a", VersionConstraint::Any)];

        let mut resolver = PubGrubResolver::new(provider.clone());
        let result = resolver.resolve(&deps);

        // Resolution should succeed
        prop_assert!(result.is_ok(), "Resolution should succeed for valid dependency graph");

        let resolution = result.unwrap();

        // All requested packages should be in the solution
        let resolved_names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
        prop_assert!(resolved_names.contains("a"), "Package 'a' should be resolved");
        prop_assert!(resolved_names.contains("b"), "Package 'b' should be resolved");
        prop_assert!(resolved_names.contains("c"), "Package 'c' should be resolved");

        // Verify version constraints are satisfied
        let resolved_map: std::collections::HashMap<_, _> = resolution.packages.iter()
            .map(|p| (p.name.as_str(), &p.version))
            .collect();

        // b's version should satisfy a's constraint
        let b_resolved = resolved_map.get("b").unwrap();
        prop_assert!(
            **b_resolved >= PackedVersion::new(b_version, 0, 0),
            "b's version should satisfy a's constraint"
        );

        // c's version should satisfy b's constraint
        let c_resolved = resolved_map.get("c").unwrap();
        prop_assert!(
            **c_resolved >= PackedVersion::new(c_version, 0, 0),
            "c's version should satisfy b's constraint"
        );
    }

    /// Property 9: Resolution Hint Cache Correctness
    /// Validates: Requirements 7.1, 7.3
    ///
    /// For any set of dependencies, if a cached resolution exists and is valid,
    /// using the cached resolution SHALL produce the same installed package set
    /// as performing a fresh resolution.
    #[test]
    fn prop_hint_cache_correctness(
        _provider in arb_registry(),
        _seed in any::<u64>(),
    ) {
        // Create a simple scenario with known packages
        let mut test_provider = InMemoryProvider::new();
        test_provider.add_package("pkg_a", "1.0.0", vec![]);
        test_provider.add_package("pkg_a", "2.0.0", vec![]);
        test_provider.add_package("pkg_b", "1.0.0", vec![]);
        test_provider.add_package("pkg_b", "1.5.0", vec![]);
        test_provider.add_package("pkg_c", "3.0.0", vec![]);

        let deps = vec![
            Dependency::new("pkg_a", VersionConstraint::Any),
            Dependency::new("pkg_b", VersionConstraint::Gte(PackedVersion::new(1, 0, 0))),
        ];

        // First resolution (no cache)
        let mut resolver1 = Resolver::new(test_provider.clone());
        let res1 = resolver1.resolve(&deps).unwrap();
        prop_assert!(!res1.from_cache);

        // Second resolution (should use cache)
        let res2 = resolver1.resolve(&deps).unwrap();
        prop_assert!(res2.from_cache);

        // Both resolutions should produce the same packages
        let names1: HashSet<_> = res1.packages.iter().map(|p| &p.name).collect();
        let names2: HashSet<_> = res2.packages.iter().map(|p| &p.name).collect();
        prop_assert_eq!(names1, names2);

        // Versions should also match
        for pkg1 in &res1.packages {
            let pkg2 = res2.packages.iter().find(|p| p.name == pkg1.name).unwrap();
            prop_assert_eq!(pkg1.version, pkg2.version);
        }
    }

    /// Property: Cache lookup returns None for unknown hashes
    #[test]
    fn prop_cache_miss_for_unknown(hash in any::<u64>()) {
        let cache = HintCache::new();
        prop_assert!(cache.lookup(hash).is_none());
    }

    /// Property: Cache stores and retrieves correctly
    #[test]
    fn prop_cache_store_retrieve(
        hash in any::<u64>(),
        pkg_name in arb_package_name(),
        version in arb_version(),
    ) {
        let mut cache = HintCache::new();

        let packages = vec![ResolvedPackage::new(
            &pkg_name,
            PackedVersion::new(version.0, version.1, version.2),
            &format!("{}.{}.{}", version.0, version.1, version.2),
        )];
        let resolution = Resolution::new(packages.clone(), 10);

        cache.store(hash, &resolution);

        let cached = cache.lookup(hash);
        prop_assert!(cached.is_some());

        let cached = cached.unwrap();
        prop_assert_eq!(cached.packages.len(), 1);
        prop_assert_eq!(&cached.packages[0].name, &pkg_name);
    }

    /// Property: Version constraint satisfaction is consistent
    #[test]
    fn prop_constraint_satisfaction_consistent(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100,
    ) {
        let version = PackedVersion::new(major, minor, patch);

        // Any always satisfies
        prop_assert!(VersionConstraint::Any.satisfies(&version));

        // Exact matches only itself
        prop_assert!(VersionConstraint::Exact(version).satisfies(&version));

        // Gte satisfies >= versions
        let lower = PackedVersion::new(
            major.saturating_sub(1),
            minor,
            patch,
        );
        prop_assert!(VersionConstraint::Gte(lower).satisfies(&version));

        // Lt satisfies < versions
        let higher = PackedVersion::new(major + 1, 0, 0);
        prop_assert!(VersionConstraint::Lt(higher).satisfies(&version));
    }

    /// Property: Resolution always picks highest valid version
    #[test]
    fn prop_resolution_picks_highest(
        v1 in 1u32..50,
        v2 in 51u32..100,
    ) {
        let mut provider = InMemoryProvider::new();
        provider.add_package("test_pkg", &format!("{}.0.0", v1), vec![]);
        provider.add_package("test_pkg", &format!("{}.0.0", v2), vec![]);

        let mut resolver = Resolver::new(provider);
        let deps = vec![Dependency::new("test_pkg", VersionConstraint::Any)];

        let resolution = resolver.resolve(&deps).unwrap();
        prop_assert_eq!(resolution.packages.len(), 1);
        prop_assert_eq!(resolution.packages[0].version.major, v2);
    }

    /// Property: Cache eviction maintains correctness
    #[test]
    fn prop_cache_eviction_correctness(
        entries in prop::collection::vec(
            (any::<u64>(), arb_package_name()),
            10..20
        ),
    ) {
        let mut cache = HintCache::with_max_size(5);

        for (hash, name) in &entries {
            let packages = vec![ResolvedPackage::new(
                name,
                PackedVersion::new(1, 0, 0),
                "1.0.0",
            )];
            let resolution = Resolution::new(packages, 10);
            cache.store(*hash, &resolution);
        }

        // Cache should not exceed max size
        prop_assert!(cache.len() <= 5);

        // All entries in cache should be valid
        for (hash, _) in entries.iter().rev().take(5) {
            if let Some(cached) = cache.lookup(*hash) {
                prop_assert!(cached.is_valid());
            }
        }
    }
}

#[test]
fn test_resolver_with_transitive_deps() {
    let mut provider = InMemoryProvider::new();

    // requests depends on urllib3 and certifi
    provider.add_package(
        "requests",
        "2.30.0",
        vec![
            Dependency::new("urllib3", VersionConstraint::Gte(PackedVersion::new(1, 21, 0))),
            Dependency::new("certifi", VersionConstraint::Any),
        ],
    );
    provider.add_package("urllib3", "2.0.0", vec![]);
    provider.add_package("certifi", "2023.5.7", vec![]);

    let mut resolver = Resolver::new(provider);
    let deps = vec![Dependency::new("requests", VersionConstraint::Any)];

    let resolution = resolver.resolve(&deps).unwrap();

    // Should resolve all 3 packages
    assert_eq!(resolution.packages.len(), 3);

    let names: HashSet<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains("requests"));
    assert!(names.contains("urllib3"));
    assert!(names.contains("certifi"));
}

#[test]
fn test_hint_cache_delta_resolution() {
    let mut provider = InMemoryProvider::new();
    provider.add_package("pkg_a", "1.0.0", vec![]);
    provider.add_package("pkg_b", "1.0.0", vec![]);
    provider.add_package("pkg_c", "1.0.0", vec![]);

    let mut resolver = Resolver::new(provider);

    // First resolution with pkg_a and pkg_b
    let deps1 = vec![
        Dependency::new("pkg_a", VersionConstraint::Any),
        Dependency::new("pkg_b", VersionConstraint::Any),
    ];
    let res1 = resolver.resolve(&deps1).unwrap();
    assert!(!res1.from_cache);

    // Second resolution with same deps should use cache
    let res2 = resolver.resolve(&deps1).unwrap();
    assert!(res2.from_cache);
}

// ============================================================================
// Property 11: Dependency Resolution Correctness for PyPI Integration
// Validates: Requirements 14.2, 14.4, 14.5
// ============================================================================

use dx_py_compat::markers::{MarkerEnvironment, MarkerEvaluator};
use dx_py_package_manager::resolver::{
    CircularDependencyDetector, CycleHandling, DependencyGraph, ExtrasResolver, PreReleaseFilter,
    PreReleasePolicy,
};

/// Generate arbitrary marker expressions
fn arb_marker_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("python_version >= '3.8'".to_string()),
        Just("python_version >= '3.9'".to_string()),
        Just("python_version >= '3.10'".to_string()),
        Just("sys_platform == 'win32'".to_string()),
        Just("sys_platform == 'linux'".to_string()),
        Just("sys_platform == 'darwin'".to_string()),
        Just("extra == 'dev'".to_string()),
        Just("extra == 'test'".to_string()),
    ]
}

/// Generate arbitrary pre-release version strings
fn arb_prerelease_version() -> impl Strategy<Value = String> {
    prop_oneof![
        (1u32..10, 0u32..10, 0u32..10)
            .prop_map(|(maj, min, pat)| format!("{}.{}.{}", maj, min, pat)),
        (1u32..10, 0u32..10, 0u32..10, 1u32..5)
            .prop_map(|(maj, min, pat, pre)| format!("{}.{}.{}a{}", maj, min, pat, pre)),
        (1u32..10, 0u32..10, 0u32..10, 1u32..5)
            .prop_map(|(maj, min, pat, pre)| format!("{}.{}.{}b{}", maj, min, pat, pre)),
        (1u32..10, 0u32..10, 0u32..10, 1u32..5)
            .prop_map(|(maj, min, pat, pre)| format!("{}.{}.{}rc{}", maj, min, pat, pre)),
        (1u32..10, 0u32..10, 0u32..10, 1u32..5)
            .prop_map(|(maj, min, pat, pre)| format!("{}.{}.{}.dev{}", maj, min, pat, pre)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11a: Environment Marker Evaluation Consistency
    /// Validates: Requirements 14.2
    ///
    /// For any environment marker expression, evaluating it multiple times
    /// with the same environment SHALL produce identical results.
    #[test]
    fn prop_marker_evaluation_consistency(
        marker in arb_marker_expr(),
    ) {
        let env = MarkerEnvironment::current();
        let extras: Vec<String> = vec![];

        // Evaluate multiple times
        let result1 = MarkerEvaluator::evaluate(&marker, &env, &extras);
        let result2 = MarkerEvaluator::evaluate(&marker, &env, &extras);
        let result3 = MarkerEvaluator::evaluate(&marker, &env, &extras);

        // All evaluations should produce the same result
        prop_assert_eq!(result1, result2, "Marker evaluation inconsistent between 1 and 2");
        prop_assert_eq!(result2, result3, "Marker evaluation inconsistent between 2 and 3");
    }

    /// Property 11b: Pre-release Version Filtering Correctness
    /// Validates: Requirements 14.4
    ///
    /// For any set of versions, filtering with PreReleasePolicy::Never SHALL
    /// exclude all pre-release versions, and PreReleasePolicy::Always SHALL
    /// include all versions.
    #[test]
    fn prop_prerelease_filtering_correctness(
        versions in prop::collection::vec(arb_prerelease_version(), 1..10),
    ) {
        let packed_versions: Vec<(PackedVersion, String)> = versions
            .iter()
            .filter_map(|v| {
                // Parse version to get major.minor.patch
                let parts: Vec<&str> = v.split('.').collect();
                if parts.len() >= 3 {
                    let major: u32 = parts[0].parse().unwrap_or(0);
                    let minor: u32 = parts[1].parse().unwrap_or(0);
                    let patch_str = parts[2];
                    let patch: u32 = patch_str.chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0);
                    Some((PackedVersion::new(major, minor, patch), v.clone()))
                } else {
                    None
                }
            })
            .collect();

        if packed_versions.is_empty() {
            return Ok(());
        }

        // Test Never policy
        let never_filter = PreReleaseFilter::new(PreReleasePolicy::Never);
        let never_filtered = never_filter.filter_versions("test", &packed_versions, None);

        // All filtered versions should be stable (not pre-release)
        for (_, v) in &never_filtered {
            prop_assert!(
                !PreReleaseFilter::is_prerelease_version(v),
                "Never policy should exclude pre-release: {}", v
            );
        }

        // Test Always policy
        let always_filter = PreReleaseFilter::new(PreReleasePolicy::Always);
        let always_filtered = always_filter.filter_versions("test", &packed_versions, None);

        // All versions should be included
        prop_assert_eq!(
            always_filtered.len(),
            packed_versions.len(),
            "Always policy should include all versions"
        );
    }

    /// Property 11c: Circular Dependency Detection Correctness
    /// Validates: Requirements 14.5
    ///
    /// For any dependency graph with a cycle, the detector SHALL identify
    /// the cycle. For any acyclic graph, no cycles SHALL be reported.
    #[test]
    fn prop_circular_dependency_detection(
        seed in any::<u64>(),
    ) {
        // Create a graph based on seed
        let mut graph = DependencyGraph::new();
        let pkg_count = ((seed % 5) + 3) as usize;

        // Add packages
        let pkg_names: Vec<String> = (0..pkg_count).map(|i| format!("pkg_{}", i)).collect();

        // Add edges based on seed bits
        let has_cycle = (seed >> 32) % 2 == 1;

        for i in 0..pkg_count - 1 {
            graph.add_edge(&pkg_names[i], &pkg_names[i + 1]);
        }

        if has_cycle {
            // Add edge from last to first to create cycle
            graph.add_edge(&pkg_names[pkg_count - 1], &pkg_names[0]);
        }

        // Detect cycles
        let cycles = graph.find_all_cycles();
        let topo_sort = graph.topological_sort();

        if has_cycle {
            // Should detect cycle
            prop_assert!(!cycles.is_empty() || topo_sort.is_none(),
                "Cycle should be detected when graph has cycle");
        } else {
            // Should not detect cycle
            prop_assert!(cycles.is_empty() && topo_sort.is_some(),
                "No cycle should be detected in acyclic graph");
        }
    }

    /// Property 11d: Extras Resolution Consistency
    /// Validates: Requirements 14.1 (via extras handling)
    ///
    /// For any package with extras, parsing and resolving extras SHALL
    /// produce consistent results.
    #[test]
    fn prop_extras_parsing_consistency(
        pkg_name in arb_package_name(),
        extras in prop::collection::vec("[a-z]{3,8}", 0..3),
    ) {
        // Build extras string
        let extras_str = if extras.is_empty() {
            pkg_name.clone()
        } else {
            format!("{}[{}]", pkg_name, extras.join(","))
        };

        // Parse multiple times
        let result1 = ExtrasResolver::parse_extras(&extras_str);
        let result2 = ExtrasResolver::parse_extras(&extras_str);

        prop_assert!(result1.is_ok(), "Parsing should succeed");
        prop_assert!(result2.is_ok(), "Parsing should succeed");

        let (name1, extras1) = result1.unwrap();
        let (name2, extras2) = result2.unwrap();

        prop_assert_eq!(name1, name2, "Package names should match");
        prop_assert_eq!(&extras1, &extras2, "Extras should match");
        prop_assert_eq!(extras1.len(), extras.len(), "Extras count should match");
    }

    /// Property 11e: Cycle Detector State Consistency
    /// Validates: Requirements 14.5
    ///
    /// For any sequence of enter/leave operations, the detector SHALL
    /// maintain consistent state.
    #[test]
    fn prop_cycle_detector_state_consistency(
        operations in prop::collection::vec(
            (arb_package_name(), prop::bool::ANY),
            1..20
        ),
    ) {
        let mut detector = CircularDependencyDetector::new(CycleHandling::Break);
        let mut expected_path: Vec<String> = Vec::new();

        for (pkg, is_enter) in operations {
            if is_enter {
                let result = detector.enter(&pkg);
                if result.is_ok() && result.unwrap() {
                    expected_path.push(pkg.to_lowercase());
                }
            } else if !expected_path.is_empty() {
                let pkg_to_leave = expected_path.last().unwrap().clone();
                detector.leave(&pkg_to_leave);
                expected_path.pop();
            }
        }

        // Path should match expected
        prop_assert_eq!(
            detector.current_path().len(),
            expected_path.len(),
            "Path length should match expected"
        );
    }

    /// Property 11f: Pre-release Version Ordering
    /// Validates: Requirements 14.4
    ///
    /// For any set of versions, sorting SHALL place pre-releases before
    /// their corresponding releases.
    #[test]
    fn prop_prerelease_ordering(
        major in 1u32..10,
        minor in 0u32..10,
        patch in 0u32..10,
    ) {
        let base = format!("{}.{}.{}", major, minor, patch);
        let alpha = format!("{}.{}.{}a1", major, minor, patch);
        let beta = format!("{}.{}.{}b1", major, minor, patch);
        let rc = format!("{}.{}.{}rc1", major, minor, patch);

        let mut versions = vec![
            (PackedVersion::new(major, minor, patch), base.clone()),
            (PackedVersion::new(major, minor, patch), alpha.clone()),
            (PackedVersion::new(major, minor, patch), beta.clone()),
            (PackedVersion::new(major, minor, patch), rc.clone()),
        ];

        PreReleaseFilter::sort_versions(&mut versions);

        // Order should be: alpha < beta < rc < release
        let version_strs: Vec<&str> = versions.iter().map(|(_, v)| v.as_str()).collect();

        let alpha_pos = version_strs.iter().position(|v| v.contains("a1"));
        let beta_pos = version_strs.iter().position(|v| v.contains("b1"));
        let rc_pos = version_strs.iter().position(|v| v.contains("rc1"));
        let release_pos = version_strs.iter().position(|v| !v.contains('a') && !v.contains('b') && !v.contains("rc"));

        if let (Some(a), Some(b), Some(r), Some(rel)) = (alpha_pos, beta_pos, rc_pos, release_pos) {
            prop_assert!(a < b, "Alpha should come before beta");
            prop_assert!(b < r, "Beta should come before rc");
            prop_assert!(r < rel, "RC should come before release");
        }
    }
}

// Additional unit tests for edge cases

#[test]
fn test_marker_with_extras() {
    let env = MarkerEnvironment::current();

    // Without extras
    assert!(!MarkerEvaluator::evaluate("extra == 'dev'", &env, &[]));

    // With matching extra
    assert!(MarkerEvaluator::evaluate("extra == 'dev'", &env, &["dev".to_string()]));

    // With non-matching extra
    assert!(!MarkerEvaluator::evaluate("extra == 'dev'", &env, &["test".to_string()]));
}

#[test]
fn test_prerelease_constraint_detection() {
    // Constraints that request pre-releases
    assert!(PreReleaseFilter::constraint_requests_prerelease(">=1.0.0a1"));
    assert!(PreReleaseFilter::constraint_requests_prerelease("==2.0.0b2"));
    assert!(PreReleaseFilter::constraint_requests_prerelease(">=1.0.0.dev1"));

    // Constraints that don't request pre-releases
    assert!(!PreReleaseFilter::constraint_requests_prerelease(">=1.0.0"));
    assert!(!PreReleaseFilter::constraint_requests_prerelease("==2.0.0"));
}

#[test]
fn test_circular_dependency_error_mode() {
    let mut detector = CircularDependencyDetector::new(CycleHandling::Error);

    assert!(detector.enter("a").unwrap());
    assert!(detector.enter("b").unwrap());

    // Creating a cycle should error
    let result = detector.enter("a");
    assert!(result.is_err());
}

#[test]
fn test_dependency_graph_scc() {
    let mut graph = DependencyGraph::new();

    // Create two SCCs: {a, b, c} and {d}
    graph.add_edge("a", "b");
    graph.add_edge("b", "c");
    graph.add_edge("c", "a");
    graph.add_edge("c", "d");

    let sccs = graph.strongly_connected_components();

    // Should have at least 2 SCCs
    assert!(sccs.len() >= 2);

    // One SCC should have 3 nodes
    let large_scc = sccs.iter().find(|scc| scc.len() == 3);
    assert!(large_scc.is_some());
}

#[test]
fn test_extras_available_detection() {
    use dx_py_package_manager::registry::DependencySpec;

    let deps = vec![
        DependencySpec {
            name: "pyopenssl".to_string(),
            version_constraint: Some(">=0.14".to_string()),
            extras: vec![],
            markers: Some("extra == 'security'".to_string()),
            url: None,
            path: None,
        },
        DependencySpec {
            name: "pysocks".to_string(),
            version_constraint: Some(">=1.5.6".to_string()),
            extras: vec![],
            markers: Some("extra == 'socks'".to_string()),
            url: None,
            path: None,
        },
    ];

    let available = ExtrasResolver::get_available_extras(&deps);
    assert!(available.contains("security"));
    assert!(available.contains("socks"));
}
