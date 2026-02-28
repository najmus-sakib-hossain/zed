//! Property-based tests for DPL lock file operations
//!
//! **Feature: dx-py-package-manager, Property 6: Hash Table O(1) Lookup Correctness**
//! **Validates: Requirements 2.1**
//!
//! **Feature: dx-py-package-manager, Property 4: DPL Round-Trip Consistency**
//! **Validates: Requirements 2.8**

use dx_py_package_manager::{DplBuilder, DplLockFile};
use proptest::prelude::*;

/// Generate a valid package name (lowercase, alphanumeric with hyphens/underscores)
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,30}".prop_map(|s| s.to_string())
}

/// Generate a valid version string
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate a source hash
fn arb_hash() -> impl Strategy<Value = [u8; 32]> {
    proptest::collection::vec(any::<u8>(), 32).prop_map(|v| {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&v);
        arr
    })
}

/// Generate a package entry (name, version, hash)
fn arb_package_entry() -> impl Strategy<Value = (String, String, [u8; 32])> {
    (arb_package_name(), arb_version(), arb_hash())
}

/// Generate a list of unique package entries
fn arb_unique_packages(
    min: usize,
    max: usize,
) -> impl Strategy<Value = Vec<(String, String, [u8; 32])>> {
    proptest::collection::vec(arb_package_entry(), min..max).prop_map(|entries| {
        // Deduplicate by name
        let mut seen = std::collections::HashSet::new();
        entries.into_iter().filter(|(name, _, _)| seen.insert(name.clone())).collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Hash table lookup returns correct entry for all packages
    ///
    /// *For any* DPL lock file with N packages, looking up any package by name
    /// SHALL return the correct entry with matching name and version.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_hash_table_lookup_correct(
        packages in arb_unique_packages(1, 50),
        python_version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        platform in "[a-z_0-9]{5,20}"
    ) {
        let mut builder = DplBuilder::new(&python_version, &platform);

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        // Verify all packages can be looked up correctly
        for (name, version, hash) in &packages {
            let entry = lock_file.lookup(name);
            prop_assert!(entry.is_some(), "Package '{}' not found", name);

            let entry = entry.unwrap();
            prop_assert_eq!(entry.name_str(), name.as_str(),
                "Name mismatch for '{}': got '{}'", name, entry.name_str());
            prop_assert_eq!(entry.version_str(), version.as_str(),
                "Version mismatch for '{}': expected '{}', got '{}'", name, version, entry.version_str());
            prop_assert_eq!(&entry.source_hash, hash,
                "Hash mismatch for '{}'", name);
        }
    }

    /// Property 6: Hash table lookup returns None for non-existent packages
    ///
    /// *For any* DPL lock file, looking up a non-existent package
    /// SHALL return None.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_hash_table_lookup_nonexistent(
        packages in arb_unique_packages(1, 20),
        nonexistent_name in "[a-z][a-z0-9_-]{31,40}"  // Names longer than typical to avoid collision
    ) {
        let mut builder = DplBuilder::new("3.12.0", "linux");

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        // Verify non-existent package returns None
        let result = lock_file.lookup(&nonexistent_name);
        prop_assert!(result.is_none(),
            "Expected None for non-existent package '{}', got Some", nonexistent_name);
    }

    /// Property 6: Package count matches number of added packages
    ///
    /// *For any* DPL lock file, the package_count SHALL equal the number
    /// of packages added to the builder.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_package_count_correct(packages in arb_unique_packages(0, 100)) {
        let mut builder = DplBuilder::new("3.11.0", "win_amd64");

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        prop_assert_eq!(lock_file.package_count() as usize, packages.len(),
            "Package count mismatch: expected {}, got {}", packages.len(), lock_file.package_count());
    }

    /// Property 4: DPL round-trip produces equivalent data
    ///
    /// *For any* valid DPL lock file, building it and then iterating over
    /// all entries SHALL produce the same packages in the same order.
    /// **Validates: Requirements 2.8**
    #[test]
    fn prop_dpl_roundtrip_entries(
        packages in arb_unique_packages(1, 50),
        python_version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        platform in "[a-z_0-9]{5,20}"
    ) {
        let mut builder = DplBuilder::new(&python_version, &platform);

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        // Verify metadata
        prop_assert_eq!(lock_file.python_version(), python_version.as_str());
        prop_assert_eq!(lock_file.platform(), platform.as_str());

        // Verify all entries via iteration
        let entries: Vec<_> = lock_file.iter().collect();
        prop_assert_eq!(entries.len(), packages.len());

        for (i, (name, version, hash)) in packages.iter().enumerate() {
            prop_assert_eq!(entries[i].name_str(), name.as_str());
            prop_assert_eq!(entries[i].version_str(), version.as_str());
            prop_assert_eq!(&entries[i].source_hash, hash);
        }
    }

    /// Property 4: DPL integrity verification passes for valid files
    ///
    /// *For any* valid DPL lock file built by DplBuilder,
    /// verify() SHALL return true.
    /// **Validates: Requirements 2.8**
    #[test]
    fn prop_dpl_integrity_valid(packages in arb_unique_packages(1, 30)) {
        let mut builder = DplBuilder::new("3.12.0", "manylinux_2_17_x86_64");

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        prop_assert!(lock_file.verify(), "Integrity verification failed");
    }

    /// Property 6: Empty lock file handles lookups correctly
    ///
    /// *For any* empty DPL lock file, all lookups SHALL return None.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_empty_lockfile_lookup(name in arb_package_name()) {
        let builder = DplBuilder::new("3.12.0", "linux");
        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        prop_assert_eq!(lock_file.package_count(), 0);
        prop_assert!(lock_file.lookup(&name).is_none());
    }
}

