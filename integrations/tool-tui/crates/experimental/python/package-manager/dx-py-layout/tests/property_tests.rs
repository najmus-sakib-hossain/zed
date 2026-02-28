//! Property-based tests for Layout Cache
//!
//! These tests validate the correctness properties defined in the design document.

use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use tempfile::TempDir;

use dx_py_layout::{LayoutCache, LayoutEntry, LayoutIndex, LayoutIndexHeader, DXLC_MAGIC};
use dx_py_store::PackageStore;

/// Generator for valid package names (PEP 503 normalized)
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,30}".prop_map(|s| s.to_lowercase())
}

/// Generator for valid versions (PEP 440)
fn arb_version() -> impl Strategy<Value = String> {
    (0u16..100, 0u16..100, 0u16..1000)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generator for package hashes
fn arb_hash() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

/// Test package for property tests
#[derive(Debug, Clone)]
struct TestPackage {
    name: String,
    version: String,
    hash: [u8; 32],
}

/// Generator for test packages
fn arb_package() -> impl Strategy<Value = TestPackage> {
    (arb_package_name(), arb_version(), arb_hash()).prop_map(|(name, version, hash)| TestPackage {
        name,
        version,
        hash,
    })
}

/// Generator for package lists with unique names
fn arb_package_list(
    size: impl Into<prop::collection::SizeRange>,
) -> impl Strategy<Value = Vec<TestPackage>> {
    prop::collection::vec(arb_package(), size).prop_map(|packages| {
        // Deduplicate by name
        let mut seen = HashSet::new();
        packages.into_iter().filter(|p| seen.insert(p.name.clone())).collect()
    })
}

/// Convert test packages to ResolvedPackage
fn to_resolved_packages(packages: &[TestPackage]) -> Vec<dx_py_layout::ResolvedPackage> {
    packages
        .iter()
        .map(|p| dx_py_layout::ResolvedPackage {
            name: p.name.clone(),
            version: p.version.clone(),
            hash: p.hash,
        })
        .collect()
}

fn create_test_store(temp: &TempDir) -> Arc<PackageStore> {
    Arc::new(PackageStore::open(temp.path().join("store")).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Layout Cache Hash Determinism
    ///
    /// For any set of resolved packages, computing the project hash multiple times
    /// SHALL always produce the same hash value, and different package sets SHALL
    /// produce different hashes.
    #[test]
    fn prop_layout_cache_hash_determinism(packages in arb_package_list(1..20)) {
        // Feature: dx-py-performance-phase1, Property 1: Layout Cache Hash Determinism
        let resolved = to_resolved_packages(&packages);

        // Computing hash multiple times should produce same result
        let hash1 = LayoutCache::compute_project_hash(&resolved);
        let hash2 = LayoutCache::compute_project_hash(&resolved);
        let hash3 = LayoutCache::compute_project_hash(&resolved);

        prop_assert_eq!(hash1, hash2, "Hash should be deterministic");
        prop_assert_eq!(hash2, hash3, "Hash should be deterministic");
    }

    /// Property 1 (continued): Different package sets produce different hashes
    #[test]
    fn prop_layout_cache_hash_uniqueness(
        packages1 in arb_package_list(1..10),
        packages2 in arb_package_list(1..10)
    ) {
        // Feature: dx-py-performance-phase1, Property 1: Layout Cache Hash Determinism
        let resolved1 = to_resolved_packages(&packages1);
        let resolved2 = to_resolved_packages(&packages2);

        // Skip if packages are identical
        if packages1.len() != packages2.len() ||
           !packages1.iter().zip(packages2.iter()).all(|(a, b)| a.name == b.name && a.version == b.version && a.hash == b.hash) {
            let hash1 = LayoutCache::compute_project_hash(&resolved1);
            let hash2 = LayoutCache::compute_project_hash(&resolved2);

            prop_assert_ne!(hash1, hash2, "Different package sets should produce different hashes");
        }
    }

    /// Property 1 (continued): Order independence
    #[test]
    fn prop_layout_cache_hash_order_independent(packages in arb_package_list(2..10)) {
        // Feature: dx-py-performance-phase1, Property 1: Layout Cache Hash Determinism
        let resolved = to_resolved_packages(&packages);

        // Reverse the order
        let mut reversed = resolved.clone();
        reversed.reverse();

        let hash1 = LayoutCache::compute_project_hash(&resolved);
        let hash2 = LayoutCache::compute_project_hash(&reversed);

        prop_assert_eq!(hash1, hash2, "Hash should be order-independent");
    }

    /// Property 2: Layout Cache Cold-to-Warm Transition
    ///
    /// For any valid package set, after building and caching a layout,
    /// subsequent lookups with the same project hash SHALL find the cached layout.
    #[test]
    fn prop_layout_cache_cold_to_warm(packages in arb_package_list(1..5)) {
        // Feature: dx-py-performance-phase1, Property 2: Layout Cache Cold-to-Warm Transition
        let temp = TempDir::new().unwrap();
        let store = create_test_store(&temp);

        // Store packages in the store first
        for pkg in &packages {
            let files = [
                (format!("{}/__init__.py", pkg.name), b"# init".to_vec()),
            ];
            let file_refs: Vec<(&str, &[u8])> = files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
            store.store_package(&pkg.hash, &file_refs).unwrap();
        }

        let mut cache = LayoutCache::open(temp.path().join("layouts"), store).unwrap();
        let resolved = to_resolved_packages(&packages);
        let project_hash = LayoutCache::compute_project_hash(&resolved);

        // Cold: layout should not exist
        prop_assert!(!cache.contains(&project_hash), "Layout should not exist initially");

        // Build layout
        cache.build_layout(&project_hash, &resolved).unwrap();

        // Warm: layout should now exist
        prop_assert!(cache.contains(&project_hash), "Layout should exist after building");

        // Lookup should return the entry
        let entry = cache.get(&project_hash);
        prop_assert!(entry.is_some(), "Lookup should find the cached layout");
    }

    /// Property 3: Layout Cache Corruption Recovery
    ///
    /// For any cached layout, if files are corrupted or removed, the Layout_Cache
    /// SHALL detect the corruption during verification and successfully rebuild the layout.
    #[test]
    fn prop_layout_cache_corruption_recovery(packages in arb_package_list(1..3)) {
        // Feature: dx-py-performance-phase1, Property 3: Layout Cache Corruption Recovery
        let temp = TempDir::new().unwrap();
        let store = create_test_store(&temp);

        // Store packages
        for pkg in &packages {
            let files = [
                (format!("{}/__init__.py", pkg.name), b"# init".to_vec()),
            ];
            let file_refs: Vec<(&str, &[u8])> = files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
            store.store_package(&pkg.hash, &file_refs).unwrap();
        }

        let mut cache = LayoutCache::open(temp.path().join("layouts"), store).unwrap();
        let resolved = to_resolved_packages(&packages);
        let project_hash = LayoutCache::compute_project_hash(&resolved);

        // Build layout
        let layout_path = cache.build_layout(&project_hash, &resolved).unwrap();

        // Verify it's valid
        prop_assert!(cache.verify_layout(&project_hash).unwrap(), "Layout should be valid initially");

        // Corrupt the layout by removing site-packages
        let site_packages = layout_path.join("site-packages");
        if site_packages.exists() {
            std::fs::remove_dir_all(&site_packages).unwrap();
        }

        // Verification should fail
        prop_assert!(!cache.verify_layout(&project_hash).unwrap(), "Layout should be invalid after corruption");

        // Rebuild should succeed
        cache.rebuild_layout(&project_hash, &resolved).unwrap();

        // Verification should pass again
        prop_assert!(cache.verify_layout(&project_hash).unwrap(), "Layout should be valid after rebuild");
    }
}

// Additional unit tests for edge cases

#[test]
fn test_layout_index_header_size() {
    assert_eq!(std::mem::size_of::<LayoutIndexHeader>(), 64);
}

#[test]
fn test_layout_entry_size() {
    assert_eq!(std::mem::size_of::<LayoutEntry>(), 128);
}

#[test]
fn test_layout_index_magic() {
    let temp = TempDir::new().unwrap();
    let index_path = temp.path().join("layouts.dxc");

    let index = LayoutIndex::open(&index_path).unwrap();
    drop(index);

    // Read the file and verify magic
    let data = std::fs::read(&index_path).unwrap();
    assert_eq!(&data[0..4], DXLC_MAGIC);
}

#[test]
fn test_empty_package_list_hash() {
    let packages: Vec<dx_py_layout::ResolvedPackage> = vec![];
    let hash = LayoutCache::compute_project_hash(&packages);

    // Empty list should still produce a valid hash
    assert_ne!(hash, [0u8; 32]);
}

#[test]
fn test_layout_cache_open_creates_directory() {
    let temp = TempDir::new().unwrap();
    let store = create_test_store(&temp);
    let layouts_path = temp.path().join("layouts");

    assert!(!layouts_path.exists());

    let _cache = LayoutCache::open(&layouts_path, store).unwrap();

    assert!(layouts_path.exists());
}

#[test]
fn test_build_layout_creates_site_packages() {
    let temp = TempDir::new().unwrap();
    let store = create_test_store(&temp);

    // Store a package
    let hash = [1u8; 32];
    let files = vec![("test/__init__.py", b"# init" as &[u8])];
    store.store_package(&hash, &files).unwrap();

    // Verify package is in store
    assert!(store.contains(&hash), "Package should be in store");

    let mut cache = LayoutCache::open(temp.path().join("layouts"), store).unwrap();
    let packages = vec![dx_py_layout::ResolvedPackage {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        hash,
    }];
    let project_hash = LayoutCache::compute_project_hash(&packages);

    println!("Project hash: {:?}", hex::encode(project_hash));
    println!("Layout count before build: {}", cache.layout_count());

    // Build layout
    let layout_path = cache.build_layout(&project_hash, &packages).unwrap();

    println!("Layout count after build: {}", cache.layout_count());

    // Check site-packages exists
    let site_packages = layout_path.join("site-packages");
    println!("Layout path: {:?}", layout_path);
    println!("Site-packages path: {:?}", site_packages);
    println!("Site-packages exists: {}", site_packages.exists());

    // List contents
    if site_packages.exists() {
        println!("Site-packages contents:");
        for entry in std::fs::read_dir(&site_packages).unwrap() {
            println!("  {:?}", entry.unwrap().path());
        }
    }

    assert!(site_packages.exists(), "site-packages should exist after build_layout");

    // Check if layout is in index using contains
    println!("Layout in index (contains): {}", cache.contains(&project_hash));

    // Check using get
    let entry = cache.get(&project_hash);
    println!("Layout entry from get: {:?}", entry.is_some());

    if let Some(e) = &entry {
        println!("Entry layout_name_str: {:?}", e.layout_name_str());
        let expected_path = cache.root().join(e.layout_name_str());
        println!("Expected path from entry: {:?}", expected_path);
        println!("Expected path exists: {}", expected_path.exists());
        let expected_site_packages = expected_path.join("site-packages");
        println!("Expected site-packages: {:?}", expected_site_packages);
        println!("Expected site-packages exists: {}", expected_site_packages.exists());
    }

    // Verify layout
    let is_valid = cache.verify_layout(&project_hash).unwrap();
    println!("Is valid: {}", is_valid);
    assert!(is_valid, "Layout should be valid after building");
}
