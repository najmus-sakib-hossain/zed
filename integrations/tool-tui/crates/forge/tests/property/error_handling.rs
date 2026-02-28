//! Property test for graceful error handling
//!
//! This test verifies that for any invalid input to a public API function,
//! the function returns a `Result::Err` with a descriptive message containing
//! context about what operation failed, rather than panicking.

use dx_forge::core::branching_engine::{BranchingEngine, FileChange};
use dx_forge::core::{Forge, ForgeConfig};
use proptest::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: forge-production-ready, Property 5: Graceful Error Handling
    /// For any invalid input to a public API function, the function SHALL return a
    /// `Result::Err` with a descriptive message containing context about what operation
    /// failed, rather than panicking.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_revert_without_application_returns_error(
        // Generate random state to ensure isolation
        _seed in any::<u64>(),
    ) {
        // Create a fresh branching engine with no prior applications
        let mut engine = BranchingEngine::new();

        // Attempting to revert when there's nothing to revert should return an error
        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.revert_most_recent()
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "revert_most_recent() panicked instead of returning error");

        // Should return an error
        let inner_result = result.unwrap();
        prop_assert!(inner_result.is_err(), "Expected error when reverting with no application");

        // Error should contain context about what failed
        let error_msg = format!("{:?}", inner_result.unwrap_err());
        prop_assert!(
            error_msg.to_lowercase().contains("no recent application") ||
            error_msg.to_lowercase().contains("revert"),
            "Error message should contain context about revert operation: {}", error_msg
        );
    }

    /// Property 5: Graceful Error Handling - Invalid file paths
    /// Applying changes to invalid paths should return errors, not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_invalid_path_returns_error(
        // Generate paths with invalid characters or patterns
        invalid_segment in "[\\x00-\\x1f]{1,5}",
    ) {
        let mut engine = BranchingEngine::new();

        // Create a path with null bytes or control characters (invalid on most filesystems)
        let invalid_path = PathBuf::from(format!("/tmp/test{}/file.txt", invalid_segment));

        let change = FileChange {
            path: invalid_path.clone(),
            old_content: None,
            new_content: b"test content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        // This may succeed or fail depending on the OS, but should never panic
        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.apply_changes(vec![change])
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "apply_changes() panicked with invalid path");
    }

    /// Property 5: Graceful Error Handling - Empty changes list
    /// Applying an empty list of changes should handle gracefully.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_empty_changes_no_panic(
        _seed in any::<u64>(),
    ) {
        let mut engine = BranchingEngine::new();

        // Empty changes list
        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.apply_changes(vec![])
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "apply_changes() panicked with empty changes");

        // Should succeed (empty operation is valid)
        let inner_result = result.unwrap();
        prop_assert!(inner_result.is_ok(), "Empty changes should succeed: {:?}", inner_result);
    }

    /// Property 5: Graceful Error Handling - Non-existent file for backup
    /// Reading a non-existent file for backup during apply should be handled gracefully.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_nonexistent_old_content_no_panic(
        content in "[a-zA-Z0-9]{10,100}",
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut engine = BranchingEngine::new();

        // Create a change for a file that doesn't exist yet (new file creation)
        let new_file = temp_dir.path().join("new_file.txt");

        let change = FileChange {
            path: new_file.clone(),
            old_content: None, // No old content - this is a new file
            new_content: content.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.apply_changes(vec![change])
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "apply_changes() panicked for new file creation");

        // Should succeed
        let inner_result = result.unwrap();
        prop_assert!(inner_result.is_ok(), "New file creation should succeed: {:?}", inner_result);
    }

    /// Property 5: Graceful Error Handling - Forge with invalid project root
    /// Creating Forge with non-existent or invalid paths should return errors.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_forge_invalid_project_root_no_panic(
        // Generate random invalid path components
        random_path in "[a-zA-Z0-9_]{5,20}",
    ) {
        // Try to create Forge with a path that likely doesn't exist
        // Note: This might succeed if the path happens to be creatable
        let nonexistent_path = format!("/nonexistent_root_{}/project", random_path);

        let result = catch_unwind(AssertUnwindSafe(|| {
            Forge::new(&nonexistent_path)
        }));

        // Should not panic - either succeeds or returns error
        prop_assert!(result.is_ok(), "Forge::new() panicked with invalid path");
    }

    /// Property 5: Graceful Error Handling - Vote submission with empty voter ID
    /// Submitting votes with edge case inputs should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_vote_edge_cases_no_panic(
        voter_id in ".*",
        reason in ".*",
        confidence in 0.0f32..=1.0f32,
    ) {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.txt");

        let vote = dx_forge::core::branching_engine::BranchingVote {
            voter_id,
            color: dx_forge::core::branching_engine::BranchColor::Green,
            reason,
            confidence,
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.submit_vote(&file, vote)
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "submit_vote() panicked with edge case inputs");
    }

    /// Property 5: Graceful Error Handling - Register voter with various inputs
    /// Registering voters with edge case inputs should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_register_voter_no_panic(
        voter_id in ".*",
    ) {
        let mut engine = BranchingEngine::new();

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.register_permanent_voter(voter_id)
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "register_permanent_voter() panicked");

        // Should succeed
        let inner_result = result.unwrap();
        prop_assert!(inner_result.is_ok(), "register_permanent_voter() should succeed: {:?}", inner_result);
    }

    /// Property 5: Graceful Error Handling - Issue veto with various inputs
    /// Issuing veto with edge case inputs should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_issue_veto_no_panic(
        voter_id in ".*",
        reason in ".*",
        file_path in "[a-zA-Z0-9_/\\.]{1,100}",
    ) {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from(file_path);

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.issue_immediate_veto(&file, &voter_id, &reason)
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "issue_immediate_veto() panicked");

        // Should succeed
        let inner_result = result.unwrap();
        prop_assert!(inner_result.is_ok(), "issue_immediate_veto() should succeed: {:?}", inner_result);
    }

    /// Property 5: Graceful Error Handling - Predict color for non-existent file
    /// Predicting branch color for files with no votes should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_predict_color_no_panic(
        file_path in "[a-zA-Z0-9_/\\.]{1,100}",
    ) {
        let engine = BranchingEngine::new();
        let file = PathBuf::from(file_path);

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.predict_color(&file)
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "predict_color() panicked");

        // Should return a valid color (default is Green)
        let color = result.unwrap();
        prop_assert!(
            matches!(color,
                dx_forge::core::branching_engine::BranchColor::Green |
                dx_forge::core::branching_engine::BranchColor::Yellow |
                dx_forge::core::branching_engine::BranchColor::Red |
                dx_forge::core::branching_engine::BranchColor::NoOpinion
            ),
            "predict_color() should return a valid BranchColor"
        );
    }

    /// Property 5: Graceful Error Handling - Reset state multiple times
    /// Resetting state multiple times should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_reset_state_no_panic(
        reset_count in 1usize..10,
    ) {
        let mut engine = BranchingEngine::new();

        for _ in 0..reset_count {
            let result = catch_unwind(AssertUnwindSafe(|| {
                engine.reset_state()
            }));

            // Should not panic
            prop_assert!(result.is_ok(), "reset_state() panicked");

            // Should succeed
            let inner_result = result.unwrap();
            prop_assert!(inner_result.is_ok(), "reset_state() should succeed: {:?}", inner_result);
        }
    }

    /// Property 5: Graceful Error Handling - ForgeConfig with edge case values
    /// Creating ForgeConfig with various inputs should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_forge_config_no_panic(
        worker_threads in 0usize..100,
        debounce_ms in 0u64..10000,
        idle_threshold_secs in 0u64..3600,
        max_backup_size in 0usize..100_000_000,
    ) {
        let temp_dir = TempDir::new().unwrap();

        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut config = ForgeConfig::new(temp_dir.path());
            config.worker_threads = worker_threads;
            config.debounce_delay = std::time::Duration::from_millis(debounce_ms);
            config.idle_threshold = std::time::Duration::from_secs(idle_threshold_secs);
            config.max_backup_size = max_backup_size;
            config
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "ForgeConfig creation panicked");
    }

    /// Property 5: Graceful Error Handling - Apply changes with binary content
    /// Applying changes with arbitrary binary content should not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_binary_content_no_panic(
        content in prop::collection::vec(any::<u8>(), 0..1000),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut engine = BranchingEngine::new();

        let file_path = temp_dir.path().join("binary_file.bin");

        let change = FileChange {
            path: file_path,
            old_content: None,
            new_content: content,
            tool_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.apply_changes(vec![change])
        }));

        // Should not panic
        prop_assert!(result.is_ok(), "apply_changes() panicked with binary content");
    }

    /// Property 5: Graceful Error Handling - Multiple consecutive reverts
    /// Attempting multiple reverts should return errors after the first, not panic.
    /// **Validates: Requirements 3.3, 3.6, 9.2**
    #[test]
    fn prop_branching_engine_multiple_reverts_no_panic(
        content in "[a-zA-Z0-9]{10,100}",
        revert_attempts in 2usize..5,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut engine = BranchingEngine::new();

        let file_path = temp_dir.path().join("test.txt");

        // First, apply a change
        let change = FileChange {
            path: file_path,
            old_content: None,
            new_content: content.as_bytes().to_vec(),
            tool_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();

        // Now try to revert multiple times
        for i in 0..revert_attempts {
            let result = catch_unwind(AssertUnwindSafe(|| {
                engine.revert_most_recent()
            }));

            // Should not panic
            prop_assert!(result.is_ok(), "revert_most_recent() panicked on attempt {}", i);

            let inner_result = result.unwrap();

            if i == 0 {
                // First revert should succeed
                prop_assert!(inner_result.is_ok(), "First revert should succeed");
            } else {
                // Subsequent reverts should fail with error
                prop_assert!(inner_result.is_err(), "Subsequent reverts should return error");
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_revert_without_application_returns_error() {
        let mut engine = BranchingEngine::new();
        let result = engine.revert_most_recent();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No recent application"));
    }

    #[test]
    fn test_empty_changes_succeeds() {
        let mut engine = BranchingEngine::new();
        let result = engine.apply_changes(vec![]);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_predict_color_default_is_green() {
        let engine = BranchingEngine::new();
        let color = engine.predict_color(&PathBuf::from("nonexistent.txt"));

        assert_eq!(color, dx_forge::core::branching_engine::BranchColor::Green);
    }

    #[test]
    fn test_reset_state_is_idempotent() {
        let mut engine = BranchingEngine::new();

        // Reset multiple times should all succeed
        for _ in 0..5 {
            let result = engine.reset_state();
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_vote_with_empty_strings() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.txt");

        let vote = dx_forge::core::branching_engine::BranchingVote {
            voter_id: String::new(),
            color: dx_forge::core::branching_engine::BranchColor::Green,
            reason: String::new(),
            confidence: 0.0,
        };

        let result = engine.submit_vote(&file, vote);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_empty_voter() {
        let mut engine = BranchingEngine::new();
        let result = engine.register_permanent_voter(String::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_veto_with_empty_strings() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.txt");

        let result = engine.issue_immediate_veto(&file, "", "");
        assert!(result.is_ok());
    }

    #[test]
    fn test_forge_config_with_zero_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = ForgeConfig::new(temp_dir.path());

        config.worker_threads = 0;
        config.debounce_delay = std::time::Duration::from_millis(0);
        config.idle_threshold = std::time::Duration::from_secs(0);
        config.max_backup_size = 0;

        // Config creation should not panic
        assert_eq!(config.worker_threads, 0);
    }
}
