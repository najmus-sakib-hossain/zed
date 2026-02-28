//! Unit tests for BranchingEngine
//!
//! Comprehensive tests covering:
//! - Vote submission (single votes, multiple votes, different voters)
//! - Color prediction (green, yellow, red based on votes)
//! - Apply operations (applying file changes)
//! - Revert operations (reverting most recent application)
//! - Edge cases and error conditions
//!
//! **Validates: Requirements 5.1, 5.3**

use chrono::Utc;
use dx_forge::core::branching_engine::{BranchColor, BranchingEngine, BranchingVote, FileChange};
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Vote Submission Tests
// ============================================================================

mod vote_submission {
    use super::*;

    #[test]
    fn test_single_vote_submission() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Looks good".to_string(),
            confidence: 0.9,
        };

        let result = engine.submit_vote(&file, vote);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_votes_same_file() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Submit multiple votes from different voters
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Approved".to_string(),
            confidence: 0.8,
        };

        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Green,
            reason: "Also approved".to_string(),
            confidence: 0.95,
        };

        let vote3 = BranchingVote {
            voter_id: "voter-3".to_string(),
            color: BranchColor::Green,
            reason: "LGTM".to_string(),
            confidence: 0.7,
        };

        assert!(engine.submit_vote(&file, vote1).is_ok());
        assert!(engine.submit_vote(&file, vote2).is_ok());
        assert!(engine.submit_vote(&file, vote3).is_ok());

        // All green votes should result in green prediction
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_votes_different_files() {
        let mut engine = BranchingEngine::new();
        let file1 = PathBuf::from("file1.ts");
        let file2 = PathBuf::from("file2.ts");

        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Good".to_string(),
            confidence: 0.9,
        };

        let vote2 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.8,
        };

        assert!(engine.submit_vote(&file1, vote1).is_ok());
        assert!(engine.submit_vote(&file2, vote2).is_ok());

        // Each file should have its own color
        assert_eq!(engine.predict_color(&file1), BranchColor::Green);
        assert_eq!(engine.predict_color(&file2), BranchColor::Yellow);
    }

    #[test]
    fn test_vote_with_empty_voter_id() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: String::new(),
            color: BranchColor::Green,
            reason: "Empty voter".to_string(),
            confidence: 0.5,
        };

        // Should still accept the vote
        assert!(engine.submit_vote(&file, vote).is_ok());
    }

    #[test]
    fn test_vote_with_empty_reason() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: String::new(),
            confidence: 0.5,
        };

        assert!(engine.submit_vote(&file, vote).is_ok());
    }

    #[test]
    fn test_vote_with_extreme_confidence_values() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Zero confidence
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Zero confidence".to_string(),
            confidence: 0.0,
        };
        assert!(engine.submit_vote(&file, vote1).is_ok());

        // Full confidence
        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Green,
            reason: "Full confidence".to_string(),
            confidence: 1.0,
        };
        assert!(engine.submit_vote(&file, vote2).is_ok());
    }

    #[test]
    fn test_same_voter_multiple_votes() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Same voter submits multiple votes (should all be recorded)
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "First vote".to_string(),
            confidence: 0.5,
        };

        let vote2 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Yellow,
            reason: "Changed mind".to_string(),
            confidence: 0.8,
        };

        assert!(engine.submit_vote(&file, vote1).is_ok());
        assert!(engine.submit_vote(&file, vote2).is_ok());

        // Yellow vote should take precedence
        assert_eq!(engine.predict_color(&file), BranchColor::Yellow);
    }
}

// ============================================================================
// Color Prediction Tests
// ============================================================================

mod color_prediction {
    use super::*;

