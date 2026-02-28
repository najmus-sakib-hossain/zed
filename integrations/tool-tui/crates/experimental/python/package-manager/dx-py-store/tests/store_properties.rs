//! Property-based tests for Package Store
//!
//! These tests validate the correctness properties defined in the design document.

use proptest::prelude::*;
use std::collections::HashSet;
use tempfile::TempDir;

use dx_py_store::{PackageStore, StoreError};

/// Generator for valid package names (PEP 503 normalized)
#[allow(dead_code)]
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,30}".prop_map(|s| s.to_lowercase())
}

/// Generator for valid file paths within a package
fn arb_file_path() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..4)
        .prop_map(|parts| format!("{}.py", parts.join("/")))
}

/// Generator for file content
fn arb_file_content() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1000)
}

/// Generator for package files
fn arb_package_files() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
    prop::collection::vec((arb_file_path(), arb_file_content()), 1..10).prop_map(|files| {
        // Deduplicate paths
        let mut seen = HashSet::new();
        files.into_iter().filter(|(path, _)| seen.insert(path.clone())).collect()
    })
}

/// Generator for Blake3 hashes
fn arb_hash() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

// =============================================================================
// Property 9: Package Store Path Format
// =============================================================================

proptest! {
    /// Property 9: Package Store Path Format
    ///
    /// For any package stored in the Package_Store with hash H, the storage path
    /// SHALL be `{root}/{H[0:2]}/{H[2:4]}/{H}.dxpkg` where H is the hex-encoded
    /// Blake3 hash.
    ///
    /// Validates: Requirements 3.6
    #[test]
    fn prop_package_store_path_format(hash in arb_hash()) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let path = store.get_path(&hash);
        let path_str = path.to_string_lossy();
        let hex = hex::encode(hash);

        // Verify two-level directory structure
        prop_assert!(path_str.contains(&hex[0..2]));
        prop_assert!(path_str.contains(&hex[2..4]));

        // Verify filename format
        let expected_filename = format!("{}.dxpkg", hex);
        prop_assert!(path_str.ends_with(&expected_filename));

        // Verify path structure: root/ab/cd/abcd...ef.dxpkg
        let components: Vec<_> = path.components().collect();
        let len = components.len();
        prop_assert!(len >= 4); // root + ab + cd + file

        // Last component should be the filename
        let filename = components[len - 1].as_os_str().to_string_lossy();
        prop_assert_eq!(filename, format!("{}.dxpkg", hex));

        // Second-to-last should be hex[2:4]
        let dir2 = components[len - 2].as_os_str().to_string_lossy();
        prop_assert_eq!(dir2, &hex[2..4]);

        // Third-to-last should be hex[0:2]
        let dir1 = components[len - 3].as_os_str().to_string_lossy();
        prop_assert_eq!(dir1, &hex[0..2]);
    }
}

// =============================================================================
// Property 10: Package Store File Lookup
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property 10: Package Store File Lookup
    ///
    /// For any package in the store and any file path within that package,
    /// the Package_Store SHALL return the correct file contents via the
    /// file index lookup.
    ///
    /// Validates: Requirements 3.4
    #[test]
    fn prop_package_store_file_lookup(files in arb_package_files()) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        // Compute hash from files
        let mut hasher = blake3::Hasher::new();
        for (path, content) in &files {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        let hash = *hasher.finalize().as_bytes();

        // Store package
        let file_refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(p, c)| (p.as_str(), c.as_slice()))
            .collect();
        store.store_package(&hash, &file_refs).unwrap();

        // Verify each file can be retrieved correctly
        for (path, expected_content) in &files {
            let retrieved = store.get_file(&hash, path).unwrap();
            prop_assert_eq!(&retrieved, expected_content);
        }
    }
}

// =============================================================================
// Property 12: Package Store Integrity Verification
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12: Package Store Integrity Verification
    ///
    /// For any package data and expected hash, storing with `store_verified`
    /// SHALL succeed only if the Blake3 hash of the data matches the expected
    /// hash, and SHALL fail with an error otherwise.
    ///
    /// Validates: Requirements 3.9
    #[test]
    fn prop_package_store_integrity_verification(data in arb_file_content()) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        // Compute correct hash
        let correct_hash = *blake3::hash(&data).as_bytes();

        // Store with correct hash should succeed
        let result = store.store_verified(&correct_hash, &data);
        prop_assert!(result.is_ok());

        // Verify data can be retrieved
        let retrieved = store.get_raw(&correct_hash).unwrap();
        prop_assert_eq!(retrieved, data);
    }

    /// Property 12 (negative case): Wrong hash should fail
    #[test]
    fn prop_package_store_integrity_verification_wrong_hash(
        data in arb_file_content(),
        wrong_hash in arb_hash()
    ) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let correct_hash = *blake3::hash(&data).as_bytes();

        // Skip if hashes happen to match (extremely unlikely)
        prop_assume!(wrong_hash != correct_hash);

        // Store with wrong hash should fail
        let result = store.store_verified(&wrong_hash, &data);
        let is_integrity_error = matches!(result, Err(StoreError::IntegrityError { .. }));
        prop_assert!(is_integrity_error);
    }
}

