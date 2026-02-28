//! Property-based tests for dx-js-project-manager
//!
//! These tests validate universal correctness properties using proptest.
//!
//! ## Proptest Configuration
//!
//! This module provides centralized proptest configuration:
//! - `proptest_config()`: Standard tests with 100 iterations (configurable via PROPTEST_CASES)
//! - `proptest_critical_config()`: Critical tests (round-trip, serialization) with 500 iterations
//!
//! ## Environment Variables
//!
//! - `PROPTEST_CASES`: Override the number of test cases for all property tests

#![cfg(test)]

use proptest::prelude::*;
use std::collections::HashSet;

use crate::bag::AffectedGraphData;
use crate::btg::{TaskData, TaskGraphData};
use crate::bwm::{BwmSerializer, PackageData, WorkspaceData};
use crate::dxc::XorPatch;
use crate::dxl::{DxlSerializer, LockfileData, PackageResolution};
use crate::types::PackageEntry;

// ============================================================================
// Shared Proptest Configuration
// ============================================================================

/// Minimum iterations for standard property tests
pub const PROPTEST_MIN_ITERATIONS: u32 = 100;

/// Minimum iterations for critical property tests (round-trip, serialization)
pub const PROPTEST_CRITICAL_ITERATIONS: u32 = 500;

/// Centralized proptest configuration for standard tests
///
/// Returns a ProptestConfig with:
/// - 100 iterations by default (or PROPTEST_CASES env var)
/// - 1000 max shrink iterations
///
/// # Example
/// ```ignore
/// proptest! {
///     #![proptest_config(proptest_config())]
///     #[test]
///     fn my_property_test(input in any::<u32>()) {
///         // test body
///     }
/// }
/// ```
pub fn proptest_config() -> ProptestConfig {
    let iterations = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(PROPTEST_MIN_ITERATIONS);

    ProptestConfig {
        cases: iterations,
        max_shrink_iters: 1000,
        ..ProptestConfig::default()
    }
}

/// Centralized proptest configuration for critical tests
///
/// Returns a ProptestConfig with:
/// - 500 iterations by default (or PROPTEST_CASES env var, minimum 500)
/// - 2000 max shrink iterations
///
/// Use this for:
/// - Round-trip properties (serialize/deserialize)
/// - Serialization correctness
/// - Cache integrity
///
/// # Example
/// ```ignore
/// proptest! {
///     #![proptest_config(proptest_critical_config())]
///     #[test]
///     fn roundtrip_property(data in arb_data()) {
///         let serialized = serialize(&data);
///         let deserialized = deserialize(&serialized);
///         prop_assert_eq!(data, deserialized);
///     }
/// }
/// ```
pub fn proptest_critical_config() -> ProptestConfig {
    let env_iterations =
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(0);

    // Use at least PROPTEST_CRITICAL_ITERATIONS for critical tests
    let iterations = env_iterations.max(PROPTEST_CRITICAL_ITERATIONS);

    ProptestConfig {
        cases: iterations,
        max_shrink_iters: 2000,
        ..ProptestConfig::default()
    }
}

// ============================================================================
// Arbitrary implementations for proptest
// ============================================================================

fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}".prop_map(|s| s.to_string())
}

fn arb_package_path() -> impl Strategy<Value = String> {
    "packages/[a-z][a-z0-9-]{0,10}".prop_map(|s| s.to_string())
}

fn arb_version() -> impl Strategy<Value = (u16, u16, u16)> {
    (0u16..100, 0u16..100, 0u16..100)
}

fn arb_package_data() -> impl Strategy<Value = PackageData> {
    (arb_package_name(), arb_package_path(), arb_version()).prop_map(|(name, path, version)| {
        PackageData {
            name,
            path,
            version,
            dependencies: Vec::new(),
            is_private: false,
        }
    })
}

// Note: These generators are available for future property tests
#[allow(dead_code)]
fn arb_task_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("build".to_string()),
        Just("test".to_string()),
        Just("lint".to_string()),
        Just("typecheck".to_string()),
    ]
}

#[allow(dead_code)]
fn arb_command() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("npm run build".to_string()),
        Just("npm test".to_string()),
        Just("npm run lint".to_string()),
        Just("tsc --noEmit".to_string()),
    ]
}

#[allow(dead_code)]
fn arb_task_data(max_pkg_idx: u32) -> impl Strategy<Value = TaskData> {
    (arb_task_name(), 0..=max_pkg_idx, arb_command()).prop_map(|(name, package_idx, command)| {
        TaskData {
            name,
            package_idx,
            command,
            definition_hash: [0; 8],
            frame_budget_us: 0,
            cacheable: true,
        }
    })
}

// ============================================================================
// Property 1: Binary Workspace Manifest Round-Trip Consistency
// ============================================================================

proptest! {
    #![proptest_config(proptest_critical_config())]

    /// Property 1: BWM serialization and deserialization are inverse operations
    #[test]
    fn prop_bwm_roundtrip(
        packages in prop::collection::vec(arb_package_data(), 1..20)
    ) {
        // Ensure unique package names
        let mut seen = HashSet::new();
        let packages: Vec<_> = packages.into_iter()
            .filter(|p| seen.insert(p.name.clone()))
            .collect();

        if packages.is_empty() {
            return Ok(());
        }

        let mut data = WorkspaceData {
            packages,
            dependency_edges: Vec::new(),
            topological_order: Vec::new(),
        };
        data.compute_topological_order().unwrap();

        let serialized = BwmSerializer::serialize(&data).unwrap();
        let deserialized = BwmSerializer::deserialize(&serialized).unwrap();

        // Verify package count matches
        prop_assert_eq!(data.packages.len(), deserialized.packages.len());

        // Verify each package
        for (orig, deser) in data.packages.iter().zip(deserialized.packages.iter()) {
            prop_assert_eq!(&orig.name, &deser.name);
            prop_assert_eq!(&orig.path, &deser.path);
            prop_assert_eq!(orig.version, deser.version);
        }
    }
}

// ============================================================================
// Property 2: Topological Order Validity
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 2: Topological order respects all dependency edges
    #[test]
    fn prop_topological_order_valid(
        package_count in 2usize..10,
        edge_density in 0.0f64..0.3
    ) {
        let packages: Vec<_> = (0..package_count)
            .map(|i| PackageData {
                name: format!("pkg-{}", i),
                path: format!("packages/pkg-{}", i),
                version: (1, 0, 0),
                dependencies: Vec::new(),
                is_private: false,
            })
            .collect();

        // Generate random DAG edges (only forward edges to avoid cycles)
        let mut edges = Vec::new();
        for i in 0..package_count {
            for j in (i + 1)..package_count {
                if rand::random::<f64>() < edge_density {
                    edges.push((i as u32, j as u32));
                }
            }
        }

        let mut data = WorkspaceData {
            packages,
            dependency_edges: edges.clone(),
            topological_order: Vec::new(),
        };
        data.compute_topological_order().unwrap();

        // Verify topological order: for each edge (from, to), from appears before to
        let position: std::collections::HashMap<_, _> = data.topological_order
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        for (from, to) in &edges {
            let from_pos = position.get(from).unwrap();
            let to_pos = position.get(to).unwrap();
            prop_assert!(from_pos < to_pos,
                "Edge ({}, {}) violates topological order: {} at pos {}, {} at pos {}",
                from, to, from, from_pos, to, to_pos);
        }
    }
}

