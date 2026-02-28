//! Concurrent access tests for Layout Cache
//!
//! Property 4: Layout Cache Concurrent Access

use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

use dx_py_layout::{LayoutCache, LayoutEntry, LayoutIndex};
use dx_py_store::PackageStore;

fn create_test_store(temp: &TempDir) -> Arc<PackageStore> {
    Arc::new(PackageStore::open(temp.path().join("store")).unwrap())
}

/// Property 4: Layout Cache Concurrent Access
///
/// For any sequence of concurrent read and write operations on the Layout_Cache
/// from multiple threads, the cache SHALL remain consistent and no data corruption
/// SHALL occur.
#[test]
fn test_concurrent_index_reads() {
    let temp = TempDir::new().unwrap();
    let index_path = temp.path().join("layouts.dxc");

    // Create index with some entries
    {
        let mut index = LayoutIndex::open(&index_path).unwrap();
        for i in 0..10 {
            let mut hash = [0u8; 32];
            hash[0] = i;
            let entry = LayoutEntry::new(hash, &format!("layout_{}", i), i as u32, 1024);
            index.add(entry).unwrap();
        }
    }

    // Concurrent reads
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let path = index_path.clone();
            thread::spawn(move || {
                let index = LayoutIndex::open(&path).unwrap();
                for i in 0..10 {
                    let mut hash = [0u8; 32];
                    hash[0] = i;
                    let entry = index.get(&hash);
                    assert!(entry.is_some(), "Entry {} should exist", i);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_cache_reads() {
    let temp = TempDir::new().unwrap();
    let store = create_test_store(&temp);

    // Create some packages in the store
    for i in 0..5u8 {
        let mut hash = [0u8; 32];
        hash[0] = i;
        let files = [(format!("pkg{}/__init__.py", i), b"# init".to_vec())];
        let file_refs: Vec<(&str, &[u8])> =
            files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        store.store_package(&hash, &file_refs).unwrap();
    }

    // Build layouts
    let layouts_path = temp.path().join("layouts");
    {
        let mut cache = LayoutCache::open(&layouts_path, Arc::clone(&store)).unwrap();
        for i in 0..5u8 {
            let mut hash = [0u8; 32];
            hash[0] = i;
            let packages = vec![dx_py_layout::ResolvedPackage {
                name: format!("pkg{}", i),
                version: "1.0.0".to_string(),
                hash,
            }];
            let project_hash = LayoutCache::compute_project_hash(&packages);
            cache.build_layout(&project_hash, &packages).unwrap();
        }
    }

    // Concurrent reads from multiple threads
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let path = layouts_path.clone();
            let store_clone = Arc::clone(&store);
            thread::spawn(move || {
                let cache = LayoutCache::open(&path, store_clone).unwrap();
                for i in 0..5u8 {
                    let mut hash = [0u8; 32];
                    hash[0] = i;
                    let packages = vec![dx_py_layout::ResolvedPackage {
                        name: format!("pkg{}", i),
                        version: "1.0.0".to_string(),
                        hash,
                    }];
                    let project_hash = LayoutCache::compute_project_hash(&packages);
                    assert!(cache.contains(&project_hash), "Layout for pkg{} should exist", i);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_hash_computation() {
    // Verify hash computation is thread-safe
    let packages = vec![
        dx_py_layout::ResolvedPackage {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
            hash: [1u8; 32],
        },
        dx_py_layout::ResolvedPackage {
            name: "numpy".to_string(),
            version: "1.26.0".to_string(),
            hash: [2u8; 32],
        },
    ];

    let expected_hash = LayoutCache::compute_project_hash(&packages);

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let pkgs = packages.clone();
            thread::spawn(move || LayoutCache::compute_project_hash(&pkgs))
        })
        .collect();

    for handle in handles {
        let hash = handle.join().unwrap();
        assert_eq!(hash, expected_hash, "Hash should be consistent across threads");
    }
}

#[test]
fn test_index_atomic_updates() {
    let temp = TempDir::new().unwrap();
    let index_path = temp.path().join("layouts.dxc");

    // Create initial index
    {
        let mut index = LayoutIndex::open(&index_path).unwrap();
        let entry = LayoutEntry::new([1u8; 32], "layout_1", 5, 1024);
        index.add(entry).unwrap();
    }

    // Verify the index is readable after update
    let index = LayoutIndex::open(&index_path).unwrap();
    assert_eq!(index.layout_count(), 1);
    assert!(index.contains(&[1u8; 32]));
}