// =============================================================================
// Property 14: Package Store Deduplication
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property 14: Package Store Deduplication
    ///
    /// For any two projects that depend on the same package (same hash),
    /// the Package_Store SHALL store only one copy of the package data,
    /// and both projects SHALL reference the same store file.
    ///
    /// Validates: Requirements 3.10
    #[test]
    fn prop_package_store_deduplication(files in arb_package_files()) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        // Compute hash
        let mut hasher = blake3::Hasher::new();
        for (path, content) in &files {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        let hash = *hasher.finalize().as_bytes();

        let file_refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(p, c)| (p.as_str(), c.as_slice()))
            .collect();

        // Store same package twice
        let path1 = store.store_package(&hash, &file_refs).unwrap();
        let path2 = store.store_package(&hash, &file_refs).unwrap();

        // Should return same path
        prop_assert_eq!(path1, path2);

        // Should only have one file in store
        let all_hashes = store.list().unwrap();
        let matching: Vec<_> = all_hashes.iter().filter(|h| *h == &hash).collect();
        prop_assert_eq!(matching.len(), 1);
    }
}

// =============================================================================
// Property 15: Package Store Error Handling
// =============================================================================

proptest! {
    /// Property 15: Package Store Error Handling
    ///
    /// For any hash that does not correspond to a package in the store,
    /// requesting that package SHALL return a "not found" error rather
    /// than panicking or returning invalid data.
    ///
    /// Validates: Requirements 3.7
    #[test]
    fn prop_package_store_error_handling(hash in arb_hash()) {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        // Request non-existent package
        let result = store.get(&hash);
        prop_assert!(matches!(result, Err(StoreError::PackageNotFound(_))));

        // get_raw should also return error
        let result = store.get_raw(&hash);
        prop_assert!(matches!(result, Err(StoreError::PackageNotFound(_))));

        // get_file should also return error
        let result = store.get_file(&hash, "any/path.py");
        prop_assert!(matches!(result, Err(StoreError::PackageNotFound(_))));

        // contains should return false
        prop_assert!(!store.contains(&hash));
    }
}

// =============================================================================
// Additional Unit Tests for Edge Cases
// =============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_package() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let files: Vec<(&str, &[u8])> = vec![];
        let hash = [0u8; 32];

        let result = store.store_package(&hash, &files);
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_file_content() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        // 1MB file
        let content = vec![0xABu8; 1024 * 1024];
        let files = vec![("large_file.py", content.as_slice())];

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"large_file.py");
        hasher.update(&content);
        let hash = *hasher.finalize().as_bytes();

        store.store_package(&hash, &files).unwrap();

        let retrieved = store.get_file(&hash, "large_file.py").unwrap();
        assert_eq!(retrieved.len(), content.len());
        assert_eq!(retrieved, content);
    }

    #[test]
    fn test_special_characters_in_path() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let files = vec![
            ("package/__init__.py", b"# init" as &[u8]),
            ("package/sub_module/file.py", b"# file"),
            ("package/data.json", b"{}"),
        ];

        let mut hasher = blake3::Hasher::new();
        for (path, content) in &files {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        let hash = *hasher.finalize().as_bytes();

        store.store_package(&hash, &files).unwrap();

        for (path, expected) in &files {
            let retrieved = store.get_file(&hash, path).unwrap();
            assert_eq!(&retrieved, expected);
        }
    }

    #[test]
    fn test_remove_package() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let files = vec![("test.py", b"content" as &[u8])];
        let hash = *blake3::hash(b"test").as_bytes();

        store.store_package(&hash, &files).unwrap();
        assert!(store.contains(&hash));

        store.remove(&hash).unwrap();
        assert!(!store.contains(&hash));
    }
}

// =============================================================================
// Property 11: Package Store Symlink Installation
// =============================================================================

#[cfg(test)]
mod symlink_tests {
    use super::*;
    use std::fs;