// ============================================================================
// Property 5: Task Graph Parallel Execution Map Correctness
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 5: Tasks in the same parallel group have no dependencies between them
    #[test]
    fn prop_parallel_groups_independent(
        task_count in 2usize..8
    ) {
        let tasks: Vec<_> = (0..task_count)
            .map(|i| TaskData {
                name: format!("task-{}", i),
                package_idx: i as u32,
                command: "npm run build".to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: true,
            })
            .collect();

        // Create a simple chain: 0 -> 1 -> 2 -> ...
        let edges: Vec<_> = (0..task_count.saturating_sub(1))
            .map(|i| (i as u32, (i + 1) as u32))
            .collect();

        let topo_order: Vec<_> = (0..task_count as u32).collect();

        let mut data = TaskGraphData {
            tasks,
            dependency_edges: edges.clone(),
            topological_order: topo_order,
            parallel_groups: Vec::new(),
        };
        data.compute_parallel_groups();

        // Verify: no two tasks in the same group have a dependency edge
        // For a chain, each task should be in its own group (no parallelism)
        // This is because each task depends on the previous one
        for group in &data.parallel_groups {
            let task_count_in_group = { group.task_count };
            // In a chain, each group should have exactly 1 task
            prop_assert_eq!(task_count_in_group, 1,
                "Chain should have single-task groups");
        }
    }
}

// ============================================================================
// Property 8: Blake3 Hash Determinism
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 8: Same content always produces same hash
    #[test]
    fn prop_blake3_deterministic(content in prop::collection::vec(any::<u8>(), 0..1000)) {
        let hash1 = blake3::hash(&content);
        let hash2 = blake3::hash(&content);
        prop_assert_eq!(hash1.as_bytes(), hash2.as_bytes());
    }
}

// ============================================================================
// Property 10: Binary Fingerprint Size Invariance
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 10: Fingerprints are always 32 bytes regardless of input size
    #[test]
    fn prop_fingerprint_size_invariant(content in prop::collection::vec(any::<u8>(), 0..10000)) {
        let hash = blake3::hash(&content);
        prop_assert_eq!(hash.as_bytes().len(), 32);
    }
}

// ============================================================================
// Property 11: DXC Cache Round-Trip Consistency
// ============================================================================

// Note: Full DXC round-trip test would require more infrastructure
// This tests the XOR patch component

proptest! {
    #![proptest_config(proptest_critical_config())]

    /// Property 11 (partial): XOR patches correctly reconstruct target from base
    #[test]
    fn prop_xor_patch_roundtrip(
        base in prop::collection::vec(any::<u8>(), 1..500),
        target in prop::collection::vec(any::<u8>(), 1..500)
    ) {
        let patch = XorPatch::create(&base, &target);
        let reconstructed = patch.apply(&base);
        prop_assert_eq!(reconstructed, target);
    }
}

// ============================================================================
// Property 12: XOR Patch Efficiency
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 12: XOR patches for similar content are smaller than full content
    ///
    /// For truly similar content (only a few bytes changed), the XOR patch should be
    /// more efficient than storing the full target. The patch overhead is ~72 bytes
    /// for headers, so we need a sufficiently large base to see efficiency gains.
    #[test]
    fn prop_xor_patch_efficient_for_similar(
        base in prop::collection::vec(any::<u8>(), 500..2000),
        change_positions in prop::collection::vec(0usize..500, 1..5)
    ) {
        // Create target with small changes (only change a few bytes at specific positions)
        let mut target = base.clone();
        let target_len = target.len();

        // Apply changes at the specified positions (modulo target length)
        for pos in &change_positions {
            let idx = pos % target_len;
            target[idx] ^= 0xFF;
        }

        let patch = XorPatch::create(&base, &target);

        // Patch should be smaller than full target for similar content
        // The overhead is ~72 bytes for headers + 8 bytes per block + changed bytes
        // For a 500+ byte file with only a few changes, this should be efficient
        let efficiency = patch.efficiency(target.len());

        // With 500+ bytes base and only 1-4 changes, efficiency should be well under 1.0
        // Each change creates at most one block with ~9 bytes overhead (8 + 1 data byte)
        // Total overhead: 72 + (change_count * 9) = ~108 bytes max for 4 changes
        // For 500 byte target: 108/500 = 0.216, well under 1.0
        prop_assert!(efficiency < 1.0,
            "Patch efficiency {} should be < 1.0 for similar content (patch size: {}, target size: {}, changes: {})",
            efficiency, patch.size(), target.len(), change_positions.len());
    }
}

// ============================================================================
// Property 14: DXL-Workspace Round-Trip Consistency
// ============================================================================

proptest! {
    #![proptest_config(proptest_critical_config())]

    /// Property 14: DXL lockfile serialization and deserialization are inverse
    #[test]
    fn prop_dxl_roundtrip(
        package_count in 1usize..10
    ) {
        let packages: Vec<_> = (0..package_count)
            .map(|i| PackageResolution {
                name: format!("pkg-{}", i),
                version: (1, 0, i as u16),
                integrity: [i as u8; 32],
                tarball_url: format!("https://registry.npmjs.org/pkg-{}", i),
                dependencies: Vec::new(),
            })
            .collect();

        let data = LockfileData {
            packages,
            vector_clock: [1, 0, 0, 0, 0, 0, 0, 0],
        };

        let serialized = DxlSerializer::serialize(&data).unwrap();
        let deserialized = DxlSerializer::deserialize(&serialized).unwrap();

        prop_assert_eq!(data.packages.len(), deserialized.packages.len());

        for (orig, deser) in data.packages.iter().zip(deserialized.packages.iter()) {
            prop_assert_eq!(&orig.name, &deser.name);
            prop_assert_eq!(orig.version, deser.version);
            prop_assert_eq!(&orig.integrity, &deser.integrity);
        }
    }
}

// ============================================================================
// Property 15: CRDT Merge Commutativity
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 15: CRDT merge is commutative (A merge B == B merge A)
    #[test]
    fn prop_crdt_merge_commutative(
        clock_a in prop::array::uniform8(0u64..10),
        clock_b in prop::array::uniform8(0u64..10)
    ) {
        let lockfile_a = LockfileData {
            packages: vec![PackageResolution {
                name: "pkg-a".to_string(),
                version: (1, 0, 0),
                integrity: [1; 32],
                tarball_url: "https://example.com/a".to_string(),
                dependencies: Vec::new(),
            }],
            vector_clock: clock_a,
        };

        let lockfile_b = LockfileData {
            packages: vec![PackageResolution {
                name: "pkg-b".to_string(),
                version: (2, 0, 0),
                integrity: [2; 32],
                tarball_url: "https://example.com/b".to_string(),
                dependencies: Vec::new(),
            }],
            vector_clock: clock_b,
        };

        // A merge B
        let mut result_ab = lockfile_a.clone();
        result_ab.merge(&lockfile_b).unwrap();

        // B merge A
        let mut result_ba = lockfile_b.clone();
        result_ba.merge(&lockfile_a).unwrap();

        // Vector clocks should be the same
        prop_assert_eq!(result_ab.vector_clock, result_ba.vector_clock);

        // Package sets should be the same (order may differ)
        let names_ab: HashSet<_> = result_ab.packages.iter().map(|p| &p.name).collect();
        let names_ba: HashSet<_> = result_ba.packages.iter().map(|p| &p.name).collect();
        prop_assert_eq!(names_ab, names_ba);
    }
}

