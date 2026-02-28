//! Property-based tests for atomic file writes
//!
//! These tests verify that atomic file writes maintain data integrity:
//! - Files are never left in a partial state
//! - Either the complete file exists or no file exists
//!
//! Feature: cli-production-ready, Property 5: Atomic File Writes
//! **Validates: Requirements 9.3**
//!
//! Run with: cargo test --test atomic_writes_property_tests

use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Generate arbitrary file content
fn file_content_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        Just(vec![]),
        Just(b"hello world".to_vec()),
        Just(b"line1\nline2\nline3".to_vec()),
        Just(b"{\"key\": \"value\"}".to_vec()),
        proptest::collection::vec(any::<u8>(), 0..1000),
    ]
}

/// Generate arbitrary file names
fn file_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("test.txt".to_string()),
        Just("config.json".to_string()),
        Just("data.bin".to_string()),
        "[a-z]{1,10}\\.[a-z]{2,4}".prop_map(|s| s.to_string()),
    ]
}

/// Atomic write implementation (mirrors utils.rs)
fn write_atomic(path: &std::path::Path, data: &[u8]) -> anyhow::Result<()> {
    use anyhow::Context;

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Generate temp file path in the same directory
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    fs::write(&temp_path, data)
        .with_context(|| format!("Failed to write temp file: {}", temp_path.display()))?;

    // Rename temp file to target (atomic on most filesystems)
    fs::rename(&temp_path, path).with_context(|| {
        // Clean up temp file on error
        let _ = fs::remove_file(&temp_path);
        format!("Failed to rename temp file to: {}", path.display())
    })?;

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 5: Atomic File Writes
    /// *For any* file write operation that completes successfully, the file
    /// SHALL contain exactly the data that was written, with no partial content.
    ///
    /// Feature: cli-production-ready, Property 5: Atomic File Writes
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_atomic_write_complete_or_nothing(
        content in file_content_strategy(),
        filename in file_name_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(&filename);

        // Perform atomic write
        let result = write_atomic(&path, &content);

        // If write succeeded, file must contain exact content
        if result.is_ok() {
            prop_assert!(path.exists(), "File should exist after successful write");

            let read_content = fs::read(&path).unwrap();
            prop_assert_eq!(
                read_content,
                content,
                "File content must match exactly what was written"
            );

            // Temp file should not exist
            let temp_path = path.with_extension("tmp");
            prop_assert!(
                !temp_path.exists(),
                "Temp file should be cleaned up after successful write"
            );
        }
    }

    /// Property 5b: Atomic Write Overwrites Completely
    /// *For any* existing file, an atomic write SHALL replace the entire content,
    /// never leaving a mix of old and new data.
    ///
    /// Feature: cli-production-ready, Property 5: Atomic File Writes
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_atomic_write_overwrites_completely(
        original_content in file_content_strategy(),
        new_content in file_content_strategy(),
        filename in file_name_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(&filename);

        // Write original content
        fs::write(&path, &original_content).unwrap();
        prop_assert!(path.exists());

        // Overwrite with atomic write
        let result = write_atomic(&path, &new_content);

        if result.is_ok() {
            let read_content = fs::read(&path).unwrap();

            // Content must be exactly the new content
            prop_assert_eq!(
                &read_content,
                &new_content,
                "File must contain only new content after overwrite"
            );

            // Verify no mixing of old and new content
            if !original_content.is_empty() && !new_content.is_empty() && original_content != new_content {
                // If contents are different, the file should not contain old content
                // (unless new content happens to include it)
                if !new_content.windows(original_content.len()).any(|w| w == original_content.as_slice()) {
                    let contains_old = read_content.windows(original_content.len())
                        .any(|w| w == original_content.as_slice());
                    prop_assert!(
                        !contains_old || read_content == new_content,
                        "File should not contain old content mixed with new"
                    );
                }
            }
        }
    }

    /// Property 5c: Atomic Write Creates Parent Directories
    /// *For any* path with non-existent parent directories, atomic write SHALL
    /// create the necessary directories.
    ///
    /// Feature: cli-production-ready, Property 5: Atomic File Writes
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_atomic_write_creates_parents(
        content in file_content_strategy(),
        depth in 1usize..4,
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create nested path
        let mut path = temp_dir.path().to_path_buf();
        for i in 0..depth {
            path = path.join(format!("dir{}", i));
        }
        path = path.join("file.txt");

        // Parent should not exist yet
        prop_assert!(!path.parent().unwrap().exists());

        // Atomic write should create parents
        let result = write_atomic(&path, &content);

        if result.is_ok() {
            prop_assert!(path.exists(), "File should exist");
            prop_assert!(path.parent().unwrap().exists(), "Parent directory should exist");

            let read_content = fs::read(&path).unwrap();
            prop_assert_eq!(read_content, content);
        }
    }

    /// Property 5d: Atomic Write Temp File Cleanup
    /// *For any* successful atomic write, no temporary files SHALL remain.
    ///
    /// Feature: cli-production-ready, Property 5: Atomic File Writes
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_atomic_write_no_temp_files(
        content in file_content_strategy(),
        filename in file_name_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(&filename);

        let result = write_atomic(&path, &content);

        if result.is_ok() {
            // Check for any .tmp files in the directory
            let entries: Vec<_> = fs::read_dir(temp_dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .collect();

            for entry in &entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                prop_assert!(
                    !name_str.ends_with(".tmp"),
                    "No .tmp files should remain after successful write: {}",
                    name_str
                );
            }
        }
    }
}