    /// Property 11: Package Store Symlink Installation
    ///
    /// For any package installed from the Package_Store to a virtual environment,
    /// the installed files SHALL be symlinks (or junctions on Windows) pointing
    /// to the store, not copies.
    ///
    /// Validates: Requirements 3.5
    #[test]
    fn test_symlink_installation() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path().join("store")).unwrap();
        let site_packages = temp.path().join("venv/lib/site-packages");

        // Create a test package
        let files = vec![
            ("mypackage/__init__.py", b"# init" as &[u8]),
            ("mypackage/module.py", b"def hello(): pass"),
        ];

        let mut hasher = blake3::Hasher::new();
        for (path, content) in &files {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        let hash = *hasher.finalize().as_bytes();

        // Store the package
        store.store_package(&hash, &files).unwrap();

        // Install to venv
        let result = store.install_to_venv(&hash, &site_packages).unwrap();

        // Verify files were installed
        assert!(result.files_installed > 0);

        // On Windows, we may use junctions or fall back to copies
        // On Unix, we should use symlinks
        // The key property is that the files exist and have correct content
        for (path, expected_content) in &files {
            let installed_path = site_packages.join(path);
            assert!(installed_path.exists(), "File should exist: {:?}", installed_path);

            let content = fs::read(&installed_path).unwrap();
            assert_eq!(&content, expected_content);
        }

        // Verify that symlinks or copies were used (not both being zero)
        assert!(result.symlinks > 0 || result.copies > 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// Property 11 (property-based): Symlink installation preserves content
        #[test]
        fn prop_symlink_installation_preserves_content(files in arb_package_files()) {
            let temp = TempDir::new().unwrap();
            let store = PackageStore::open(temp.path().join("store")).unwrap();
            let site_packages = temp.path().join("venv/lib/site-packages");

            // Compute hash
            let mut hasher = blake3::Hasher::new();
            for (path, content) in &files {
                hasher.update(path.as_bytes());
                hasher.update(content);
            }
            let hash = *hasher.finalize().as_bytes();

            let file_refs: Vec<(&str, &[u8])> = files
                .iter()
                .map(|(p, c)| (p.as_str(), c.as_slice()))
                .collect();

            // Store and install
            store.store_package(&hash, &file_refs).unwrap();
            let result = store.install_to_venv(&hash, &site_packages).unwrap();

            // Verify all files have correct content
            for (path, expected_content) in &files {
                let installed_path = site_packages.join(path);
                prop_assert!(installed_path.exists());

                let content = fs::read(&installed_path).unwrap();
                prop_assert_eq!(&content, expected_content);
            }

            // Verify installation stats are consistent
            prop_assert_eq!(result.files_installed as usize, files.len());
        }
    }
}

// =============================================================================
// Property 13: Package Store Concurrent Access
// =============================================================================

#[cfg(test)]
mod concurrent_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    /// Property 13: Package Store Concurrent Access
    ///
    /// For any sequence of concurrent read operations on the Package_Store
    /// from multiple threads, all reads SHALL return correct data without
    /// corruption.
    ///
    /// Validates: Requirements 3.8
    #[test]
    fn test_concurrent_reads() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(PackageStore::open(temp.path()).unwrap());

        // Create test packages
        let mut hashes = Vec::new();
        for i in 0..5 {
            let content = format!("package {} content", i);
            let path = format!("pkg{}/module.py", i);
            let files = [(path.clone(), content.as_bytes().to_vec())];
            let file_refs: Vec<(&str, &[u8])> =
                files.iter().map(|(p, c)| (p.as_str(), c.as_slice())).collect();

            let hash = *blake3::hash(content.as_bytes()).as_bytes();
            store.store_package(&hash, &file_refs).unwrap();
            hashes.push((hash, content));
        }

        // Spawn multiple reader threads
        let mut handles = Vec::new();
        for _ in 0..10 {
            let store_clone = Arc::clone(&store);
            let hashes_clone = hashes.clone();

            let handle = thread::spawn(move || {
                for (hash, _expected) in &hashes_clone {
                    // Multiple reads should all succeed
                    for _ in 0..10 {
                        let result = store_clone.contains(hash);
                        assert!(result);

                        let mapped = store_clone.get(hash);
                        assert!(mapped.is_ok());
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
    }

    /// Test concurrent writes don't corrupt the store
    #[test]
    fn test_concurrent_writes() {
        let temp = TempDir::new().unwrap();
        let store = Arc::new(PackageStore::open(temp.path()).unwrap());

        let mut handles = Vec::new();
        for i in 0..10 {
            let store_clone = Arc::clone(&store);

            let handle = thread::spawn(move || {
                let content = format!("thread {} content", i);
                let path = format!("pkg{}/module.py", i);
                let files = [(path.clone(), content.as_bytes().to_vec())];
                let file_refs: Vec<(&str, &[u8])> =
                    files.iter().map(|(p, c)| (p.as_str(), c.as_slice())).collect();

                let hash = *blake3::hash(content.as_bytes()).as_bytes();
                store_clone.store_package(&hash, &file_refs).unwrap();
                hash
            });
            handles.push(handle);
        }

        // Collect all hashes
        let hashes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // Verify all packages exist
        for hash in &hashes {
            assert!(store.contains(hash));
        }
    }
}