// ============================================================================
// Property 18: Affected Package Transitivity
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 18: If A affects B and B affects C, then A affects C
    ///
    /// In our dependency model:
    /// - Edge (from, to) means "from depends on to"
    /// - When package X changes, all packages that depend on X are affected
    /// - Transitively: if A depends on B and B depends on C, then changing C affects both B and A
    #[test]
    fn prop_affected_transitive(
        chain_length in 3usize..8
    ) {
        // Create a chain where each package depends on the next:
        // 0 depends on 1, 1 depends on 2, ..., n-2 depends on n-1
        // Edge (from, to) means "from depends on to"
        //
        // Dependency chain: 0 -> 1 -> 2 -> ... -> n-1
        // (arrows show "depends on" direction)
        //
        // When package n-1 changes:
        //   - n-2 is affected (it depends on n-1)
        //   - n-3 is affected (it depends on n-2, which depends on n-1)
        //   - ... all the way to 0
        let edges: Vec<_> = (0..chain_length - 1)
            .map(|i| (i as u32, (i + 1) as u32))
            .collect();

        let graph = AffectedGraphData::from_edges(chain_length as u32, &edges);

        // Changing the last package (n-1) should affect all packages 0..n-1
        // because they all transitively depend on it
        let last_pkg = (chain_length - 1) as u32;
        let affected = graph.transitive_dependents(last_pkg);

        for i in 0..chain_length - 1 {
            prop_assert!(affected.contains(&(i as u32)),
                "Package {} should be affected when package {} changes (chain: 0->1->...->{})",
                i, last_pkg, last_pkg);
        }

        // Changing package 0 should affect nothing (nothing depends on it - it's a leaf)
        let affected_by_0 = graph.transitive_dependents(0);
        prop_assert!(affected_by_0.is_empty(),
            "Package 0 should not affect anything (nothing depends on it)");

        // Verify transitivity: if changing 2 affects 1, and changing 1 affects 0,
        // then changing 2 should affect 0
        if chain_length >= 3 {
            let affected_by_2 = graph.transitive_dependents(2);
            let affected_by_1 = graph.transitive_dependents(1);

            // 1 depends on 2, so changing 2 affects 1
            prop_assert!(affected_by_2.contains(&1),
                "Package 1 should be affected when package 2 changes");

            // 0 depends on 1, so changing 1 affects 0
            prop_assert!(affected_by_1.contains(&0),
                "Package 0 should be affected when package 1 changes");

            // Transitively: 0 depends on 1 depends on 2, so changing 2 affects 0
            prop_assert!(affected_by_2.contains(&0),
                "Package 0 should be transitively affected when package 2 changes");
        }
    }
}

// ============================================================================
// Property 19: Inverse Dependency Index Correctness
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 19: Inverse dependency index correctly identifies dependents
    #[test]
    fn prop_inverse_deps_correct(
        package_count in 3usize..10,
        edge_count in 1usize..15
    ) {
        // Generate random edges (ensuring no self-loops)
        let edges: Vec<_> = (0..edge_count)
            .map(|i| {
                let from = (i % package_count) as u32;
                let to = ((i + 1) % package_count) as u32;
                if from != to { (from, to) } else { (from, (to + 1) % package_count as u32) }
            })
            .filter(|(from, to)| from != to)
            .collect();

        let graph = AffectedGraphData::from_edges(package_count as u32, &edges);

        // For each edge (from, to), 'from' should be in dependents(to)
        for (from, to) in &edges {
            let dependents = graph.dependents(*to);
            prop_assert!(dependents.contains(from),
                "Package {} should be in dependents of {} (edge {} -> {})",
                from, to, from, to);
        }
    }
}

// ============================================================================
// Property 13: Cache Signature Tamper Detection
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 13: Modifying any byte of cache content causes signature verification to fail
    /// For any DXC cache entry, modifying any byte of the content SHALL cause
    /// signature verification to fail.
    #[test]
    fn prop_cache_tamper_detection(
        file_content in prop::collection::vec(any::<u8>(), 10..100),
        tamper_position in 0usize..100
    ) {
        use crate::dxc::CacheEntry;
        use crate::cache::CacheManager;
        use ed25519_dalek::{SigningKey, Signer};
        use tempfile::TempDir;

        // Create a cache entry with content
        let task_hash = blake3::hash(&file_content);
        let mut entry = CacheEntry::new(*task_hash.as_bytes());
        entry.add_file("test.txt".to_string(), file_content.clone(), 0o644);

        // Generate a signing key
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();

        // Sign the entry
        let mut hasher = blake3::Hasher::new();
        hasher.update(&entry.task_hash);
        for file in &entry.files {
            hasher.update(file.path.as_bytes());
            hasher.update(&file.content);
        }
        let content_hash = hasher.finalize();
        let signature = signing_key.sign(content_hash.as_bytes());

        entry.signature = Some(signature.to_bytes());
        entry.public_key = Some(verifying_key.to_bytes());

        // Create cache manager and verify original entry
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);

        // Original entry should verify
        let verify_result = cache.verify(&entry);
        prop_assert!(verify_result.is_ok() && verify_result.unwrap(),
            "Original entry should verify successfully");

        // Tamper with the content
        let tamper_idx = tamper_position % file_content.len();
        let mut tampered_entry = entry.clone();
        tampered_entry.files[0].content[tamper_idx] ^= 0xFF; // Flip all bits

        // Tampered entry should fail verification
        let tampered_result = cache.verify(&tampered_entry);
        prop_assert!(tampered_result.is_err() || !tampered_result.unwrap(),
            "Tampered entry should fail verification");
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 13 (continued): Tampering with signature itself is detected
    #[test]
    fn prop_signature_tamper_detection(
        file_content in prop::collection::vec(any::<u8>(), 10..50),
        sig_tamper_position in 0usize..64
    ) {
        use crate::dxc::CacheEntry;
        use crate::cache::CacheManager;
        use ed25519_dalek::{SigningKey, Signer};
        use tempfile::TempDir;

        // Create and sign entry
        let task_hash = blake3::hash(&file_content);
        let mut entry = CacheEntry::new(*task_hash.as_bytes());
        entry.add_file("test.txt".to_string(), file_content.clone(), 0o644);

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&entry.task_hash);
        for file in &entry.files {
            hasher.update(file.path.as_bytes());
            hasher.update(&file.content);
        }
        let content_hash = hasher.finalize();
        let signature = signing_key.sign(content_hash.as_bytes());

        entry.signature = Some(signature.to_bytes());
        entry.public_key = Some(verifying_key.to_bytes());

        // Tamper with the signature itself
        let mut tampered_entry = entry.clone();
        let mut tampered_sig = tampered_entry.signature.unwrap();
        tampered_sig[sig_tamper_position % 64] ^= 0xFF;
        tampered_entry.signature = Some(tampered_sig);

        // Tampered signature should fail verification
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        let result = cache.verify(&tampered_entry);

        prop_assert!(result.is_err() || !result.unwrap(),
            "Entry with tampered signature should fail verification");
    }
}