    #[test]
    fn test_predict_green_with_all_green_votes() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        for i in 0..3 {
            let vote = BranchingVote {
                voter_id: format!("voter-{}", i),
                color: BranchColor::Green,
                reason: "Approved".to_string(),
                confidence: 0.9,
            };
            engine.submit_vote(&file, vote).unwrap();
        }

        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_predict_yellow_with_any_yellow_vote() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Two green votes
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Good".to_string(),
            confidence: 0.9,
        };
        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Green,
            reason: "Good".to_string(),
            confidence: 0.9,
        };

        // One yellow vote
        let vote3 = BranchingVote {
            voter_id: "voter-3".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.7,
        };

        engine.submit_vote(&file, vote1).unwrap();
        engine.submit_vote(&file, vote2).unwrap();
        engine.submit_vote(&file, vote3).unwrap();

        // Yellow should take precedence over green
        assert_eq!(engine.predict_color(&file), BranchColor::Yellow);
    }

    #[test]
    fn test_predict_red_with_any_red_vote() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Green vote
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Good".to_string(),
            confidence: 0.9,
        };

        // Yellow vote
        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.8,
        };

        // Red vote (veto)
        let vote3 = BranchingVote {
            voter_id: "voter-3".to_string(),
            color: BranchColor::Red,
            reason: "Critical issue".to_string(),
            confidence: 1.0,
        };

        engine.submit_vote(&file, vote1).unwrap();
        engine.submit_vote(&file, vote2).unwrap();
        engine.submit_vote(&file, vote3).unwrap();

        // Red should take precedence over all
        assert_eq!(engine.predict_color(&file), BranchColor::Red);
    }

    #[test]
    fn test_predict_green_default_no_votes() {
        let engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // No votes should default to green
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_predict_green_with_no_opinion_votes() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::NoOpinion,
            reason: "Abstain".to_string(),
            confidence: 0.0,
        };

        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Green,
            reason: "Good".to_string(),
            confidence: 0.9,
        };

        engine.submit_vote(&file, vote1).unwrap();
        engine.submit_vote(&file, vote2).unwrap();

        // NoOpinion + Green should result in Green
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_predict_color_nonexistent_file() {
        let engine = BranchingEngine::new();
        let file = PathBuf::from("/nonexistent/path/file.ts");

        // Should return default green for files with no votes
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_is_change_guaranteed_safe_all_green() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Safe".to_string(),
            confidence: 1.0,
        };

        engine.submit_vote(&file, vote).unwrap();

        assert!(engine.is_change_guaranteed_safe(&file));
    }

    #[test]
    fn test_is_change_not_guaranteed_safe_with_yellow() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.7,
        };

        engine.submit_vote(&file, vote).unwrap();

        assert!(!engine.is_change_guaranteed_safe(&file));
    }

    #[test]
    fn test_is_change_not_guaranteed_safe_no_votes() {
        let engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // No votes means not guaranteed safe
        assert!(!engine.is_change_guaranteed_safe(&file));
    }
}

// ============================================================================
// Apply Operations Tests
// ============================================================================

mod apply_operations {
    use super::*;

    #[test]
    fn test_apply_single_file_change() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Create original file
        std::fs::write(&test_file, b"original content").unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original content".to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test-tool".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], test_file);
        assert_eq!(std::fs::read(&test_file).unwrap(), b"new content");
    }

    #[test]
    fn test_apply_multiple_file_changes() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");

        std::fs::write(&file1, b"content1").unwrap();
        std::fs::write(&file2, b"content2").unwrap();
        std::fs::write(&file3, b"content3").unwrap();

        let mut engine = BranchingEngine::new();

        let changes = vec![
            FileChange {
                path: file1.clone(),
                old_content: Some(b"content1".to_vec()),
                new_content: b"new1".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file2.clone(),
                old_content: Some(b"content2".to_vec()),
                new_content: b"new2".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file3.clone(),
                old_content: Some(b"content3".to_vec()),
                new_content: b"new3".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
        ];

        let applied = engine.apply_changes(changes).unwrap();

        assert_eq!(applied.len(), 3);
        assert_eq!(std::fs::read(&file1).unwrap(), b"new1");
        assert_eq!(std::fs::read(&file2).unwrap(), b"new2");
        assert_eq!(std::fs::read(&file3).unwrap(), b"new3");
    }

    #[test]
    fn test_apply_creates_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let new_file = temp_dir.path().join("new_file.txt");

        assert!(!new_file.exists());

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: new_file.clone(),
            old_content: None, // New file
            new_content: b"brand new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        assert_eq!(applied.len(), 1);
        assert!(new_file.exists());
        assert_eq!(std::fs::read(&new_file).unwrap(), b"brand new content");
    }

    #[test]
    fn test_apply_creates_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_file = temp_dir.path().join("a/b/c/deep_file.txt");

        assert!(!nested_file.exists());

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: nested_file.clone(),
            old_content: None,
            new_content: b"deep content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        assert_eq!(applied.len(), 1);
        assert!(nested_file.exists());
        assert_eq!(std::fs::read(&nested_file).unwrap(), b"deep content");
    }

    #[test]
    fn test_apply_empty_changes_list() {
        let mut engine = BranchingEngine::new();

        let applied = engine.apply_changes(vec![]).unwrap();

        assert!(applied.is_empty());
    }

    #[test]
    fn test_apply_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let binary_file = temp_dir.path().join("binary.bin");

        let original_bytes: Vec<u8> = (0..255).collect();
        std::fs::write(&binary_file, &original_bytes).unwrap();

        let mut engine = BranchingEngine::new();

        let new_bytes: Vec<u8> = (0..255).rev().collect();
        let change = FileChange {
            path: binary_file.clone(),
            old_content: Some(original_bytes),
            new_content: new_bytes.clone(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&binary_file).unwrap(), new_bytes);
    }

    #[test]
    fn test_apply_with_red_vote_rejects_change() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        // Submit a red vote (veto)
        let vote = BranchingVote {
            voter_id: "security-scanner".to_string(),
            color: BranchColor::Red,
            reason: "Security vulnerability detected".to_string(),
            confidence: 1.0,
        };
        engine.submit_vote(&test_file, vote).unwrap();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        // Red vote should reject the change
        assert!(applied.is_empty());
        // Original content should remain
        assert_eq!(std::fs::read(&test_file).unwrap(), b"original");
    }

    #[test]
    fn test_apply_with_green_vote_auto_approves() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        // Submit a green vote
        let vote = BranchingVote {
            voter_id: "linter".to_string(),
            color: BranchColor::Green,
            reason: "Looks good".to_string(),
            confidence: 0.9,
        };
        engine.submit_vote(&test_file, vote).unwrap();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();

        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&test_file).unwrap(), b"new content");
    }

    #[test]
    fn test_apply_stores_backup_for_revert() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original = b"original content for backup";
        std::fs::write(&test_file, original).unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original.to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();

        // Verify backup was stored by reverting
        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 1);
        assert_eq!(std::fs::read(&test_file).unwrap(), original);
    }
}

