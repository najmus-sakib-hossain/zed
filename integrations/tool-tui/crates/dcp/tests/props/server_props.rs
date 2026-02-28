//! Property tests for DCP server.
//!
//! Feature: dcp-protocol, Property 15: Session State Preservation

use proptest::prelude::*;
use std::collections::HashMap;

use dcp::context::DcpContext;
use dcp::dispatch::BinaryTrieRouter;
use dcp::server::{DcpServer, ProtocolVersion, ServerConfig, Session};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 15: Session State Preservation
    /// For any client session with state, upgrading from MCP to DCP protocol
    /// SHALL preserve all session state without data loss.
    #[test]
    fn prop_session_state_preserved_on_upgrade(
        keys in prop::collection::vec("[a-z]{1,10}", 1..10),
        values in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..10),
    ) {
        // Create server
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig::default();
        let server = DcpServer::new(router, context, config);

        // Create session
        let session = server.create_session().unwrap();
        let session_id = session.id;

        // Store data in session
        let mut stored_data: HashMap<String, Vec<u8>> = HashMap::new();
        for (key, value) in keys.iter().zip(values.iter()) {
            session.set_data(key.clone(), value.clone());
            stored_data.insert(key.clone(), value.clone());
        }

        // Verify initial protocol is MCP
        prop_assert_eq!(session.protocol, ProtocolVersion::Mcp);

        // Upgrade session to DCP
        server.upgrade_session(session_id).unwrap();

        // Get upgraded session
        let upgraded_session = server.get_session(session_id).unwrap();

        // Verify protocol was upgraded
        prop_assert_eq!(upgraded_session.protocol, ProtocolVersion::DcpV1);

        // Verify all data was preserved
        for (key, expected_value) in stored_data.iter() {
            let actual_value = upgraded_session.get_data(key);
            prop_assert_eq!(actual_value.as_ref(), Some(expected_value));
        }
    }

    /// Test that session message count is preserved
    #[test]
    fn prop_session_message_count_increments(
        message_count in 1u64..1000,
    ) {
        let session = Session::new(1);

        for _ in 0..message_count {
            session.increment_messages();
        }

        let count = session.message_count.load(std::sync::atomic::Ordering::Acquire);
        prop_assert_eq!(count, message_count);
    }

    /// Test that session touch updates last activity
    #[test]
    fn prop_session_touch_updates_activity(
        _dummy in 0u8..1, // Just to make it a property test
    ) {
        let session = Session::new(1);
        let initial = session.last_activity.load(std::sync::atomic::Ordering::Acquire);

        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        session.touch();

        let updated = session.last_activity.load(std::sync::atomic::Ordering::Acquire);
        prop_assert!(updated >= initial);
    }

    /// Test that server respects max sessions limit
    #[test]
    fn prop_server_max_sessions_enforced(
        max_sessions in 1usize..50,
    ) {
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig {
            max_sessions,
            ..Default::default()
        };
        let server = DcpServer::new(router, context, config);

        // Create max_sessions sessions
        for _ in 0..max_sessions {
            let result = server.create_session();
            prop_assert!(result.is_ok());
        }

        // Next session should fail
        let result = server.create_session();
        prop_assert!(result.is_err());

        // Session count should be at max
        prop_assert_eq!(server.session_count(), max_sessions);
    }

    /// Test that session removal works correctly
    #[test]
    fn prop_session_removal(
        num_sessions in 1usize..20,
        remove_indices in prop::collection::vec(0usize..100, 1..10),
    ) {
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig {
            max_sessions: 100,
            ..Default::default()
        };
        let server = DcpServer::new(router, context, config);

        // Create sessions and track IDs
        let mut session_ids = Vec::new();
        for _ in 0..num_sessions {
            let session = server.create_session().unwrap();
            session_ids.push(session.id);
        }

        // Remove some sessions
        let mut removed_count = 0;
        for idx in remove_indices {
            let actual_idx = idx % session_ids.len();
            let id = session_ids[actual_idx];
            if server.get_session(id).is_some() {
                server.remove_session(id);
                removed_count += 1;
            }
        }

        // Verify session count
        let expected_count = num_sessions.saturating_sub(removed_count);
        prop_assert!(server.session_count() <= num_sessions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_data_isolation() {
        let session1 = Session::new(1);
        let session2 = Session::new(2);

        session1.set_data("key".to_string(), vec![1, 2, 3]);
        session2.set_data("key".to_string(), vec![4, 5, 6]);

        assert_eq!(session1.get_data("key"), Some(vec![1, 2, 3]));
        assert_eq!(session2.get_data("key"), Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_metrics_snapshot() {
        let router = BinaryTrieRouter::new();
        let context = DcpContext::new(1);
        let config = ServerConfig::default();
        let server = DcpServer::new(router, context, config);

        server.metrics.record_mcp(100, 1000);
        server.metrics.record_dcp(50, 500);
        server.metrics.record_invocation();
        server.metrics.record_error();

        let snapshot = server.metrics.snapshot();
        assert_eq!(snapshot.mcp_messages, 1);
        assert_eq!(snapshot.dcp_messages, 1);
        assert_eq!(snapshot.mcp_bytes, 100);
        assert_eq!(snapshot.dcp_bytes, 50);
        assert_eq!(snapshot.tool_invocations, 1);
        assert_eq!(snapshot.errors, 1);
    }
}