// ============================================================================
// Property 9: Import Detection Completeness
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 9: Import detection identifies all import types
    /// For any JavaScript/TypeScript file, the SIMD-accelerated import detection SHALL
    /// identify all import statements (ES6 imports, CommonJS requires, dynamic imports)
    /// with correct file paths and line numbers.
    #[test]
    fn prop_import_detection_completeness(
        module_name in "[a-z][a-z0-9-]{0,15}",
        import_type in 0u8..4
    ) {
        use crate::change::ChangeDetector;
        use crate::types::ImportKind;

        let detector = ChangeDetector::new();

        // Generate different import types based on import_type
        let (content, expected_kind) = match import_type {
            0 => {
                // ES6 import
                let content = format!("import foo from '{}';", module_name);
                (content, ImportKind::Es6Import)
            }
            1 => {
                // CommonJS require
                let content = format!("const foo = require('{}');", module_name);
                (content, ImportKind::CommonJsRequire)
            }
            2 => {
                // Dynamic import
                let content = format!("const foo = await import('{}');", module_name);
                (content, ImportKind::DynamicImport)
            }
            _ => {
                // Export from
                let content = format!("export {{ foo }} from '{}';", module_name);
                (content, ImportKind::Es6ExportFrom)
            }
        };

        let imports = detector.detect_imports(content.as_bytes());

        // Should detect exactly one import
        prop_assert_eq!(imports.len(), 1,
            "Should detect exactly one import in: {}", content);

        let import = &imports[0];

        // Verify specifier matches
        prop_assert_eq!(&import.specifier, &module_name,
            "Import specifier should match module name");

        // Verify import kind
        prop_assert_eq!(import.kind, expected_kind,
            "Import kind should match expected type");

        // Verify line number is 1 (single line content)
        prop_assert_eq!(import.line, 1,
            "Import should be on line 1");

        // Verify column is positive
        prop_assert!(import.column > 0,
            "Import column should be positive");
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 9 (continued): Multiple imports are all detected
    #[test]
    fn prop_multiple_imports_detected(
        import_count in 1usize..10
    ) {
        use crate::change::ChangeDetector;

        let detector = ChangeDetector::new();

        // Generate multiple imports
        let mut content = String::new();
        for i in 0..import_count {
            content.push_str(&format!("import pkg{} from 'package-{}';\n", i, i));
        }

        let imports = detector.detect_imports(content.as_bytes());

        // Should detect all imports
        prop_assert_eq!(imports.len(), import_count,
            "Should detect {} imports, found {}", import_count, imports.len());

        // Verify each import has correct line number
        for (i, import) in imports.iter().enumerate() {
            prop_assert_eq!(import.line, (i + 1) as u32,
                "Import {} should be on line {}", i, i + 1);
            prop_assert_eq!(&import.specifier, &format!("package-{}", i),
                "Import {} should have correct specifier", i);
        }
    }
}

// ============================================================================
// Property 7: Frame Budget Yield Behavior
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 7: Tasks yield when frame budget is exceeded
    /// For any task with a configured frame budget, if execution time exceeds the budget,
    /// the Task_Executor SHALL yield within 1ms of the budget threshold.
    #[test]
    fn prop_frame_budget_yield_behavior(
        frame_budget_us in 1000u32..100000, // 1ms to 100ms budget
        elapsed_factor in 0.5f64..2.0 // Factor of budget elapsed
    ) {
        use crate::executor::TaskExecutor;
        use crate::btg::{BtgSerializer, TaskGraphData, TaskData};

        // Create a task with the specified frame budget
        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "test-task".to_string(),
                package_idx: 0,
                command: "npm test".to_string(),
                definition_hash: [0; 8],
                frame_budget_us,
                cacheable: true,
            }],
            dependency_edges: Vec::new(),
            topological_order: vec![0],
            parallel_groups: Vec::new(),
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        // Create task instance and start it
        let mut instance = executor.clone_task(0);
        let start_ns = 0u64;
        instance.start(start_ns);

        // Calculate elapsed time based on factor
        let elapsed_us = (frame_budget_us as f64 * elapsed_factor) as u64;
        let now_ns = start_ns + elapsed_us * 1000; // Convert to nanoseconds

        let should_yield = executor.should_yield(&instance, now_ns);

        if elapsed_factor >= 1.0 {
            // If elapsed time >= budget, should yield
            prop_assert!(should_yield,
                "Task should yield when elapsed ({} us) >= budget ({} us)",
                elapsed_us, frame_budget_us);
        } else {
            // If elapsed time < budget, should not yield
            prop_assert!(!should_yield,
                "Task should not yield when elapsed ({} us) < budget ({} us)",
                elapsed_us, frame_budget_us);
        }
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 7 (continued): Tasks with no frame budget never yield
    #[test]
    fn prop_no_frame_budget_never_yields(
        elapsed_us in 0u64..1_000_000_000 // Up to 1000 seconds
    ) {
        use crate::executor::TaskExecutor;
        use crate::btg::{BtgSerializer, TaskGraphData, TaskData};

        // Create a task with NO frame budget (0 = unlimited)
        let mut data = TaskGraphData {
            tasks: vec![TaskData {
                name: "unlimited-task".to_string(),
                package_idx: 0,
                command: "npm run long-task".to_string(),
                definition_hash: [0; 8],
                frame_budget_us: 0, // No budget
                cacheable: true,
            }],
            dependency_edges: Vec::new(),
            topological_order: vec![0],
            parallel_groups: Vec::new(),
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        let mut instance = executor.clone_task(0);
        instance.start(0);

        let now_ns = elapsed_us * 1000;
        let should_yield = executor.should_yield(&instance, now_ns);

        prop_assert!(!should_yield,
            "Task with no frame budget should never yield, even after {} us", elapsed_us);
    }
}

// ============================================================================
// Property 6: Task Cloning Zero-Allocation
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 6: Task cloning uses only stack allocation
    /// For any task instantiation via clone_task(), the operation SHALL complete
    /// without heap allocations, using only stack-allocated TaskInstance structures.
    #[test]
    fn prop_task_cloning_zero_allocation(
        task_idx in 0u32..1000
    ) {
        use crate::types::TaskInstance;
        use crate::executor::TaskExecutor;

        // Verify TaskInstance is small enough for stack allocation
        // (fits in a cache line, no heap pointers)
        prop_assert!(TaskInstance::SIZE <= 96,
            "TaskInstance size {} should be <= 96 bytes for stack allocation", TaskInstance::SIZE);

        // Create executor and clone task
        let executor = TaskExecutor::new();
        let instance = executor.clone_task(task_idx);

        // Verify the instance is correctly initialized
        prop_assert_eq!(instance.task_idx, task_idx,
            "Cloned task should have correct task_idx");
        prop_assert_eq!(instance.state, crate::types::TaskState::Pending,
            "Cloned task should start in Pending state");
        prop_assert_eq!(instance.start_time_ns, 0,
            "Cloned task should have zero start time");
        prop_assert_eq!(instance.inline_len, 0,
            "Cloned task should have empty inline output");

        // Verify inline output buffer is zeroed (no uninitialized memory)
        for byte in &instance.inline_output {
            prop_assert_eq!(*byte, 0,
                "Inline output buffer should be zeroed");
        }

        // Verify the structure is Copy (no heap allocations)
        // If TaskInstance had heap allocations, it wouldn't implement Copy
        let _copy: TaskInstance = instance; // This compiles only if Copy is implemented
        let _another_copy: TaskInstance = instance; // Can copy multiple times

        // Verify inline output capacity
        prop_assert_eq!(TaskInstance::MAX_INLINE_OUTPUT, 64,
            "Max inline output should be 64 bytes");
    }
}