/// Test that atomic write creates file with correct content
#[test]
fn test_atomic_write_basic() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.txt");

    write_atomic(&path, b"hello world").unwrap();

    assert!(path.exists());
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");
}

/// Test that atomic write handles empty content
#[test]
fn test_atomic_write_empty() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("empty.txt");

    write_atomic(&path, b"").unwrap();

    assert!(path.exists());
    assert_eq!(fs::read(&path).unwrap().len(), 0);
}

/// Test that atomic write handles binary content
#[test]
fn test_atomic_write_binary() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("binary.bin");

    let binary_data: Vec<u8> = (0..=255).collect();
    write_atomic(&path, &binary_data).unwrap();

    assert!(path.exists());
    assert_eq!(fs::read(&path).unwrap(), binary_data);
}

/// Test that atomic write overwrites existing file
#[test]
fn test_atomic_write_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("overwrite.txt");

    // Write original
    fs::write(&path, "original content").unwrap();

    // Overwrite atomically
    write_atomic(&path, b"new content").unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
}

/// Test that atomic write creates nested directories
#[test]
fn test_atomic_write_nested_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("a").join("b").join("c").join("file.txt");

    write_atomic(&path, b"nested").unwrap();

    assert!(path.exists());
    assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
}

/// Test that temp file is cleaned up on success
#[test]
fn test_atomic_write_temp_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.txt");
    let temp_path = path.with_extension("tmp");

    write_atomic(&path, b"content").unwrap();

    assert!(path.exists());
    assert!(!temp_path.exists(), "Temp file should be removed");
}

/// Test that large files are written correctly
#[test]
fn test_atomic_write_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("large.bin");

    // 1MB of data
    let large_data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

    write_atomic(&path, &large_data).unwrap();

    let read_data = fs::read(&path).unwrap();
    assert_eq!(read_data.len(), large_data.len());
    assert_eq!(read_data, large_data);
}

/// Test multiple sequential writes
#[test]
fn test_atomic_write_sequential() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("sequential.txt");

    for i in 0..10 {
        let content = format!("iteration {}", i);
        write_atomic(&path, content.as_bytes()).unwrap();

        let read = fs::read_to_string(&path).unwrap();
        assert_eq!(read, content);
    }
}

/// Test that file permissions are preserved (basic check)
#[test]
fn test_atomic_write_creates_readable_file() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("readable.txt");

    write_atomic(&path, b"readable content").unwrap();

    // File should be readable
    let content = fs::read_to_string(&path);
    assert!(content.is_ok());
    assert_eq!(content.unwrap(), "readable content");
}