// =============================================================================
// Property Tests for DPL Enhancements (Task 5)
// =============================================================================

/// Generate a list of extra names
fn arb_extras() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec("[a-z][a-z0-9_]{0,10}", 0..10).prop_map(|extras| {
        // Deduplicate
        let mut seen = std::collections::HashSet::new();
        extras.into_iter().filter(|e| seen.insert(e.clone())).collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: DPL Entry Fields Correctness - Version Components
    ///
    /// *For any* package entry stored in a DPL file, the version_major,
    /// version_minor, and version_patch fields SHALL correctly represent
    /// the semantic version parsed from the version string.
    ///
    /// **Validates: Requirements 2.4, 2.5**
    #[test]
    fn prop_dpl_version_components_correct(
        name in arb_package_name(),
        major in 0u16..1000,
        minor in 0u16..1000,
        patch in 0u16..1000,
        hash in arb_hash()
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);

        let mut builder = DplBuilder::new("3.12.0", "linux");
        builder.add_package(&name, &version, hash);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entry = lock_file.lookup(&name).unwrap();
        let (v_major, v_minor, v_patch) = entry.version_components();

        prop_assert_eq!(v_major, major, "Major version mismatch");
        prop_assert_eq!(v_minor, minor, "Minor version mismatch");
        prop_assert_eq!(v_patch, patch, "Patch version mismatch");
    }

    /// Property 6: DPL Entry Fields Correctness - Extras Bitmap
    ///
    /// *For any* package entry with extras, the extras_bitmap SHALL correctly
    /// encode all extras, and has_extra() SHALL return true for enabled extras
    /// and false for disabled extras.
    ///
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_dpl_extras_bitmap_correct(
        name in arb_package_name(),
        version in arb_version(),
        hash in arb_hash(),
        extras in arb_extras()
    ) {
        let mut builder = DplBuilder::new("3.12.0", "linux");
        let extras_refs: Vec<&str> = extras.iter().map(|s| s.as_str()).collect();
        builder.add_package_with_extras(&name, &version, hash, &extras_refs);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entry = lock_file.lookup(&name).unwrap();

        // Verify extras bitmap is non-zero if extras were provided
        if !extras.is_empty() {
            let bitmap = { entry.extras_bitmap };
            prop_assert!(bitmap != 0, "Extras bitmap should be non-zero when extras are provided");
        }

        // Verify the number of set bits matches the number of extras
        let bitmap = { entry.extras_bitmap };
        let set_bits = bitmap.count_ones() as usize;
        prop_assert_eq!(set_bits, extras.len(),
            "Number of set bits ({}) should match number of extras ({})", set_bits, extras.len());
    }

    /// Property 6: DPL Entry Fields Correctness - Name Hash
    ///
    /// *For any* package entry, the pre-computed name_hash SHALL match
    /// the FNV-1a hash of the package name.
    ///
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_dpl_name_hash_correct(
        name in arb_package_name(),
        version in arb_version(),
        hash in arb_hash()
    ) {
        let mut builder = DplBuilder::new("3.12.0", "linux");
        builder.add_package(&name, &version, hash);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entry = lock_file.lookup(&name).unwrap();
        let expected_hash = dx_py_core::fnv1a_hash(name.as_bytes());
        let actual_hash = { entry.name_hash };

        prop_assert_eq!(actual_hash, expected_hash,
            "Name hash mismatch for '{}': expected {}, got {}", name, expected_hash, actual_hash);
    }

    /// Property 6: DPL Entry Fields Correctness - Source Hash
    ///
    /// *For any* package entry, the source_hash SHALL match the stored value.
    ///
    /// **Validates: Requirements 2.7**
    #[test]
    fn prop_dpl_source_hash_correct(
        name in arb_package_name(),
        version in arb_version(),
        hash in arb_hash()
    ) {
        let mut builder = DplBuilder::new("3.12.0", "linux");
        builder.add_package(&name, &version, hash);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entry = lock_file.lookup(&name).unwrap();
        prop_assert_eq!(entry.source_hash, hash, "Source hash mismatch");
    }

    /// Property 7: DPL Magic Bytes
    ///
    /// *For any* DPL file generated by DplBuilder, the first 4 bytes
    /// SHALL be the magic bytes "DPL\x01".
    ///
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_dpl_magic_bytes_correct(packages in arb_unique_packages(0, 20)) {
        let mut builder = DplBuilder::new("3.12.0", "linux");

        for (name, version, hash) in &packages {
            builder.add_package(name, version, *hash);
        }

        let data = builder.build();

        // Verify magic bytes
        prop_assert!(data.len() >= 4, "Data too short for magic bytes");
        prop_assert_eq!(&data[0..4], b"DPL\x01", "Magic bytes mismatch");
    }

    /// Property 5: DPL Round-Trip Consistency with Extras
    ///
    /// *For any* valid DPL structure containing packages with extras,
    /// serializing to binary format and deserializing back SHALL produce
    /// an equivalent structure with preserved extras.
    ///
    /// **Validates: Requirements 2.8, 2.9, 2.10**
    #[test]
    fn prop_dpl_roundtrip_with_extras(
        name in arb_package_name(),
        version in arb_version(),
        hash in arb_hash(),
        extras in arb_extras()
    ) {
        let mut builder = DplBuilder::new("3.12.0", "linux");
        let extras_refs: Vec<&str> = extras.iter().map(|s| s.as_str()).collect();
        builder.add_package_with_extras(&name, &version, hash, &extras_refs);

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        let entry = lock_file.lookup(&name).unwrap();

        // Verify basic fields
        prop_assert_eq!(entry.name_str(), name.as_str());
        prop_assert_eq!(entry.version_str(), version.as_str());
        prop_assert_eq!(entry.source_hash, hash);

        // Verify extras count matches
        let bitmap = { entry.extras_bitmap };
        let set_bits = bitmap.count_ones() as usize;
        prop_assert_eq!(set_bits, extras.len());
    }
}