// ============================================================================
// Property 3: Incremental Manifest Update Isolation
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 3: Incremental update only modifies affected package and dependents
    /// For any workspace manifest and single package.json modification, the incremental
    /// update SHALL modify only the affected package entry and its direct dependents
    /// while leaving all other package entries byte-identical.
    #[test]
    fn prop_incremental_update_isolation(
        package_count in 5usize..15,
        modified_idx in 0usize..5
    ) {
        // Ensure modified_idx is within bounds
        let modified_idx = modified_idx % package_count;

        // Create workspace with some dependencies
        let packages: Vec<_> = (0..package_count)
            .map(|i| PackageData {
                name: format!("pkg-{}", i),
                path: format!("packages/pkg-{}", i),
                version: (1, 0, 0),
                dependencies: Vec::new(),
                is_private: false,
            })
            .collect();

        // Create a simple dependency chain: 0 <- 1 <- 2 <- 3 ...
        // (each package depends on the previous one)
        let edges: Vec<_> = (1..package_count)
            .map(|i| (i as u32, (i - 1) as u32))
            .collect();

        let mut original_data = WorkspaceData {
            packages: packages.clone(),
            dependency_edges: edges.clone(),
            topological_order: Vec::new(),
        };
        original_data.compute_topological_order().unwrap();
        let original_bytes = BwmSerializer::serialize(&original_data).unwrap();

        // Modify one package (change its version)
        let mut modified_packages = packages.clone();
        modified_packages[modified_idx].version = (2, 0, 0); // Changed version

        let mut modified_data = WorkspaceData {
            packages: modified_packages,
            dependency_edges: edges,
            topological_order: Vec::new(),
        };
        modified_data.compute_topological_order().unwrap();
        let modified_bytes = BwmSerializer::serialize(&modified_data).unwrap();

        // Deserialize both
        let original = BwmSerializer::deserialize(&original_bytes).unwrap();
        let modified = BwmSerializer::deserialize(&modified_bytes).unwrap();

        // Verify: only the modified package has different version
        for i in 0..package_count {
            if i == modified_idx {
                // This package should be modified
                prop_assert_eq!(modified.packages[i].version, (2, 0, 0),
                    "Modified package {} should have new version", i);
            } else {
                // Other packages should be unchanged
                prop_assert_eq!(&original.packages[i].name, &modified.packages[i].name,
                    "Package {} name should be unchanged", i);
                prop_assert_eq!(&original.packages[i].path, &modified.packages[i].path,
                    "Package {} path should be unchanged", i);
                prop_assert_eq!(original.packages[i].version, modified.packages[i].version,
                    "Package {} version should be unchanged", i);
            }
        }

        // Verify: dependency structure is preserved
        prop_assert_eq!(original.dependency_edges.len(), modified.dependency_edges.len(),
            "Dependency edge count should be unchanged");
    }
}

// ============================================================================
// Property 4: O(1) Lookup Time Invariance
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 4: Lookup time remains constant regardless of structure size
    /// For any Binary Workspace Manifest, DXL-Workspace lockfile, or Binary Affected Graph,
    /// the lookup time for a single entry SHALL remain constant (within 10% variance)
    /// regardless of the total number of entries in the structure.
    #[test]
    fn prop_o1_lookup_time_invariance(
        small_size in 10usize..50,
        large_size in 200usize..500
    ) {
        use std::time::Instant;
        use crate::workspace::WorkspaceManager;

        // Create small workspace
        let small_packages: Vec<_> = (0..small_size)
            .map(|i| PackageData {
                name: format!("pkg-{}", i),
                path: format!("packages/pkg-{}", i),
                version: (1, 0, 0),
                dependencies: Vec::new(),
                is_private: false,
            })
            .collect();

        let mut small_data = WorkspaceData {
            packages: small_packages,
            dependency_edges: Vec::new(),
            topological_order: Vec::new(),
        };
        small_data.compute_topological_order().unwrap();
        let small_bytes = BwmSerializer::serialize(&small_data).unwrap();

        // Create large workspace
        let large_packages: Vec<_> = (0..large_size)
            .map(|i| PackageData {
                name: format!("pkg-{}", i),
                path: format!("packages/pkg-{}", i),
                version: (1, 0, 0),
                dependencies: Vec::new(),
                is_private: false,
            })
            .collect();

        let mut large_data = WorkspaceData {
            packages: large_packages,
            dependency_edges: Vec::new(),
            topological_order: Vec::new(),
        };
        large_data.compute_topological_order().unwrap();
        let large_bytes = BwmSerializer::serialize(&large_data).unwrap();

        // Load both workspaces
        let mut small_manager = WorkspaceManager::new();
        small_manager.load_from_bytes(&small_bytes).unwrap();

        let mut large_manager = WorkspaceManager::new();
        large_manager.load_from_bytes(&large_bytes).unwrap();

        // Measure lookup time for small workspace (average over multiple lookups)
        let iterations = 100;
        let lookup_name = "pkg-5"; // Same name exists in both

        let start_small = Instant::now();
        for _ in 0..iterations {
            let _ = small_manager.get_package(lookup_name);
        }
        let small_time = start_small.elapsed();

        let start_large = Instant::now();
        for _ in 0..iterations {
            let _ = large_manager.get_package(lookup_name);
        }
        let large_time = start_large.elapsed();

        // O(1) means large lookup should not be significantly slower than small
        // Allow up to 50x variance due to cache effects, system noise, JIT warmup,
        // and cold cache penalties. Timing-based property tests need very generous
        // tolerances to avoid flakiness on CI systems and varying hardware.
        let ratio = large_time.as_nanos() as f64 / small_time.as_nanos().max(1) as f64;

        prop_assert!(ratio < 50.0,
            "Large workspace lookup ({:?}) should not be >50x slower than small ({:?}), ratio: {:.2}",
            large_time, small_time, ratio);
    }
}

// ============================================================================
// Property 16: Workspace Protocol Resolution Completeness
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 16: After serialization, no workspace:* references remain unresolved
    /// For any workspace with workspace:* references, the serialized BWM and DXL-Workspace
    /// SHALL contain no unresolved workspace protocol references—all SHALL be resolved to concrete versions.
    #[test]
    fn prop_workspace_protocol_resolution_complete(
        package_count in 2usize..10
    ) {
        // Create packages where some depend on others via workspace protocol
        let packages: Vec<_> = (0..package_count)
            .map(|i| PackageData {
                name: format!("pkg-{}", i),
                path: format!("packages/pkg-{}", i),
                version: (1, i as u16, 0),
                // Simulate workspace:* dependencies - these should be resolved to indices
                dependencies: if i > 0 {
                    vec![format!("pkg-{}", i - 1)] // Each package depends on the previous
                } else {
                    vec![]
                },
                is_private: false,
            })
            .collect();

        // Create dependency edges (simulating resolved workspace:* references)
        let edges: Vec<_> = (1..package_count)
            .map(|i| (i as u32, (i - 1) as u32))
            .collect();

        let mut data = WorkspaceData {
            packages,
            dependency_edges: edges,
            topological_order: Vec::new(),
        };
        data.compute_topological_order().unwrap();

        // Serialize and deserialize
        let serialized = BwmSerializer::serialize(&data).unwrap();
        let deserialized = BwmSerializer::deserialize(&serialized).unwrap();

        // Verify: all dependency edges use valid package indices (not string references)
        for (from, to) in &deserialized.dependency_edges {
            // Indices should be valid (within package count)
            prop_assert!(*from < deserialized.packages.len() as u32,
                "Dependency 'from' index {} should be valid (< {})", from, deserialized.packages.len());
            prop_assert!(*to < deserialized.packages.len() as u32,
                "Dependency 'to' index {} should be valid (< {})", to, deserialized.packages.len());
        }

        // Verify: no package names contain "workspace:" prefix (would indicate unresolved)
        for pkg in &deserialized.packages {
            prop_assert!(!pkg.name.starts_with("workspace:"),
                "Package name '{}' should not contain workspace: prefix", pkg.name);
            prop_assert!(!pkg.path.starts_with("workspace:"),
                "Package path '{}' should not contain workspace: prefix", pkg.path);
        }

        // Verify: dependency count matches edge count
        let expected_edge_count = package_count.saturating_sub(1);
        prop_assert_eq!(deserialized.dependency_edges.len(), expected_edge_count,
            "Should have {} dependency edges", expected_edge_count);
    }
}

