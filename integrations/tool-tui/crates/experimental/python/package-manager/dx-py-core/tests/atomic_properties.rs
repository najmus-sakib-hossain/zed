//! Property-based tests for atomic file operations
//!
//! **Property 8: Cleanup on Failure**
//! *For any* operation that fails, the system SHALL leave no partial state
//! (corrupted cache entries, incomplete venvs, etc.).
//!
//! **Validates: Requirements 5.6**

use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

use dx_py_core::{atomic_write, AtomicDir, AtomicFile, CleanupGuard};

/// Strategy for generating random file content
fn content_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1024)
}

/// Strategy for generating valid file names
fn filename_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9]{2,10}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that uncommitted AtomicFile leaves no partial state.
    /// For any file content, if AtomicFile is dropped without commit,
    /// neither the target nor temp file SHALL exist.
    #[test]
    fn prop_atomic_file_cleanup_on_drop(
        content in content_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join(&filename);

        // Create and write to atomic file, then drop without commit
        {
            let mut atomic = AtomicFile::new(&target).unwrap();
            atomic.write_all(&content).unwrap();
            // Drop without commit
        }

        // Target should not exist
        prop_assert!(!target.exists(), "Target file should not exist after drop");

        // No temp files should remain
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        prop_assert!(
            entries.is_empty(),
            "No temp files should remain after drop"
        );
    }

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that aborted AtomicFile leaves no partial state.
    /// For any file content, if AtomicFile.abort() is called,
    /// neither the target nor temp file SHALL exist.
    #[test]
    fn prop_atomic_file_cleanup_on_abort(
        content in content_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join(&filename);

        // Create, write, and abort
        let mut atomic = AtomicFile::new(&target).unwrap();
        atomic.write_all(&content).unwrap();
        atomic.abort();

        // Target should not exist
        prop_assert!(!target.exists(), "Target file should not exist after abort");

        // No temp files should remain
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        prop_assert!(
            entries.is_empty(),
            "No temp files should remain after abort"
        );
    }

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that committed AtomicFile contains the correct content.
    /// For any file content, if AtomicFile.commit() is called,
    /// the target file SHALL contain exactly the written content.
    #[test]
    fn prop_atomic_file_commit_preserves_content(
        content in content_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join(&filename);

        // Create, write, and commit
        let mut atomic = AtomicFile::new(&target).unwrap();
        atomic.write_all(&content).unwrap();
        atomic.commit().unwrap();

        // Target should exist with correct content
        prop_assert!(target.exists(), "Target file should exist after commit");
        let read_content = fs::read(&target).unwrap();
        prop_assert_eq!(read_content, content, "Content should match");
    }

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that uncommitted AtomicDir leaves no partial state.
    /// For any directory with files, if AtomicDir is dropped without commit,
    /// neither the target nor temp directory SHALL exist.
    #[test]
    fn prop_atomic_dir_cleanup_on_drop(
        content in content_strategy(),
        dirname in filename_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join(&dirname);

        // Create atomic dir, add a file, then drop without commit
        {
            let atomic = AtomicDir::new(&target).unwrap();
            fs::write(atomic.path().join(&filename), &content).unwrap();
            // Drop without commit
        }

        // Target should not exist
        prop_assert!(!target.exists(), "Target dir should not exist after drop");

        // No temp directories should remain (only the temp_dir itself)
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        prop_assert!(
            entries.is_empty(),
            "No temp directories should remain after drop"
        );
    }

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that CleanupGuard removes the path on drop.
    /// For any file, if CleanupGuard is dropped without disarm,
    /// the file SHALL be removed.
    #[test]
    fn prop_cleanup_guard_removes_on_drop(
        content in content_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(&filename);

        // Create file
        fs::write(&path, &content).unwrap();
        prop_assert!(path.exists(), "File should exist before guard");

        // Create guard and drop
        {
            let _guard = CleanupGuard::new(path.clone());
        }

        // File should be removed
        prop_assert!(!path.exists(), "File should be removed after guard drop");
    }

    /// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
    ///
    /// Test that disarmed CleanupGuard preserves the path.
    /// For any file, if CleanupGuard is disarmed before drop,
    /// the file SHALL remain.
    #[test]
    fn prop_cleanup_guard_preserves_when_disarmed(
        content in content_strategy(),
        filename in filename_strategy(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(&filename);

        // Create file
        fs::write(&path, &content).unwrap();
        prop_assert!(path.exists(), "File should exist before guard");

        // Create guard, disarm, and drop
        {
            let mut guard = CleanupGuard::new(path.clone());
            guard.disarm();
        }

        // File should still exist
        prop_assert!(path.exists(), "File should remain after disarmed guard drop");
    }
}

/// **Feature: dx-py-hardening, Property 8: Cleanup on Failure**
///
/// Test that atomic_write is atomic - either succeeds completely or leaves no trace.
#[test]
fn test_atomic_write_is_atomic() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.txt");

    // Successful write
    atomic_write(&target, b"hello world").unwrap();
    assert!(target.exists());
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello world");

    // Overwrite
    atomic_write(&target, b"new content").unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "new content");
}

/// Test that AtomicDir commit preserves all files
#[test]
fn test_atomic_dir_preserves_files() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test_dir");

    let atomic = AtomicDir::new(&target).unwrap();
    fs::write(atomic.path().join("file1.txt"), "content1").unwrap();
    fs::write(atomic.path().join("file2.txt"), "content2").unwrap();
    fs::create_dir(atomic.path().join("subdir")).unwrap();
    fs::write(atomic.path().join("subdir").join("file3.txt"), "content3").unwrap();
    atomic.commit().unwrap();

    assert!(target.exists());
    assert!(target.join("file1.txt").exists());
    assert!(target.join("file2.txt").exists());
    assert!(target.join("subdir").join("file3.txt").exists());
}
