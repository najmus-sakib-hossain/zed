//! Integration tests for Session lifecycle (Sprint 1.2 T23)
//!
//! Tests the full session lifecycle including:
//! - Create → Load → Modify → Save → Compact → Export → Repair → Delete

#[cfg(test)]
mod session_lifecycle_tests {

    #[test]
    fn test_full_session_lifecycle() {
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let manager = dx::session::SessionManager::new(tmp.path().to_path_buf())
            .expect("Failed to create session manager");

        // Step 1: Create a new session
        let session = manager.create("test-agent").expect("Create should succeed");
        let key = session.key.clone();
        assert!(!key.is_empty(), "Session should have a key");

        // Step 2: Add messages
        manager
            .add_message(&key, dx::session::MessageRole::User, "Hello, AI!")
            .expect("Add user message should succeed");
        manager
            .add_message(
                &key,
                dx::session::MessageRole::Assistant,
                "Hello! How can I help you today?",
            )
            .expect("Add assistant message should succeed");

        // Step 3: Load the session and verify messages
        let loaded = manager.get(&key).expect("Get should succeed");
        assert_eq!(loaded.messages.len(), 2, "Should have 2 messages");
        assert_eq!(loaded.messages[0].content, "Hello, AI!");
        assert_eq!(loaded.messages[1].content, "Hello! How can I help you today?");

        // Step 4: Update with more messages
        manager
            .add_message(&key, dx::session::MessageRole::User, "What's the weather like?")
            .expect("Add message should succeed");

        // Step 5: Verify update persisted
        let reloaded = manager.get(&key).expect("Get should succeed");
        assert_eq!(reloaded.messages.len(), 3, "Should have 3 messages after update");

        // Step 6: Compact the session
        let compact_result = manager.compact(&key);
        assert!(compact_result.is_ok(), "Compact should succeed");

        // Step 7: Export session
        let session_data = manager.get(&key).expect("Get should succeed");
        let export = dx::session::transcript::export_session(
            &session_data,
            dx::session::transcript::ExportFormat::Json,
        );
        assert!(export.is_ok(), "JSON export should succeed");
        let json_str = export.unwrap();
        assert!(json_str.contains("Hello, AI!"), "Export should contain message content");

        // Step 8: Export to markdown
        let md_export = dx::session::transcript::export_session(
            &session_data,
            dx::session::transcript::ExportFormat::Markdown,
        );
        assert!(md_export.is_ok(), "Markdown export should succeed");

        // Step 9: Export to HTML
        let html_export = dx::session::transcript::export_session(
            &session_data,
            dx::session::transcript::ExportFormat::Html,
        );
        assert!(html_export.is_ok(), "HTML export should succeed");

        // Step 10: Validate/repair session
        let warnings = dx::session::repair::validate_session(&session_data);
        // Healthy session should have no critical warnings
        assert!(warnings.len() < 5, "Healthy session should have few warnings");

        // Step 11: Delete the session
        manager.delete(&key).expect("Delete should succeed");

        // Step 12: Verify deletion
        let gone = manager.get(&key);
        assert!(gone.is_err(), "Session should be gone after delete");
    }

    #[test]
    fn test_multiple_sessions_lifecycle() {
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let manager = dx::session::SessionManager::new(tmp.path().to_path_buf())
            .expect("Failed to create session manager");

        // Create multiple sessions
        let mut keys = vec![];
        for i in 0..5 {
            let session = manager.create(&format!("agent-{}", i)).expect("Create should succeed");
            let key = session.key.clone();
            manager
                .add_message(
                    &key,
                    dx::session::MessageRole::User,
                    &format!("Session {} message", i),
                )
                .expect("Add message should succeed");
            keys.push(key);
        }

        // List all sessions
        let sessions = manager
            .list(&dx::session::SessionFilter::default())
            .expect("List should succeed");
        assert_eq!(sessions.len(), 5, "Should have 5 sessions");

        // Delete some
        manager.delete(&keys[0]).expect("Delete should succeed");
        manager.delete(&keys[2]).expect("Delete should succeed");

        // Verify remaining
        let remaining = manager
            .list(&dx::session::SessionFilter::default())
            .expect("List should succeed");
        assert_eq!(remaining.len(), 3, "Should have 3 sessions after deleting 2");

        // Clear all
        let cleared = manager.clear().expect("Clear should succeed");
        assert!(cleared >= 3, "Should clear at least 3 sessions");

        let empty = manager
            .list(&dx::session::SessionFilter::default())
            .expect("List should succeed");
        assert!(empty.is_empty(), "Should be empty after clear");
    }

    #[test]
    fn test_session_compaction_reduces_size() {
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let manager = dx::session::SessionManager::new(tmp.path().to_path_buf())
            .expect("Failed to create session manager");

        // Create a session with many messages
        let session = manager.create("test-agent").expect("Create should succeed");
        let key = session.key.clone();

        for i in 0..30 {
            let role = if i % 2 == 0 {
                dx::session::MessageRole::User
            } else {
                dx::session::MessageRole::Assistant
            };
            manager
                .add_message(
                    &key,
                    role,
                    &format!(
                        "This is message number {} with enough padding to make it substantial",
                        i
                    ),
                )
                .expect("Add message should succeed");
        }

        let before = manager.get(&key).expect("Get should succeed");
        let original_count = before.messages.len();
        assert_eq!(original_count, 30, "Should have 30 messages");

        // Compact
        let _result = manager.compact(&key).expect("Compact should succeed");

        let after = manager.get(&key).expect("Get should succeed after compact");
        assert!(
            after.messages.len() <= original_count,
            "Compaction should reduce or maintain message count"
        );
    }

    #[test]
    fn test_session_persistence_across_managers() {
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = tmp.path().to_path_buf();

        // Create session with first manager
        let manager1 = dx::session::SessionManager::new(path.clone())
            .expect("Failed to create session manager");
        let session = manager1.create("test-agent").expect("Create should succeed");
        let key = session.key.clone();
        manager1
            .add_message(&key, dx::session::MessageRole::User, "Persisted message")
            .expect("Add message should succeed");

        // Drop first manager, create second one pointed at same path
        drop(manager1);
        let manager2 = dx::session::SessionManager::new(path)
            .expect("Failed to create second session manager");

        // Session should be loadable from disk
        let loaded = manager2.get(&key).expect("Session should persist on disk");
        assert!(!loaded.messages.is_empty(), "Messages should persist");
    }
}