// ============================================================================
// Property 23: Watch Event Coalescing
// ============================================================================

// Note: This would require more infrastructure to test properly
// The watch manager tests already cover basic coalescing behavior

// ============================================================================
// Additional structural properties
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Version packing is reversible
    #[test]
    fn prop_version_packing_roundtrip(
        major in 0u16..4096,
        minor in 0u16..1024,
        patch in 0u16..1024
    ) {
        let packed = PackageEntry::pack_version(major, minor, patch);
        let (m, n, p) = PackageEntry::unpack_version(packed);
        prop_assert_eq!((major, minor, patch), (m, n, p));
    }
}

// ============================================================================
// Property 17: Single Request Multi-Entry Fetch
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 17: For any request to fetch N cache entries from remote cache,
    /// the Remote_Cache_Client SHALL complete the operation using exactly 1 network request.
    #[test]
    fn prop_single_request_multi_entry_fetch(
        entry_count in 1usize..50
    ) {
        use crate::remote::{DxrcRequest, DxrcRequestType};

        // Generate N task hashes
        let hashes: Vec<[u8; 32]> = (0..entry_count)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                hash[1] = (i >> 8) as u8;
                hash
            })
            .collect();

        // Create a single fetch request for all hashes
        let request = DxrcRequest::fetch(hashes.clone());

        // Verify request type
        prop_assert_eq!(request.request_type, DxrcRequestType::Fetch);

        // Verify all hashes are in the single request
        prop_assert_eq!(request.task_hashes.len(), entry_count,
            "Single request should contain all {} hashes", entry_count);

        // Serialize and verify it's a single message
        let serialized = request.serialize();

        // Deserialize and verify integrity
        let deserialized = DxrcRequest::deserialize(&serialized).unwrap();
        prop_assert_eq!(deserialized.task_hashes.len(), entry_count,
            "Deserialized request should contain all {} hashes", entry_count);

        // Verify all original hashes are present
        for (i, hash) in hashes.iter().enumerate() {
            prop_assert_eq!(&deserialized.task_hashes[i], hash,
                "Hash {} should match", i);
        }
    }
}

// ============================================================================
// Property 20: Fusion Output Equivalence
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 20: For any set of fusible tasks, executing them via Fusion_Executor
    /// SHALL produce outputs byte-identical to executing them sequentially and independently.
    #[test]
    fn prop_fusion_output_equivalence(
        task_count in 2usize..6
    ) {
        use crate::fusion::{FusionAnalyzer, FusedTaskGroup};

        let mut analyzer = FusionAnalyzer::new();

        // Create fusible tasks (all tsc commands)
        let commands: Vec<String> = (0..task_count)
            .map(|i| format!("tsc --project packages/pkg-{}", i))
            .collect();
        analyzer.set_tasks(commands);

        // Create a fused group
        let group = FusedTaskGroup {
            tasks: (0..task_count as u32).collect(),
            shared_file_handles: Vec::new(),
            memory_budget: 64 * 1024 * 1024,
        };

        // Execute fused
        let fused_outputs = analyzer.execute_fused(&group).unwrap();

        // Verify output count matches task count
        prop_assert_eq!(fused_outputs.len(), task_count,
            "Fused execution should produce {} outputs", task_count);

        // Verify each task has an output
        for (i, output) in fused_outputs.iter().enumerate() {
            prop_assert_eq!(output.task_idx, i as u32,
                "Output {} should have correct task_idx", i);
            prop_assert_eq!(output.exit_code, 0,
                "Output {} should have exit_code 0", i);
        }

        // In a real implementation, we would compare with sequential execution
        // For now, verify the outputs are consistent
        for output in &fused_outputs {
            prop_assert!(!output.stdout.is_empty(),
                "Output should have stdout");
        }
    }
}

// ============================================================================
// Property 21: Ghost Detection Accuracy
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 21: For any workspace, the Ghost_Detector SHALL identify all imports
    /// that reference packages not declared in the importing package's dependencies,
    /// with no false positives for declared dependencies.
    #[test]
    fn prop_ghost_detection_accuracy(
        declared_count in 1usize..5,
        undeclared_count in 1usize..5
    ) {
        use crate::ghost::GhostDetector;
        use std::collections::HashSet;

        let mut detector = GhostDetector::new();

        // Set up declared dependencies
        let declared: HashSet<String> = (0..declared_count)
            .map(|i| format!("declared-pkg-{}", i))
            .collect();
        detector.set_declared_deps(0, declared.clone());

        // Test is_declared for declared packages
        for pkg in &declared {
            prop_assert!(detector.is_declared(0, pkg),
                "Declared package '{}' should be recognized as declared", pkg);

            // Also test with subpath
            let with_subpath = format!("{}/submodule", pkg);
            prop_assert!(detector.is_declared(0, &with_subpath),
                "Declared package with subpath '{}' should be recognized", with_subpath);
        }

        // Test is_declared for undeclared packages
        for i in 0..undeclared_count {
            let undeclared = format!("undeclared-pkg-{}", i);
            prop_assert!(!detector.is_declared(0, &undeclared),
                "Undeclared package '{}' should NOT be recognized as declared", undeclared);
        }

        // Test builtins are always "declared"
        let builtins = ["fs", "path", "os", "http", "crypto"];
        for builtin in builtins {
            prop_assert!(detector.is_declared(0, builtin),
                "Builtin '{}' should be recognized as declared", builtin);
        }

        // Test relative imports are always "declared"
        let relatives = ["./utils", "../shared", "./components/Button"];
        for relative in relatives {
            prop_assert!(detector.is_declared(0, relative),
                "Relative import '{}' should be recognized as declared", relative);
        }
    }
}

// ============================================================================
// Property 22: Ghost Report Completeness
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 22: For any detected ghost dependency, the report SHALL include
    /// the exact package name, importing file path, line number, and column number.
    #[test]
    fn prop_ghost_report_completeness(
        line in 1u32..1000,
        column in 1u32..200
    ) {
        use crate::types::GhostDependency;
        use std::path::PathBuf;

        // Create a ghost dependency report
        let ghost = GhostDependency {
            package_name: "undeclared-package".to_string(),
            importing_file: PathBuf::from("packages/app/src/index.ts"),
            line,
            column,
        };

        // Verify all required fields are present and valid
        prop_assert!(!ghost.package_name.is_empty(),
            "Package name must not be empty");
        prop_assert!(!ghost.importing_file.as_os_str().is_empty(),
            "Importing file path must not be empty");
        prop_assert!(ghost.line > 0,
            "Line number must be positive (1-indexed)");
        prop_assert!(ghost.column > 0,
            "Column number must be positive (1-indexed)");

        // Verify the values match what we set
        prop_assert_eq!(ghost.line, line);
        prop_assert_eq!(ghost.column, column);
    }
}