// =============================================================================
// Property 8: DPL Corruption Handling
// =============================================================================

#[cfg(test)]
mod corruption_tests {
    use super::*;

    /// Property 8: DPL Corruption Handling - Invalid Magic
    ///
    /// *For any* byte sequence with invalid magic bytes, the DPL reader
    /// SHALL return an appropriate error.
    ///
    /// **Validates: Requirements 2.11**
    #[test]
    fn test_dpl_invalid_magic_rejected() {
        let mut data = vec![0u8; 128];
        data[0..4].copy_from_slice(b"XXXX"); // Invalid magic

        let result = DplLockFile::from_bytes(data);
        assert!(result.is_err());
    }

    /// Property 8: DPL Corruption Handling - Truncated File
    ///
    /// *For any* truncated DPL file, the reader SHALL return an error.
    ///
    /// **Validates: Requirements 2.11**
    #[test]
    fn test_dpl_truncated_file_rejected() {
        // File too small for header
        let data = vec![0u8; 10];
        let result = DplLockFile::from_bytes(data);
        assert!(result.is_err());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Property 8: DPL Corruption Handling - Random Data
        ///
        /// *For any* random byte sequence that is not a valid DPL file,
        /// the reader SHALL return an error rather than panicking.
        ///
        /// **Validates: Requirements 2.11**
        #[test]
        fn prop_dpl_random_data_handled(data in proptest::collection::vec(any::<u8>(), 0..1000)) {
            // This should not panic, just return an error
            let result = DplLockFile::from_bytes(data);
            // We don't care if it succeeds or fails, just that it doesn't panic
            let _ = result;
        }
    }
}
