//! Property-based tests for the sync module
//!
//! These tests validate the correctness properties defined in the design document.

use dx_www_sync::*;
use proptest::prelude::*;
use std::collections::HashSet;

/// Generate a valid channel ID
fn channel_id_strategy() -> impl Strategy<Value = ChannelId> {
    1u16..1000u16
}

/// Generate a valid message ID
fn message_id_strategy() -> impl Strategy<Value = MessageId> {
    1u32..100000u32
}

/// Generate a valid user ID
fn user_id_strategy() -> impl Strategy<Value = String> {
    "[a-z]{3,10}".prop_map(|s| s)
}

/// Generate binary message data
fn message_data_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..100)
}

/// Generate a BinaryMessage
fn binary_message_strategy() -> impl Strategy<Value = BinaryMessage> {
    (message_id_strategy(), channel_id_strategy(), message_data_strategy()).prop_map(
        |(message_id, channel_id, data)| BinaryMessage {
            message_id,
            channel_id,
            data,
            timestamp: 12345,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: production-readiness, Property 22: Pub/Sub Delivery**
    /// *For any* message published to a channel, all active subscribers to that channel
    /// SHALL receive the message exactly once.
    /// **Validates: Requirements 9.2**
    #[test]
    fn prop_pubsub_delivery(
        user_ids in prop::collection::vec(user_id_strategy(), 2..5),
        channel_id in channel_id_strategy(),
        message in binary_message_strategy(),
    ) {
        let manager = WebSocketManager::default();
        let mut receivers = Vec::new();
        let mut connection_ids = Vec::new();

        // Create connections and subscribe to channel
        for user_id in &user_ids {
            let (conn_id, rx) = manager.handle_connection(user_id.clone());
            manager.subscribe(conn_id, channel_id).unwrap();
            receivers.push(rx);
            connection_ids.push(conn_id);
        }

        // Create message with correct channel
        let msg = BinaryMessage {
            channel_id,
            ..message
        };
        let expected_data = msg.data.clone();

        // Publish message
        manager.publish(channel_id, msg.clone()).unwrap();

        // Verify all subscribers received exactly once
        for rx in &receivers {
            let received = rx.try_recv();
            prop_assert!(received.is_ok(), "Subscriber should receive message");
            let received_msg = received.unwrap();
            prop_assert_eq!(received_msg.message_id, msg.message_id);
            prop_assert_eq!(received_msg.data, expected_data.clone());

            // Should not receive duplicate
            prop_assert!(rx.try_recv().is_err(), "Should not receive duplicate");
        }
    }

    /// **Feature: production-readiness, Property 23: Message Buffer Replay**
    /// *For any* messages sent while a client is disconnected, upon reconnection
    /// the client SHALL receive all buffered messages in order.
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_message_buffer_replay(
        messages in prop::collection::vec(binary_message_strategy(), 1..10),
        _user_id in user_id_strategy(),
    ) {
        let buffer = MessageBuffer::new(100);
        let connection_id = 1;

        // Buffer messages
        for msg in &messages {
            buffer.push(connection_id, msg.clone()).unwrap();
        }

        // Drain and verify order
        let drained = buffer.drain(connection_id);
        prop_assert_eq!(drained.len(), messages.len());

        for (i, (original, replayed)) in messages.iter().zip(drained.iter()).enumerate() {
            prop_assert_eq!(
                original.message_id, replayed.message_id,
                "Message {} should match", i
            );
            prop_assert_eq!(
                &original.data, &replayed.data,
                "Message {} data should match", i
            );
        }

        // Buffer should be empty after drain
        prop_assert_eq!(buffer.size(connection_id), 0);
    }

    /// **Feature: production-readiness, Property 24: Presence Accuracy**
    /// *For any* channel, the presence list SHALL contain exactly the user IDs
    /// of currently connected clients subscribed to that channel.
    /// **Validates: Requirements 9.4**
    #[test]
    fn prop_presence_accuracy(
        user_ids in prop::collection::hash_set(user_id_strategy(), 1..10),
        channel_id in channel_id_strategy(),
    ) {
        let tracker = PresenceTracker::new();

        // Add all users
        for user_id in &user_ids {
            tracker.add(channel_id, user_id.clone());
        }

        // Verify presence matches
        let presence: HashSet<String> = tracker.get(channel_id).into_iter().collect();
        prop_assert_eq!(presence, user_ids.clone());

        // Remove half the users
        let to_remove: Vec<_> = user_ids.iter().take(user_ids.len() / 2).cloned().collect();
        for user_id in &to_remove {
            tracker.remove(channel_id, user_id);
        }

        // Verify updated presence
        let remaining: HashSet<String> = user_ids.difference(&to_remove.into_iter().collect()).cloned().collect();
        let presence: HashSet<String> = tracker.get(channel_id).into_iter().collect();
        prop_assert_eq!(presence, remaining);
    }

    /// **Feature: production-readiness, Property 25: Acknowledgment Guarantee**
    /// *For any* message with acknowledgment enabled, the sender SHALL receive an ack
    /// if and only if the message was successfully delivered to at least one subscriber.
    /// **Validates: Requirements 9.5**
    #[test]
    fn prop_acknowledgment_guarantee(
        connection_ids in prop::collection::vec(1u64..1000u64, 1..5),
        message_id in message_id_strategy(),
    ) {
        let tracker = AckTracker::new(3, std::time::Duration::from_secs(5));
        let unique_ids: Vec<_> = connection_ids.into_iter().collect::<HashSet<_>>().into_iter().collect();

        // Track message
        tracker.track(message_id, unique_ids.clone());
        prop_assert!(!tracker.is_complete(message_id), "Should not be complete initially");

        // Acknowledge from all connections
        for conn_id in &unique_ids {
            tracker.ack(message_id, *conn_id);
        }

        // Should be complete after all acks
        prop_assert!(tracker.is_complete(message_id), "Should be complete after all acks");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that buffer respects max size and evicts oldest messages
    #[test]
    fn prop_buffer_max_size(
        messages in prop::collection::vec(binary_message_strategy(), 10..20),
    ) {
        let max_size = 5;
        let buffer = MessageBuffer::new(max_size);
        let connection_id = 1;

        for msg in &messages {
            buffer.push(connection_id, msg.clone()).unwrap();
        }

        // Buffer should not exceed max size
        prop_assert!(buffer.size(connection_id) <= max_size);

        // Should contain the most recent messages
        let drained = buffer.drain(connection_id);
        let expected_start = messages.len().saturating_sub(max_size);
        let expected: Vec<_> = messages[expected_start..].to_vec();

        prop_assert_eq!(drained.len(), expected.len());
        for (d, e) in drained.iter().zip(expected.iter()) {
            prop_assert_eq!(d.message_id, e.message_id);
        }
    }

    /// Test that presence tracker correctly handles multiple channels
    #[test]
    fn prop_presence_multi_channel(
        user_id in user_id_strategy(),
        channels in prop::collection::vec(channel_id_strategy(), 2..5),
    ) {
        let tracker = PresenceTracker::new();
        let unique_channels: Vec<_> = channels.into_iter().collect::<HashSet<_>>().into_iter().collect();

        // Add user to all channels
        for &channel_id in &unique_channels {
            tracker.add(channel_id, user_id.clone());
        }

        // Verify user is present in all channels
        for &channel_id in &unique_channels {
            prop_assert!(tracker.is_present(channel_id, &user_id));
        }

        // Remove from first channel
        if let Some(&first_channel) = unique_channels.first() {
            tracker.remove(first_channel, &user_id);
            prop_assert!(!tracker.is_present(first_channel, &user_id));

            // Should still be present in other channels
            for &channel_id in unique_channels.iter().skip(1) {
                prop_assert!(tracker.is_present(channel_id, &user_id));
            }
        }
    }

    /// Test WebSocket connection state transitions
    #[test]
    fn prop_connection_state_transitions(
        user_id in user_id_strategy(),
    ) {
        let manager = WebSocketManager::default();

        // Connect
        let (conn_id, _rx) = manager.handle_connection(user_id.clone());
        prop_assert_eq!(manager.active_connection_count(), 1);

        // Disconnect
        manager.handle_disconnect(conn_id);
        prop_assert_eq!(manager.active_connection_count(), 0);

        // Reconnect
        let result = manager.handle_reconnect(conn_id);
        prop_assert!(result.is_ok());
        prop_assert_eq!(manager.active_connection_count(), 1);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_pubsub_no_subscribers() {
        let manager = WebSocketManager::default();
        let message = BinaryMessage {
            message_id: 1,
            channel_id: 999,
            data: vec![1, 2, 3],
            timestamp: 100,
        };

        // Should fail when no subscribers
        let result = manager.publish(999, message);
        assert!(result.is_err());
    }

    #[test]
    fn test_ack_partial_delivery() {
        let tracker = AckTracker::new(3, std::time::Duration::from_secs(5));

        tracker.track(1, vec![100, 101, 102]);

        // Only ack from some connections
        tracker.ack(1, 100);
        tracker.ack(1, 101);

        assert!(!tracker.is_complete(1));
        assert_eq!(tracker.pending(1), vec![102]);
    }

    #[test]
    fn test_buffer_empty_drain() {
        let buffer = MessageBuffer::new(10);
        let drained = buffer.drain(999);
        assert!(drained.is_empty());
    }

    #[test]
    fn test_presence_empty_channel() {
        let tracker = PresenceTracker::new();
        let presence = tracker.get(999);
        assert!(presence.is_empty());
        assert!(!tracker.is_present(999, "user1"));
    }
}