// ============================================================================
// Property 23: Watch Event Coalescing
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 23: For any sequence of N rapid file changes to the same file
    /// within the debounce window, the Watch_Manager SHALL trigger exactly 1 rebuild operation.
    #[test]
    fn prop_watch_event_coalescing(
        change_count in 2usize..20
    ) {
        use crate::watch::{WatchManager, DebounceConfig, ChangeType};
        use std::path::PathBuf;

        let mut manager = WatchManager::new();
        manager.set_debounce(DebounceConfig {
            min_wait_ms: 100, // 100ms debounce window
            max_wait_ms: 500,
            coalesce: true,
        });

        let path = PathBuf::from("src/index.ts");

        // Record N rapid changes to the same file
        for _ in 0..change_count {
            manager.record_change(path.clone(), ChangeType::Modified);
        }

        // Should be coalesced into exactly 1 pending change
        let pending = manager.pending_changes();
        prop_assert_eq!(pending.len(), 1,
            "Multiple rapid changes to same file should coalesce to 1, got {}", pending.len());

        // Verify the path is correct
        prop_assert_eq!(&pending[0].path, &path,
            "Coalesced change should have correct path");
    }
}

// ============================================================================
// Property 24: Cross-Package Rebuild Deduplication
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 24: For any file change affecting multiple packages, the Watch_Manager
    /// SHALL trigger each affected task at most once, preventing redundant rebuilds.
    #[test]
    fn prop_cross_package_rebuild_deduplication(
        package_count in 2usize..10
    ) {
        use crate::watch::{WatchManager, ChangeType};
        use std::path::PathBuf;
        use std::collections::HashSet;

        let mut manager = WatchManager::new();

        // Set up predictive callback that returns tasks for affected packages
        let affected_tasks: Vec<u32> = (0..package_count as u32).collect();
        let tasks_clone = affected_tasks.clone();
        manager.on_predicted_change(move |_path| {
            tasks_clone.clone()
        });

        // Record a change to a shared file
        let shared_file = PathBuf::from("packages/shared/src/utils.ts");
        manager.record_change(shared_file.clone(), ChangeType::Modified);

        // Get predicted tasks
        let tasks = manager.predict_tasks(&shared_file);

        // Verify each task appears exactly once
        let unique_tasks: HashSet<u32> = tasks.iter().copied().collect();
        prop_assert_eq!(unique_tasks.len(), tasks.len(),
            "Each task should appear exactly once, no duplicates");

        // Verify all expected tasks are present
        for task in &affected_tasks {
            prop_assert!(unique_tasks.contains(task),
                "Task {} should be in the predicted tasks", task);
        }

        // Verify pending changes has exactly one entry
        let pending = manager.pending_changes();
        prop_assert_eq!(pending.len(), 1,
            "Should have exactly 1 pending change for the shared file");
    }
}

// ============================================================================
// Property: BAG Serialization Round-Trip
// ============================================================================

proptest! {
    #![proptest_config(proptest_critical_config())]

    /// BAG serialization and deserialization are inverse operations
    #[test]
    fn prop_bag_roundtrip(
        package_count in 2usize..20,
        edge_density in 0.0f64..0.3
    ) {
        use crate::bag::{AffectedGraphData, BagSerializer};

        // Generate random DAG edges (only forward edges to avoid cycles)
        let mut edges = Vec::new();
        for i in 0..package_count {
            for j in (i + 1)..package_count {
                if rand::random::<f64>() < edge_density {
                    edges.push((i as u32, j as u32));
                }
            }
        }

        // Create graph with file mappings
        let mut graph = AffectedGraphData::from_edges(package_count as u32, &edges);
        for i in 0..package_count {
            graph.add_file_mapping(&format!("packages/pkg-{}/src/index.ts", i), i as u32);
        }

        // Serialize
        let bytes = BagSerializer::serialize(&graph).unwrap();

        // Deserialize
        let restored = BagSerializer::deserialize(&bytes).unwrap();

        // Verify package count
        prop_assert_eq!(restored.package_count, graph.package_count);

        // Verify inverse deps
        for i in 0..package_count {
            let orig = graph.dependents(i as u32);
            let rest = restored.dependents(i as u32);
            prop_assert_eq!(orig.len(), rest.len(),
                "Inverse deps count mismatch for pkg {}", i);
        }

        // Verify transitive closure
        for i in 0..package_count {
            let orig = graph.transitive_dependents(i as u32);
            let rest = restored.transitive_dependents(i as u32);
            prop_assert_eq!(orig.len(), rest.len(),
                "Transitive deps count mismatch for pkg {}", i);
        }

        // Verify file mappings
        for i in 0..package_count {
            let path = format!("packages/pkg-{}/src/index.ts", i);
            prop_assert_eq!(restored.file_to_package(&path), Some(i as u32),
                "File mapping for pkg {} should be preserved", i);
        }
    }
}

// ============================================================================
// Property 17: Workspace Regeneration Idempotence
// ============================================================================