// ============================================================================
// Revert Operations Tests
// ============================================================================

mod revert_operations {
    use super::*;

    #[test]
    fn test_revert_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original = b"original content";
        std::fs::write(&test_file, original).unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original.to_vec()),
            new_content: b"modified content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b"modified content");

        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 1);
        assert_eq!(reverted[0], test_file);
        assert_eq!(std::fs::read(&test_file).unwrap(), original);
    }

    #[test]
    fn test_revert_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");

        let original1 = b"original1";
        let original2 = b"original2";
        let original3 = b"original3";

        std::fs::write(&file1, original1).unwrap();
        std::fs::write(&file2, original2).unwrap();
        std::fs::write(&file3, original3).unwrap();

        let mut engine = BranchingEngine::new();

        let changes = vec![
            FileChange {
                path: file1.clone(),
                old_content: Some(original1.to_vec()),
                new_content: b"new1".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file2.clone(),
                old_content: Some(original2.to_vec()),
                new_content: b"new2".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file3.clone(),
                old_content: Some(original3.to_vec()),
                new_content: b"new3".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
        ];

        engine.apply_changes(changes).unwrap();

        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 3);

        // All files should be restored
        assert_eq!(std::fs::read(&file1).unwrap(), original1);
        assert_eq!(std::fs::read(&file2).unwrap(), original2);
        assert_eq!(std::fs::read(&file3).unwrap(), original3);
    }

    #[test]
    fn test_revert_newly_created_file_deletes_it() {
        let temp_dir = TempDir::new().unwrap();
        let new_file = temp_dir.path().join("new_file.txt");

        assert!(!new_file.exists());

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: new_file.clone(),
            old_content: None, // New file - no backup
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        assert!(new_file.exists());

        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 1);
        // File should be deleted since it was newly created
        assert!(!new_file.exists());
    }

    #[test]
    fn test_revert_no_application_returns_error() {
        let mut engine = BranchingEngine::new();

        let result = engine.revert_most_recent();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No recent application"));
    }

    #[test]
    fn test_revert_clears_last_application() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        engine.revert_most_recent().unwrap();

        // Second revert should fail - no more applications to revert
        let result = engine.revert_most_recent();
        assert!(result.is_err());
    }

    #[test]
    fn test_revert_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let binary_file = temp_dir.path().join("binary.bin");

        let original_bytes: Vec<u8> = (0..255).collect();
        std::fs::write(&binary_file, &original_bytes).unwrap();

        let mut engine = BranchingEngine::new();

        let new_bytes: Vec<u8> = vec![0xFF; 100];
        let change = FileChange {
            path: binary_file.clone(),
            old_content: Some(original_bytes.clone()),
            new_content: new_bytes,
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        engine.revert_most_recent().unwrap();

        assert_eq!(std::fs::read(&binary_file).unwrap(), original_bytes);
    }

    #[test]
    fn test_revert_creates_parent_directories_if_needed() {
        let temp_dir = TempDir::new().unwrap();
        let nested_file = temp_dir.path().join("a/b/c/file.txt");

        // Create nested directories and file
        std::fs::create_dir_all(nested_file.parent().unwrap()).unwrap();
        std::fs::write(&nested_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: nested_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();

        // Delete the parent directories to simulate a scenario where they're gone
        std::fs::remove_file(&nested_file).unwrap();
        std::fs::remove_dir_all(temp_dir.path().join("a")).unwrap();

        // Revert should recreate directories and restore file
        let reverted = engine.revert_most_recent().unwrap();
        assert_eq!(reverted.len(), 1);
        assert!(nested_file.exists());
        assert_eq!(std::fs::read(&nested_file).unwrap(), b"original");
    }

    #[test]
    fn test_revert_only_reverts_most_recent_application() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"v1").unwrap();

        let mut engine = BranchingEngine::new();

        // First application: v1 -> v2
        let change1 = FileChange {
            path: test_file.clone(),
            old_content: Some(b"v1".to_vec()),
            new_content: b"v2".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };
        engine.apply_changes(vec![change1]).unwrap();

        // Second application: v2 -> v3
        let change2 = FileChange {
            path: test_file.clone(),
            old_content: Some(b"v2".to_vec()),
            new_content: b"v3".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };
        engine.apply_changes(vec![change2]).unwrap();

        assert_eq!(std::fs::read(&test_file).unwrap(), b"v3");

        // Revert should go back to v2 (the state before the most recent application)
        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b"v2");

        // No more reverts available (only tracks most recent)
        assert!(engine.revert_most_recent().is_err());
    }

    #[test]
    fn test_revert_with_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("empty.txt");
        std::fs::write(&test_file, b"").unwrap(); // Empty file

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(vec![]),
            new_content: b"now has content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b"now has content");

        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b""); // Back to empty
    }

    #[test]
    fn test_revert_to_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"has content").unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"has content".to_vec()),
            new_content: vec![], // Make it empty
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        engine.apply_changes(vec![change]).unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b"");

        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), b"has content");
    }
}

