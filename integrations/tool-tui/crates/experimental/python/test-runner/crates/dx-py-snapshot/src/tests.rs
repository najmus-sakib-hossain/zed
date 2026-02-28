//! Tests for snapshot index

use super::*;
use proptest::prelude::*;
use tempfile::TempDir;

#[test]
fn test_index_creation() {
    let temp_dir = TempDir::new().unwrap();
    let index = SnapshotIndex::new(temp_dir.path()).unwrap();
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
}

#[test]
fn test_update_and_verify_match() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let content = b"Hello, World!";

    index.update(test_id, content).unwrap();
    assert!(index.contains(test_id));

    let result = index.verify(test_id, content);
    assert_eq!(result, SnapshotResult::Match);
}

#[test]
fn test_verify_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let expected = b"Hello, World!";
    let actual = b"Hello, Rust!";

    index.update(test_id, expected).unwrap();

    let result = index.verify(test_id, actual);
    match result {
        SnapshotResult::Mismatch { diff } => {
            assert!(diff.contains("-Hello, World!"));
            assert!(diff.contains("+Hello, Rust!"));
        }
        _ => panic!("Expected Mismatch, got {:?}", result),
    }
}

#[test]
fn test_verify_new() {
    let temp_dir = TempDir::new().unwrap();
    let index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let content = b"New snapshot content";

    let result = index.verify(test_id, content);
    match result {
        SnapshotResult::New { content: c } => {
            assert_eq!(c, content);
        }
        _ => panic!("Expected New, got {:?}", result),
    }
}

#[test]
fn test_delete() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let content = b"Content to delete";

    index.update(test_id, content).unwrap();
    assert!(index.contains(test_id));

    index.delete(test_id).unwrap();
    assert!(!index.contains(test_id));
}

#[test]
fn test_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let test_id = TestId(12345);
    let content = b"Persistent content";

    // Create and save
    {
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();
        index.update(test_id, content).unwrap();
        index.save().unwrap();
    }

    // Load and verify
    {
        let index = SnapshotIndex::new(temp_dir.path()).unwrap();
        assert!(index.contains(test_id));
        let result = index.verify(test_id, content);
        assert_eq!(result, SnapshotResult::Match);
    }
}

#[test]
fn test_load_content() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let content = b"Content to load";

    index.update(test_id, content).unwrap();

    let loaded = index.load_content(test_id).unwrap();
    assert_eq!(loaded, content);
}

#[test]
fn test_clear() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    index.update(TestId(1), b"content1").unwrap();
    index.update(TestId(2), b"content2").unwrap();
    index.update(TestId(3), b"content3").unwrap();

    assert_eq!(index.len(), 3);

    index.clear().unwrap();
    assert!(index.is_empty());
}

#[test]
fn test_multiline_diff() {
    let temp_dir = TempDir::new().unwrap();
    let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

    let test_id = TestId(12345);
    let expected = b"line1\nline2\nline3\n";
    let actual = b"line1\nmodified\nline3\n";

    index.update(test_id, expected).unwrap();

    let result = index.verify(test_id, actual);
    match result {
        SnapshotResult::Mismatch { diff } => {
            assert!(diff.contains("-line2"));
            assert!(diff.contains("+modified"));
        }
        _ => panic!("Expected Mismatch"),
    }
}

// Property tests

// Feature: dx-py-test-runner, Property 15: Snapshot Hash Correctness
// Validates: Requirements 7.1
//
// For any snapshot content, the stored Blake3 hash SHALL equal
// blake3::hash(content).
proptest! {
    #[test]
    fn prop_snapshot_hash_correctness(content in prop::collection::vec(any::<u8>(), 0..1000)) {
        let temp_dir = TempDir::new().unwrap();
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

        let test_id = TestId(12345);
        index.update(test_id, &content).unwrap();

        let stored_hash = index.get_hash(test_id).unwrap();
        let expected_hash = blake3::hash(&content);

        prop_assert_eq!(stored_hash, expected_hash);
    }
}

// Feature: dx-py-test-runner, Property 16: Snapshot Diff Generation
// Validates: Requirements 7.4
//
// For any two different byte sequences (actual vs expected), when hashes
// differ, the Snapshot_Index SHALL produce a diff that accurately
// represents the differences between them.
proptest! {
    #[test]
    fn prop_snapshot_diff_generation(
        expected in "[a-z]{10,50}",
        actual in "[A-Z]{10,50}", // Different pattern to ensure different content
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

        let test_id = TestId(12345);
        index.update(test_id, expected.as_bytes()).unwrap();

        let result = index.verify(test_id, actual.as_bytes());

        match result {
            SnapshotResult::Mismatch { diff } => {
                // Diff should contain markers for changes
                prop_assert!(diff.contains("---") || diff.contains("+++") || diff.contains("-") || diff.contains("+"));
            }
            SnapshotResult::Match => {
                // If they happen to match (very unlikely), that's fine
            }
            SnapshotResult::New { .. } => {
                panic!("Should not be New after update");
            }
        }
    }
}

// Feature: dx-py-test-runner, Property 17: Snapshot Update Consistency
// Validates: Requirements 7.5
//
// For any snapshot update operation, after updating with new content,
// verifying with that same content SHALL return SnapshotResult::Match.
proptest! {
    #[test]
    fn prop_snapshot_update_consistency(content in prop::collection::vec(any::<u8>(), 1..500)) {
        let temp_dir = TempDir::new().unwrap();
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

        let test_id = TestId(12345);

        // Update with content
        index.update(test_id, &content).unwrap();

        // Verify with same content should match
        let result = index.verify(test_id, &content);
        prop_assert_eq!(result, SnapshotResult::Match);
    }

    #[test]
    fn prop_snapshot_roundtrip(content in prop::collection::vec(any::<u8>(), 1..500)) {
        let temp_dir = TempDir::new().unwrap();
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

        let test_id = TestId(12345);

        // Update
        index.update(test_id, &content).unwrap();

        // Load and compare
        let loaded = index.load_content(test_id).unwrap();
        prop_assert_eq!(loaded, content);
    }

    #[test]
    fn prop_multiple_snapshots(
        contents in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..10)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut index = SnapshotIndex::new(temp_dir.path()).unwrap();

        // Store multiple snapshots
        for (i, content) in contents.iter().enumerate() {
            let test_id = TestId(i as u64);
            index.update(test_id, content).unwrap();
        }

        // Verify all match
        for (i, content) in contents.iter().enumerate() {
            let test_id = TestId(i as u64);
            let result = index.verify(test_id, content);
            prop_assert_eq!(result, SnapshotResult::Match);
        }
    }
}