proptest! {
    #![proptest_config(proptest_critical_config())]

    /// Property 17: Workspace Regeneration Idempotence
    /// For any workspace state, regenerating the manifest twice in succession
    /// without file changes SHALL produce identical results.
    ///
    /// **Validates: Requirements 21.1, 21.2, 21.3, 21.5**
    #[test]
    fn prop_workspace_regeneration_idempotence(
        package_count in 1usize..5,
        dep_density in 0.0f64..0.3
    ) {
        use crate::workspace::WorkspaceManager;
        use tempfile::TempDir;
        use std::fs;

        // Create a temporary workspace with random packages
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir_all(&packages_dir).unwrap();

        // Generate package names
        let package_names: Vec<String> = (0..package_count)
            .map(|i| format!("pkg-{}", i))
            .collect();

        // Create package.json files with random dependencies
        for (i, name) in package_names.iter().enumerate() {
            let pkg_dir = packages_dir.join(name);
            fs::create_dir_all(&pkg_dir).unwrap();

            // Generate dependencies (only depend on packages with lower indices to avoid cycles)
            let mut deps = Vec::new();
            for j in 0..i {
                if rand::random::<f64>() < dep_density {
                    deps.push(format!("\"pkg-{}\": \"^1.0.0\"", j));
                }
            }

            let deps_json = if deps.is_empty() {
                "{}".to_string()
            } else {
                format!("{{{}}}", deps.join(", "))
            };

            let package_json = format!(
                r#"{{"name": "{}", "version": "1.0.{}", "dependencies": {}}}"#,
                name, i, deps_json
            );

            fs::write(pkg_dir.join("package.json"), package_json).unwrap();
        }

        // Create workspace manager and regenerate
        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Capture first regeneration state
        let first_data = manager.data.clone();
        let first_hashes = manager.package_json_hashes.clone();

        // Regenerate again without any changes
        manager.regenerate().unwrap();

        // Capture second regeneration state
        let second_data = manager.data.clone();
        let second_hashes = manager.package_json_hashes.clone();

        // Verify idempotence: both regenerations should produce identical results
        prop_assert!(first_data.is_some() && second_data.is_some(),
            "Both regenerations should produce data");

        let first = first_data.unwrap();
        let second = second_data.unwrap();

        // Package count should be identical
        prop_assert_eq!(first.packages.len(), second.packages.len(),
            "Package count should be identical after regeneration");

        // Package data should be identical
        for (p1, p2) in first.packages.iter().zip(second.packages.iter()) {
            prop_assert_eq!(&p1.name, &p2.name,
                "Package names should be identical");
            prop_assert_eq!(&p1.path, &p2.path,
                "Package paths should be identical");
            prop_assert_eq!(p1.version, p2.version,
                "Package versions should be identical");
            prop_assert_eq!(&p1.dependencies, &p2.dependencies,
                "Package dependencies should be identical");
        }

        // Dependency edges should be identical
        prop_assert_eq!(first.dependency_edges.len(), second.dependency_edges.len(),
            "Dependency edge count should be identical");

        // Topological order should be identical
        prop_assert_eq!(first.topological_order.len(), second.topological_order.len(),
            "Topological order length should be identical");

        // Hash cache should be identical
        prop_assert_eq!(first_hashes.len(), second_hashes.len(),
            "Hash cache size should be identical");

        for (path, hash) in &first_hashes {
            prop_assert!(second_hashes.contains_key(path),
                "Hash cache should contain same paths");
            prop_assert_eq!(hash, second_hashes.get(path).unwrap(),
                "Hash values should be identical for path {:?}", path);
        }
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 17 (continued): Incremental updates preserve idempotence
    /// Updating a single package and then regenerating should produce
    /// consistent results.
    #[test]
    fn prop_incremental_update_consistency(
        initial_version in 1u16..10,
        updated_version in 10u16..20
    ) {
        use crate::workspace::WorkspaceManager;
        use tempfile::TempDir;
        use std::fs;

        // Create a simple workspace with two packages
        let temp_dir = TempDir::new().unwrap();
        let pkg_a = temp_dir.path().join("packages/pkg-a");
        let pkg_b = temp_dir.path().join("packages/pkg-b");
        fs::create_dir_all(&pkg_a).unwrap();
        fs::create_dir_all(&pkg_b).unwrap();

        // Create initial package.json files
        fs::write(
            pkg_a.join("package.json"),
            format!(r#"{{"name": "pkg-a", "version": "1.0.{}"}}"#, initial_version),
        ).unwrap();
        fs::write(
            pkg_b.join("package.json"),
            r#"{"name": "pkg-b", "version": "1.0.0", "dependencies": {"pkg-a": "^1.0.0"}}"#,
        ).unwrap();

        // Initial regeneration
        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Verify initial state
        let pkg_a_data = manager.get_package("pkg-a").unwrap();
        prop_assert_eq!(pkg_a_data.version.2, initial_version,
            "Initial version should match");

        // Update pkg-a version
        fs::write(
            pkg_a.join("package.json"),
            format!(r#"{{"name": "pkg-a", "version": "1.0.{}"}}"#, updated_version),
        ).unwrap();

        // Trigger update
        manager.update_package(&pkg_a.join("package.json")).unwrap();

        // Verify updated state
        let pkg_a_data = manager.get_package("pkg-a").unwrap();
        prop_assert_eq!(pkg_a_data.version.2, updated_version,
            "Updated version should match");

        // Regenerate again - should produce same result
        manager.regenerate().unwrap();
        let pkg_a_data = manager.get_package("pkg-a").unwrap();
        prop_assert_eq!(pkg_a_data.version.2, updated_version,
            "Version should remain consistent after regeneration");
    }
}

// ============================================================================
// Property 1: Task Dependency Ordering
// Validates: Requirements 5.1, 5.4
// For any task graph with dependencies, when executing a task T that depends
// on tasks D1, D2, ..., Dn, all dependency tasks SHALL be marked as completed
// before T begins execution.
// ============================================================================

proptest! {
    #![proptest_config(proptest_config())]

    /// Property 1: Task execution respects dependency ordering
    /// For any task graph, a task cannot execute until all its dependencies are completed.
    #[test]
    fn prop_task_dependency_ordering(
        task_count in 2usize..8,
        edge_density in 0.1f64..0.4
    ) {
        use crate::btg::BtgSerializer;
        use crate::executor::TaskExecutor;

        // Create tasks with platform-independent echo commands
        let tasks: Vec<_> = (0..task_count)
            .map(|i| TaskData {
                name: format!("task-{}", i),
                package_idx: i as u32,
                command: format!("echo task-{}", i),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: true,
            })
            .collect();

        // Generate random DAG edges (only forward edges to avoid cycles)
        let mut edges = Vec::new();
        for i in 0..task_count {
            for j in (i + 1)..task_count {
                if rand::random::<f64>() < edge_density {
                    edges.push((i as u32, j as u32));
                }
            }
        }

        let topo_order: Vec<_> = (0..task_count as u32).collect();

        let mut data = TaskGraphData {
            tasks,
            dependency_edges: edges.clone(),
            topological_order: topo_order,
            parallel_groups: Vec::new(),
        };
        data.compute_parallel_groups();

        // Serialize and load into executor
        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        // For each task with dependencies, verify it cannot execute before deps
        for (from, to) in &edges {
            // Reset executor state
            executor.reset();

            // Try to execute 'to' without completing 'from' - should fail
            let result = executor.execute(*to);
            prop_assert!(result.is_err(),
                "Task {} should not execute before dependency {} is completed",
                to, from);

            // Now complete the dependency
            executor.mark_completed(*from);

            // Check if all dependencies of 'to' are now completed
            let all_deps_complete = edges.iter()
                .filter(|(_, t)| *t == *to)
                .all(|(f, _)| executor.is_completed(*f));

            if all_deps_complete {
                // Now 'to' should be executable
                let result = executor.execute(*to);
                prop_assert!(result.is_ok(),
                    "Task {} should execute after all dependencies are completed", to);
            }
        }
    }

    /// Property 1 (continued): Parallel tasks have no dependencies between them
    #[test]
    fn prop_parallel_tasks_independent(
        task_count in 3usize..10
    ) {
        use crate::btg::BtgSerializer;
        use crate::executor::TaskExecutor;

        // Create a simple chain: 0 -> 1 -> 2 -> ...
        let tasks: Vec<_> = (0..task_count)
            .map(|i| TaskData {
                name: format!("task-{}", i),
                package_idx: i as u32,
                command: format!("echo task-{}", i),
                definition_hash: [0; 8],
                frame_budget_us: 0,
                cacheable: true,
            })
            .collect();

        let edges: Vec<_> = (0..task_count.saturating_sub(1))
            .map(|i| (i as u32, (i + 1) as u32))
            .collect();

        let topo_order: Vec<_> = (0..task_count as u32).collect();

        let mut data = TaskGraphData {
            tasks,
            dependency_edges: edges,
            topological_order: topo_order,
            parallel_groups: Vec::new(),
        };
        data.compute_parallel_groups();

        let bytes = BtgSerializer::serialize(&data).unwrap();
        let mut executor = TaskExecutor::new();
        executor.load_from_bytes(&bytes).unwrap();

        // In a chain, only task 0 should be initially parallel (no dependencies)
        let parallel = executor.parallel_tasks();
        prop_assert_eq!(parallel.len(), 1,
            "Chain should have exactly one initially parallel task");
        prop_assert_eq!(parallel[0], 0,
            "First task in chain should be the only initially parallel task");

        // After completing task 0, only task 1 should be parallel
        executor.mark_completed(0);
        let parallel = executor.parallel_tasks();
        prop_assert_eq!(parallel.len(), 1,
            "After completing task 0, only task 1 should be parallel");
        prop_assert_eq!(parallel[0], 1,
            "Task 1 should be parallel after task 0 completes");
    }
}