// ============================================================================
// Edge Cases and Error Conditions Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_register_permanent_voter() {
        let mut engine = BranchingEngine::new();

        let result = engine.register_permanent_voter("voter-1".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_same_voter_twice() {
        let mut engine = BranchingEngine::new();

        engine.register_permanent_voter("voter-1".to_string()).unwrap();
        // Registering same voter again should be idempotent
        let result = engine.register_permanent_voter("voter-1".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_empty_voter_id() {
        let mut engine = BranchingEngine::new();

        let result = engine.register_permanent_voter(String::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_issue_immediate_veto() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let result =
            engine.issue_immediate_veto(&file, "security-scanner", "Critical vulnerability");
        assert!(result.is_ok());

        // File should now be red
        assert_eq!(engine.predict_color(&file), BranchColor::Red);
    }

    #[test]
    fn test_issue_veto_with_empty_reason() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let result = engine.issue_immediate_veto(&file, "voter", "");
        assert!(result.is_ok());
        assert_eq!(engine.predict_color(&file), BranchColor::Red);
    }

    #[test]
    fn test_issue_veto_with_empty_voter_id() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let result = engine.issue_immediate_veto(&file, "", "Some reason");
        assert!(result.is_ok());
        assert_eq!(engine.predict_color(&file), BranchColor::Red);
    }

    #[test]
    fn test_reset_state() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Add some votes
        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Yellow,
            reason: "Test".to_string(),
            confidence: 0.8,
        };
        engine.submit_vote(&file, vote).unwrap();
        assert_eq!(engine.predict_color(&file), BranchColor::Yellow);

        // Reset state
        engine.reset_state().unwrap();

        // Votes should be cleared - default to green
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_reset_state_multiple_times() {
        let mut engine = BranchingEngine::new();

        // Reset multiple times should be idempotent
        assert!(engine.reset_state().is_ok());
        assert!(engine.reset_state().is_ok());
        assert!(engine.reset_state().is_ok());
    }

    #[test]
    fn test_default_trait_implementation() {
        let engine = BranchingEngine::default();
        let file = PathBuf::from("test.ts");

        // Should behave same as new()
        assert_eq!(engine.predict_color(&file), BranchColor::Green);
    }

    #[test]
    fn test_apply_with_yellow_vote_still_applies() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        // Submit a yellow vote
        let vote = BranchingVote {
            voter_id: "reviewer".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.7,
        };
        engine.submit_vote(&test_file, vote).unwrap();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // Yellow votes should still allow the change (with review prompt)
        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&test_file).unwrap(), b"new content");
    }

    #[test]
    fn test_apply_with_no_opinion_vote() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        // Submit a NoOpinion vote
        let vote = BranchingVote {
            voter_id: "abstainer".to_string(),
            color: BranchColor::NoOpinion,
            reason: "No comment".to_string(),
            confidence: 0.0,
        };
        engine.submit_vote(&test_file, vote).unwrap();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new content".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        // NoOpinion should allow the change
        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn test_vote_priority_red_over_yellow_over_green() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Add green vote first
        engine
            .submit_vote(
                &file,
                BranchingVote {
                    voter_id: "voter-1".to_string(),
                    color: BranchColor::Green,
                    reason: "Good".to_string(),
                    confidence: 0.9,
                },
            )
            .unwrap();
        assert_eq!(engine.predict_color(&file), BranchColor::Green);

        // Add yellow vote - should override green
        engine
            .submit_vote(
                &file,
                BranchingVote {
                    voter_id: "voter-2".to_string(),
                    color: BranchColor::Yellow,
                    reason: "Needs review".to_string(),
                    confidence: 0.8,
                },
            )
            .unwrap();
        assert_eq!(engine.predict_color(&file), BranchColor::Yellow);

        // Add red vote - should override yellow
        engine
            .submit_vote(
                &file,
                BranchingVote {
                    voter_id: "voter-3".to_string(),
                    color: BranchColor::Red,
                    reason: "Critical".to_string(),
                    confidence: 1.0,
                },
            )
            .unwrap();
        assert_eq!(engine.predict_color(&file), BranchColor::Red);
    }

    #[test]
    fn test_is_change_guaranteed_safe_with_red() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Red,
            reason: "Dangerous".to_string(),
            confidence: 1.0,
        };
        engine.submit_vote(&file, vote).unwrap();

        assert!(!engine.is_change_guaranteed_safe(&file));
    }

    #[test]
    fn test_is_change_guaranteed_safe_with_no_opinion() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        let vote = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::NoOpinion,
            reason: "Abstain".to_string(),
            confidence: 0.0,
        };
        engine.submit_vote(&file, vote).unwrap();

        // NoOpinion alone is not "guaranteed safe"
        assert!(!engine.is_change_guaranteed_safe(&file));
    }

    #[test]
    fn test_apply_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large.txt");

        // Create a 1MB file
        let original: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&large_file, &original).unwrap();

        let mut engine = BranchingEngine::new();

        let new_content: Vec<u8> = (0..1_000_000).map(|i| ((i + 1) % 256) as u8).collect();
        let change = FileChange {
            path: large_file.clone(),
            old_content: Some(original.clone()),
            new_content: new_content.clone(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&large_file).unwrap(), new_content);

        // Revert should also work with large files
        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read(&large_file).unwrap(), original);
    }

    #[test]
    fn test_apply_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("unicode.txt");

        let original = "Hello, 世界! 🌍 Привет мир!".as_bytes().to_vec();
        std::fs::write(&test_file, &original).unwrap();

        let mut engine = BranchingEngine::new();

        let new_content = "Goodbye, 世界! 🌎 До свидания!".as_bytes().to_vec();
        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(original.clone()),
            new_content: new_content.clone(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&test_file).unwrap(), new_content);

        engine.revert_most_recent().unwrap();
        assert_eq!(std::fs::read(&test_file).unwrap(), original);
    }

    #[test]
    fn test_apply_file_with_special_characters_in_name() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("file with spaces & special.txt");

        std::fs::write(&test_file, b"original").unwrap();

        let mut engine = BranchingEngine::new();

        let change = FileChange {
            path: test_file.clone(),
            old_content: Some(b"original".to_vec()),
            new_content: b"new".to_vec(),
            tool_id: "test".to_string(),
            timestamp: Utc::now(),
        };

        let applied = engine.apply_changes(vec![change]).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(std::fs::read(&test_file).unwrap(), b"new");
    }

    #[test]
    fn test_mixed_changes_some_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("approved.txt");
        let file2 = temp_dir.path().join("rejected.txt");

        std::fs::write(&file1, b"original1").unwrap();
        std::fs::write(&file2, b"original2").unwrap();

        let mut engine = BranchingEngine::new();

        // Red vote on file2 only
        engine
            .submit_vote(
                &file2,
                BranchingVote {
                    voter_id: "security".to_string(),
                    color: BranchColor::Red,
                    reason: "Blocked".to_string(),
                    confidence: 1.0,
                },
            )
            .unwrap();

        let changes = vec![
            FileChange {
                path: file1.clone(),
                old_content: Some(b"original1".to_vec()),
                new_content: b"new1".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
            FileChange {
                path: file2.clone(),
                old_content: Some(b"original2".to_vec()),
                new_content: b"new2".to_vec(),
                tool_id: "test".to_string(),
                timestamp: Utc::now(),
            },
        ];

        let applied = engine.apply_changes(changes).unwrap();

        // Only file1 should be applied
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], file1);
        assert_eq!(std::fs::read(&file1).unwrap(), b"new1");
        assert_eq!(std::fs::read(&file2).unwrap(), b"original2"); // Unchanged
    }

    #[test]
    fn test_votes_for_different_paths_are_independent() {
        let mut engine = BranchingEngine::new();

        let file1 = PathBuf::from("src/main.rs");
        let file2 = PathBuf::from("src/lib.rs");
        let file3 = PathBuf::from("tests/test.rs");

        // Different votes for different files
        engine
            .submit_vote(
                &file1,
                BranchingVote {
                    voter_id: "v1".to_string(),
                    color: BranchColor::Green,
                    reason: "OK".to_string(),
                    confidence: 0.9,
                },
            )
            .unwrap();

        engine
            .submit_vote(
                &file2,
                BranchingVote {
                    voter_id: "v1".to_string(),
                    color: BranchColor::Yellow,
                    reason: "Review".to_string(),
                    confidence: 0.7,
                },
            )
            .unwrap();

        engine
            .submit_vote(
                &file3,
                BranchingVote {
                    voter_id: "v1".to_string(),
                    color: BranchColor::Red,
                    reason: "Block".to_string(),
                    confidence: 1.0,
                },
            )
            .unwrap();

        // Each file should have its own color
        assert_eq!(engine.predict_color(&file1), BranchColor::Green);
        assert_eq!(engine.predict_color(&file2), BranchColor::Yellow);
        assert_eq!(engine.predict_color(&file3), BranchColor::Red);
    }

    #[test]
    fn test_confidence_values_boundary() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("test.ts");

        // Test boundary confidence values
        let vote_zero = BranchingVote {
            voter_id: "v1".to_string(),
            color: BranchColor::Green,
            reason: "Zero confidence".to_string(),
            confidence: 0.0,
        };
        assert!(engine.submit_vote(&file, vote_zero).is_ok());

        let vote_one = BranchingVote {
            voter_id: "v2".to_string(),
            color: BranchColor::Green,
            reason: "Full confidence".to_string(),
            confidence: 1.0,
        };
        assert!(engine.submit_vote(&file, vote_one).is_ok());

        let vote_mid = BranchingVote {
            voter_id: "v3".to_string(),
            color: BranchColor::Green,
            reason: "Half confidence".to_string(),
            confidence: 0.5,
        };
        assert!(engine.submit_vote(&file, vote_mid).is_ok());
    }

    #[test]
    fn test_many_voters_same_file() {
        let mut engine = BranchingEngine::new();
        let file = PathBuf::from("popular.ts");

        // Add 100 green votes
        for i in 0..100 {
            let vote = BranchingVote {
                voter_id: format!("voter-{}", i),
                color: BranchColor::Green,
                reason: format!("Approved by voter {}", i),
                confidence: 0.9,
            };
            engine.submit_vote(&file, vote).unwrap();
        }

        assert_eq!(engine.predict_color(&file), BranchColor::Green);
        assert!(engine.is_change_guaranteed_safe(&file));

        // One red vote should veto all
        engine
            .submit_vote(
                &file,
                BranchingVote {
                    voter_id: "security".to_string(),
                    color: BranchColor::Red,
                    reason: "Veto".to_string(),
                    confidence: 1.0,
                },
            )
            .unwrap();

        assert_eq!(engine.predict_color(&file), BranchColor::Red);
        assert!(!engine.is_change_guaranteed_safe(&file));
    }
}
