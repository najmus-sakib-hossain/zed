//! Property test for revert round-trip
//!
//! This test verifies that for any set of file changes applied via `apply_changes()`,
//! calling `revert_most_recent()` restores all affected files to their exact previous state.

use chrono::Utc;
use dx_forge::core::branching_engine::{BranchingEngine, FileChange};
use proptest::prelude::*;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: forge-production-ready, Property 4: Revert Round-Trip
    /// For any set of file changes applied via `apply_changes()`, calling `revert_most_recent()`
    /// SHALL restore all affected files to their exact previous state.
    /// **Validates: Requirements 2.9**
    #[test]
    fn prop_revert_roundtrip_single_file(
        original_content in "[a-zA-Z0-9\\s]{10,1000}",
        new_content in "[a-zA-Z0-9\\s]{10,1000}",
    ) {
        // Create a temporary directory for isolated testing
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write original content
        std::fs::write(&file_path, &original_content).unwrap();

        // Create a new branching engine instance
        let mut engine = BranchingEngine::new();

        // Create a file change
        let change = FileChange {
            path: file_path.clone(),
            old_content: Some(original_content.as_bytes().to_vec()),
            new_content: new_content.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Apply change
        let applied = engine.apply_changes(vec![change]).unwrap();
        prop_assert_eq!(applied.len(), 1);
        prop_assert_eq!(&applied[0], &file_path);

        // Verify new content was written
        let current_content = std::fs::read_to_string(&file_path).unwrap();
        prop_assert_eq!(&current_content, &new_content);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        prop_assert_eq!(reverted.len(), 1);
        prop_assert_eq!(&reverted[0], &file_path);

        // Verify original content restored
        let restored_content = std::fs::read_to_string(&file_path).unwrap();
        prop_assert_eq!(&restored_content, &original_content);
    }

    /// Property 4: Revert Round-Trip - Multiple files
    /// For any set of multiple file changes, reverting restores all files to their original state.
    /// **Validates: Requirements 2.9**
    #[test]
    fn prop_revert_roundtrip_multiple_files(
        original1 in "[a-zA-Z0-9]{10,200}",
        original2 in "[a-zA-Z0-9]{10,200}",
        new1 in "[a-zA-Z0-9]{10,200}",
        new2 in "[a-zA-Z0-9]{10,200}",
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        // Write original content to both files
        std::fs::write(&file1, &original1).unwrap();
        std::fs::write(&file2, &original2).unwrap();

        let mut engine = BranchingEngine::new();

        // Create changes for both files
        let changes = vec![
            FileChange {
                path: file1.clone(),
                old_content: Some(original1.as_bytes().to_vec()),
                new_content: new1.as_bytes().to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file2.clone(),
                old_content: Some(original2.as_bytes().to_vec()),
                new_content: new2.as_bytes().to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Apply changes
        let applied = engine.apply_changes(changes).unwrap();
        prop_assert_eq!(applied.len(), 2);

        // Verify new content
        prop_assert_eq!(std::fs::read_to_string(&file1).unwrap(), new1);
        prop_assert_eq!(std::fs::read_to_string(&file2).unwrap(), new2);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        prop_assert_eq!(reverted.len(), 2);

        // Verify original content restored for both files
        prop_assert_eq!(std::fs::read_to_string(&file1).unwrap(), original1);
        prop_assert_eq!(std::fs::read_to_string(&file2).unwrap(), original2);
    }

    /// Property 4: Revert Round-Trip - New file creation
    /// For newly created files, reverting should delete them.
    /// **Validates: Requirements 2.9**
    #[test]
    fn prop_revert_roundtrip_new_file(
        new_content in "[a-zA-Z0-9\\s]{10,500}",
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        // File doesn't exist initially
        prop_assert!(!file_path.exists());

        let mut engine = BranchingEngine::new();

        // Create a file change for a new file (no old_content)
        let change = FileChange {
            path: file_path.clone(),
            old_content: None,
            new_content: new_content.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Apply change
        let applied = engine.apply_changes(vec![change]).unwrap();
        prop_assert_eq!(applied.len(), 1);

        // Verify file was created
        prop_assert!(file_path.exists());
        prop_assert_eq!(std::fs::read_to_string(&file_path).unwrap(), new_content);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        prop_assert_eq!(reverted.len(), 1);

        // Verify file was deleted
        prop_assert!(!file_path.exists());
    }

    /// Property 4: Revert Round-Trip - Nested directory
    /// For files in nested directories, reverting should restore them correctly.
    /// **Validates: Requirements 2.9**
    #[test]
    fn prop_revert_roundtrip_nested_directory(
        original_content in "[a-zA-Z0-9]{10,200}",
        new_content in "[a-zA-Z0-9]{10,200}",
        dir_depth in 1usize..4,
    ) {
        let temp_dir = TempDir::new().unwrap();

        // Create nested path
        let mut nested_path = temp_dir.path().to_path_buf();
        for i in 0..dir_depth {
            nested_path = nested_path.join(format!("dir{}", i));
        }
        let file_path = nested_path.join("test.txt");

        // Create directories and write original content
        std::fs::create_dir_all(nested_path).unwrap();
        std::fs::write(&file_path, &original_content).unwrap();

        let mut engine = BranchingEngine::new();

        // Create a file change
        let change = FileChange {
            path: file_path.clone(),
            old_content: Some(original_content.as_bytes().to_vec()),
            new_content: new_content.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Apply change
        let applied = engine.apply_changes(vec![change]).unwrap();
        prop_assert_eq!(applied.len(), 1);

        // Verify new content
        prop_assert_eq!(std::fs::read_to_string(&file_path).unwrap(), new_content);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        prop_assert_eq!(reverted.len(), 1);

        // Verify original content restored
        prop_assert_eq!(std::fs::read_to_string(&file_path).unwrap(), original_content);
    }

    /// Property 4: Revert Round-Trip - Binary content
    /// For binary file content, reverting should restore exact bytes.
    /// **Validates: Requirements 2.9**
    #[test]
    fn prop_revert_roundtrip_binary_content(
        original_bytes in prop::collection::vec(any::<u8>(), 10..500),
        new_bytes in prop::collection::vec(any::<u8>(), 10..500),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");

        // Write original binary content
        std::fs::write(&file_path, &original_bytes).unwrap();

        let mut engine = BranchingEngine::new();

        // Create a file change with binary content
        let change = FileChange {
            path: file_path.clone(),
            old_content: Some(original_bytes.clone()),
            new_content: new_bytes.clone(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Apply change
        let applied = engine.apply_changes(vec![change]).unwrap();
        prop_assert_eq!(applied.len(), 1);

        // Verify new content
        let current_bytes = std::fs::read(&file_path).unwrap();
        prop_assert_eq!(&current_bytes, &new_bytes);

        // Revert
        let reverted = engine.revert_most_recent().unwrap();
        prop_assert_eq!(reverted.len(), 1);

        // Verify original content restored exactly
        let restored_bytes = std::fs::read(&file_path).unwrap();
        prop_assert_eq!(&restored_bytes, &original_bytes);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_basic_revert_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let original = "original content";
        let new = "new content";

        std::fs::write(&file_path, original).unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: file_path.clone(),
            old_content: Some(original.as_bytes().to_vec()),
            new_content: new.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), new);

        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), original);
    }

    #[test]
    fn test_revert_no_application_fails() {
        let mut engine = BranchingEngine::new();
        let result = engine.revert_most_recent();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No recent application to revert"));
    }

    #[test]
    fn test_revert_file_deleted_before_revert() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let original = "original content";
        let new = "new content";

        std::fs::write(&file_path, original).unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: file_path.clone(),
            old_content: Some(original.as_bytes().to_vec()),
            new_content: new.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();

        // Delete the file before reverting
        std::fs::remove_file(&file_path).unwrap();
        assert!(!file_path.exists());

        // Revert should still work and restore the file
        engine.revert_most_recent().unwrap();
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), original);
    }
}
